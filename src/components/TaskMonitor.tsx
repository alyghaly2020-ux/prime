import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, RefreshCw, CheckCircle2, XCircle, Clock, Play, Ban, AlertCircle } from "lucide-react";
import type { TaskInfo, TaskSummary } from "@/types";

type FilterStatus = "Running" | "Pending" | "Completed" | "Failed" | "all";

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const sec = s % 60;
  if (m < 60) return `${m}m ${sec}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
}

function statusIcon(status: string, className: string) {
  switch (status) {
    case "Running": return <Loader2 className={`animate-spin ${className}`} />;
    case "Pending": return <Clock className={className} />;
    case "Completed": return <CheckCircle2 className={className} />;
    case "Failed": return <XCircle className={className} />;
  }
}

function statusColor(status: string) {
  switch (status) {
    case "Running": return "text-blue-500";
    case "Pending": return "text-yellow-500";
    case "Completed": return "text-green-500";
    case "Failed": return "text-red-500";
  }
}

function statusBg(status: string) {
  switch (status) {
    case "Running": return "bg-blue-500/10 border-blue-500/20";
    case "Pending": return "bg-yellow-500/10 border-yellow-500/20";
    case "Completed": return "bg-green-500/10 border-green-500/20";
    case "Failed": return "bg-red-500/10 border-red-500/20";
  }
}

function label(status: string) {
  switch (status) {
    case "Running": return "running";
    case "Pending": return "pending";
    case "Completed": return "completed";
    case "Failed": return "failed";
  }
}

export function TaskMonitor() {
  const [tab, setTab] = useState<FilterStatus>("all");

  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["task_monitor"],
    queryFn: async () => {
      const [tasks, summary] = await Promise.all([
        invoke<TaskInfo[]>("task_list"),
        invoke<TaskSummary>("task_summary"),
      ]);
      return { tasks, summary };
    },
    refetchInterval: 10000,
  });

  const tasks = data?.tasks ?? [];
  const summary = data?.summary;

  const filtered = tab === "all" ? tasks : tasks.filter((t) => t.status === tab);

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">Task Monitor</h1>
          <p className="text-sm text-muted-foreground">Real-time task execution tracking</p>
        </div>
        <button onClick={() => refetch()} className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent">
          <RefreshCw className="h-4 w-4" />
          Refresh
        </button>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>Failed to load tasks: {String(error)}</p>
        </div>
      )}

      {isLoading && (
        <div className="flex items-center justify-center py-24">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {!isLoading && !error && (
        <>
          {/* Summary bar */}
          <div className="grid grid-cols-4 gap-3">
            {(["Completed", "Running", "Pending", "Failed"] as const).map((s) => (
              <div key={s} className={`rounded-lg border p-3 ${statusBg(s)}`}>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-muted-foreground capitalize">{label(s)}</span>
                  {statusIcon(s, `h-4 w-4 ${statusColor(s)}`)}
                </div>
                <p className={`mt-1 text-xl font-bold ${statusColor(s)}`}>
                  {s === "Completed" ? summary?.completed ?? 0
                    : s === "Running" ? summary?.running ?? 0
                    : s === "Pending" ? summary?.pending ?? 0
                    : summary?.failed ?? 0}
                </p>
              </div>
            ))}
          </div>

          {/* Stats row */}
          {summary && (
            <div className="flex items-center gap-4 text-xs text-muted-foreground">
              <span>{summary.running} active</span>
              <span className="text-border">|</span>
              <span>Avg duration: {formatDuration(summary.avg_duration_ms)}</span>
              <span className="text-border">|</span>
              <span>Success rate: {Math.round((summary.completed / Math.max(summary.completed + summary.failed, 1)) * 100)}%</span>
            </div>
          )}

          {/* Tabs */}
          <div className="flex gap-1 rounded-lg bg-muted p-1 w-fit">
            {(["all", "Running", "Pending", "Completed", "Failed"] as const).map((t) => (
              <button
                key={t}
                onClick={() => setTab(t)}
                className={`rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${
                  tab === t ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {t === "all" ? "All" : label(t)} ({t === "all" ? tasks.length
                  : t === "Running" ? summary?.running ?? 0
                  : t === "Pending" ? summary?.pending ?? 0
                  : t === "Completed" ? summary?.completed ?? 0
                  : summary?.failed ?? 0})
              </button>
            ))}
          </div>

          {/* Task list */}
          <div className="space-y-2">
            {filtered.map((task) => (
              <div
                key={task.id}
                className={`flex items-start gap-3 rounded-lg border p-3 ${statusBg(task.status)}`}
              >
                <div className="mt-0.5">{statusIcon(task.status, `h-4 w-4 ${statusColor(task.status)}`)}</div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-medium text-foreground">{task.metadata?.name ?? task.id}</p>
                    {task.metadata?.category && (
                      <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{task.metadata.category}</span>
                    )}
                  </div>
                  <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
                    <span>{formatDuration(task.duration_ms ?? 0)}</span>
                    {task.error && <span className="text-red-400">Error: {task.error}</span>}
                  </div>
                </div>
                {task.status === "Running" && (
                  <button className="rounded-md border border-border bg-background p-1.5 text-muted-foreground hover:bg-accent">
                    <Ban className="h-3.5 w-3.5" />
                  </button>
                )}
              </div>
            ))}
            {filtered.length === 0 && (
              <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                <Play className="mb-3 h-10 w-10 text-muted-foreground/30" />
                <p className="text-sm font-medium">No {tab === "all" ? "" : label(tab)} tasks</p>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
