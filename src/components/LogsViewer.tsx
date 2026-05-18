import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LogEntry, LogLevel } from "@/types";
import { AlertCircle, RefreshCw, Search, Terminal, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";

const LEVEL_COLORS: Record<LogLevel, string> = {
  error: "text-red-500 bg-red-500/10",
  warn: "text-yellow-500 bg-yellow-500/10",
  info: "text-blue-500 bg-blue-500/10",
  debug: "text-gray-500 bg-gray-500/10",
  trace: "text-gray-400 bg-gray-400/10",
};

const LEVEL_ORDER: LogLevel[] = ["error", "warn", "info", "debug", "trace"];

export function LogsViewer() {
  const { t } = useTranslation();
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [levelFilter, setLevelFilter] = useState<LogLevel | "all">("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<string>("get_logs");
      const parsed = JSON.parse(result) as LogEntry[];
      setLogs(parsed);
    } catch (e) {
      const msg = `logs: ${e}`;
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchLogs();
    const interval = setInterval(fetchLogs, 3000);
    return () => clearInterval(interval);
  }, [fetchLogs]);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const filtered = logs.filter((log) => {
    if (levelFilter !== "all" && log.level !== levelFilter) return false;
    if (searchQuery && !log.message.toLowerCase().includes(searchQuery.toLowerCase())) {
      return false;
    }
    return true;
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("logs.title")}</h2>
          <p className="text-sm text-muted-foreground">{t("logs.description")}</p>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="rounded border-border"
            />
            {t("logs.auto_scroll")}
          </label>
          <button
            onClick={fetchLogs}
            disabled={loading}
            className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            {t("logs.refresh")}
          </button>
        </div>
      </div>

      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive"
        >
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{error}</p>
        </div>
      )}

      {/* Controls */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t("logs.search")}
            className="w-full rounded-md border border-input bg-background py-2 pl-9 pr-3 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
        </div>
        <div className="flex gap-1">
          {(["all", ...LEVEL_ORDER] as const).map((level) => (
            <button
              key={level}
              onClick={() => setLevelFilter(level)}
              className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors ${
                levelFilter === level
                  ? level === "all"
                    ? "bg-primary text-primary-foreground"
                    : `${LEVEL_COLORS[level]} border border-current`
                  : "bg-muted text-muted-foreground hover:bg-accent"
              }`}
            >
              {level === "all" ? t("logs.filter_all") : t("logs.filter_" + level)}
            </button>
          ))}
        </div>
      </div>

      {/* Log output */}
      <div
        ref={containerRef}
        className="max-h-[500px] overflow-y-auto rounded-lg border border-border bg-card font-mono text-xs"
      >
        {loading && logs.length === 0 && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        )}

        {!loading && filtered.length === 0 && (
          <div className="flex flex-col items-center justify-center py-8 text-center text-muted-foreground">
            <Terminal className="mb-2 h-8 w-8 opacity-30" />
            <p>{t("logs.empty")}</p>
          </div>
        )}

        {filtered.map((log) => (
          <div
            key={log.id}
            className="flex border-b border-border/50 px-3 py-1.5 hover:bg-accent/30"
          >
            <span className="mr-2 shrink-0 text-muted-foreground">
              {new Date(log.timestamp).toLocaleTimeString()}
            </span>
            <span
              className={`mr-2 shrink-0 rounded px-1 font-medium ${LEVEL_COLORS[log.level]}`}
            >
              {log.level.toUpperCase().padEnd(5)}
            </span>
            <span className="mr-2 shrink-0 text-muted-foreground">{log.target}</span>
            <span className="text-foreground/90">{log.message}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
