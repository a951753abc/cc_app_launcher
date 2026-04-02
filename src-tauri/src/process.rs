use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub app_id: String,
    pub line: String,
    pub is_stderr: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessStatus {
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessState {
    pub app_id: String,
    pub status: ProcessStatus,
}

// ---------------------------------------------------------------------------
// Internal structs
// ---------------------------------------------------------------------------

struct ManagedProcess {
    child: Child,
}

pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// ProcessManager implementation
// ---------------------------------------------------------------------------

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn the process and return immediately.  Log streaming and exit
    /// monitoring happen in background threads.
    pub fn start(
        &self,
        app_id: String,
        command: String,
        working_dir: String,
        app_handle: AppHandle,
    ) -> Result<(), String> {
        // Guard: already running?
        {
            let lock = self
                .processes
                .lock()
                .map_err(|e| format!("Mutex poisoned: {e}"))?;
            if lock.contains_key(&app_id) {
                return Err(format!("App '{app_id}' is already running"));
            }
        }

        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut child = Command::new("cmd")
            .args(["/C", &command])
            .current_dir(&working_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn '{command}': {e}"))?;

        let pid = child.id();

        // Take piped handles before moving child into the map
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;

        // Insert into map
        {
            let mut lock = self
                .processes
                .lock()
                .map_err(|e| format!("Mutex poisoned: {e}"))?;
            lock.insert(app_id.clone(), ManagedProcess { child });
        }

        // Emit Running immediately
        let _ = app_handle.emit(
            "process-status",
            ProcessState {
                app_id: app_id.clone(),
                status: ProcessStatus::Running,
            },
        );

        // --- stdout thread ---
        {
            let app_handle = app_handle.clone();
            let app_id = app_id.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = app_handle.emit(
                        "process-log",
                        LogLine {
                            app_id: app_id.clone(),
                            line,
                            is_stderr: false,
                            timestamp: now_millis(),
                        },
                    );
                }
            });
        }

        // --- stderr thread ---
        {
            let app_handle = app_handle.clone();
            let app_id = app_id.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = app_handle.emit(
                        "process-log",
                        LogLine {
                            app_id: app_id.clone(),
                            line,
                            is_stderr: true,
                            timestamp: now_millis(),
                        },
                    );
                }
            });
        }

        // --- monitor thread: polls try_wait() every second ---
        {
            let processes = Arc::clone(&self.processes);
            let app_id = app_id.clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(1));

                let exited = {
                    let mut lock = match processes.lock() {
                        Ok(l) => l,
                        Err(_) => break,
                    };
                    if let Some(proc) = lock.get_mut(&app_id) {
                        match proc.child.try_wait() {
                            Ok(Some(_)) => {
                                lock.remove(&app_id);
                                true
                            }
                            Ok(None) => false, // still running
                            Err(_) => {
                                lock.remove(&app_id);
                                true
                            }
                        }
                    } else {
                        break; // entry already removed (e.g. stop() was called)
                    }
                };

                if exited {
                    let _ = app_handle.emit(
                        "process-status",
                        ProcessState {
                            app_id: app_id.clone(),
                            status: ProcessStatus::Stopped,
                        },
                    );
                    break;
                }
            });
        }

        let _ = pid; // suppress unused warning
        Ok(())
    }

    /// Kill the process forcefully.
    pub fn stop(&self, app_id: &str) -> Result<(), String> {
        let mut lock = self
            .processes
            .lock()
            .map_err(|e| format!("Mutex poisoned: {e}"))?;

        let proc = lock
            .get_mut(app_id)
            .ok_or_else(|| format!("App '{app_id}' is not running"))?;

        let pid = proc.child.id();

        // taskkill /PID {pid} /T /F — terminates the whole process tree
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .output();

        // Belt-and-suspenders: also call child.kill()
        let _ = proc.child.kill();

        lock.remove(app_id);
        Ok(())
    }

    pub fn get_running_ids(&self) -> Vec<String> {
        self.processes
            .lock()
            .map(|lock| lock.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn stop_all(&self) {
        let ids: Vec<String> = self.get_running_ids();
        for id in ids {
            let _ = self.stop(&id);
        }
    }
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        self.stop_all();
    }
}
