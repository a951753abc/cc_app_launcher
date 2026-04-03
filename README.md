# App Launcher

A lightweight desktop application for managing and launching multiple local projects from a single interface. Built with Tauri v2 (Rust) + React + TypeScript.

## Why

After building many small projects with Claude Code, they end up scattered across various folders, and it's easy to forget where to start each one. So this tool was made to provide a quick overview and one-click launch.

## Features

- **One-click start/stop** — launch any project with its configured command (`npm run dev`, `python app.py`, `cargo run`, etc.)
- **Real-time log streaming** — stdout/stderr piped to a collapsible log panel
- **Auto-scan** — discovers projects from `.claude/projects/` directory (with greedy path resolution for encoded names)
- **External process detection** — detects services already running outside the app via:
  - TCP port check (for web apps)
  - Process name matching via sysinfo (for background services like `pythonw.exe`)
  - Working directory / command-line matching (fallback)
- **System tray** — minimize to tray, start/stop all from tray menu
- **Manual add/edit** — add any project manually with custom command, type, and port
- **Project types** — Web, CLI, GUI, Script — with per-type filtering tabs

## Project Structure

```
src-tauri/src/
├── config.rs      # AppEntry, Settings, ConfigManager (JSON persistence + file watch)
├── process.rs     # ProcessManager (spawn, kill, log streaming, external detection)
├── scanner.rs     # Auto-scan .claude/projects/, detect project types
└── lib.rs         # Tauri commands, system tray, app setup

src/
├── components/    # TitleBar, AppList, AppRow, LogPanel, AppForm, SettingsPanel, ScanResults
├── hooks/         # useApps, useProcesses, useLogs
├── lib/commands.ts
└── types.ts
```

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- Tauri CLI: `cargo install tauri-cli --version "^2"`

### Development

```bash
npm install
npm run tauri dev
```

### Production Build

```bash
npm run tauri build
```

Installers are output to `src-tauri/target/release/bundle/` (NSIS + MSI).

## Configuration

App data is stored at `%APPDATA%/app-launcher/apps.json`. You can edit it directly or use the in-app settings panel.

### App Entry Fields

| Field         | Description                                       |
|---------------|---------------------------------------------------|
| `name`        | Display name                                      |
| `path`        | Working directory                                 |
| `command`     | Shell command to run                              |
| `type`        | `web` \| `cli` \| `gui` \| `script`              |
| `port`        | (Optional) Port number for web apps               |
| `processName` | (Optional) Process name for external detection    |
| `autoStart`   | Auto-start when App Launcher opens                |

## License

MIT
