use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use reqwest::Client;
use tokio::sync::Notify;
use tokio::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, AsyncSeekExt, BufWriter};

use crate::models::{DownloadState, ActivePart};
use crate::state::broadcast;

fn get_next_part(
    state: &Arc<DownloadState>,
    download_id: &str,
    active_parts_lock: &Arc<RwLock<Vec<ActivePart>>>,
) -> Option<(usize, u64, Arc<AtomicU64>, Arc<AtomicU64>, u64)> {
    let mut list = state.list.write().unwrap();
    let d = list.iter_mut().find(|d| d.id == download_id)?;

    if d.total == 0 {
        if d.parts.is_empty() {
            d.parts.push(crate::models::Part { start: 0, end: 0, downloaded: d.downloaded });
        }
        let mut active = active_parts_lock.write().unwrap();
        if active.is_empty() {
            let current_pos = Arc::new(AtomicU64::new(d.parts[0].downloaded));
            let end_atomic = Arc::new(AtomicU64::new(0));
            active.push(ActivePart { part_index: 0, start: 0, current_pos: current_pos.clone(), end: end_atomic.clone() });
            return Some((0, 0, current_pos, end_atomic, d.parts[0].downloaded));
        }
        return None;
    }
    let min_split_size = if d.total > 0 {
        std::cmp::max(1024 * 1024 * 5, d.total / 32) // at least 5MB, or total / 32
    } else {
        1024 * 1024 * 10 // 10MB fallback
    };

    // 1. Find the largest UNASSIGNED incomplete part
    let mut best_unassigned = None;
    let mut max_unassigned_size = 0;

    for (i, p) in d.parts.iter().enumerate() {
        if p.start == u64::MAX { continue; }
        if p.downloaded < (p.end.saturating_sub(p.start) + 1) {
            let active = active_parts_lock.read().unwrap();
            if !active.iter().any(|ap| ap.part_index == i) {
                let size = (p.end.saturating_sub(p.start) + 1).saturating_sub(p.downloaded);
                if size > max_unassigned_size {
                    max_unassigned_size = size;
                    best_unassigned = Some(i);
                }
            }
        }
    }

    if let Some(i) = best_unassigned {
        // DO NOT split unassigned parts! Just take the whole unassigned part.
        // Splitting unassigned parts causes dying workers to aggressively fragment the file.

        // Assign part `i`
        let p = &d.parts[i];
        let mut active = active_parts_lock.write().unwrap();
        let current_pos = Arc::new(AtomicU64::new(p.start + p.downloaded));
        let end_atomic = Arc::new(AtomicU64::new(p.end));
        active.push(ActivePart { part_index: i, start: p.start, current_pos: current_pos.clone(), end: end_atomic.clone() });
        return Some((i, p.start, current_pos, end_atomic, p.downloaded));
    }

    // 2. No unassigned parts available. As a last resort, dynamically split an ACTIVE part!
    // This will cause a connection abort for the victim worker, but keeps all workers downloading in parallel.
    let mut active = active_parts_lock.write().unwrap();
    let mut max_remaining = 0;
    let mut best_active_idx = None;
    let mut best_mid = 0;

    for (active_idx, ap) in active.iter().enumerate() {
        let current_end = ap.end.load(Ordering::SeqCst);
        let current_pos = ap.current_pos.load(Ordering::SeqCst);
        
        if current_end > current_pos {
            let remaining = current_end - current_pos;
            if remaining > min_split_size * 2 && remaining > max_remaining { 
                max_remaining = remaining;
                best_active_idx = Some(active_idx);
                best_mid = current_pos + remaining / 2;
            }
        }
    }

    if let Some(idx) = best_active_idx {
        let old_end;
        let part_idx;
        {
            let ap = &active[idx];
            old_end = ap.end.load(Ordering::SeqCst);
            ap.end.store(best_mid - 1, Ordering::SeqCst);
            part_idx = ap.part_index;
        }

        d.parts[part_idx].end = best_mid - 1;

        let new_part = crate::models::Part {
            start: best_mid,
            end: old_end,
            downloaded: 0,
        };
        d.parts.push(new_part);
        
        let new_part_index = d.parts.len() - 1;
        let current_pos = Arc::new(AtomicU64::new(best_mid));
        let new_end_atomic = Arc::new(AtomicU64::new(old_end));
        
        active.push(ActivePart {
            part_index: new_part_index,
            start: best_mid,
            current_pos: current_pos.clone(),
            end: new_end_atomic.clone(),
        });

        return Some((new_part_index, best_mid, current_pos, new_end_atomic, 0));
    }

    None
}

pub async fn worker(
    worker_index: usize,
    download_id: String,
    mut url: String,
    temp_path: PathBuf,
    final_path: PathBuf,
    client: Client,
    state: Arc<DownloadState>,
    notify: Arc<Notify>,
    retry_notify: Arc<Notify>,
    active_count: Arc<AtomicUsize>,
) {
    use futures_util::StreamExt;

    let active_parts_lock = {
        let tasks = state.active_tasks.read().unwrap();
        if let Some(t) = tasks.get(&download_id) {
            t.active_parts.clone()
        } else {
            active_count.fetch_sub(1, Ordering::SeqCst);
            return;
        }
    };

    // Stagger initial connections to avoid burst rate limits from servers
    if worker_index > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(150 * worker_index as u64)).await;
    }

    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true);
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(3);
    }

    let file = match options.open(&temp_path) {
        Ok(f) => tokio::fs::File::from_std(f),
        Err(_) => { 
            active_count.fetch_sub(1, Ordering::SeqCst);
            return; 
        }
    };

    let mut writer = BufWriter::with_capacity(4 * 1024 * 1024, file);

    let session_start = Instant::now();
    let mut previously_accumulated = 0;
    if let Ok(list) = state.list.read() {
        if let Some(d) = list.iter().find(|d| d.id == download_id) {
            previously_accumulated = d.elapsed;
        }
    }

    let mut has_started_transfer = false;
    let mut chunks_completed = 0;

    'part_loop: loop {
        let part_info = get_next_part(&state, &download_id, &active_parts_lock);
        if part_info.is_none() {
            break 'part_loop; // No more parts available
        }
        let (part_index, start_byte, current_pos_atomic, end_atomic, current_offset) = part_info.unwrap();
        
        let mut pos = start_byte + current_offset;
        let mut retries = 0;
        const MAX_RETRIES: u32 = 100;
        let (cookies, user_agent, referer) = {
            let list = state.list.read().unwrap();
            if let Some(d) = list.iter().find(|d| d.id == download_id) {
                (d.cookies.clone(), d.user_agent.clone(), d.referer.clone())
            } else {
                (None, None, None)
            }
        };

        let mut success = false;
        let mut unsynced_bytes: u64 = 0;
        'retry_loop: loop {
            success = false;
            if retries > 0 {
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {}
                    _ = notify.notified() => { break 'part_loop; }
                }
            }

            let end_byte = end_atomic.load(Ordering::Relaxed);
            
            let range_header = if end_byte > 0 {
                format!("bytes={}-{}", pos, end_byte)
            } else {
                format!("bytes={}-", pos)
            };

            let mut req = client.get(&url).header("Range", &range_header);
            if let Some(c) = &cookies { req = req.header(reqwest::header::COOKIE, c); }
            if let Some(ua) = &user_agent {
                req = req.header(reqwest::header::USER_AGENT, ua);
                
                // Extract Chrome version to match sec-ch-ua
                let mut cv = "120";
                if let Some(idx) = ua.find("Chrome/") {
                    let rest = &ua[idx + 7..];
                    if let Some(end) = rest.find('.') {
                        cv = &rest[..end];
                    }
                }
                
                req = req.header("sec-ch-ua", format!("\"Google Chrome\";v=\"{0}\", \"Chromium\";v=\"{0}\", \"Not?A_Brand\";v=\"24\"", cv));
                req = req.header("sec-ch-ua-mobile", "?0");
                req = req.header("sec-ch-ua-platform", "\"Windows\"");
            }
            if let Some(ref_str) = &referer { 
                if let Ok(val) = reqwest::header::HeaderValue::from_str(ref_str) {
                    req = req.header(reqwest::header::REFERER, val); 
                }
            }
            

            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    println!("Worker {} req.send error: {}", worker_index, e);
                    if chunks_completed == 0 && !has_started_transfer {
                        active_parts_lock.write().unwrap().retain(|ap| ap.part_index != part_index);
                        if let Ok(mut list) = state.list.write() {
                            if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                                if d.speeds.len() > worker_index { d.speeds[worker_index] = 0; }
                                d.speed = d.speeds.iter().sum();
                                if part_index < d.parts.len() {
                                    let p_start = d.parts[part_index].start;
                                    let p_end = d.parts[part_index].end;
                                    if d.parts[part_index].downloaded == 0 {
                                        if let Some(prev_idx) = d.parts.iter().position(|p| p.end == p_start.saturating_sub(1)) {
                                            d.parts[prev_idx].end = p_end;
                                            let active = active_parts_lock.read().unwrap();
                                            if let Some(active_prev) = active.iter().find(|ap| ap.part_index == prev_idx) {
                                                active_prev.end.store(p_end, Ordering::Relaxed);
                                            }
                                        }
                                        d.parts[part_index].start = u64::MAX;
                                        d.parts[part_index].end = u64::MAX;
                                        d.parts[part_index].downloaded = 0;
                                    }
                                }
                            }
                        }
                        break 'part_loop;
                    } else {
                        if retries < MAX_RETRIES { retries += 1; continue 'retry_loop; }
                        break 'retry_loop;
                    }
                }
            };

            if resp.status().is_redirection() {
                if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                    if let Ok(loc_str) = loc.to_str() {
                        if let Ok(new_url) = url::Url::parse(loc_str) {
                            url = new_url.to_string();
                            if retries < MAX_RETRIES { retries += 1; continue 'retry_loop; }
                        } else if let Ok(base_url) = url::Url::parse(&url) {
                            if let Ok(new_url) = base_url.join(loc_str) {
                                url = new_url.to_string();
                                if retries < MAX_RETRIES { retries += 1; continue 'retry_loop; }
                            }
                        }
                    }
                }
            }

            let is_partial = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
            let status = resp.status();
            if !status.is_success() && !is_partial {
                println!("Worker {} failed with status: {}", worker_index, status);
                if status == reqwest::StatusCode::FORBIDDEN {
                    println!("--- 403 Response Headers ---");
                    for (k, v) in resp.headers().iter() {
                        println!("{}: {:?}", k, v);
                    }
                    if let Ok(body) = resp.text().await {
                        println!("--- 403 Response Body ---");
                        println!("{}", &body[..std::cmp::min(500, body.len())]);
                    }
                } else {
                    let _ = resp.bytes().await; // Consume body
                }
                active_parts_lock.write().unwrap().retain(|ap| ap.part_index != part_index); // Release part
                
                if chunks_completed == 0 && !has_started_transfer {
                    if let Ok(mut list) = state.list.write() {
                        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                            if d.speeds.len() > worker_index { d.speeds[worker_index] = 0; }
                            d.speed = d.speeds.iter().sum();
                            
                            // MERGE logic for dynamic chunking:
                            // If an excess worker dies, recombine the part so the UI doesn't show 8 parts for 2 active workers!
                            if part_index < d.parts.len() {
                                let p_start = d.parts[part_index].start;
                                let p_end = d.parts[part_index].end;
                                
                                if d.parts[part_index].downloaded == 0 {
                                    if let Some(prev_idx) = d.parts.iter().position(|p| p.end == p_start.saturating_sub(1)) {
                                        d.parts[prev_idx].end = p_end;
                                        // Update active part if the adjacent part is running
                                        let active = active_parts_lock.read().unwrap();
                                        if let Some(active_prev) = active.iter().find(|ap| ap.part_index == prev_idx) {
                                            active_prev.end.store(p_end, Ordering::Relaxed);
                                        }
                                    }
                                    
                                    // Mark as dead (1 to 0) so is_actually_done ignores it and UI hides it
                                    d.parts[part_index].start = u64::MAX;
                                    d.parts[part_index].end = u64::MAX;
                                    d.parts[part_index].downloaded = 0;
                                }
                            }
                        }
                    }
                    break 'part_loop; // Excess worker dies permanently
                } else {
                    if retries < MAX_RETRIES {
                        retries += 1;
                        continue 'retry_loop; // Successful worker retries
                    }
                    break 'retry_loop;
                }
            }

            if !is_partial && pos > 0 {
                // Server ignored range header (returned 200 OK instead of 206).
                // For testfile.org and Cloudflare, this is often a temporary HTML challenge page.
                // Instead of aborting the entire download, we should retry!
                let _ = resp.bytes().await;
                if retries < MAX_RETRIES {
                    retries += 1;
                    continue 'retry_loop;
                }
                break 'retry_loop;
            }

            if pos > 0 && writer.seek(tokio::io::SeekFrom::Start(pos)).await.is_err() {
                break 'retry_loop;
            }

            let mut stream = resp.bytes_stream();
            let mut last_update = Instant::now();
            let mut bytes_since_last_update: u64 = 0;

            'stream_loop: loop {
                let end_byte = end_atomic.load(Ordering::Relaxed);
                tokio::select! {
                    _ = notify.notified() => {
                        let _ = writer.flush().await;
                        break 'part_loop; 
                    }
                    res = stream.next() => {
                        match res {
                            Some(Ok(chunk)) => {
                                let mut len = chunk.len() as u64;
                                
                                // Prevent overwriting if we hit dynamic end_byte
                                let mut over_read = false;
                                if end_byte > 0 && pos + len - 1 > end_byte {
                                    len = end_byte + 1 - pos;
                                    over_read = true;
                                }

                                if writer.write_all(&chunk[..len as usize]).await.is_err() { 
                                    success = false;
                                    break 'retry_loop; 
                                }

                                pos += len;
                                current_pos_atomic.store(pos, Ordering::Relaxed);
                                bytes_since_last_update += len;
                                unsynced_bytes += len;

                                if over_read {
                                    success = true;
                                    break 'retry_loop;
                                }

                                if !has_started_transfer {
                                    if let Ok(mut list) = state.list.try_write() {
                                        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                                            if d.state == "Connecting" {
                                                d.state = "Downloading".into();
                                            }
                                        }
                                        has_started_transfer = true;
                                        drop(list);
                                        broadcast(&state);
                                    }
                                }

                                if last_update.elapsed().as_millis() >= 500 {
                                    if let Ok(mut list) = state.list.try_write() {
                                        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                                            if part_index < d.parts.len() {
                                                d.parts[part_index].downloaded += unsynced_bytes;
                                                d.downloaded = d.parts.iter().map(|p| p.downloaded).sum();
                                                let speed = (bytes_since_last_update as f64 / last_update.elapsed().as_secs_f64()) as u64;
                                                if d.speeds.len() > worker_index { d.speeds[worker_index] = speed; }
                                                d.speed = d.speeds.iter().sum();
                                            }
                                        }
                                        unsynced_bytes = 0;
                                        bytes_since_last_update = 0;
                                        last_update = Instant::now();
                                        drop(list); 
                                        broadcast(&state);
                                    }
                                } else if over_read || (end_byte > 0 && pos > end_byte) {
                                    if let Ok(mut list) = state.list.try_write() {
                                        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                                            if part_index < d.parts.len() {
                                                d.parts[part_index].downloaded += unsynced_bytes;
                                                d.downloaded = d.parts.iter().map(|p| p.downloaded).sum();
                                            }
                                        }
                                        unsynced_bytes = 0;
                                    }
                                }
                                
                                if end_byte > 0 && pos > end_byte {
                                    if over_read {
                                        success = true;
                                        break 'retry_loop;
                                    }
                                    // If we reached end_byte naturally (no over_read), we continue loop!
                                    // The next stream.next() will return None cleanly and reqwest will reuse the socket!
                                }
                            }
                            None => { 
                                if end_byte > 0 && pos <= end_byte {
                                    if retries < MAX_RETRIES {
                                        retries += 1;
                                        break 'stream_loop;
                                    }
                                    success = false;
                                } else {
                                    success = true;
                                }
                                break 'retry_loop; 
                            }
                            _ => { 
                                if retries < MAX_RETRIES { retries += 1; break 'stream_loop; }
                                break 'retry_loop; 
                            }
                        }
                    }
                }
            }
        }

        // Clean up active part
        active_parts_lock.write().unwrap().retain(|ap| ap.part_index != part_index);

        if let Ok(mut list) = state.list.write() {
            if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                if d.speeds.len() > worker_index {
                    d.speeds[worker_index] = 0;
                }
                d.speed = d.speeds.iter().sum();
            }
            drop(list);
            broadcast(&state);
        }

        if unsynced_bytes > 0 {
            if let Ok(mut list) = state.list.write() {
                if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                    if part_index < d.parts.len() {
                        d.parts[part_index].downloaded += unsynced_bytes;
                        d.downloaded = d.parts.iter().map(|p| p.downloaded).sum();
                    }
                }
            }
        }

        if !success {
            let mut list = state.list.write().unwrap();
            if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
                if d.state == "Downloading" || d.state == "Connecting" {
                    d.state = "Failed".into();
                    if let Some(task) = state.active_tasks.read().unwrap().get(&download_id) {
                        for h in &task.handles {
                            h.abort();
                        }
                    }
                }
            }
            drop(list);
            broadcast(&state);
            break 'part_loop;
        }

        chunks_completed += 1;
        retry_notify.notify_one();
    }

    let _ = writer.flush().await;
    drop(writer);
    let prev_count = active_count.fetch_sub(1, Ordering::SeqCst);

    if prev_count == 1 {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
            d.speed = 0;
            for speed in d.speeds.iter_mut() {
                *speed = 0;
            }

            if d.state != "Failed" && d.state != "Paused" && d.state != "Cancelled" {
                let is_actually_done = if d.total > 0 {
                    d.downloaded >= d.total
                } else {
                    true // unsized download success
                };

                if is_actually_done {
                    d.state = "Completed".into();
                    if temp_path.exists() {
                        let _ = std::fs::rename(&temp_path, &final_path);
                    }
                } else if d.state == "Downloading" || d.state == "Connecting" {
                    d.state = "Failed".into();
                }
            }
        }
        drop(list);
        broadcast(&state);
    }
}
