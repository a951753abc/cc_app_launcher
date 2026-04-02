import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { LogLine } from "../types";

const MAX_LINES = 500;

export function useLogs() {
  const [logs, setLogs] = useState<Record<string, LogLine[]>>({});
  const [activeAppId, setActiveAppId] = useState<string | null>(null);
  const logsRef = useRef(logs);
  logsRef.current = logs;

  useEffect(() => {
    const unlisten = listen<LogLine>("process-log", (event) => {
      const line = event.payload;
      setLogs((prev) => {
        const existing = prev[line.appId] ?? [];
        const updated =
          existing.length >= MAX_LINES
            ? [...existing.slice(existing.length - MAX_LINES + 1), line]
            : [...existing, line];
        return { ...prev, [line.appId]: updated };
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const clearLogs = useCallback((appId: string) => {
    setLogs((prev) => {
      const next = { ...prev };
      delete next[appId];
      return next;
    });
  }, []);

  const openLog = useCallback((appId: string) => {
    setActiveAppId(appId);
  }, []);

  const closeLog = useCallback(() => {
    setActiveAppId(null);
  }, []);

  return {
    logs,
    activeAppId,
    activeLines: activeAppId ? (logs[activeAppId] ?? []) : [],
    clearLogs,
    openLog,
    closeLog,
  };
}
