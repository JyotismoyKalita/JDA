use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tokio::sync::Notify;

#[derive(Serialize)]
pub struct UrlInfo {
    pub filename: String,
    pub resume_supported: bool,
    pub size: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Part {
    pub start: u64,
    pub end: u64,
    pub downloaded: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Download {
    pub id: String,
    pub link: String,
    pub name: String,
    pub location: String,
    pub resume: bool,
    pub downloaded: u64,
    pub total: u64,
    pub start_time: Option<u64>,
    pub elapsed: u64,

    pub speed: u64,
    pub state: String,
    pub is_selected: bool,

    pub connections: Vec<u64>,

    pub parts: Vec<Part>,

    pub speeds: Vec<u64>,

    pub cookies: Option<String>,

    pub user_agent: Option<String>,

    pub referer: Option<String>,

    pub headers: HashMap<String, String>,
}

pub struct ActivePart {
    pub part_index: usize,
    pub start: u64,
    pub current_pos: Arc<std::sync::atomic::AtomicU64>,
    pub end: Arc<std::sync::atomic::AtomicU64>,
}

pub struct ActiveTask {
    pub handles: Vec<JoinHandle<()>>,
    pub notify: Arc<Notify>,
    pub active_parts: Arc<RwLock<Vec<ActivePart>>>,
}

pub struct DownloadState {
    pub list: RwLock<Vec<Download>>,
    pub sender: RwLock<Option<Channel<Vec<Download>>>>,
    pub active_tasks: RwLock<HashMap<String, ActiveTask>>,
    pub client: reqwest::Client,
}
