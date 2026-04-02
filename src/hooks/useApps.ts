import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { AppConfig, AppEntry, Settings } from "../types";
import {
  getConfig,
  addApp as cmdAddApp,
  updateApp as cmdUpdateApp,
  removeApp as cmdRemoveApp,
  updateSettings as cmdUpdateSettings,
} from "../lib/commands";

export function useApps() {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    try {
      const cfg = await getConfig();
      setConfig(cfg);
    } catch {
      /* config unavailable */
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    reload();

    const unlisten = listen("config-changed", () => {
      reload();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [reload]);

  const addApp = useCallback(
    async (app: AppEntry) => {
      await cmdAddApp(app);
      await reload();
    },
    [reload],
  );

  const updateApp = useCallback(
    async (app: AppEntry) => {
      await cmdUpdateApp(app);
      await reload();
    },
    [reload],
  );

  const removeApp = useCallback(
    async (id: string) => {
      await cmdRemoveApp(id);
      await reload();
    },
    [reload],
  );

  const updateSettings = useCallback(
    async (settings: Settings) => {
      await cmdUpdateSettings(settings);
      await reload();
    },
    [reload],
  );

  return {
    config,
    apps: config?.apps ?? [],
    settings: config?.settings ?? null,
    loading,
    reload,
    addApp,
    updateApp,
    removeApp,
    updateSettings,
  };
}
