use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, Emitter
};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{oneshot, Notify};
use fs2::FileExt;
use reqwest::{Client};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize};
use std::sync::{Arc, RwLock};
use std::time::{Duration};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tauri_plugin_deep_link::DeepLinkExt;

#[derive(serde::Serialize)]
struct UrlInfo {
    filename: String,
    resume_supported: bool,
    size: Option<u64>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
struct Download {
    id: String,
    link: String,
    name: String,
    location: String,
    resume: bool,
    downloaded: u64,
    total: u64,
    start_time: Option<u64>,
    elapsed: u64,

    speed: u64,
    state: String,
    is_selected: bool,

    connections: Vec<u64>,

    #[serde(default)]
    speeds: Vec<u64>,
}

struct ActiveTask {
    handles: Vec<JoinHandle<()>>,
    notify: Arc<Notify>,
}

struct DownloadState {
    list: RwLock<Vec<Download>>,
    sender: RwLock<Option<Channel<Vec<Download>>>>,
    active_tasks: RwLock<HashMap<String, ActiveTask>>,
    client: Client,
}

fn broadcast(state: &Arc<DownloadState>) {
    let sender_guard = state.sender.read().unwrap();
    if let Some(ch) = sender_guard.as_ref() {
        let list = state.list.read().unwrap().clone();
        if ch.send(list).is_err() {
            drop(sender_guard);
            *state.sender.write().unwrap() = None;
        }
    }
}

fn range_for(total: u64, parts: usize, i: usize) -> (u64, u64) {
    if total == 0 {
        return (0, 0);
    }

    let chunk = total / parts as u64;
    let start = i as u64 * chunk;
    let end = if i == parts - 1 {
        total - 1
    } else {
        start + chunk - 1
    };

    (start, end)
}

fn save_state(app_handle: &tauri::AppHandle, state: &Arc<DownloadState>) {
    if let Ok(path) = app_handle.path().app_data_dir() {
        let file_path = path.join("downloads.json");
        let list = state.list.read().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*list) {
            let _ = std::fs::write(file_path, json);
        }
    }
}

async fn worker(
    index: usize,
    download_id: String,
    url: String,
    temp_path: PathBuf,
    final_path: PathBuf,
    start_byte: u64,
    end_byte: u64,
    current_offset: u64,
    client: Client,
    state: Arc<DownloadState>,
    notify: Arc<Notify>,
    active_count: Arc<AtomicUsize>,
) {
    use futures_util::StreamExt;
    use tokio::io::{AsyncWriteExt, AsyncSeekExt, BufWriter};
    use std::sync::atomic::Ordering;
    use tokio::time::{Duration, Instant};

    let mut pos = start_byte + current_offset;
    let mut retries = 0;
    const MAX_RETRIES: u32 = 5;
    let mut success;
    let mut unsynced_bytes: u64 = 0;
    let mut has_started_transfer = false;

    let mut previously_accumulated = 0;
    if let Ok(list) = state.list.read() {
        if let Some(d) = list.iter().find(|d| d.id == download_id) {
            previously_accumulated = d.elapsed;
        }
    }
    let session_start = Instant::now();

    macro_rules! exit_worker {
        () => {
            active_count.fetch_sub(1, Ordering::SeqCst);
            return;
        };
    }

    let file = match std::fs::OpenOptions::new().write(true).open(&temp_path) {
        Ok(f) => tokio::fs::File::from_std(f),
        Err(_) => { exit_worker!(); }
    };

    let mut writer = BufWriter::with_capacity(4 * 1024 * 1024, file);

    'retry_loop: loop {
        success = false;
        if retries > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        let resp = match client.get(&url)
            .header("Range", format!("bytes={}-{}", pos, if end_byte > 0 { end_byte.to_string() } else { "".to_string() }))
            .send().await 
        {
            Ok(r) => r,
            Err(_) => {
                if retries < MAX_RETRIES { retries += 1; continue 'retry_loop; }
                break 'retry_loop;
            }
        };

        if pos > 0 && writer.seek(tokio::io::SeekFrom::Start(pos)).await.is_err() {
            break 'retry_loop;
        }

        let mut stream = resp.bytes_stream();
        let mut last_update = Instant::now();
        let mut bytes_since_last_update: u64 = 0;

        'stream_loop: loop {
            tokio::select! {
                _ = notify.notified() => {
                    let _ = writer.flush().await;
                    exit_worker!(); 
                }
                res = stream.next() => {
                    match res {
                        Some(Ok(chunk)) => {
                            let len = chunk.len() as u64;
                            if writer.write_all(&chunk).await.is_err() { break 'retry_loop; }

                            pos += len;
                            bytes_since_last_update += len;
                            unsynced_bytes += len;

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
                                        if d.connections.len() > index {
                                            d.connections[index] += unsynced_bytes;
                                            d.downloaded = d.connections.iter().sum();
                                            let speed = (bytes_since_last_update as f64 / last_update.elapsed().as_secs_f64()) as u64;
                                            if d.speeds.len() > index { d.speeds[index] = speed; }
                                            d.speed = d.speeds.iter().sum();
                                            d.elapsed = previously_accumulated + session_start.elapsed().as_secs();
                                        }
                                    }
                                    unsynced_bytes = 0;
                                    bytes_since_last_update = 0;
                                    last_update = Instant::now();
                                    drop(list); 
                                    broadcast(&state);
                                }
                            }
                        }
                        None => { success = true; break 'retry_loop; }
                        _ => { 
                            if retries < MAX_RETRIES { retries += 1; break 'stream_loop; }
                            break 'retry_loop; 
                        }
                    }
                }
            }
        }
    }

    let _ = writer.flush().await;
    let prev_count = active_count.fetch_sub(1, Ordering::SeqCst);

    {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == download_id) {
            d.connections[index] += unsynced_bytes;
            d.downloaded = d.connections.iter().sum();
            
            if index < d.speeds.len() { d.speeds[index] = 0; }
            d.speed = d.speeds.iter().sum();

            if prev_count == 1 {
                let is_actually_done = if d.total > 0 {
                    d.downloaded >= d.total
                } else {
                    success
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
    }
    broadcast(&state);
}


#[tauri::command]
async fn open_file_dir(path: String) -> Result<(), String> {
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
async fn resume_download(
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

    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true) // Creates if missing, opens if exists
        .open(&temp_path)
        .map_err(|e| format!("Disk error: {}", e))?;

    if download.total > 0 {
        let current_metadata = file.metadata().map_err(|e| e.to_string())?;
        if current_metadata.len() < download.total {
            file.set_len(download.total).map_err(|e| format!("Allocation failed: {}", e))?;
        }
    }


    let final_path = Path::new(&download.location).join(&download.name);
    
    let mut parts_to_spawn = Vec::new();

    if download.total == 0 {
        parts_to_spawn.push((0, 0, 0, download.downloaded));
    } else {
        for i in 0..download.connections.len() {
            let (start, end) = range_for(download.total, download.connections.len(), i);
            let done_so_far = download.connections[i];
            
            if done_so_far < (end - start + 1) {
                parts_to_spawn.push((i, start, end, done_so_far));
            }
        }
    }

    if parts_to_spawn.is_empty() && download.total > 0 {
        if temp_path.exists() {
            let _ = tokio::fs::rename(&temp_path, &final_path).await;
        }
        return Ok(());
    }

    {
        let mut list = state.list.write().unwrap();
        if let Some(d) = list.iter_mut().find(|d| d.id == id) {
            d.state = "Connecting".into(); 
            
            d.speeds = vec![0; d.connections.len()];
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
    let active_count = Arc::new(AtomicUsize::new(parts_to_spawn.len()));
    let mut handles = Vec::new();

    for (i, start, end, done_so_far) in parts_to_spawn {
        let state_clone = state.clone();
        let notify_clone = notify.clone();
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
                start,
                end,
                done_so_far,
                client_clone,
                state_clone,
                notify_clone,
                count_clone,
            )
            .await;
        });
        handles.push(handle);
    }

    state
        .active_tasks
        .write()
        .unwrap()
        .insert(id, ActiveTask { handles, notify });

    Ok(())
}

#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn paste() -> Result<String, String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.get_text().map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle<tauri::Wry>) -> String {
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
fn pause_download(state: tauri::State<'_, Arc<DownloadState>>, id: String) -> Result<(), String> {
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
        }
    }
    broadcast(&state);
    Ok(())
}

#[tauri::command]
fn change_link(state: tauri::State<'_, Arc<DownloadState>>, id: String, url: String) -> Result<(), String> {
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
        }
    }
    broadcast(&state);
    Ok(())
}

#[tauri::command]
fn cancel_download(state: tauri::State<'_, Arc<DownloadState>>, id: String) -> Result<(), String> {
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
async fn verify_url(
    state: tauri::State<'_, Arc<DownloadState>>,
    url: String,
) -> Result<Option<UrlInfo>, String> {
    let state = (*state).clone();
    
    let client = &state.client; 
    let parsed = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;

    let head_resp = client.head(parsed.clone()).send().await;

    let mut size = None;
    let mut filename = None;
    let mut content_type = None;
    let mut final_url = parsed.clone();

    if let Ok(resp) = head_resp {
        if resp.status().is_success() {
            final_url = resp.url().clone();
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

    let probe = client
        .get(parsed.clone())
        .header(reqwest::header::RANGE, "bytes=0-0")
        .send()
        .await;

    let resume_supported = match &probe {
        Ok(r) => r.status() == reqwest::StatusCode::PARTIAL_CONTENT,
        Err(_) => false,
    };

    if size.is_none() {
        if let Ok(resp) = &probe {
            if let Some(cr) = resp.headers().get(reqwest::header::CONTENT_RANGE) {
                // "bytes 0-0/12345" -> 12345
                if let Ok(s) = cr.to_str() {
                    if let Some(total) = s.rsplit('/').next() {
                         size = total.parse::<u64>().ok();
                    }
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

fn parse_filename(header: &str) -> Option<String> {
    if let Some(name) = header.split(';').find_map(|p| p.trim().strip_prefix("filename*=")) {
        if let Some(encoded) = name.split("''").nth(1) {
            if let Ok(decoded) = urlencoding::decode(encoded) {
                return Some(decoded.into_owned());
            }
        }
    }
    if let Some(name) = header.split(';').find_map(|p| p.trim().strip_prefix("filename=")) {
        return Some(name.trim_matches('"').to_string());
    }
    None
}

fn extension_from_mime(mime: &str) -> &str {
    match mime {
        "application/pdf" => ".pdf",
        "application/zip" => ".zip",
        "application/x-rar-compressed" => ".rar",
        "application/json" => ".json",
        "text/html" => ".html",
        "text/plain" => ".txt",
        "image/jpeg" => ".jpg",
        "image/png" => ".png",
        "video/mp4" => ".mp4",
        "audio/mpeg" => ".mp3",
        "application/octet-stream" => ".bin",
        "application/x-msdownload" => ".exe",
        _ => "",
    }
}

#[tauri::command]
fn add_download(state: tauri::State<Arc<DownloadState>>, download: Download) -> Result<(), String> {

    if download.resume && download.total > 0 {
        let path = Path::new(&download.location).join(format!("{}.jdm", download.name));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .map_err(|e| format!("IO Error: {}", e))?;

        if download.total > 0 {
            file.allocate(download.total)
            .map_err(|e| format!("Allocation Error: {}", e))?; 
        }
    }

    let mut list = state.list.write().unwrap();
    list.push(download);
    drop(list);

    broadcast(&state);
    Ok(())
}

#[tauri::command]
fn stream_downloads(state: tauri::State<Arc<DownloadState>>, channel: Channel<Vec<Download>>) {
    *state.sender.write().unwrap() = Some(channel.clone());
    let snapshot = state.list.read().unwrap().clone();
    let _ = channel.send(snapshot);
}

#[tauri::command]
fn toggle_select(state: tauri::State<Arc<DownloadState>>, tab: String) {
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
fn toggle_one(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();
    if let Some(d) = list.iter_mut().find(|d| d.id == id) {
        d.is_selected = !d.is_selected;
    }
    drop(list);
    broadcast(&state);
}

#[tauri::command]
fn select_one(state: tauri::State<Arc<DownloadState>>, id: String) {
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
fn delete_selected(state: tauri::State<Arc<DownloadState>>, delete_file: bool) {
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
fn delete_download(state: tauri::State<Arc<DownloadState>>, id: String) {
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
fn remove_download(state: tauri::State<Arc<DownloadState>>, id: String) {
    let mut list = state.list.write().unwrap();

    list.retain(|d| d.id != id);

    drop(list);
    broadcast(&state);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            
            if let Some(url) = args.get(1) {
                let url_str = url.as_str();
                
                if url_str.starts_with("jda://") {
                    let download_url = url_str.replace("jda://", "");
                    
                    let _ = app.emit("process-deep-link", download_url);
                }
            }
            
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let handle = app.handle();

            let show = MenuItem::with_id(handle, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(handle, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(handle, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    _ => {}
                })
                .icon(app.default_window_icon().unwrap().clone())
                .build(app)?;

            let state = app.state::<Arc<DownloadState>>().inner().clone();
            if let Ok(path) = app.path().app_data_dir() {
                let _file_path = path.join("downloads.json");

                let app_handle = app.handle().clone();
                let state_clone = state.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        save_state(&app_handle, &state_clone);
                    }
                });
            }

            #[cfg(desktop)]
            let _ = app.handle().plugin(
                tauri_plugin_deep_link::init(),
            );

            #[cfg(all(desktop, debug_assertions))]
            let _ = app.deep_link().register("jda");

            Ok(())
        })
        .manage(Arc::new(DownloadState {
            list: RwLock::new(vec![]),
            sender: RwLock::new(None),
            active_tasks: RwLock::new(HashMap::new()),
            
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
                .cookie_store(true)
                .connect_timeout(Duration::from_secs(10)) 
                .pool_idle_timeout(Duration::from_secs(90))
                .http2_keep_alive_interval(None) // Keep connection stable
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .expect("Failed to create HTTP client"),
        }))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            quit,
            paste,
            pick_folder,
            verify_url,
            add_download,
            stream_downloads,
            toggle_select,
            toggle_one,
            delete_selected,
            resume_download,
            pause_download,
            cancel_download,
            open_file_dir,
            change_link,
            delete_download,
            remove_download,
            select_one
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
                let state = window.state::<Arc<DownloadState>>();
                save_state(window.app_handle(), &state);
            }
        })
        .build(tauri::generate_context!())
        .expect("error building app");

    app.run(|_, _| {});
}
