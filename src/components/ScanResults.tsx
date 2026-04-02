import { useState, useEffect } from "react";
import type { ScanCandidate } from "../types";
import { scanProjects, addScannedApps } from "../lib/commands";

interface ScanResultsProps {
  onClose: () => void;
  onDone: () => void;
}

export function ScanResults({ onClose, onDone }: ScanResultsProps) {
  const [candidates, setCandidates] = useState<ScanCandidate[]>([]);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    scanProjects()
      .then((results) => {
        setCandidates(results);
        setSelected(new Set(results.map((_, i) => i)));
      })
      .finally(() => setLoading(false));
  }, []);

  const toggleSelect = (index: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  const toggleAll = () => {
    if (selected.size === candidates.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(candidates.map((_, i) => i)));
    }
  };

  const handleAdd = async () => {
    const toAdd = candidates.filter((_, i) => selected.has(i));
    if (toAdd.length === 0) return;
    setSubmitting(true);
    try {
      await addScannedApps(toAdd);
      onDone();
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/40 z-40"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="fixed top-0 right-0 bottom-0 w-96 bg-surface-1 z-50 shadow-xl flex flex-col animate-slide-in-right">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-surface-2">
          <h2 className="text-sm font-medium text-text-primary">
            掃描結果
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

        {/* Content */}
        <div className="flex-1 overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center h-full">
              <div className="flex flex-col items-center gap-2">
                <svg
                  className="w-6 h-6 text-text-secondary animate-spin"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                </svg>
                <span className="text-sm text-text-secondary">
                  掃描專案中...
                </span>
              </div>
            </div>
          ) : candidates.length === 0 ? (
            <div className="flex items-center justify-center h-full">
              <p className="text-sm text-text-secondary">
                未找到任何專案。
              </p>
            </div>
          ) : (
            <div className="divide-y divide-surface-2">
              {/* Select all */}
              <label className="flex items-center gap-3 px-4 py-2 cursor-pointer hover:bg-surface-2 transition-colors duration-150">
                <input
                  type="checkbox"
                  checked={selected.size === candidates.length}
                  onChange={toggleAll}
                  className="w-4 h-4 rounded bg-surface-0 text-accent cursor-pointer"
                />
                <span className="text-xs font-medium text-text-secondary">
                  全選 ({candidates.length})
                </span>
              </label>

              {candidates.map((c, i) => (
                <label
                  key={i}
                  className="flex items-start gap-3 px-4 py-2 cursor-pointer hover:bg-surface-2 transition-colors duration-150"
                >
                  <input
                    type="checkbox"
                    checked={selected.has(i)}
                    onChange={() => toggleSelect(i)}
                    className="w-4 h-4 rounded bg-surface-0 text-accent cursor-pointer mt-0.5"
                  />
                  <div className="flex flex-col min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-text-primary truncate">
                        {c.name}
                      </span>
                      <span className="text-[10px] font-mono uppercase text-text-secondary">
                        {c.appType}
                      </span>
                      {c.port != null && (
                        <span className="text-[10px] font-mono text-text-secondary">
                          :{c.port}
                        </span>
                      )}
                    </div>
                    <span className="text-xs font-mono text-text-secondary truncate">
                      {c.path}
                    </span>
                    <span className="text-xs font-mono text-text-secondary">
                      {c.command}
                    </span>
                  </div>
                </label>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        {!loading && candidates.length > 0 && (
          <div className="px-4 py-3 border-t border-surface-2">
            <button
              onClick={handleAdd}
              disabled={selected.size === 0 || submitting}
              className="w-full h-8 rounded bg-accent text-surface-0 text-sm font-medium cursor-pointer hover:opacity-90 transition-opacity duration-150 disabled:opacity-40 disabled:cursor-not-allowed focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
            >
              {submitting
                ? "新增中..."
                : `加入 ${selected.size} 個程式`}
            </button>
          </div>
        )}
      </div>
    </>
  );
}
