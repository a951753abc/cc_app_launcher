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
    External,
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

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn spawn_log_reader(
    stream: impl std::io::Read + Send + 'static,
    app_id: String,
    app_handle: AppHandle,
    is_stderr: bool,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            let _ = app_handle.emit(
                "process-log",
                LogLine {
                    app_id: app_id.clone(),
                    line,
                    is_stderr,
                    timestamp: now_millis(),
                },
            );
        }
    });
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

        // Use raw_arg to bypass Rust's default Windows escape rules, which
        // emit `\"` — a sequence cmd.exe does not recognise.  Wrap the whole
        // command in an outer pair of quotes so cmd's `/C` parser hits rule 2
        // ("first char is a quote → strip leading and trailing quote") and
        // recovers the original command verbatim.  Without this, commands
        // containing multiple quoted paths (e.g. `"conda.bat" ... "python.exe" ...`)
        // are misparsed and fail silently before any output can be captured.
        let mut child = Command::new("cmd")
            .raw_arg("/C")
            .raw_arg(format!("\"{command}\""))
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

        spawn_log_reader(stdout, app_id.clone(), app_handle.clone(), false);
        spawn_log_reader(stderr, app_id.clone(), app_handle.clone(), true);

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
            .creation_flags(CREATE_NO_WINDOW)
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

/// Strip the Windows `\\?\` UNC prefix that `canonicalize()` adds.
/// Without this, string comparisons against non-canonicalized paths fail.
fn strip_unc_prefix(s: &str) -> String {
    s.strip_prefix(r"\\?\").unwrap_or(s).to_string()
}

/// Check whether a TCP port is currently in use on localhost.
pub fn is_port_in_use(port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).is_ok()
}

/// Snapshot of all running processes. Build once, query many times.
pub struct ProcessSnapshot {
    sys: sysinfo::System,
}

impl ProcessSnapshot {
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        Self { sys }
    }

    /// Check whether a process with the given name is currently running.
    pub fn has_process_named(&self, target: &str) -> bool {
        let target_lower = target.to_lowercase();
        self.sys.processes().values().any(|p| {
            p.name().to_string_lossy().to_lowercase() == target_lower
        })
    }

    /// Check whether a process whose cwd, command line, or executable path
    /// matches `app_path` exists. Also detects venv-launched processes whose
    /// executable lives inside the project's venv directory.
    pub fn has_process_at_path(&self, app_path: &str) -> bool {
        let normalized_lower = strip_unc_prefix(app_path)
            .to_lowercase()
            .replace('/', "\\");

        for process in self.sys.processes().values() {
            // Check cwd
            if let Some(cwd) = process.cwd() {
                let cwd_str = strip_unc_prefix(&cwd.to_string_lossy())
                    .to_lowercase()
                    .replace('/', "\\");
                if cwd_str == normalized_lower {
                    return true;
                }
            }
            // Check command-line arguments
            for arg in process.cmd() {
                let arg_lower = strip_unc_prefix(&arg.to_string_lossy())
                    .to_lowercase()
                    .replace('/', "\\");
                if arg_lower.contains(&normalized_lower) {
                    return true;
                }
            }
            // Check executable path (catches venv python inside project dir)
            if let Some(exe) = process.exe() {
                let exe_lower = strip_unc_prefix(&exe.to_string_lossy())
                    .to_lowercase()
                    .replace('/', "\\");
                if exe_lower.starts_with(&normalized_lower) {
                    return true;
                }
            }
        }
        false
    }
}
