use std::sync::Arc;
use crate::models::{DownloadState, Download};
use tauri::Manager;

pub fn broadcast(state: &Arc<DownloadState>) {
    let sender_guard = state.sender.read().unwrap();
    if let Some(ch) = sender_guard.as_ref() {
        let list = state.list.read().unwrap().clone();
        if ch.send(list).is_err() {
            drop(sender_guard);
            *state.sender.write().unwrap() = None;
        }
    }
}

pub fn save_state(app_handle: &tauri::AppHandle, state: &Arc<DownloadState>) {
    if let Ok(path) = app_handle.path().app_data_dir() {
        let _ = std::fs::create_dir_all(&path);
        let file_path = path.join("downloads.json");
        let list = state.list.read().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*list) {
            let _ = std::fs::write(file_path, json);
        }
    }
}

pub fn load_state(app_handle: &tauri::AppHandle) -> Vec<Download> {
    if let Ok(path) = app_handle.path().app_data_dir() {
        let file_path = path.join("downloads.json");
        if let Ok(content) = std::fs::read_to_string(file_path) {
            if let Ok(mut list) = serde_json::from_str::<Vec<Download>>(&content) {
                for d in list.iter_mut() {
                    // Set ongoing downloads to Paused since they are not actively downloading on startup
                    if d.state == "Downloading" || d.state == "Connecting" {
                        d.state = "Paused".into();
                        d.speed = 0;
                        for speed in d.speeds.iter_mut() {
                            *speed = 0;
                        }
                    }
                }
                return list;
            }
        }
    }
    vec![]
}
