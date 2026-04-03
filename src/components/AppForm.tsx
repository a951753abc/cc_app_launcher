import { useState } from "react";
import type { AppEntry, AppType } from "../types";

interface AppFormProps {
  app: AppEntry | null;
  onSave: (app: AppEntry) => void;
  onDelete?: (id: string) => void;
  onClose: () => void;
}

const APP_TYPES: { value: AppType; label: string }[] = [
  { value: "web", label: "Web" },
  { value: "cli", label: "CLI" },
  { value: "gui", label: "GUI" },
  { value: "script", label: "Script" },
];

function newId(): string {
  return crypto.randomUUID();
}

export function AppForm({ app, onSave, onDelete, onClose }: AppFormProps) {
  const isEdit = app !== null;

  const [name, setName] = useState(app?.name ?? "");
  const [path, setPath] = useState(app?.path ?? "");
  const [command, setCommand] = useState(app?.command ?? "");
  const [type, setType] = useState<AppType>(app?.type ?? "web");
  const [port, setPort] = useState<string>(
    app?.port != null ? String(app.port) : "",
  );
  const [processName, setProcessName] = useState(app?.processName ?? "");
  const [autoStart, setAutoStart] = useState(app?.autoStart ?? false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const entry: AppEntry = {
      id: app?.id ?? newId(),
      name: name.trim(),
      path: path.trim(),
      command: command.trim(),
      type,
      port: type === "web" && port ? Number(port) : undefined,
      processName: processName.trim() || undefined,
      autoStart,
      tags: app?.tags ?? [],
    };
    onSave(entry);
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/40 z-40"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="fixed top-0 right-0 bottom-0 w-80 bg-surface-1 z-50 shadow-xl flex flex-col animate-slide-in-right">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-surface-2">
          <h2 className="text-sm font-medium text-text-primary">
            {isEdit ? "編輯應用程式" : "新增應用程式"}
          </h2>
          <button
            onClick={onClose}
            className="flex items-center justify-center w-6 h-6 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
          >
            <svg
              className="w-4 h-4"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Form */}
        <form
          onSubmit={handleSubmit}
          className="flex-1 overflow-y-auto px-4 py-4 flex flex-col gap-4"
        >
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              名稱
            </label>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="h-8 rounded bg-surface-0 px-2 text-sm text-text-primary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
            />
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              路徑
            </label>
            <input
              type="text"
              required
              value={path}
              onChange={(e) => setPath(e.target.value)}
              placeholder="C:\path\to\project"
              className="h-8 rounded bg-surface-0 px-2 text-sm font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
            />
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              指令
            </label>
            <input
              type="text"
              required
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder="npm run dev"
              className="h-8 rounded bg-surface-0 px-2 text-sm font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
            />
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              類型
            </label>
            <select
              value={type}
              onChange={(e) => setType(e.target.value as AppType)}
              className="h-8 rounded bg-surface-0 px-2 text-sm text-text-primary outline-none cursor-pointer focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
            >
              {APP_TYPES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </select>
          </div>

          {type === "web" && (
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-text-secondary">
                Port
              </label>
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                placeholder="3000"
                min={1}
                max={65535}
                className="h-8 rounded bg-surface-0 px-2 text-sm font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
              />
            </div>
          )}

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-text-secondary">
              Process Name（偵測用，選填）
            </label>
            <input
              type="text"
              value={processName}
              onChange={(e) => setProcessName(e.target.value)}
              placeholder="pythonw.exe"
              className="h-8 rounded bg-surface-0 px-2 text-sm font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
            />
            <span className="text-[10px] text-text-secondary">
              用於偵測外部已啟動的服務（如 Windows 排程工作）
            </span>
          </div>

          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={autoStart}
              onChange={(e) => setAutoStart(e.target.checked)}
              className="w-4 h-4 rounded bg-surface-0 text-accent cursor-pointer focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1"
            />
            <span className="text-sm text-text-primary">自動啟動</span>
          </label>

          {/* Action buttons */}
          <div className="flex items-center gap-2 mt-auto pt-4 border-t border-surface-2">
            <button
              type="submit"
              className="flex-1 h-8 rounded bg-accent text-surface-0 text-sm font-medium cursor-pointer hover:opacity-90 transition-opacity duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
            >
              {isEdit ? "儲存" : "新增"}
            </button>
            {isEdit && onDelete && (
              <button
                type="button"
                onClick={() => onDelete(app.id)}
                className="h-8 px-3 rounded text-error text-sm font-medium cursor-pointer hover:bg-error/10 transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
              >
                刪除
              </button>
            )}
          </div>
        </form>
      </div>
    </>
  );
}
