export type AppType = "web" | "cli" | "gui" | "script";
export type FilterType = "all" | AppType;

export interface AppEntry {
  id: string;
  name: string;
  path: string;
  command: string;
  type: AppType;
  port?: number;
  processName?: string;
  autoStart: boolean;
  tags: string[];
}

export interface Settings {
  startMinimized: boolean;
  closeToTray: boolean;
  autoStartWithWindows: boolean;
  excludeWorktrees: boolean;
  pythonInterpreter?: string;
}

export interface AppConfig {
  apps: AppEntry[];
  scanPaths: string[];
  extraScanPaths: string[];
  settings: Settings;
}

export interface ScanCandidate {
  name: string;
  path: string;
  command: string;
  appType: string;
  port?: number;
}

export type ProcessStatus = "stopped" | "running" | "error" | "external";

export interface ProcessState {
  appId: string;
  status: ProcessStatus;
}

export interface LogLine {
  appId: string;
  line: string;
  isStderr: boolean;
  timestamp: number;
}
