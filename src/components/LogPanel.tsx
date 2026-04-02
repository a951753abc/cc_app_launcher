import { useEffect, useRef } from "react";
import type { LogLine } from "../types";

interface LogPanelProps {
  appName: string;
  lines: LogLine[];
  onClear: () => void;
  onClose: () => void;
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp);
  const h = String(d.getHours()).padStart(2, "0");
  const m = String(d.getMinutes()).padStart(2, "0");
  const s = String(d.getSeconds()).padStart(2, "0");
  return `${h}:${m}:${s}`;
}

export function LogPanel({ appName, lines, onClear, onClose }: LogPanelProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isAutoScroll = useRef(true);

  useEffect(() => {
    const el = scrollRef.current;
    if (el && isAutoScroll.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [lines]);

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 32;
    isAutoScroll.current = atBottom;
  };

  return (
    <div className="flex flex-col h-48 border-t border-surface-2 bg-surface-0 shrink-0">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-surface-1 border-b border-surface-2">
        <span className="text-xs font-medium text-text-primary truncate">
          {appName}
        </span>
        <div className="flex items-center gap-1">
          <button
            onClick={onClear}
            className="flex items-center justify-center w-5 h-5 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
            title="清除日誌"
          >
            <svg
              className="w-3 h-3"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polyline points="3 6 5 6 21 6" />
              <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            </svg>
          </button>
          <button
            onClick={onClose}
            className="flex items-center justify-center w-5 h-5 rounded text-text-secondary hover:text-text-primary hover:bg-surface-2 cursor-pointer transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent focus-visible:outline-offset-2"
            title="關閉"
          >
            <svg
              className="w-3 h-3"
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
      </div>

      {/* Log content */}
      <div
        ref={scrollRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto overflow-x-hidden px-3 py-1 font-mono text-[11px] leading-5"
      >
        {lines.length === 0 ? (
          <p className="text-text-secondary py-2">尚無日誌輸出。</p>
        ) : (
          lines.map((line, i) => (
            <div key={i} className="flex gap-2 min-w-0">
              <span className="shrink-0 text-text-secondary select-none">
                {formatTime(line.timestamp)}
              </span>
              <span
                className={`break-all ${line.isStderr ? "text-error" : "text-text-primary"}`}
              >
                {line.line}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
