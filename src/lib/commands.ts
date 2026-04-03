import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  AppEntry,
  ProcessState,
  ScanCandidate,
  Settings,
} from "../types";

export function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export function addApp(app: AppEntry): Promise<void> {
  return invoke("add_app", { app });
}

export function updateApp(app: AppEntry): Promise<void> {
  return invoke("update_app", { app });
}

export function removeApp(id: string): Promise<void> {
  return invoke("remove_app", { id });
}

export function updateSettings(settings: Settings): Promise<void> {
  return invoke("update_settings", { settings });
}

export function scanProjects(): Promise<ScanCandidate[]> {
  return invoke<ScanCandidate[]>("scan_projects");
}

export function addScannedApps(candidates: ScanCandidate[]): Promise<void> {
  return invoke("add_scanned_apps", { candidates });
}

export function checkPathExists(path: string): Promise<boolean> {
  return invoke<boolean>("check_path_exists", { path });
}

export function startApp(id: string): Promise<void> {
  return invoke("start_app", { id });
}

export function stopApp(id: string): Promise<void> {
  return invoke("stop_app", { id });
}

export function getRunningApps(): Promise<string[]> {
  return invoke<string[]>("get_running_apps");
}

export function detectRunning(): Promise<ProcessState[]> {
  return invoke<ProcessState[]>("detect_running");
}

export function stopAllApps(): Promise<void> {
  return invoke("stop_all_apps");
}
