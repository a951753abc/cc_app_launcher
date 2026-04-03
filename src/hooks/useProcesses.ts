import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ProcessState, ProcessStatus } from "../types";
import {
  startApp as cmdStartApp,
  stopApp as cmdStopApp,
  getRunningApps,
  detectRunning,
  stopAllApps as cmdStopAllApps,
} from "../lib/commands";

export function useProcesses() {
  const [statusMap, setStatusMap] = useState<Record<string, ProcessStatus>>({});

  const refreshExternal = useCallback(async () => {
    const states = await detectRunning();
    setStatusMap((prev) => {
      const next = { ...prev };
      // Clear previous "external" entries (they may no longer be running)
      for (const [id, st] of Object.entries(next)) {
        if (st === "external") delete next[id];
      }
      // Merge freshly detected externals
      for (const s of states) {
        next[s.appId] = s.status;
      }
      return next;
    });
  }, []);

  useEffect(() => {
    // 1. Load managed processes
    getRunningApps().then((ids) => {
      const map: Record<string, ProcessStatus> = {};
      for (const id of ids) {
        map[id] = "running";
      }
      setStatusMap(map);
    });

    // 2. Detect externally-running processes (port check)
    refreshExternal();

    // 3. Poll every 30s to catch external start/stop
    const interval = setInterval(refreshExternal, 30_000);

    // 4. Listen for status events from managed processes
    const unlisten = listen<ProcessState>("process-status", (event) => {
      const { appId, status } = event.payload;
      setStatusMap((prev) => ({ ...prev, [appId]: status }));
    });

    return () => {
      clearInterval(interval);
      unlisten.then((fn) => fn());
    };
  }, [refreshExternal]);

  const startApp = useCallback(async (id: string) => {
    await cmdStartApp(id);
  }, []);

  const stopApp = useCallback(async (id: string) => {
    await cmdStopApp(id);
  }, []);

  const stopAllApps = useCallback(async () => {
    await cmdStopAllApps();
  }, []);

  const getStatus = useCallback(
    (id: string): ProcessStatus => statusMap[id] ?? "stopped",
    [statusMap],
  );

  return { statusMap, startApp, stopApp, stopAllApps, getStatus };
}
