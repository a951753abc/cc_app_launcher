mod config;
mod process;
mod scanner;

use config::{AppConfig, AppEntry, ConfigManager, Settings};
use process::ProcessManager;
use scanner::{candidate_to_app, scan_projects as do_scan_projects, ScanCandidate};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_config(state: State<Arc<ConfigManager>>) -> Result<AppConfig, String> {
    state.get_config()
}

#[tauri::command]
fn add_app(app: AppEntry, state: State<Arc<ConfigManager>>) -> Result<(), String> {
    state.add_app(app)
}

#[tauri::command]
fn update_app(app: AppEntry, state: State<Arc<ConfigManager>>) -> Result<(), String> {
    state.update_app(app)
}

#[tauri::command]
fn remove_app(id: String, state: State<Arc<ConfigManager>>) -> Result<(), String> {
    state.remove_app(&id)
}

#[tauri::command]
fn update_settings(settings: Settings, state: State<Arc<ConfigManager>>) -> Result<(), String> {
    state.update_settings(settings)
}

#[tauri::command]
fn scan_projects(state: State<Arc<ConfigManager>>) -> Vec<ScanCandidate> {
    do_scan_projects(&state)
}

#[tauri::command]
fn add_scanned_apps(
    candidates: Vec<ScanCandidate>,
    state: State<Arc<ConfigManager>>,
) -> Result<(), String> {
    for candidate in candidates {
        let app = candidate_to_app(candidate);
        state.add_app(app)?;
    }
    Ok(())
}

#[tauri::command]
fn check_path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

// ---------------------------------------------------------------------------
// Process commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn start_app(
    app_handle: AppHandle,
    config_state: State<Arc<ConfigManager>>,
    proc_mgr: State<Arc<ProcessManager>>,
    id: String,
) -> Result<(), String> {
    let config = config_state.get_config()?;
    let entry = config
        .apps
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| format!("App '{id}' not found in config"))?
        .clone();

    if !std::path::Path::new(&entry.path).exists() {
        return Err(format!("Working directory does not exist: {}", entry.path));
    }

    proc_mgr.start(entry.id, entry.command, entry.path, app_handle)
}

#[tauri::command]
fn stop_app(
    app_handle: AppHandle,
    proc_mgr: State<Arc<ProcessManager>>,
    id: String,
) -> Result<(), String> {
    proc_mgr.stop(&id)?;
    let _ = app_handle.emit(
        "process-status",
        process::ProcessState {
            app_id: id,
            status: process::ProcessStatus::Stopped,
        },
    );
    Ok(())
}

#[tauri::command]
fn get_running_apps(proc_mgr: State<Arc<ProcessManager>>) -> Vec<String> {
    proc_mgr.get_running_ids()
}

#[tauri::command]
fn stop_all_apps(proc_mgr: State<Arc<ProcessManager>>) -> Result<(), String> {
    proc_mgr.stop_all();
    Ok(())
}

// ---------------------------------------------------------------------------
// App setup
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_manager = Arc::new(
        ConfigManager::new().expect("Failed to initialize ConfigManager"),
    );
    let process_manager = Arc::new(ProcessManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(config_manager.clone())
        .manage(process_manager.clone())
        .setup(move |app| {
            let app_handle: AppHandle = app.handle().clone();

            // --- Config file watcher ---
            let watcher_handle = app_handle.clone();
            let watcher = config_manager
                .watch_config(move |_event| {
                    let _ = watcher_handle.emit("config-changed", ());
                })
                .expect("Failed to start config watcher");
            Box::leak(Box::new(watcher));

            // --- System Tray ---
            let show_item = MenuItem::with_id(app, "show", "顯示主視窗", true, None::<&str>)?;
            let start_all_item =
                MenuItem::with_id(app, "start-all", "啟動全部", true, None::<&str>)?;
            let stop_all_item =
                MenuItem::with_id(app, "stop-all", "停止全部", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&show_item, &start_all_item, &stop_all_item, &quit_item],
            )?;

            let tray_handle = app_handle.clone();
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |_tray, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = tray_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "start-all" => {
                        let _ = tray_handle.emit("tray-start-all", ());
                    }
                    "stop-all" => {
                        if let Some(state) = tray_handle.try_state::<Arc<ProcessManager>>() {
                            state.stop_all();
                        }
                    }
                    "quit" => {
                        tray_handle.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Read close_to_tray from config
                let close_to_tray = window
                    .app_handle()
                    .try_state::<Arc<ConfigManager>>()
                    .and_then(|mgr| mgr.get_config().ok())
                    .map(|cfg| cfg.settings.close_to_tray)
                    .unwrap_or(true);

                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            add_app,
            update_app,
            remove_app,
            update_settings,
            scan_projects,
            add_scanned_apps,
            check_path_exists,
            start_app,
            stop_app,
            get_running_apps,
            stop_all_apps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
