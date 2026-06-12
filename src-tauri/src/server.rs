use axum::{
    routing::post,
    Router,
    Json,
    extract::State,
    http::Method,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use tauri::{AppHandle, Manager, Emitter};
use tower_http::cors::{Any, CorsLayer};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DownloadPayload {
    pub url: String,
    pub name: String,
    pub size: u64,
    pub resume: String,
    pub cookie: String,
    #[serde(rename = "userAgent")]
    pub user_agent: String,
    pub referer: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

pub fn spawn_local_server(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        // Setup CORS to allow the browser extension to POST to this endpoint
        let cors = CorsLayer::new()
            .allow_methods([Method::POST, Method::OPTIONS])
            .allow_origin(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/download", post(handle_download))
            .layer(cors)
            .with_state(app_handle);

        let addr = SocketAddr::from(([127, 0, 0, 1], 14732));
        println!("Local JDA server listening on {}", addr);
        
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            let _ = axum::serve(listener, app).await;
        } else {
            eprintln!("Failed to bind to local server port 14732");
        }
    });
}

async fn handle_download(
    State(app): State<AppHandle>,
    Json(payload): Json<DownloadPayload>,
) -> &'static str {
    // Send the payload to the frontend via Tauri event
    // The frontend will listen to "open_add_download_from_server"
    if let Err(e) = app.emit("open_add_download_from_server", payload) {
        eprintln!("Failed to emit event to frontend: {}", e);
    }
    
    // Also show the main window if it was hidden
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    
    "OK"
}
