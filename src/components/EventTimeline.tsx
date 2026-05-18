import { useState, useEffect, useCallback, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { SystemEvent, EventSeverity } from "@/types";
import { AlertCircle, RefreshCw, Filter, Loader2 } from "lucide-react";

const SEVERITY_COLORS: Record<EventSeverity, string> = {
  info: "border-l-blue-500",
  warning: "border-l-yellow-500",
  error: "border-l-red-500",
  debug: "border-l-gray-500",
};

const SEVERITY_BG: Record<EventSeverity, string> = {
  info: "bg-blue-500/10 text-blue-600 dark:text-blue-400",
  warning: "bg-yellow-500/10 text-yellow-600 dark:text-yellow-400",
  error: "bg-red-500/10 text-red-600 dark:text-red-400",
  debug: "bg-gray-500/10 text-gray-600 dark:text-gray-400",
};

export function EventTimeline() {
  const { t } = useTranslation();
  const [events, setEvents] = useState<SystemEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<EventSeverity | "all">("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const containerRef = useRef<HTMLDivElement>(null);

  const fetchEvents = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<string>("get_events");
      const parsed = JSON.parse(result) as SystemEvent[];
      setEvents(parsed);
    } catch (e) {
      const msg = `events: ${e}`;
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchEvents();
    const interval = setInterval(fetchEvents, 5000);
    return () => clearInterval(interval);
  }, [fetchEvents]);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [events, autoScroll]);

  const filtered = severityFilter === "all"
    ? events
    : events.filter((e) => e.severity === severityFilter);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("events.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("events.description")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="rounded border-border"
            />
            {t("events.auto_scroll")}
          </label>
          <button
            onClick={fetchEvents}
            disabled={loading}
            className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            {t("events.refresh")}
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

      {/* Filter chips */}
      <div className="flex items-center gap-2">
        <Filter className="h-4 w-4 text-muted-foreground" />
        {(["all", "info", "warning", "error", "debug"] as const).map((sev) => (
          <button
            key={sev}
            onClick={() => setSeverityFilter(sev)}
            className={`rounded-full px-3 py-1 text-xs font-medium transition-colors ${
              severityFilter === sev
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-accent"
            }`}
          >
            {t(`events.filter_${sev}`)}
          </button>
        ))}
      </div>

      {/* Event list */}
      <div
        ref={containerRef}
        className="max-h-[500px] space-y-1 overflow-y-auto rounded-lg border border-border"
      >
        {loading && events.length === 0 && (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        )}

        {!loading && filtered.length === 0 && (
          <div className="py-8 text-center text-sm text-muted-foreground">
            {t("events.empty")}
          </div>
        )}

        {filtered.map((event) => (
          <div
            key={event.id}
            className={`border-l-4 px-4 py-2 text-sm transition-colors hover:bg-accent/50 ${SEVERITY_COLORS[event.severity]}`}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span
                  className={`rounded px-1.5 py-0.5 text-xs font-medium ${SEVERITY_BG[event.severity]}`}
                >
                  {event.severity.toUpperCase()}
                </span>
                <span className="font-medium text-foreground">{event.type}</span>
                <span className="text-xs text-muted-foreground">
                  {t("events.from", { source: event.source })}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {new Date(event.timestamp).toLocaleTimeString()}
              </span>
            </div>
            <p className="mt-0.5 text-foreground/80">{event.message}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
