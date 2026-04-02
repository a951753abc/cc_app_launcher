mod config;
mod process;
mod scanner;

use config::{AppConfig, AppEntry, ConfigManager, Settings};
use scanner::{candidate_to_app, scan_projects as do_scan_projects, ScanCandidate};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

// ---------------------------------------------------------------------------
// Tauri commands
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
// ScanCandidate needs Serialize/Deserialize for Tauri commands
// ---------------------------------------------------------------------------

// We keep ScanCandidate defined in scanner.rs but add derives via a wrapper
// here — actually, we need to add serde derives on the struct itself.
// See scanner.rs for the derive attributes.

// ---------------------------------------------------------------------------
// App setup
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config_manager = Arc::new(
        ConfigManager::new().expect("Failed to initialize ConfigManager"),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(config_manager.clone())
        .setup(move |app| {
            let app_handle: AppHandle = app.handle().clone();
            let manager = config_manager.clone();

            // Watch for external config file changes and emit event to frontend
            let watcher = manager
                .watch_config(move |_event| {
                    let _ = app_handle.emit("config-changed", ());
                })
                .expect("Failed to start config watcher");

            // Leak the watcher so it stays alive for the lifetime of the app
            Box::leak(Box::new(watcher));

            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
