import { useState, useRef, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import type { AppEntry, AppType } from "./types";
import { useApps } from "./hooks/useApps";
import { useProcesses } from "./hooks/useProcesses";
import { useLogs } from "./hooks/useLogs";
import { TitleBar } from "./components/TitleBar";
import { AppList } from "./components/AppList";
import { LogPanel } from "./components/LogPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { AppForm } from "./components/AppForm";
import { ScanResults } from "./components/ScanResults";

type FilterType = "all" | AppType;

function App() {
  const { apps, settings, loading, addApp, updateApp, removeApp, updateSettings, reload } =
    useApps();
  const { startApp, stopApp, getStatus } = useProcesses();
  const { activeAppId, activeLines, clearLogs, openLog, closeLog } = useLogs();

  const [searchQuery, setSearchQuery] = useState("");
  const [filterType, setFilterType] = useState<FilterType>("all");
  const [showSettings, setShowSettings] = useState(false);
  const [editingApp, setEditingApp] = useState<AppEntry | null>(null);
  const [showScan, setShowScan] = useState(false);
  const [showAddForm, setShowAddForm] = useState(false);

  const autoStarted = useRef(false);

  // Auto-start apps with autoStart=true on first load
  useEffect(() => {
    if (loading || autoStarted.current) return;
    autoStarted.current = true;
    for (const app of apps) {
      if (app.autoStart) {
        startApp(app.id).catch(() => {
          /* ignore start errors on auto-start */
        });
      }
    }
  }, [loading, apps, startApp]);

  // Listen for tray "start all" event
  useEffect(() => {
    const unlisten = listen("tray-start-all", () => {
      for (const app of apps) {
        startApp(app.id).catch(() => {
          /* ignore */
        });
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [apps, startApp]);

  const handleSaveApp = useCallback(
    async (app: AppEntry) => {
      const exists = apps.some((a) => a.id === app.id);
      if (exists) {
        await updateApp(app);
      } else {
        await addApp(app);
      }
      setEditingApp(null);
      setShowAddForm(false);
    },
    [apps, addApp, updateApp],
  );

  const handleDeleteApp = useCallback(
    async (id: string) => {
      await removeApp(id);
      setEditingApp(null);
    },
    [removeApp],
  );

  const handleScanDone = useCallback(() => {
    setShowScan(false);
    reload();
  }, [reload]);

  const activeAppName =
    activeAppId !== null
      ? (apps.find((a) => a.id === activeAppId)?.name ?? activeAppId)
      : "";

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-surface-0">
        <span className="text-sm text-text-secondary">載入中...</span>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TitleBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        onAddClick={() => setShowAddForm(true)}
        onScanClick={() => setShowScan(true)}
        onSettingsClick={() => setShowSettings(true)}
      />

      <AppList
        apps={apps}
        searchQuery={searchQuery}
        filterType={filterType}
        onFilterChange={setFilterType}
        getStatus={getStatus}
        onStart={startApp}
        onStop={stopApp}
        onViewLog={openLog}
        onEdit={setEditingApp}
      />

      {activeAppId !== null && (
        <LogPanel
          appName={activeAppName}
          lines={activeLines}
          onClear={() => clearLogs(activeAppId)}
          onClose={closeLog}
        />
      )}

      {showSettings && settings && (
        <SettingsPanel
          settings={settings}
          onUpdate={updateSettings}
          onClose={() => setShowSettings(false)}
        />
      )}

      {(showAddForm || editingApp !== null) && (
        <AppForm
          app={editingApp}
          onSave={handleSaveApp}
          onDelete={editingApp ? handleDeleteApp : undefined}
          onClose={() => {
            setEditingApp(null);
            setShowAddForm(false);
          }}
        />
      )}

      {showScan && (
        <ScanResults
          onClose={() => setShowScan(false)}
          onDone={handleScanDone}
        />
      )}
    </div>
  );
}

export default App;
