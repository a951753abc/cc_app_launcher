import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ProcessState, ProcessStatus } from "../types";
import {
  startApp as cmdStartApp,
  stopApp as cmdStopApp,
  getRunningApps,
  stopAllApps as cmdStopAllApps,
} from "../lib/commands";

export function useProcesses() {
  const [statusMap, setStatusMap] = useState<Record<string, ProcessStatus>>({});

  useEffect(() => {
    getRunningApps().then((ids) => {
      const map: Record<string, ProcessStatus> = {};
      for (const id of ids) {
        map[id] = "running";
      }
      setStatusMap(map);
    });

    const unlisten = listen<ProcessState>("process-status", (event) => {
      const { appId, status } = event.payload;
      setStatusMap((prev) => ({ ...prev, [appId]: status }));
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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
