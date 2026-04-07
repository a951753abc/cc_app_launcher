mod config;
mod process;
mod scanner;

use config::{AppConfig, AppEntry, ConfigManager, Settings};
use process::ProcessManager;
use scanner::{candidate_to_app, detect_project, scan_projects as do_scan_projects, ScanCandidate};
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

#[tauri::command]
fn detect_project_command(
    path: String,
    state: State<Arc<ConfigManager>>,
) -> Result<String, String> {
    let cfg = state.get_config()?;
    let project_path = std::path::Path::new(&path);
    if !project_path.exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    detect_project(project_path, &cfg.settings)
        .map(|c| c.command)
        .ok_or_else(|| format!("Could not detect project type at {path}"))
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

/// Detect externally-running apps.
/// - Apps with a port: TCP connect check
/// - Apps with a processName: match by process name via sysinfo
/// - Others: match by cwd / command-line path via sysinfo
/// Only checks apps NOT already managed by ProcessManager.
/// A single sysinfo snapshot is taken for all non-port checks.
#[tauri::command]
fn detect_running(
    config_state: State<Arc<ConfigManager>>,
    proc_mgr: State<Arc<ProcessManager>>,
) -> Vec<process::ProcessState> {
    let config = match config_state.get_config() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let managed_ids = proc_mgr.get_running_ids();

    // Collect apps that need process-level detection
    let unmanaged: Vec<_> = config
        .apps
        .iter()
        .filter(|a| !managed_ids.contains(&a.id))
        .collect();

    // Build sysinfo snapshot only once, and only if needed
    let needs_sysinfo = unmanaged.iter().any(|a| a.port.is_none());
    let snapshot = if needs_sysinfo {
        Some(process::ProcessSnapshot::new())
    } else {
        None
    };

    let mut results = Vec::new();
    for app in &unmanaged {
        let detected = if let Some(port) = app.port {
            process::is_port_in_use(port)
        } else if let Some(ref pname) = app.process_name {
            snapshot.as_ref().unwrap().has_process_named(pname)
        } else {
            snapshot.as_ref().unwrap().has_process_at_path(&app.path)
        };
        if detected {
            results.push(process::ProcessState {
                app_id: app.id.clone(),
                status: process::ProcessStatus::External,
            });
        }
    }
    results
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
                .icon(tauri::include_image!("./icons/32x32.png"))
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
            detect_project_command,
            start_app,
            stop_app,
            get_running_apps,
            detect_running,
            stop_all_apps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
