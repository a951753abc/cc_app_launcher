use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub command: String,
    #[serde(rename = "type")]
    pub app_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    pub auto_start: bool,
    pub tags: Vec<String>,
}

impl AppEntry {
    pub fn new(name: String, path: String, command: String, app_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            command,
            app_type,
            port: None,
            auto_start: false,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    pub auto_start_with_windows: bool,
    #[serde(default = "default_true")]
    pub exclude_worktrees: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            start_minimized: false,
            close_to_tray: true,
            auto_start_with_windows: false,
            exclude_worktrees: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub apps: Vec<AppEntry>,
    pub scan_paths: Vec<String>,
    pub extra_scan_paths: Vec<String>,
    pub settings: Settings,
}

impl Default for AppConfig {
    fn default() -> Self {
        let scan_paths = dirs::home_dir()
            .map(|h| {
                vec![h
                    .join(".claude")
                    .join("projects")
                    .to_string_lossy()
                    .to_string()]
            })
            .unwrap_or_default();

        Self {
            apps: Vec::new(),
            scan_paths,
            extra_scan_paths: Vec::new(),
            settings: Settings::default(),
        }
    }
}

pub struct ConfigManager {
    pub config: Arc<Mutex<AppConfig>>,
    pub config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Result<Self, String> {
        let config_path = dirs::data_dir()
            .ok_or_else(|| "Cannot determine APPDATA directory".to_string())?
            .join("app-launcher")
            .join("apps.json");

        let config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config: {e}"))?;
            serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse config: {e}"))?
        } else {
            let default_config = AppConfig::default();
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config dir: {e}"))?;
            }
            let content = serde_json::to_string_pretty(&default_config)
                .map_err(|e| format!("Failed to serialize default config: {e}"))?;
            std::fs::write(&config_path, content)
                .map_err(|e| format!("Failed to write default config: {e}"))?;
            default_config
        };

        Ok(Self {
            config: Arc::new(Mutex::new(config)),
            config_path,
        })
    }

    pub fn get_config(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|c| c.clone())
            .map_err(|e| format!("Mutex poisoned: {e}"))
    }

    pub fn save(&self) -> Result<(), String> {
        let config = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        let content = serde_json::to_string_pretty(&*config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {e}"))?;
        }
        std::fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write config: {e}"))
    }

    pub fn reload(&self) -> Result<(), String> {
        let content = std::fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read config: {e}"))?;
        let new_config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {e}"))?;
        let mut lock = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        *lock = new_config;
        Ok(())
    }

    pub fn add_app(&self, app: AppEntry) -> Result<(), String> {
        let mut lock = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        lock.apps.push(app);
        drop(lock);
        self.save()
    }

    pub fn update_app(&self, app: AppEntry) -> Result<(), String> {
        let mut lock = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        let pos = lock
            .apps
            .iter()
            .position(|a| a.id == app.id)
            .ok_or_else(|| format!("App not found: {}", app.id))?;
        lock.apps[pos] = app;
        drop(lock);
        self.save()
    }

    pub fn remove_app(&self, id: &str) -> Result<(), String> {
        let mut lock = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        let before = lock.apps.len();
        lock.apps.retain(|a| a.id != id);
        if lock.apps.len() == before {
            return Err(format!("App not found: {id}"));
        }
        drop(lock);
        self.save()
    }

    pub fn update_settings(&self, settings: Settings) -> Result<(), String> {
        let mut lock = self
            .config
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;
        lock.settings = settings;
        drop(lock);
        self.save()
    }

    pub fn watch_config<F>(&self, on_change: F) -> Result<RecommendedWatcher, String>
    where
        F: Fn(Event) + Send + 'static,
    {
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    on_change(event);
                }
            },
            Config::default(),
        )
        .map_err(|e| format!("Failed to create watcher: {e}"))?;

        watcher
            .watch(&self.config_path, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch config path: {e}"))?;

        Ok(watcher)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_manager_in_temp(dir: &TempDir) -> ConfigManager {
        let config_path = dir.path().join("apps.json");
        let config = AppConfig::default();
        let content = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&config_path, content).unwrap();
        ConfigManager {
            config: Arc::new(Mutex::new(config)),
            config_path,
        }
    }

    #[test]
    fn test_add_and_remove_app() {
        let dir = TempDir::new().unwrap();
        let manager = make_manager_in_temp(&dir);

        let app = AppEntry::new(
            "TestApp".to_string(),
            "/some/path".to_string(),
            "npm start".to_string(),
            "node".to_string(),
        );
        let app_id = app.id.clone();

        manager.add_app(app).unwrap();
        let config = manager.get_config().unwrap();
        assert_eq!(config.apps.len(), 1);
        assert_eq!(config.apps[0].name, "TestApp");

        manager.remove_app(&app_id).unwrap();
        let config = manager.get_config().unwrap();
        assert_eq!(config.apps.len(), 0);
    }

    #[test]
    fn test_update_app() {
        let dir = TempDir::new().unwrap();
        let manager = make_manager_in_temp(&dir);

        let mut app = AppEntry::new(
            "OriginalName".to_string(),
            "/some/path".to_string(),
            "npm start".to_string(),
            "node".to_string(),
        );
        manager.add_app(app.clone()).unwrap();

        app.name = "UpdatedName".to_string();
        manager.update_app(app.clone()).unwrap();

        let config = manager.get_config().unwrap();
        assert_eq!(config.apps[0].name, "UpdatedName");
    }

    #[test]
    fn test_save_and_reload() {
        let dir = TempDir::new().unwrap();
        let manager = make_manager_in_temp(&dir);

        let app = AppEntry::new(
            "PersistApp".to_string(),
            "/persist/path".to_string(),
            "cargo run".to_string(),
            "rust".to_string(),
        );
        manager.add_app(app).unwrap();
        manager.save().unwrap();

        // Simulate a fresh load from file
        let content = fs::read_to_string(&manager.config_path).unwrap();
        let loaded: AppConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.apps.len(), 1);
        assert_eq!(loaded.apps[0].name, "PersistApp");

        // Now test reload()
        // Modify the file externally
        let mut modified = loaded.clone();
        modified.apps[0].name = "ReloadedApp".to_string();
        let new_content = serde_json::to_string_pretty(&modified).unwrap();
        fs::write(&manager.config_path, new_content).unwrap();

        manager.reload().unwrap();
        let config = manager.get_config().unwrap();
        assert_eq!(config.apps[0].name, "ReloadedApp");
    }
}
