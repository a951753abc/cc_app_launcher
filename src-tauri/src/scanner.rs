use crate::config::{AppEntry, ConfigManager};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCandidate {
    pub name: String,
    pub path: String,
    pub command: String,
    pub app_type: String,
    pub port: Option<u16>,
}

/// Decode a `.claude/projects/` directory name into a filesystem path.
///
/// Encoding rules (Claude's convention):
/// - Drive letter + `--` marks the drive boundary: `L--` → `L:\`
/// - Single `-` is a path separator: `Users-JP6` → `Users\JP6`
///
/// Problem: folder names containing `-` (e.g. `doujin-tagger`) become ambiguous.
/// Solution: greedy filesystem-based resolution — try joining segments with `\`,
/// and when a path doesn't exist, try keeping the `-` literal instead.
pub fn decode_project_dir_name(name: &str) -> Option<String> {
    let (drive_part, rest) = name.split_once("--")?;

    if drive_part.len() != 1 || !drive_part.chars().next()?.is_ascii_alphabetic() {
        return None;
    }

    let drive = drive_part.to_uppercase();

    if rest.is_empty() {
        return Some(format!("{drive}:\\"));
    }

    let segments: Vec<&str> = rest.split('-').collect();

    // Greedy filesystem resolution: try to match real directories
    let base = format!("{drive}:\\");
    let resolved = resolve_segments(&base, &segments);
    Some(resolved)
}

/// Greedily resolve path segments against the filesystem.
/// For each position, try joining the next N segments with various separators
/// (`-`, `_`, or as a single path component) and check if that directory exists.
/// Longest filesystem match wins.
fn resolve_segments(base: &str, segments: &[&str]) -> String {
    let mut current = PathBuf::from(base);
    let mut i = 0;

    while i < segments.len() {
        let mut best_len = 0;
        let mut best_component = String::new();

        // Try longest possible combination first (greedy)
        for end in (i + 1..=segments.len()).rev() {
            let slice = &segments[i..end];

            // Try different join separators: `-` (original), `_` (common in paths)
            for sep in &["-", "_"] {
                let candidate: String = slice.join(*sep);
                let test_path = current.join(&candidate);
                if test_path.exists() {
                    best_len = end - i;
                    best_component = candidate;
                    break;
                }
            }
            if best_len > 0 {
                break;
            }
        }

        // Fallback: single segment as-is
        if best_len == 0 {
            best_len = 1;
            best_component = segments[i].to_string();
        }

        current = current.join(&best_component);
        i += best_len;
    }

    current.to_string_lossy().to_string()
}

/// Parse a port number from an npm script string.
///
/// Handles:
///   - `-p 3000` / `--port 3000`
///   - `PORT=3000`
///   - Framework conventions: `next dev` → 3000, `vite` → 5173, `ng serve` → 4200
pub fn extract_port(script: &str) -> Option<u16> {
    let tokens: Vec<&str> = script.split_whitespace().collect();

    // Explicit flags: -p <num> or --port <num>
    for i in 0..tokens.len().saturating_sub(1) {
        if tokens[i] == "-p" || tokens[i] == "--port" {
            if let Ok(p) = tokens[i + 1].parse::<u16>() {
                return Some(p);
            }
        }
    }

    // PORT=<num> (may appear anywhere in the string, possibly as `PORT=3000` token)
    for token in &tokens {
        if let Some(val) = token.strip_prefix("PORT=") {
            if let Ok(p) = val.parse::<u16>() {
                return Some(p);
            }
        }
    }

    // Framework defaults
    if script.contains("next dev") || script.contains("next start") {
        return Some(3000);
    }
    if script.contains("vite") {
        return Some(5173);
    }
    if script.contains("ng serve") {
        return Some(4200);
    }
    if script.contains("gatsby develop") {
        return Some(8000);
    }

    None
}

/// Inspect a directory and, if it looks like a project, return a `ScanCandidate`.
pub fn detect_project(path: &Path) -> Option<ScanCandidate> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    // Node.js / JavaScript
    let pkg_path = path.join("package.json");
    if pkg_path.exists() {
        let port = std::fs::read_to_string(&pkg_path)
            .ok()
            .and_then(|content| {
                let v: serde_json::Value = serde_json::from_str(&content).ok()?;
                let scripts = v.get("scripts")?;
                // Try "dev" → "start" → any script
                let priority = ["dev", "start", "serve", "preview"];
                for key in &priority {
                    if let Some(s) = scripts.get(key).and_then(|v| v.as_str()) {
                        let p = extract_port(s);
                        if p.is_some() {
                            return p;
                        }
                    }
                }
                // Try all scripts
                if let Some(obj) = scripts.as_object() {
                    for (_, v) in obj {
                        if let Some(s) = v.as_str() {
                            let p = extract_port(s);
                            if p.is_some() {
                                return p;
                            }
                        }
                    }
                }
                None
            });

        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "npm run dev".to_string(),
            app_type: if port.is_some() { "web" } else { "script" }.to_string(),
            port,
        });
    }

    // Python — explicit markers
    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
    {
        let entry = find_python_entry(path);
        let command = build_python_command(path, &entry);
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command,
            app_type: "script".to_string(),
            port: None,
        });
    }

    // Python — fallback: directory contains .py files but no manifest
    if has_python_files(path) {
        let entry = find_python_entry(path);
        let command = build_python_command(path, &entry);
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command,
            app_type: "script".to_string(),
            port: None,
        });
    }

    // Rust
    if path.join("Cargo.toml").exists() {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "cargo run".to_string(),
            app_type: "cli".to_string(),
            port: None,
        });
    }

    // Go
    if path.join("go.mod").exists() {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "go run .".to_string(),
            app_type: "cli".to_string(),
            port: None,
        });
    }

    // .NET / C#
    let has_sln = std::fs::read_dir(path).ok()?.any(|entry| {
        entry
            .ok()
            .and_then(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".sln") {
                    Some(true)
                } else {
                    None
                }
            })
            .unwrap_or(false)
    });
    if has_sln {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "dotnet run".to_string(),
            app_type: "cli".to_string(),
            port: None,
        });
    }

    None
}

/// Parse the stdout of `where python` into a list of candidate paths.
/// One path per non-blank line, trimmed.
fn parse_where_output(s: &str) -> Vec<PathBuf> {
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Detect Microsoft Store's Python alias stub. The stub lives at
/// `%LOCALAPPDATA%\Microsoft\WindowsApps\python.exe` and only opens
/// the Store install dialog — never use it as a real interpreter.
fn is_windows_store_stub(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase().replace('/', "\\");
    s.contains("\\appdata\\local\\microsoft\\windowsapps\\")
}

/// Pick the first candidate that is not a Windows Store stub and exists
/// on disk. Used by tests via the `_with` variant for predicate injection.
fn pick_real_python(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    pick_real_python_with(candidates, |p| p.exists())
}

/// Testable variant of [`pick_real_python`]; accepts an `exists` predicate
/// instead of calling `Path::exists`, enabling pure unit tests without I/O.
fn pick_real_python_with<F>(candidates: Vec<PathBuf>, exists: F) -> Option<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    candidates
        .into_iter()
        .find(|p| !is_windows_store_stub(p) && exists(p))
}

/// Find the venv Python executable in a project directory (Windows).
/// Checks `venv\Scripts\python.exe` and `.venv\Scripts\python.exe`.
fn find_venv_python(path: &Path) -> Option<PathBuf> {
    for dir_name in &["venv", ".venv"] {
        let python = path.join(dir_name).join("Scripts").join("python.exe");
        if python.exists() {
            return Some(python);
        }
    }
    None
}

/// Build the Python command for a project, using venv if available.
fn build_python_command(path: &Path, entry: &str) -> String {
    if let Some(venv_python) = find_venv_python(path) {
        format!("\"{}\" {}", venv_python.to_string_lossy(), entry)
    } else {
        format!("python {}", entry)
    }
}

/// Find the best Python entry point in a directory.
fn find_python_entry(path: &Path) -> String {
    // Exact-name priority list
    for candidate in &["app.py", "main.py", "run.py", "server.py"] {
        if path.join(candidate).exists() {
            return candidate.to_string();
        }
    }
    // Pattern-based: files containing "server" or "api" in the name
    if let Ok(entries) = std::fs::read_dir(path) {
        let mut py_files: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".py") && !name.starts_with('_') && !name.starts_with("test") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        py_files.sort();

        // Prefer files with "server" or "api" in the name
        for f in &py_files {
            let lower = f.to_lowercase();
            if lower.contains("server") || lower.contains("api") {
                return f.clone();
            }
        }
        // Fallback: first non-test .py file
        if let Some(first) = py_files.first() {
            return first.clone();
        }
    }
    "main.py".to_string()
}

/// Check if a directory contains any .py files (top-level only).
fn has_python_files(path: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".py") {
                return true;
            }
        }
    }
    false
}

/// Whether a directory looks like a git worktree (not the main worktree).
fn is_worktree(path: &Path) -> bool {
    let git_file = path.join(".git");
    if git_file.is_file() {
        // Worktrees have `.git` as a *file* (not a directory)
        return true;
    }
    false
}

/// Main scan function. Reads scan_paths from config, decodes dir names,
/// filters non-existent paths, optionally excludes worktrees, deduplicates.
pub fn scan_projects(config: &ConfigManager) -> Vec<ScanCandidate> {
    let cfg = match config.get_config() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let exclude_worktrees = cfg.settings.exclude_worktrees;
    let mut seen: HashSet<String> = HashSet::new();
    let mut results: Vec<ScanCandidate> = Vec::new();

    let all_scan_dirs: Vec<PathBuf> = cfg
        .scan_paths
        .iter()
        .chain(cfg.extra_scan_paths.iter())
        .map(PathBuf::from)
        .collect();

    for scan_dir in &all_scan_dirs {
        if !scan_dir.exists() {
            continue;
        }

        let entries = match std::fs::read_dir(scan_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let encoded_name = entry.file_name().to_string_lossy().to_string();

            // Decode the .claude/projects dir name convention
            let decoded_path_str = match decode_project_dir_name(&encoded_name) {
                Some(p) => p,
                // Not a Claude-encoded dir — try treating as a plain path directly
                None => entry.path().to_string_lossy().to_string(),
            };

            let project_path = PathBuf::from(&decoded_path_str);

            // Filter: must exist
            if !project_path.exists() {
                continue;
            }

            // Filter: deduplicate by resolved path
            let canonical = project_path
                .canonicalize()
                .unwrap_or_else(|_| project_path.clone());
            let key = canonical.to_string_lossy().to_string();
            if !seen.insert(key) {
                continue;
            }

            // Filter: exclude worktrees if configured
            if exclude_worktrees && is_worktree(&project_path) {
                continue;
            }

            if let Some(candidate) = detect_project(&project_path) {
                results.push(candidate);
            }
        }
    }

    results
}

/// Convert a `ScanCandidate` into a full `AppEntry` with a new UUID.
pub fn candidate_to_app(candidate: ScanCandidate) -> AppEntry {
    let mut app = AppEntry::new(candidate.name, candidate.path, candidate.command, candidate.app_type);
    app.port = candidate.port;
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_dir_name() {
        // Basic — no ambiguity
        assert_eq!(
            decode_project_dir_name("L--temp"),
            Some("L:\\temp".to_string())
        );

        // Invalid: no double-dash
        assert_eq!(decode_project_dir_name("no-double-dash"), None);

        // Invalid: drive letter part too long
        assert_eq!(decode_project_dir_name("AB--path"), None);

        // Drive only
        assert_eq!(decode_project_dir_name("L--"), Some("L:\\".to_string()));
    }

    #[test]
    fn test_extract_port() {
        // Explicit flag
        assert_eq!(extract_port("node server.js -p 8080"), Some(8080));
        assert_eq!(extract_port("node server.js --port 4000"), Some(4000));

        // Environment variable
        assert_eq!(extract_port("PORT=3001 node index.js"), Some(3001));

        // Framework defaults
        assert_eq!(extract_port("next dev"), Some(3000));
        assert_eq!(extract_port("vite"), Some(5173));
        assert_eq!(extract_port("ng serve"), Some(4200));
        assert_eq!(extract_port("gatsby develop"), Some(8000));

        // No port
        assert_eq!(extract_port("cargo run"), None);
        assert_eq!(extract_port("python app.py"), None);
    }

    #[test]
    fn test_parse_where_output_basic() {
        let raw = "C:\\Users\\JP6\\anaconda3\\python.exe\r\nC:\\Python314\\python.exe\r\n";
        let parsed = parse_where_output(raw);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], PathBuf::from("C:\\Users\\JP6\\anaconda3\\python.exe"));
        assert_eq!(parsed[1], PathBuf::from("C:\\Python314\\python.exe"));
    }

    #[test]
    fn test_parse_where_output_skips_blank_lines() {
        let raw = "\nC:\\Python314\\python.exe\n\n  \n";
        let parsed = parse_where_output(raw);
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_is_windows_store_stub_detects_appdata_windowsapps() {
        assert!(is_windows_store_stub(&PathBuf::from(
            "C:\\Users\\JP6\\AppData\\Local\\Microsoft\\WindowsApps\\python.exe"
        )));
        assert!(is_windows_store_stub(&PathBuf::from(
            "C:\\Users\\JP6\\AppData\\Local\\Microsoft\\WindowsApps\\python3.exe"
        )));
    }

    #[test]
    fn test_is_windows_store_stub_real_python_is_not_stub() {
        assert!(!is_windows_store_stub(&PathBuf::from(
            "C:\\Python314\\python.exe"
        )));
        assert!(!is_windows_store_stub(&PathBuf::from(
            "C:\\Users\\JP6\\anaconda3\\python.exe"
        )));
    }

    #[test]
    fn test_pick_real_python_skips_stub_and_picks_first_real() {
        let candidates = vec![
            PathBuf::from("C:\\Users\\JP6\\AppData\\Local\\Microsoft\\WindowsApps\\python.exe"),
            PathBuf::from("C:\\Users\\JP6\\anaconda3\\python.exe"),
            PathBuf::from("C:\\Python314\\python.exe"),
        ];
        let picked = pick_real_python_with(candidates, |_| true);
        assert_eq!(
            picked,
            Some(PathBuf::from("C:\\Users\\JP6\\anaconda3\\python.exe"))
        );
    }

    #[test]
    fn test_pick_real_python_returns_none_when_only_stub() {
        let candidates = vec![PathBuf::from(
            "C:\\Users\\JP6\\AppData\\Local\\Microsoft\\WindowsApps\\python.exe",
        )];
        let picked = pick_real_python_with(candidates, |_| true);
        assert_eq!(picked, None);
    }

    #[test]
    fn test_pick_real_python_returns_none_when_none_exist() {
        let candidates = vec![PathBuf::from("C:\\Definitely\\Not\\Real\\python.exe")];
        let picked = pick_real_python_with(candidates, |_| false);
        assert_eq!(picked, None);
    }

    #[test]
    fn test_pick_real_python_uses_real_existence_check() {
        // Use current_exe() — guaranteed to exist on the test machine
        let real_exe = std::env::current_exe().unwrap();
        let candidates = vec![
            PathBuf::from("C:\\Definitely\\Not\\Real\\python.exe"),
            real_exe.clone(),
        ];
        assert_eq!(pick_real_python(candidates), Some(real_exe));
    }
}
