import { useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useWorkflowStore } from "@/stores/useWorkflowStore";
import type { Workflow, WorkflowStep } from "@/types";
import {
  Play,
  Square,
  Pause,
  RefreshCw,
  Loader2,
  AlertCircle,
  CheckCircle2,
  Clock,
  XCircle,
} from "lucide-react";

const statusIcon = (status: WorkflowStep["status"]) => {
  switch (status) {
    case "completed":
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    case "running":
      return <Loader2 className="h-4 w-4 animate-spin text-blue-500" />;
    case "failed":
      return <XCircle className="h-4 w-4 text-red-500" />;
    case "skipped":
      return <XCircle className="h-4 w-4 text-muted-foreground/50" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
};

function WorkflowCard({ workflow }: { workflow: Workflow }) {
  const { t } = useTranslation();
  const { startWorkflow, cancelWorkflow, pauseWorkflow, resumeWorkflow } =
    useWorkflowStore();

  const handleAction = useCallback(async () => {
    try {
      switch (workflow.status) {
        case "idle":
          await startWorkflow(workflow.id);
          break;
        case "running":
          await pauseWorkflow(workflow.id);
          break;
        case "paused":
          await resumeWorkflow(workflow.id);
          break;
        case "completed":
        case "failed":
        case "cancelled":
          await startWorkflow(workflow.id);
          break;
      }
    } catch {
      // Error handled by store
    }
  }, [workflow, startWorkflow, pauseWorkflow, resumeWorkflow]);

  const progressColor =
    workflow.status === "failed"
      ? "bg-red-500"
      : workflow.status === "completed"
        ? "bg-green-500"
        : "bg-blue-500";

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="mb-3 flex items-start justify-between">
        <div>
          <h3 className="font-medium text-card-foreground">{workflow.name}</h3>
          <p className="mt-0.5 text-sm text-muted-foreground">
            {workflow.description}
          </p>
        </div>
        <div className="flex items-center gap-1">
          {workflow.status === "running" && (
            <button
              onClick={() => cancelWorkflow(workflow.id)}
              className="rounded p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
              title={t("workflows.cancel")}
            >
              <Square className="h-4 w-4" />
            </button>
          )}
          <button
            onClick={handleAction}
            className="rounded p-1.5 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            title={
              workflow.status === "running"
                ? t("workflows.pause")
                : workflow.status === "paused"
                  ? t("workflows.resume")
                  : t("workflows.start")
            }
          >
            {workflow.status === "running" ? (
              <Pause className="h-4 w-4" />
            ) : workflow.status === "idle" || workflow.status === "completed" ? (
              <Play className="h-4 w-4" />
            ) : (
              <Play className="h-4 w-4" />
            )}
          </button>
        </div>
      </div>

      {/* Progress bar */}
      <div className="mb-3 h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className={`h-full rounded-full transition-all duration-500 ${progressColor}`}
          style={{ width: `${workflow.progress_pct}%` }}
        />
      </div>
      <p className="mb-3 text-xs text-muted-foreground">
        {t("workflows.progress", { progress: workflow.progress_pct, steps: workflow.steps.length })}
      </p>

      {/* Steps */}
      <div className="space-y-1">
        {workflow.steps.map((step) => (
          <div
            key={step.id}
            className="flex items-center gap-2 rounded px-2 py-1 text-sm hover:bg-muted/50"
          >
            {statusIcon(step.status)}
            <span className="flex-1 text-foreground/80">{step.name}</span>
            {step.error && (
              <span className="text-xs text-red-500" title={step.error}>
                <AlertCircle className="h-3 w-3" />
              </span>
            )}
            {step.duration_ms && (
              <span className="text-xs text-muted-foreground">
                {step.duration_ms < 1000
                  ? `${step.duration_ms}ms`
                  : `${(step.duration_ms / 1000).toFixed(1)}s`}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export function WorkflowPanel() {
  const { t } = useTranslation();
  const { workflows, loading, error, fetchWorkflows } = useWorkflowStore();

  useEffect(() => {
    fetchWorkflows();
  }, [fetchWorkflows]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("workflows.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("workflows.description")}
          </p>
        </div>
        <button
          onClick={() => fetchWorkflows()}
          disabled={loading}
          className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          {t("workflows.refresh")}
        </button>
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

      {loading && workflows.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {!loading && workflows.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Clock className="mb-2 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">{t("workflows.empty")}</p>
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {workflows.map((w) => (
          <WorkflowCard key={w.id} workflow={w} />
        ))}
      </div>
    </div>
  );
}
