pub mod models;
pub mod utils;
pub mod state;
pub mod downloader;
pub mod commands;
pub mod server;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, Emitter
};

use tauri_plugin_deep_link::DeepLinkExt;

use models::DownloadState;
use state::save_state;
use commands::*;

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
            
            crate::server::spawn_local_server(handle.clone());
            
            {
               let mut loaded = state::load_state(&handle);
               for d in loaded.iter_mut() {
                    let path = std::path::Path::new(&d.location);
                    if path.is_relative() {
                        if d.location == "Downloads" {
                            if let Ok(dir) = app.path().download_dir() {
                                d.location = dir.to_string_lossy().into_owned();
                            }
                        } else {
                            if let Ok(dir) = app.path().download_dir() {
                                d.location = dir.join(&d.location).to_string_lossy().into_owned();
                            }
                        }
                    }

                }
                *state.list.write().unwrap() = loaded;
            }

            if let Ok(_path) = app.path().app_data_dir() {
                let app_handle = app.handle().clone();
                let state_clone = state.clone();
                tauri::async_runtime::spawn(async move {
                    let mut last_save = std::time::Instant::now();
                    loop {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        
                        let mut needs_broadcast = false;
                        if let Ok(mut list) = state_clone.list.write() {
                            for d in list.iter_mut() {
                                if d.state == "Downloading" {
                                    d.elapsed += 1;
                                    needs_broadcast = true;
                                }
                            }
                        }

                        if needs_broadcast {
                            crate::state::broadcast(&state_clone);
                        }

                        if last_save.elapsed().as_secs() >= 5 {
                            save_state(&app_handle, &state_clone);
                            last_save = std::time::Instant::now();
                        }
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
            list: RwLock::new(vec![]), // Populated in setup
            sender: RwLock::new(None),
            active_tasks: RwLock::new(HashMap::new()),
            
            client: {
                reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(10)) 
                    .pool_idle_timeout(std::time::Duration::from_secs(90))
                    .redirect(reqwest::redirect::Policy::limited(10))
                    .build()
                    .expect("Failed to create HTTP client")
            },
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
