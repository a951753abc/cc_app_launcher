import { useState, useEffect } from "react";
import type { AppEntry, ProcessStatus } from "../types";
import { checkPathExists } from "../lib/commands";
import { openUrl, openPath } from "@tauri-apps/plugin-opener";

interface AppRowProps {
  app: AppEntry;
  status: ProcessStatus;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onViewLog: (id: string) => void;
  onEdit: (app: AppEntry) => void;
}

const typeBadgeColors: Record<string, string> = {
  web: "text-accent",
  cli: "text-text-secondary",
  gui: "text-running",
  script: "text-error",
};

export function AppRow({
  app,
  status,
  onStart,
  onStop,
  onViewLog,
  onEdit,
}: AppRowProps) {
  const [pathExists, setPathExists] = useState(true);
  const [showMenu, setShowMenu] = useState(false);

  useEffect(() => {
    checkPathExists(app.path).then(setPathExists);
  }, [app.path]);

  const isRunning = status === "running";
  const isError = status === "error";

  const borderColor = isRunning
    ? "border-l-running"
    : isError
      ? "border-l-error"
      : "border-l-transparent";

  const handleOpenFolder = async () => {
    try {
      await openPath(app.path);
    } catch {
      /* folder might not exist */
    }
  };

  const handleOpenBrowser = async () => {
    if (app.port) {
      try {
        await openUrl(`http://localhost:${app.port}`);
      } catch {
        /* browser open failed */
      }
    }
  };

  return (
    <div
      className={`group flex items-center gap-3 px-3 py-2 border-l-2 ${borderColor} hover:bg-surface-2 transition-colors duration-150 relative`}
    >
      {/* Status dot */}
      <div className="shrink-0">
        {isRunning ? (
          <span className="relative flex h-2.5 w-2.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-running opacity-75" />
            <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-running" />
          </span>
        ) : isError ? (
          <span className="inline-flex h-2.5 w-2.5 rounded-full bg-error" />
        ) : (
          <span className="inline-flex h-2.5 w-2.5 rounded-full bg-stopped" />
        )}
      </div>

      {/* Info */}
      <div className="flex flex-col min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-text-primary truncate">
            {app.name}
          </span>
          <span
            className={`text-[10px] font-mono uppercase ${typeBadgeColors[app.type] ?? "text-text-secondary"}`}
          >
            {app.type}
          </span>
          {app.port != null && (
            <span className="text-[10px] font-mono text-text-secondary">
              :{app.port}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <span className="text-xs font-mono text-text-secondary truncate">
            {app.path}
          </span>
          {!pathExists && (
            <span title="路徑不存在" className="shrink-0">
              <svg
                className="w-3 h-3 text-error"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
              </svg>
            </span>
          )}
        </div>
      </div>

      {/* Hover actions */}
      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
        {app.type === "web" && isRunning && app.port != null && (
          <button
            onClick={handleOpenBrowser}
            className="flex items-center justify-center w-6 h-6 rounded text-text-secondary hover:text-text-primary hover:bg-surface-1 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
            title="在瀏覽器中開啟"
          >
            <svg
              className="w-3.5 h-3.5"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="10" />
              <line x1="2" y1="12" x2="22" y2="12" />
              <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
            </svg>
          </button>
        )}

        <button
          onClick={() => onViewLog(app.id)}
          className="flex items-center justify-center w-6 h-6 rounded text-text-secondary hover:text-text-primary hover:bg-surface-1 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
          title="檢視日誌"
        >
          <svg
            className="w-3.5 h-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
          </svg>
        </button>

        <button
          onClick={handleOpenFolder}
          className="flex items-center justify-center w-6 h-6 rounded text-text-secondary hover:text-text-primary hover:bg-surface-1 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
          title="開啟資料夾"
        >
          <svg
            className="w-3.5 h-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </button>
      </div>

      {/* Start/Stop button - always visible */}
      <button
        onClick={() => (isRunning ? onStop(app.id) : onStart(app.id))}
        disabled={!pathExists && !isRunning}
        className={`flex items-center justify-center w-7 h-7 rounded shrink-0 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2 disabled:opacity-40 disabled:cursor-not-allowed ${
          isRunning
            ? "text-error hover:bg-error/10"
            : "text-running hover:bg-running/10"
        }`}
        title={isRunning ? "停止" : "啟動"}
      >
        {isRunning ? (
          <svg
            className="w-4 h-4"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <rect x="6" y="4" width="4" height="16" rx="1" />
            <rect x="14" y="4" width="4" height="16" rx="1" />
          </svg>
        ) : (
          <svg
            className="w-4 h-4"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <polygon points="5 3 19 12 5 21 5 3" />
          </svg>
        )}
      </button>

      {/* More menu */}
      <div className="relative shrink-0">
        <button
          onClick={() => setShowMenu(!showMenu)}
          className="flex items-center justify-center w-6 h-6 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 cursor-pointer transition-colors duration-150 opacity-0 group-hover:opacity-100 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
          title="更多"
        >
          <svg
            className="w-3.5 h-3.5"
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <circle cx="12" cy="5" r="1.5" />
            <circle cx="12" cy="12" r="1.5" />
            <circle cx="12" cy="19" r="1.5" />
          </svg>
        </button>

        {showMenu && (
          <>
            <div
              className="fixed inset-0 z-10"
              onClick={() => setShowMenu(false)}
            />
            <div className="absolute right-0 top-7 z-20 bg-surface-1 border border-surface-2 rounded shadow-lg py-1 min-w-[120px]">
              <button
                onClick={() => {
                  setShowMenu(false);
                  onEdit(app);
                }}
                className="w-full text-left px-3 py-1.5 text-xs text-text-primary hover:bg-surface-2 cursor-pointer transition-colors duration-150"
              >
                編輯
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
