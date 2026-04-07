import type { Settings } from "../types";

interface SettingsPanelProps {
  settings: Settings;
  onUpdate: (settings: Settings) => void;
  onClose: () => void;
}

interface ToggleRowProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

function ToggleRow({ label, checked, onChange }: ToggleRowProps) {
  return (
    <label className="flex items-center justify-between py-2 cursor-pointer">
      <span className="text-sm text-text-primary">{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200 cursor-pointer focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2 ${
          checked ? "bg-accent" : "bg-surface-2"
        }`}
      >
        <span
          className={`inline-block h-4 w-4 rounded-full bg-text-primary transition-transform duration-200 mt-0.5 ${
            checked ? "translate-x-[18px] ml-0" : "translate-x-0.5"
          }`}
        />
      </button>
    </label>
  );
}

interface TextRowProps {
  label: string;
  value: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

function TextRow({ label, value, placeholder, onChange }: TextRowProps) {
  return (
    <div className="flex flex-col gap-1 py-2">
      <span className="text-sm text-text-primary">{label}</span>
      <input
        type="text"
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className="h-8 rounded bg-surface-0 px-2 text-xs font-mono text-text-primary placeholder:text-text-secondary outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 focus-visible:ring-offset-surface-1 transition-colors duration-150"
      />
    </div>
  );
}

export function SettingsPanel({
  settings,
  onUpdate,
  onClose,
}: SettingsPanelProps) {
  const handleToggle = (key: keyof Settings) => (checked: boolean) => {
    onUpdate({ ...settings, [key]: checked });
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
          <h2 className="text-sm font-medium text-text-primary">設定</h2>
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

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-4 py-4">
          <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mb-2">
            一般
          </h3>
          <div className="flex flex-col divide-y divide-surface-2">
            <ToggleRow
              label="啟動時最小化"
              checked={settings.startMinimized}
              onChange={handleToggle("startMinimized")}
            />
            <ToggleRow
              label="關閉至系統匣"
              checked={settings.closeToTray}
              onChange={handleToggle("closeToTray")}
            />
            <ToggleRow
              label="隨 Windows 啟動"
              checked={settings.autoStartWithWindows}
              onChange={handleToggle("autoStartWithWindows")}
            />
          </div>

          <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mt-6 mb-2">
            掃描
          </h3>
          <div className="flex flex-col divide-y divide-surface-2">
            <ToggleRow
              label="排除 Git Worktree"
              checked={settings.excludeWorktrees}
              onChange={handleToggle("excludeWorktrees")}
            />
          </div>

          <h3 className="text-xs font-medium text-text-secondary uppercase tracking-wider mt-6 mb-2">
            Python
          </h3>
          <div className="flex flex-col">
            <TextRow
              label="預設 Python 直譯器"
              value={settings.pythonInterpreter ?? ""}
              placeholder="C:\Users\xxx\anaconda3\python.exe"
              onChange={(v) =>
                onUpdate({
                  ...settings,
                  pythonInterpreter: v.trim() === "" ? undefined : v,
                })
              }
            />
            <span className="text-[10px] text-text-secondary mt-1">
              留空則自動以 <code className="font-mono">where python</code> 解析。新偵測或重新偵測 Python 專案時生效。
            </span>
          </div>
        </div>
      </div>
    </>
  );
}
