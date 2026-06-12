use std::sync::{Arc, atomic::AtomicUsize};
use std::path::Path;
use tauri::ipc::Channel;
use tokio::sync::{oneshot, Notify};
use std::process::Command;
use tauri::Manager;

use crate::models::{DownloadState, Download, UrlInfo, ActiveTask};
use crate::utils::{range_for, parse_filename, extension_from_mime};
use crate::state::broadcast;
use crate::downloader::worker;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn open_file_dir(path: String) -> Result<(), String> {
    let path = Path::new(&path);
    
    if !path.exists() {
        return Err("Path does not exist".into());
    }

    let dir = if path.is_dir() {
        path
    } else {
        path.parent().ok_or("Could not find parent directory")?
    };

    #[cfg(target_os = "windows")]
    {
        if path.is_file() {
            Command::new("explorer")
                .arg("/select,")
                .arg(path)
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            Command::new("explorer")
                .arg(dir)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn resume_download(
    state: tauri::State<'_, Arc<DownloadState>>,
    id: String,
) -> Result<(), String> {
    let state = (*state).clone();

    let download = {
        let list = state.list.read().unwrap();
        list.iter().find(|d| d.id == id).cloned()
    };
    let download = match download {
        Some(d) => d,
        None => return Err("Download not found".into()),
    };

    if state.active_tasks.read().unwrap().contains_key(&id) {
        return Ok(());
    }

    let temp_path = Path::new(&download.location).join(format!("{}.jdm", download.name));

    if let Some(parent) = temp_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true);
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(3);
    }

    let file = options.open(&temp_path)
        .map_err(|e| format!("Disk error: {}", e))?;

    if download.total > 0 {
        let current_metadata = file.metadata().map_err(|e| e.to_string())?;
        if current_metadata.len() < download.total {
            file.set_len(download.total).map_err(|e| format!("Allocation failed: {}", e))?;
        }
    }
    
    drop(file); // Ensure it's closed before workers open it

    let final_path = Path::new(&download.location).join(&download.name);
    
    if download.total > 0 && download.parts.iter().all(|p| p.downloaded >= (p.end - p.start + 1)) {
        if temp_path.exists() {
            let _ = tokio::fs::rename(&temp_path, &final_path).await;
        }
        return Ok(());
    }

    let workers_to_spawn = if !download.resume {
        1
    } else if download.total > 0 {
        if download.speeds.len() > 0 { download.speeds.len() } else { 8 }
    } else {
        1
    };

    {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == id) {
            d.state = "Connecting".into(); 
            
            d.speeds = vec![0; workers_to_spawn];
            d.speed = 0;
            
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            if d.elapsed > now {
                d.elapsed = 0;
            }
            d.start_time = Some(now.saturating_sub(d.elapsed));
        }
    }
    broadcast(&state);

    let notify = Arc::new(Notify::new());
    let retry_notify = Arc::new(Notify::new());
    let active_count = Arc::new(AtomicUsize::new(workers_to_spawn));
    let mut handles = Vec::new();
    
    // Initialize active parts tracking
    {
        let mut tasks = state.active_tasks.write().unwrap();
        tasks.insert(id.clone(), ActiveTask { 
            handles: Vec::new(), 
            notify: notify.clone(),
            active_parts: Arc::new(std::sync::RwLock::new(Vec::new())),
        });
    }

    for i in 0..workers_to_spawn {
        let state_clone = state.clone();
        let notify_clone = notify.clone();
        let retry_notify_clone = retry_notify.clone();
        let client_clone = state.client.clone();
        let temp_path_clone = temp_path.clone();
        let final_path_clone = final_path.clone();
        let id_clone = id.clone();
        let url_clone = download.link.clone();
        let count_clone = active_count.clone();

        let handle = tauri::async_runtime::spawn(async move {
            worker(
                i,
                id_clone,
                url_clone,
                temp_path_clone,
                final_path_clone,
                client_clone,
                state_clone,
                notify_clone,
                retry_notify_clone,
                count_clone,
            )
            .await;
        });
        handles.push(handle);
    }

    if let Some(task) = state.active_tasks.write().unwrap().get_mut(&id) {
        task.handles = handles;
    }

    Ok(())
}

#[tauri::command]
pub fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn paste() -> Result<String, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.get_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pick_folder(app: tauri::AppHandle<tauri::Wry>) -> String {
    let (tx, rx) = oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });

    match rx.await {
        Ok(Some(path)) => path.to_string(),
        _ => String::new(),
    }
}

#[tauri::command]
pub fn pause_download(state: tauri::State<'_, Arc<DownloadState>>, id: String) -> Result<(), String> {
    if let Some(task) = state.active_tasks.write().unwrap().remove(&id) {
        task.notify.notify_waiters();
        for h in task.handles {
            h.abort();
        }
    }

    {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == id) {
            d.state = "Paused".into();
            d.speed = 0;
            for speed in d.speeds.iter_mut() {
                *speed = 0;
            }
        }
    }
    broadcast(&state);
    Ok(())
}

#[tauri::command]
pub fn change_link(state: tauri::State<'_, Arc<DownloadState>>, id: String, url: String) -> Result<(), String> {
    if let Some(task) = state.active_tasks.write().unwrap().remove(&id) {
        task.notify.notify_waiters();
        for h in task.handles {
            h.abort();
        }
    }

    {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == id) {
            d.state = "Paused".into();
            d.speed = 0;
            d.link = url;
            for speed in d.speeds.iter_mut() {
                *speed = 0;
            }
        }
    }
    broadcast(&state);
    Ok(())
}

#[tauri::command]
pub fn cancel_download(state: tauri::State<'_, Arc<DownloadState>>, id: String) -> Result<(), String> {
    if let Some(task) = state.active_tasks.write().unwrap().remove(&id) {
        task.notify.notify_waiters();
        for h in task.handles {
            h.abort();
        }
    }

    let file_path = {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == id){
            d.downloaded = 0;
            d.state = String::from("Cancelled");
            d.speed = 0;
            for c in d.connections.iter_mut(){
                *c = 0;
            }
            if !d.parts.is_empty() {
                d.parts = vec![crate::models::Part {
                    start: 0,
                    end: d.total.saturating_sub(1),
                    downloaded: 0,
                }];
            }
            for speed in d.speeds.iter_mut(){
                *speed = 0;
            }
            Some(Path::new(&d.location).join(format!("{}.jdm", d.name)))
        } else {
            None
        }
    };

    if let Some(path) = file_path {
        let _ = std::fs::remove_file(path);
    }

    broadcast(&state);
    Ok(())
}

#[tauri::command]
pub async fn verify_url(
    state: tauri::State<'_, Arc<DownloadState>>,
    url: String,
    cookies: Option<String>,
    user_agent: Option<String>,
    referer: Option<String>,
) -> Result<Option<UrlInfo>, String> {
    let state = (*state).clone();
    
    let client = &state.client; 
    let parsed = url::Url::parse(&url).map_err(|e| e.to_string())?;

    let mut head_req = client.head(parsed.to_string());
    if let Some(c) = &cookies { head_req = head_req.header(reqwest::header::COOKIE, c); }
    if let Some(ref_str) = &referer { head_req = head_req.header(reqwest::header::REFERER, ref_str); }
    let head_resp = head_req.send().await;

    let mut size: Option<u64> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut final_url = parsed.clone();

    if let Ok(resp) = head_resp {
        if resp.status().is_success() {
            final_url = url::Url::parse(&resp.url().to_string()).unwrap_or(final_url);
            let headers = resp.headers();

            size = headers
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            content_type = headers
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            if let Some(header) = headers.get(reqwest::header::CONTENT_DISPOSITION) {
                if let Ok(header_str) = header.to_str() {
                    filename = parse_filename(header_str);
                }
            }
        }
    }

    let mut probe_req = client.get(parsed.to_string()).header(reqwest::header::RANGE, "bytes=0-");
    if let Some(c) = &cookies { probe_req = probe_req.header(reqwest::header::COOKIE, c); }
    if let Some(ua) = &user_agent {
        probe_req = probe_req.header(reqwest::header::USER_AGENT, ua);
        
        let mut cv = "120";
        if let Some(idx) = ua.find("Chrome/") {
            let rest = &ua[idx + 7..];
            if let Some(end) = rest.find('.') {
                cv = &rest[..end];
            }
        }
        probe_req = probe_req.header("sec-ch-ua", format!("\"Google Chrome\";v=\"{0}\", \"Chromium\";v=\"{0}\", \"Not?A_Brand\";v=\"24\"", cv));
        probe_req = probe_req.header("sec-ch-ua-mobile", "?0");
        probe_req = probe_req.header("sec-ch-ua-platform", "\"Windows\"");
    }
    if let Some(ref_str) = &referer { probe_req = probe_req.header(reqwest::header::REFERER, ref_str); }
    let probe = probe_req.send().await;

    let resume_supported = match &probe {
        Ok(r) => r.status() == reqwest::StatusCode::PARTIAL_CONTENT,
        Err(_) => false,
    };

    if size.is_none() {
        if let Ok(resp) = &probe {
            if let Some(cr) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
                // "bytes 0-12344/12345" -> 12345
                if let Ok(s) = cr.to_str() {
                    if let Some(total) = s.rsplit('/').next() {
                         size = total.parse::<u64>().ok();
                    }
                }
            } else if let Some(cl) = resp.headers().get(reqwest::header::CONTENT_LENGTH) {
                if let Ok(s) = cl.to_str() {
                    size = s.parse::<u64>().ok();
                }
            }
            if filename.is_none() {
                 if let Some(header) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
                    if let Ok(header_str) = header.to_str() {
                        filename = parse_filename(header_str);
                    }
                }
            }
        }
    }

    let final_name = filename.unwrap_or_else(|| {
        let path_name = final_url
            .path_segments()
            .and_then(|s| s.last())
            .filter(|s| !s.is_empty())
            .unwrap_or("download")
            .to_string();

        if !path_name.contains('.') {
            if let Some(ref ct) = content_type {
                let ext = extension_from_mime(ct);
                if !ext.is_empty() {
                    return format!("{}{}", path_name, ext);
                }
            }
        }
        path_name
    });

    Ok(Some(UrlInfo {
        filename: final_name,
        resume_supported,
        size,
    }))
}

#[tauri::command]
pub fn add_download(app: tauri::AppHandle, state: tauri::State<Arc<DownloadState>>, mut download: Download) -> Result<(), String> {
    let loc_path = Path::new(&download.location);
    if loc_path.is_relative() {
        if download.location == "Downloads" {
            if let Ok(dir) = app.path().download_dir() {
                download.location = dir.to_string_lossy().into_owned();
            }
        } else {
            if let Ok(dir) = app.path().download_dir() {
                download.location = dir.join(&download.location).to_string_lossy().into_owned();
            }
        }
    }

    if download.resume && download.total > 0 {
        let path = Path::new(&download.location).join(format!("{}.jdm", download.name));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut options = std::fs::OpenOptions::new();
        options.create(true).write(true);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(3);
        }

        let file = options.open(&path)
            .map_err(|e| format!("IO Error: {}", e))?;

        if download.total > 0 {
            let current_metadata = file.metadata().map_err(|e| e.to_string())?;
            if current_metadata.len() < download.total {
                file.set_len(download.total).map_err(|e| format!("Allocation failed: {}", e))?;
            }
        }
    }

    if download.parts.is_empty() {
        download.parts = vec![crate::models::Part {
            start: 0,
            end: if download.total > 0 { download.total.saturating_sub(1) } else { 0 },
            downloaded: 0,
        }];
    }

    let mut list = state.list.write().unwrap();
    list.push(download);
    drop(list);

    broadcast(&state);
    Ok(())
}

#[tauri::command]
pub fn stream_downloads(state: tauri::State<Arc<DownloadState>>, channel: Channel<Vec<Download>>) {
    *state.sender.write().unwrap() = Some(channel.clone());
    let snapshot = state.list.read().unwrap().clone();
    let _ = channel.send(snapshot);
}

#[tauri::command]
pub fn toggle_select(state: tauri::State<Arc<DownloadState>>, tab: String) {
    let mut list = state.list.write().unwrap();

    let should_select = !list
        .iter()
        .filter(|d| tab == "All" || d.state == tab)
        .any(|d| d.is_selected);

    for d in list.iter_mut() {
        if tab == "All" || d.state == tab {
            d.is_selected = should_select;
        }
    }
    drop(list);
    broadcast(&state);
}

#[tauri::command]
pub fn toggle_one(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        d.is_selected = !d.is_selected;
    }
    drop(list);
    broadcast(&state);
}

#[tauri::command]
pub fn select_one(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();
    list.iter_mut().for_each(|d| {
        if d.id == id {
            d.is_selected = !d.is_selected;
        }
        else {
            d.is_selected = false;
        };
    });
    drop(list);
    broadcast(&state);
}

#[tauri::command]
pub fn delete_selected(state: tauri::State<Arc<DownloadState>>, delete_file: bool) {
    let mut list = state.list.write().unwrap();

    if delete_file {
        for d in list.iter().filter(|d| d.is_selected) {
            let temp_path = std::path::Path::new(&d.location).join(format!("{}.jdm", d.name));
            let final_path = std::path::Path::new(&d.location).join(&d.name);

            let _ = std::fs::remove_file(temp_path);
            let _ = std::fs::remove_file(final_path);
        }
    }

    list.retain(|d| !d.is_selected);

    drop(list);
    broadcast(&state);
}

#[tauri::command]
pub fn delete_download(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();

    if let Some(d) = list.iter().find(|d| d.id == id) {

        let temp_path = std::path::Path::new(&d.location).join(format!("{}.jdm", d.name));
        let final_path = std::path::Path::new(&d.location).join(&d.name);

        let _ = std::fs::remove_file(temp_path);
        let _ = std::fs::remove_file(final_path);
    }

    list.retain(|d| d.id != id);

    drop(list);
    broadcast(&state);
}

#[tauri::command]
pub fn remove_download(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();

    list.retain(|d| d.id != id);

    drop(list);
    broadcast(&state);
}
