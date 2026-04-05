import type { AppEntry, FilterType, ProcessStatus } from "../types";
import { AppRow } from "./AppRow";

interface AppListProps {
  apps: AppEntry[];
  searchQuery: string;
  filterType: FilterType;
  onFilterChange: (filter: FilterType) => void;
  getStatus: (id: string) => ProcessStatus;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onViewLog: (id: string) => void;
  onEdit: (app: AppEntry) => void;
  onDelete: (id: string) => void;
}

const FILTERS: { value: FilterType; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "web", label: "Web" },
  { value: "cli", label: "CLI" },
  { value: "gui", label: "GUI" },
  { value: "script", label: "Script" },
];

export function AppList({
  apps,
  searchQuery,
  filterType,
  onFilterChange,
  getStatus,
  onStart,
  onStop,
  onViewLog,
  onEdit,
  onDelete,
}: AppListProps) {
  const filtered = apps.filter((app) => {
    if (filterType !== "all" && app.type !== filterType) return false;
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      return (
        app.name.toLowerCase().includes(q) ||
        app.path.toLowerCase().includes(q) ||
        app.command.toLowerCase().includes(q) ||
        app.tags.some((t) => t.toLowerCase().includes(q))
      );
    }
    return true;
  });

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Filter tabs */}
      <div className="flex items-center gap-1 px-3 py-2 border-b border-surface-2">
        {FILTERS.map((f) => (
          <button
            key={f.value}
            onClick={() => onFilterChange(f.value)}
            className={`px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2 ${
              filterType === f.value
                ? "bg-surface-2 text-text-primary"
                : "text-text-secondary hover:text-text-primary hover:bg-surface-2/50"
            }`}
          >
            {f.label}
          </button>
        ))}
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto">
        {filtered.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <p className="text-sm text-text-secondary">
              {apps.length === 0
                ? "尚無應用程式。點擊 + 新增或掃描專案。"
                : "沒有符合條件的應用程式。"}
            </p>
          </div>
        ) : (
          <div className="divide-y divide-surface-2">
            {filtered.map((app) => (
              <AppRow
                key={app.id}
                app={app}
                status={getStatus(app.id)}
                onStart={onStart}
                onStop={onStop}
                onViewLog={onViewLog}
                onEdit={onEdit}
                onDelete={onDelete}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
