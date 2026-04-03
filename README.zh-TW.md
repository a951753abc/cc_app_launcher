# App Launcher

輕量級桌面應用程式，用來統一管理與啟動散落各處的本機專案。以 Tauri v2（Rust）+ React + TypeScript 打造。

## 為什麼需要

透過 Claude Code 建立許多小專案後，它們散落在各處資料夾，常常忘記去哪裡啟動哪個程式。所以決定做一個方便一覽跟啟動的小工具。

## 功能

- **一鍵啟動/停止** — 用設定好的指令啟動任何專案（`npm run dev`、`python app.py`、`cargo run` 等）
- **即時日誌串流** — stdout/stderr 導向可收合的日誌面板
- **自動掃描** — 從 `.claude/projects/` 目錄自動發現專案（以 greedy 路徑解析處理編碼名稱歧義）
- **外部程序偵測** — 偵測已在 App Launcher 外部執行的服務：
  - TCP port 檢測（適用 web 應用）
  - Process name 比對（透過 sysinfo，適用背景服務如 `pythonw.exe`）
  - 工作目錄 / 命令列比對（兜底方案）
- **系統匣** — 最小化至系統匣，可從匣選單啟動/停止全部
- **手動新增/編輯** — 自訂指令、類型、port 手動加入任何專案
- **專案分類** — Web、CLI、GUI、Script，搭配分類篩選標籤

## 專案結構

```
src-tauri/src/
├── config.rs      # AppEntry、Settings、ConfigManager（JSON 持久化 + 檔案監聽）
├── process.rs     # ProcessManager（啟動、終止、日誌串流、外部偵測）
├── scanner.rs     # 自動掃描 .claude/projects/、偵測專案類型
└── lib.rs         # Tauri commands、系統匣、應用程式設定

src/
├── components/    # TitleBar、AppList、AppRow、LogPanel、AppForm、SettingsPanel、ScanResults
├── hooks/         # useApps、useProcesses、useLogs
├── lib/commands.ts
└── types.ts
```

## 開始使用

### 前置需求

- [Rust](https://rustup.rs/)（1.70+）
- [Node.js](https://nodejs.org/)（18+）
- Tauri CLI：`cargo install tauri-cli --version "^2"`

### 開發模式

```bash
npm install
npm run tauri dev
```

### 正式建置

```bash
npm run tauri build
```

安裝檔輸出至 `src-tauri/target/release/bundle/`（NSIS + MSI）。

## 設定

應用程式資料儲存於 `%APPDATA%/app-launcher/apps.json`，可直接編輯或透過應用程式內的設定面板操作。

### App Entry 欄位

| 欄位 | 說明 |
|------|------|
| `name` | 顯示名稱 |
| `path` | 工作目錄 |
| `command` | 執行的 shell 指令 |
| `type` | `web` \| `cli` \| `gui` \| `script` |
| `port` | （選填）web 應用的 port |
| `processName` | （選填）外部偵測用的 process name |
| `autoStart` | App Launcher 開啟時自動啟動 |

## 授權

MIT
