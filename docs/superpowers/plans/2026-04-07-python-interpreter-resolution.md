# Python Interpreter Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** App Launcher 偵測 Python 專案時，自動把 `python xxx.py` 解析成絕對路徑寫進 command，並提供全域「預設 Python 直譯器」設定與每個 app 的「重新偵測指令」按鈕，避免 launcher 從 Start Menu 啟動時 PATH 不一致導致 ModuleNotFoundError。

**Architecture:** 在 `scanner.rs` 新增純函式 `parse_where_output` / `is_windows_store_stub` / `pick_real_python` 處理 `where python` 結果，包成 `resolve_system_python`。新增 `resolve_python_for_project(path, settings)` 統一決定優先順序：venv → 使用者設定 → 系統 `where python` → fallback `python`。`detect_project` 與 `build_python_command` 改吃 `&Settings`。新增 `detect_project_command` Tauri command 給 UI 重新偵測既有 app。前端 SettingsPanel 加 text input，AppForm 在「指令」欄位旁加「重新偵測」按鈕。

**Tech Stack:** Rust (Tauri v2 backend, sysinfo, serde), TypeScript + React (Vite frontend), Tailwind CSS, Windows-only (CREATE_NO_WINDOW、`where`、venv\Scripts)。

**Working directory assumption:** 所有 `cd` 指令的起點都假設是 `L:\temp\app-launcher`（repo root）。Bash 用 unix 風格 `L:/temp/app-launcher`。

---

## Background

當前 `src-tauri/src/scanner.rs:274-280` 的 `build_python_command` 在沒有 venv 時產生裸 `python <entry>`：

```rust
fn build_python_command(path: &Path, entry: &str) -> String {
    if let Some(venv_python) = find_venv_python(path) {
        format!("\"{}\" {}", venv_python.to_string_lossy(), entry)
    } else {
        format!("python {}", entry)
    }
}
```

裸 `python` 在 spawn 時由 Windows PATH 解析。`cmd /C python ...` 在 Tauri 行程下繼承的是純 Windows 註冊表 PATH（System PATH 第一個 Python 是 `C:\Python314\`），而使用者 CLI 平常用的是 conda base（`C:\Users\JP6\anaconda3\python.exe`，conda activate 會動態注入 PATH）。兩個 Python 套件不同，引發 `ModuleNotFoundError`（2026-04-07 doujin-tagger 的 `pykakasi` 缺失就是這原因）。

繞道方案目前是手動把 command 改成絕對路徑（如 `"C:\Users\JP6\anaconda3\python.exe" app.py`）。本 plan 要根治：scanner 主動解析絕對路徑，並讓使用者可全域設定。

## File Structure

**Rust（修改）：**
- `src-tauri/src/config.rs` — `Settings` struct 新增 `python_interpreter: Option<String>`
- `src-tauri/src/scanner.rs` — 新增 `parse_where_output` / `is_windows_store_stub` / `pick_real_python` / `resolve_system_python` / `resolve_python_for_project`；改 `build_python_command` / `detect_project` 簽章吃 `&Settings`；改 `scan_projects` 傳 `&cfg.settings`
- `src-tauri/src/lib.rs` — 新增 `detect_project_command` Tauri command 並註冊到 `invoke_handler!`

**TypeScript（修改）：**
- `src/types.ts` — `Settings` interface 新增 `pythonInterpreter?: string`
- `src/lib/commands.ts` — 新增 `detectProjectCommand(path)` wrapper
- `src/components/SettingsPanel.tsx` — 新增「預設 Python 直譯器」text input row
- `src/components/AppForm.tsx` — 在「指令」input 旁邊加「重新偵測」按鈕

**版本與文件：**
- `package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json` — 0.2.0 → 0.3.0
- `C:\Users\JP6\.claude\projects\L--temp\memory\project_app_launcher_python_path_todo.md` — 標記為已解決
- `C:\Users\JP6\.claude\projects\L--temp\memory\MEMORY.md` — 移除 TODO 條目

---

## Task 1: 新增 pythonInterpreter 設定欄位（後端）

**Files:**
- Modify: `src-tauri/src/config.rs:41-65`

- [ ] **Step 1: 修改 Settings struct，新增 python_interpreter**

把 `Settings` 改成（保留現有欄位順序）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    pub auto_start_with_windows: bool,
    #[serde(default = "default_true")]
    pub exclude_worktrees: bool,
    /// Optional global override for Python interpreter path. When set,
    /// scanner uses this for Python projects without their own venv.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_interpreter: Option<String>,
}
```

並更新 `Default` impl：

```rust
impl Default for Settings {
    fn default() -> Self {
        Self {
            start_minimized: false,
            close_to_tray: true,
            auto_start_with_windows: false,
            exclude_worktrees: true,
            python_interpreter: None,
        }
    }
}
```

- [ ] **Step 2: 跑既有 config tests，確認沒打壞反序列化**

Run: `cd L:/temp/app-launcher/src-tauri && cargo test config::tests --quiet`
Expected: 全部 PASS（既有測試 `test_add_and_remove_app`、`test_update_app`、`test_save_and_reload`）。`#[serde(default)]` 確保舊的 `apps.json` 沒這欄位也能讀進來。

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: add pythonInterpreter setting field"
```

---

## Task 2: where 輸出解析的純函式 + 測試

**Files:**
- Modify: `src-tauri/src/scanner.rs` (在現有 `find_venv_python` 上方加新函式，在 `mod tests` 加測試)

- [ ] **Step 1: 寫失敗測試**

在 `src-tauri/src/scanner.rs` 的 `mod tests` 區塊（第 419 行附近的 `#[cfg(test)] mod tests { ... }`）加入：

```rust
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
    // Both anaconda and Python314 may not exist on the test machine.
    // pick_real_python uses an existence check, so we test the variant
    // that takes a custom existence predicate.
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
```

The last test calls `pick_real_python` (not `_with`) so the wrapper has a caller in tests, avoiding a `dead_code` warning before Task 3 lands.

- [ ] **Step 2: 跑測試確認失敗**

Run: `cd L:/temp/app-launcher/src-tauri && cargo test scanner::tests::test_parse_where_output_basic --quiet`
Expected: FAIL — `cannot find function 'parse_where_output' in this scope`

- [ ] **Step 3: 實作純函式**

在 `src-tauri/src/scanner.rs` 的 `find_venv_python`（第 261 行）**之前**插入：

```rust
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

fn pick_real_python_with<F>(candidates: Vec<PathBuf>, exists: F) -> Option<PathBuf>
where
    F: Fn(&Path) -> bool,
{
    candidates
        .into_iter()
        .find(|p| !is_windows_store_stub(p) && exists(p))
}
```

- [ ] **Step 4: 跑測試確認通過**

Run: `cd L:/temp/app-launcher/src-tauri && cargo test scanner::tests --quiet`
Expected: 所有 scanner 測試 PASS（含新加的 8 個 + 既有的 `test_decode_project_dir_name` / `test_extract_port`）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "feat: add pure helpers for parsing where python output"
```

---

## Task 3: 系統 Python 解析 wrapper + 專案層解析器

**Files:**
- Modify: `src-tauri/src/scanner.rs`

- [ ] **Step 1: 加入 imports + resolve_system_python**

把 `src-tauri/src/scanner.rs` 第 1 行的 `use crate::config::{AppEntry, ConfigManager};` 改成：

```rust
use crate::config::{AppEntry, ConfigManager, Settings};
```

然後在第 4 行 `use std::path::{Path, PathBuf};` 之後新增：

```rust
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
```

注意：`CREATE_NO_WINDOW` 已在 `process.rs` 定義過一份，scanner 模組獨立使用一份是合理的（避免跨模組依賴）。

在 `pick_real_python_with` 之後新增：

```rust
/// Run `where python` on Windows and return the first usable interpreter.
/// Returns `None` if `where` fails or only finds Windows Store stubs.
fn resolve_system_python() -> Option<PathBuf> {
    let output = Command::new("where")
        .arg("python")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    pick_real_python(parse_where_output(&stdout))
}

/// Resolve which Python executable to use for a given project directory.
///
/// Priority:
/// 1. Project-local venv (`venv\Scripts\python.exe` or `.venv\...`)
/// 2. User-configured `settings.python_interpreter` (if file exists)
/// 3. System `where python` (skipping Windows Store stub)
/// 4. `None` — caller falls back to bare `python`
fn resolve_python_for_project(
    path: &Path,
    settings: &Settings,
) -> Option<PathBuf> {
    if let Some(venv) = find_venv_python(path) {
        return Some(venv);
    }
    if let Some(custom) = settings
        .python_interpreter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let p = PathBuf::from(custom);
        if p.exists() {
            return Some(p);
        }
    }
    resolve_system_python()
}
```

- [ ] **Step 2: 加 resolve_python_for_project 的單元測試**

在 `mod tests` 加入（`Settings` 已經透過 `use super::*` 自動可見，因為 Step 1 已經把 `Settings` 加到 scanner.rs 頂層 use）：

```rust
fn settings_with_python(p: Option<&str>) -> Settings {
    let mut s = Settings::default();
    s.python_interpreter = p.map(|x| x.to_string());
    s
}

#[test]
fn test_resolve_python_for_project_uses_custom_setting_when_exists() {
    // Use a path that definitely exists on the test machine
    let exists_path = std::env::current_exe().unwrap();
    let exists_str = exists_path.to_string_lossy().to_string();
    let tmp = tempfile::tempdir().unwrap();
    let resolved = resolve_python_for_project(
        tmp.path(),
        &settings_with_python(Some(&exists_str)),
    );
    assert_eq!(resolved, Some(exists_path));
}

#[test]
fn test_resolve_python_for_project_skips_custom_when_not_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let resolved = resolve_python_for_project(
        tmp.path(),
        &settings_with_python(Some("C:\\Definitely\\Not\\Real\\python.exe")),
    );
    // Falls through to resolve_system_python — result depends on test machine
    // but it must NOT be the bogus path.
    assert_ne!(
        resolved,
        Some(PathBuf::from("C:\\Definitely\\Not\\Real\\python.exe"))
    );
}

#[test]
fn test_resolve_python_for_project_prefers_venv_over_setting() {
    let tmp = tempfile::tempdir().unwrap();
    let venv_python = tmp.path().join("venv").join("Scripts").join("python.exe");
    std::fs::create_dir_all(venv_python.parent().unwrap()).unwrap();
    std::fs::write(&venv_python, b"fake").unwrap();
    let exists_path = std::env::current_exe().unwrap();
    let resolved = resolve_python_for_project(
        tmp.path(),
        &settings_with_python(Some(&exists_path.to_string_lossy())),
    );
    assert_eq!(resolved, Some(venv_python));
}
```

`tempfile` 已經是 `dev-dependencies`（見 `src-tauri/Cargo.toml:32`），不用再加。

- [ ] **Step 3: 跑測試確認通過**

Run: `cd L:/temp/app-launcher/src-tauri && cargo test scanner::tests --quiet`
Expected: 所有 scanner 測試 PASS（含 3 個新測試）。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "feat: add resolve_python_for_project with venv/settings/system priority"
```

---

## Task 4: build_python_command 與 detect_project 改吃 Settings

**Files:**
- Modify: `src-tauri/src/scanner.rs:135-280`

- [ ] **Step 1: 改 build_python_command 簽章**

把 `src-tauri/src/scanner.rs` 第 273-280 行的 `build_python_command` 整段換成：

```rust
/// Build the Python command for a project, using the resolved interpreter
/// (venv > settings.python_interpreter > system `where python` > bare `python`).
fn build_python_command(path: &Path, entry: &str, settings: &Settings) -> String {
    match resolve_python_for_project(path, settings) {
        Some(python) => format!("\"{}\" {}", python.to_string_lossy(), entry),
        None => format!("python {}", entry),
    }
}
```

- [ ] **Step 2: 改 detect_project 簽章與內部呼叫**

把第 134-259 行 `detect_project` 的簽章改為：

```rust
pub fn detect_project(path: &Path, settings: &Settings) -> Option<ScanCandidate> {
```

並把內部兩個 `build_python_command(path, &entry)` 呼叫（約第 188、201 行）改成：

```rust
let command = build_python_command(path, &entry, settings);
```

- [ ] **Step 3: 改 scan_projects 把 settings 傳下去**

把第 403 行附近的：

```rust
if let Some(candidate) = detect_project(&project_path) {
```

改成：

```rust
if let Some(candidate) = detect_project(&project_path, &cfg.settings) {
```

- [ ] **Step 4: cargo build 確認 type-check 通過**

Run: `cd L:/temp/app-launcher/src-tauri && cargo build 2>&1 | tail -20`
Expected: `Finished` 沒有 error。可能會有 unused warnings，但不該有 error。

- [ ] **Step 5: 跑全套測試（scanner + config）**

Run: `cd L:/temp/app-launcher/src-tauri && cargo test --quiet`
Expected: 所有測試 PASS。注意 `test_decode_project_dir_name` 等舊測試不受影響。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/scanner.rs
git commit -m "refactor: thread Settings through detect_project and build_python_command"
```

---

## Task 5: 新增 detect_project_command Tauri command

**Files:**
- Modify: `src-tauri/src/lib.rs:62-64, 270-284`

- [ ] **Step 1: 擴充 scanner import + 加入 command 函式**

把 `src-tauri/src/lib.rs` 第 7 行：

```rust
use scanner::{candidate_to_app, scan_projects as do_scan_projects, ScanCandidate};
```

改成（多 import 一個 `detect_project`）：

```rust
use scanner::{candidate_to_app, detect_project, scan_projects as do_scan_projects, ScanCandidate};
```

然後在第 64 行 `check_path_exists` 之後插入：

```rust
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
```

- [ ] **Step 2: 註冊到 invoke_handler**

把第 270-284 行的 `invoke_handler!` 巨集改成：

```rust
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
```

- [ ] **Step 3: cargo build 確認通過**

Run: `cd L:/temp/app-launcher/src-tauri && cargo build 2>&1 | tail -20`
Expected: `Finished` 沒有 error。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: add detect_project_command tauri command for re-detection"
```

---

## Task 6: 前端 types + commands wrapper

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/commands.ts`

- [ ] **Step 1: 新增 pythonInterpreter 到 Settings type**

把 `src/types.ts:16-21` 的 `Settings` interface 改成：

```typescript
export interface Settings {
  startMinimized: boolean;
  closeToTray: boolean;
  autoStartWithWindows: boolean;
  excludeWorktrees: boolean;
  pythonInterpreter?: string;
}
```

- [ ] **Step 2: 新增 detectProjectCommand wrapper**

在 `src/lib/commands.ts:40` 的 `checkPathExists` 之後加入：

```typescript
export function detectProjectCommand(path: string): Promise<string> {
  return invoke<string>("detect_project_command", { path });
}
```

- [ ] **Step 3: 跑 type check**

Run: `cd L:/temp/app-launcher && npx tsc --noEmit 2>&1 | tail -20`
Expected: 沒有 type error。

- [ ] **Step 4: Commit**

```bash
git add src/types.ts src/lib/commands.ts
git commit -m "feat: add pythonInterpreter setting type and detect command wrapper"
```

---

## Task 7: SettingsPanel — 預設 Python 直譯器輸入欄

**Files:**
- Modify: `src/components/SettingsPanel.tsx`

- [ ] **Step 1: 在 ToggleRow 旁邊定義一個 TextRow 元件**

在 `src/components/SettingsPanel.tsx` 的 `ToggleRow` 函式（第 15-36 行）之後加入：

```typescript
interface TextRowProps {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

function TextRow({ label, value, placeholder, onChange }: TextRowProps) {
  return (
    <div className="flex flex-col gap-1 py-2">
      <span className="text-sm text-text-primary">{label}</span>
      <input
        type="text"
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="h-8 rounded bg-surface-0 px-2 text-xs font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
      />
    </div>
  );
}
```

- [ ] **Step 2: 在「掃描」section 之後加新 section「Python」**

在第 105-111 行的「掃描」section（含 `ToggleRow excludeWorktrees`）之後插入：

```typescript
          <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mt-6 mb-2">
            Python
          </h3>
          <div className="flex flex-col">
            <TextRow
              label="預設 Python 直譯器"
              value={settings.pythonInterpreter ?? ""}
              placeholder="C:\Users\xxx\anaconda3\python.exe"
              onChange={(v) =>
                onUpdate({
                  ...settings,
                  pythonInterpreter: v.trim() === "" ? undefined : v,
                })
              }
            />
            <span className="text-[10px] text-text-secondary mt-1">
              留空則自動以 <code className="font-mono">where python</code> 解析。新偵測或重新偵測 Python 專案時生效。
            </span>
          </div>
```

- [ ] **Step 3: 確認 type check 與 build**

Run: `cd L:/temp/app-launcher && npx tsc --noEmit 2>&1 | tail -10`
Expected: 沒有 type error。

- [ ] **Step 4: Commit**

```bash
git add src/components/SettingsPanel.tsx
git commit -m "feat: add Python interpreter input to settings panel"
```

---

## Task 8: AppForm — 重新偵測指令按鈕

**Files:**
- Modify: `src/components/AppForm.tsx`

- [ ] **Step 1: 加 import 與 state**

在 `src/components/AppForm.tsx:1-2` 的 import 區加入 `detectProjectCommand`：

```typescript
import { useState } from "react";
import type { AppEntry, AppType } from "../types";
import { detectProjectCommand } from "../lib/commands";
```

在 `AppForm` 函式內，現有 `useState` 區塊之後（第 33 行 `autoStart` 之後）加入：

```typescript
  const [redetectError, setRedetectError] = useState<string | null>(null);
  const [redetecting, setRedetecting] = useState(false);

  const handleRedetect = async () => {
    if (!path.trim()) {
      setRedetectError("請先填入路徑");
      return;
    }
    setRedetecting(true);
    setRedetectError(null);
    try {
      const detected = await detectProjectCommand(path.trim());
      setCommand(detected);
    } catch (err) {
      setRedetectError(String(err));
    } finally {
      setRedetecting(false);
    }
  };
```

- [ ] **Step 2: 在「指令」input 旁邊加按鈕**

把 `src/components/AppForm.tsx` 第 117-129 行的「指令」整個 div 換成：

```typescript
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              指令
            </label>
            <div className="flex items-center gap-2">
              <input
                type="text"
                required
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="npm run dev"
                className="flex-1 h-8 rounded bg-surface-0 px-2 text-sm font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
              />
              <button
                type="button"
                onClick={handleRedetect}
                disabled={redetecting}
                className="h-8 px-2 rounded bg-surface-2 text-xs text-text-primary cursor-pointer hover:bg-surface-0 disabled:opacity-50 disabled:cursor-not-allowed transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
              >
                {redetecting ? "偵測中…" : "重新偵測"}
              </button>
            </div>
            {redetectError && (
              <span className="text-[10px] text-error">{redetectError}</span>
            )}
          </div>
```

- [ ] **Step 3: 跑 type check**

Run: `cd L:/temp/app-launcher && npx tsc --noEmit 2>&1 | tail -10`
Expected: 沒有 type error。

- [ ] **Step 4: Commit**

```bash
git add src/components/AppForm.tsx
git commit -m "feat: add re-detect command button to app form"
```

---

## Task 9: 版本號 0.2.0 → 0.3.0、整合測試、清理 memory

**Files:**
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml:3`
- Modify: `src-tauri/tauri.conf.json:4`
- Modify: `C:\Users\JP6\.claude\projects\L--temp\memory\project_app_launcher_python_path_todo.md`
- Modify: `C:\Users\JP6\.claude\projects\L--temp\memory\MEMORY.md`

- [ ] **Step 1: 把三個版本號改成 0.3.0**

在 `package.json` 找到 `"version": "0.2.0"` 改成 `"version": "0.3.0"`。
在 `src-tauri/Cargo.toml:3` 把 `version = "0.2.0"` 改成 `version = "0.3.0"`。
在 `src-tauri/tauri.conf.json:4` 把 `"version": "0.2.0"` 改成 `"version": "0.3.0"`。

- [ ] **Step 2: build 整套確認沒壞**

Run（清掉 CI 環境變數，跟之前一樣的眉角）：
```bash
cd L:/temp/app-launcher && unset CI && npm run build 2>&1 | tail -20
```
Expected: Vite build 成功，看到 `dist/` 產出。

接著跑 Rust 測試：
```bash
cd L:/temp/app-launcher/src-tauri && cargo test --quiet
```
Expected: 所有 scanner / config 測試 PASS。

- [ ] **Step 3: 手動 smoke test（dev 模式）**

```bash
cd L:/temp/app-launcher && unset CI && npm run tauri dev
```

驗證項目：
1. 開啟 launcher → 設定面板 → 確認看到「Python」section 跟新欄位
2. 在 Python 直譯器欄位填入 `C:\Users\JP6\anaconda3\python.exe` → 關閉 → apps.json 應該寫入 `pythonInterpreter` 欄位
3. 編輯 doujin-tagger → 路徑保留 `L:\doujin-tagger` → 按「重新偵測」→ 指令應該變成 `"C:\Users\JP6\anaconda3\python.exe" app.py`
4. 儲存 → 啟動 doujin-tagger → log panel 應該看到 Flask 啟動訊息（沒有 ModuleNotFoundError）
5. 清空設定的 Python 直譯器 → 編輯一個沒 venv 的新 Python 專案 → 重新偵測 → 應該看到 `where python` 解析出來的絕對路徑（**不能**是 WindowsApps 的 stub）

如果項目 5 解出來的是 `C:\Python314\python.exe`（系統 PATH 第一個），代表 fallback 順序正確；如果使用者想用 conda，在 Settings 設定即可。

- [ ] **Step 4: 更新 memory — 標記 TODO 已解決**

把 `C:\Users\JP6\.claude\projects\L--temp\memory\project_app_launcher_python_path_todo.md` 整份內容改成（保留檔案，標記為已解決，當作歷史紀錄）：

```markdown
---
name: App Launcher Python PATH 解析改進（已解決）
description: [已解決 2026-04-07] App Launcher scanner 偵測 Python 專案時，現已自動把 `python` 解析成絕對路徑寫進 command。優先順序：venv > settings.pythonInterpreter > where python（避開 Windows Store stub）> 裸 python。
type: project
---

# App Launcher Python PATH 解析改進（已解決於 0.3.0）

**狀態：** 已實作於 v0.3.0（2026-04-07）。

**原問題：** scanner 產生裸 `python xxx.py`，使用者從 CLI 跟從 Start Menu 啟動 launcher 時的 PATH 不一致，導致 ModuleNotFoundError。

**實作摘要：**
- `scanner.rs::resolve_python_for_project(path, settings)` 統一處理優先順序：venv → 使用者設定 → `where python`（避開 `\AppData\Local\Microsoft\WindowsApps\` stub） → fallback 裸 `python`
- `Settings.python_interpreter: Option<String>` 全域設定欄位，UI 在 SettingsPanel
- `detect_project_command` Tauri command 支援既有 app 透過 AppForm 的「重新偵測」按鈕重跑偵測
- 既有 apps 不會自動遷移；需要時手動按「重新偵測」

**Why:** 避免使用者每次新增 Python 專案都要手動把 command 改成絕對路徑。
**How to apply:** 之後只要在 launcher 的設定一次設好預設 Python 直譯器，所有沒 venv 的新專案都會自動用該直譯器；既有專案編輯後按「重新偵測」即可同步。
```

把 `C:\Users\JP6\.claude\projects\L--temp\memory\MEMORY.md` 對應的條目改成：

```markdown
- [App Launcher Python PATH（已解決）](project_app_launcher_python_path_todo.md) — v0.3.0 已實作 venv→設定→`where python`→fallback 解析鏈
```

- [ ] **Step 5: Commit**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore: bump version to 0.3.0"
```

memory 檔案不在 git repo 內，不需要 commit。

---

## 後續（不在本 plan 內）

以下項目刻意排除，留給未來迭代：

1. **既有 app 自動遷移**：太 magic，容易覆蓋使用者手動微調的 command。讓使用者主動按「重新偵測」即可。
2. **Browse 按鈕**：SettingsPanel 的「預設 Python 直譯器」目前只是 text input。若要加 file picker，需引入 `tauri-plugin-dialog`，是另一個獨立功能。
3. **解析結果快取**：`resolve_system_python` 每次呼叫都 spawn `where`，若效能成問題再考慮，目前只在掃描/重新偵測時呼叫。
4. **macOS / Linux 支援**：本 app 現在 Windows-only（`os::windows::process::CommandExt`、`venv\Scripts`），跨平台需另開 plan。
