use crate::config::{AppEntry, ConfigManager};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use uuid::Uuid;

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
/// Examples:
///   `L--mylisbeth`                        → `L:\mylisbeth`
///   `C--Users-JP6-Documents-ai-trpg`      → `C:\Users\JP6\Documents\ai-trpg`
pub fn decode_project_dir_name(name: &str) -> Option<String> {
    // Must start with a drive letter followed by `--`
    let (drive_part, rest) = name.split_once("--")?;

    // drive_part should be a single ASCII alphabetic letter
    if drive_part.len() != 1 || !drive_part.chars().next()?.is_ascii_alphabetic() {
        return None;
    }

    let drive = drive_part.to_uppercase();

    // Replace `-` with the OS path separator in the remainder
    let path_part = rest.replace('-', std::path::MAIN_SEPARATOR_STR);

    if path_part.is_empty() {
        Some(format!("{drive}:\\"))
    } else {
        Some(format!("{drive}:\\{path_part}"))
    }
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
            app_type: "node".to_string(),
            port,
        });
    }

    // Python
    if path.join("requirements.txt").exists()
        || path.join("pyproject.toml").exists()
        || path.join("setup.py").exists()
    {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "python main.py".to_string(),
            app_type: "python".to_string(),
            port: None,
        });
    }

    // Rust
    if path.join("Cargo.toml").exists() {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "cargo run".to_string(),
            app_type: "rust".to_string(),
            port: None,
        });
    }

    // Go
    if path.join("go.mod").exists() {
        return Some(ScanCandidate {
            name,
            path: path.to_string_lossy().to_string(),
            command: "go run .".to_string(),
            app_type: "go".to_string(),
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
            app_type: "dotnet".to_string(),
            port: None,
        });
    }

    None
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
    AppEntry {
        id: Uuid::new_v4().to_string(),
        name: candidate.name,
        path: candidate.path,
        command: candidate.command,
        app_type: candidate.app_type,
        port: candidate.port,
        auto_start: false,
        tags: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_dir_name() {
        // Basic drive letter
        assert_eq!(
            decode_project_dir_name("L--mylisbeth"),
            Some("L:\\mylisbeth".to_string())
        );

        // Multi-segment path: each `-` is a path separator
        // "C--Users-JP6-Documents-ai-trpg" → C:\Users\JP6\Documents\ai\trpg
        // (if a folder name itself contains `-`, Claude can't round-trip it losslessly)
        assert_eq!(
            decode_project_dir_name("C--Users-JP6-Documents-ai-trpg"),
            Some("C:\\Users\\JP6\\Documents\\ai\\trpg".to_string())
        );

        // Single path component after drive
        assert_eq!(
            decode_project_dir_name("D--projects"),
            Some("D:\\projects".to_string())
        );

        // Invalid: no double-dash
        assert_eq!(decode_project_dir_name("no-double-dash"), None);

        // Invalid: drive letter part too long
        assert_eq!(decode_project_dir_name("AB--path"), None);
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
}
