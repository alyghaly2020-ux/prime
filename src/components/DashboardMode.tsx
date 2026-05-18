import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { SystemState, AgentInfo } from "@/types";
import { PhiBrainStatus } from "@/components/PhiBrainStatus";
import {
  Activity,
  Brain,
  Cpu,
  Puzzle,
  Zap,
  Clock,
  RefreshCw,
  Loader2,
  AlertCircle,
  LayoutDashboard,
} from "lucide-react";

function formatDuration(secs: number): string {
  const hours = Math.floor(secs / 3600);
  const minutes = Math.floor((secs % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${secs % 60}s`;
  return `${secs}s`;
}

function MetricCard({
  label,
  value,
  icon: Icon,
  trend,
}: {
  label: string;
  value: string;
  icon: typeof Activity;
  trend?: "up" | "down" | "stable";
}) {
  const { t } = useTranslation();
  return (
    <div className="rounded-xl border border-border/60 bg-card p-5 transition-all hover:border-primary/20 hover:shadow-sm dark:hover:shadow-gold/5">
      <div className="flex items-center justify-between mb-3">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">{label}</p>
        <div className="rounded-lg bg-primary/10 p-2 gold-glow dark:gold-glow">
          <Icon className="h-4 w-4 text-primary" />
        </div>
      </div>
      <p className="text-2xl font-bold text-card-foreground tracking-tight">{value}</p>
      {trend && (
        <div className="mt-1 flex items-center gap-1">
          <div
            className={`h-2 w-2 rounded-full ${
              trend === "up" ? "bg-green-500" : trend === "down" ? "bg-red-500" : "bg-yellow-500"
            }`}
          />
          <span className="text-[10px] text-muted-foreground capitalize">{t(`dashboard.trend.${trend}`)}</span>
        </div>
      )}
    </div>
  );
}

function HealthBar({ label, value, max }: { label: string; value: number; max: number }) {
  const pct = Math.min((value / max) * 100, 100);
  const color = pct > 80 ? "bg-red-500" : pct > 50 ? "bg-yellow-500" : "bg-green-500";
  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs">
        <span className="text-muted-foreground">{label}</span>
        <span className="font-medium text-foreground">
          {value.toFixed(1)} / {max}
        </span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-muted">
        <div
          className={`h-full rounded-full transition-all duration-500 ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function EventsFeed() {
  const { t } = useTranslation();
  const events = [
    { time: "2m ago", label: "System initialized", type: "info" as const },
    { time: "1m ago", label: "MCP servers started", type: "info" as const },
    { time: "30s ago", label: "Chat engine ready", type: "success" as const },
  ];

  return (
    <div className="rounded-xl border border-border/60 bg-card p-4">
      <div className="flex items-center gap-2 mb-3">
        <Activity className="h-3.5 w-3.5 text-primary" />
        <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("dashboard.recent_events")}</h3>
      </div>
      <div className="space-y-2">
        {events.map((ev, i) => (
          <div key={i} className="flex items-center gap-3 text-xs">
            <div
              className={`h-2 w-2 rounded-full ${
                ev.type === "success" ? "bg-green-500" : "bg-red-500"
              }`}
            />
            <span className="text-muted-foreground w-14 shrink-0">{ev.time}</span>
            <span className="text-foreground">{ev.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function QuickActions() {
  const { t } = useTranslation();
  const actions = [
    { label: t("dashboard.new_chat"), icon: Zap, shortcut: "F1" },
    { label: t("dashboard.open_code"), icon: Cpu, shortcut: "F2" },
    { label: t("dashboard.run_tests"), icon: Puzzle, shortcut: "" },
    { label: t("dashboard.verify"), icon: RefreshCw, shortcut: "" },
  ];

  return (
    <div className="rounded-xl border border-border/60 bg-card p-4">
      <div className="flex items-center gap-2 mb-3">
        <Zap className="h-3.5 w-3.5 text-primary" />
        <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("dashboard.actions")}</h3>
      </div>
      <div className="grid grid-cols-2 gap-2">
        {actions.map((action) => (
          <button
            key={action.label}
            className="flex items-center gap-2 rounded-lg border border-border/50 bg-background px-3 py-2.5 text-xs font-medium text-foreground hover:bg-accent transition-colors"
          >
            <action.icon className="h-3.5 w-3.5 text-muted-foreground" />
            <span>{action.label}</span>
            {action.shortcut && (
              <kbd className="ml-auto rounded border border-border/50 bg-muted px-1 py-0.5 text-[10px] text-muted-foreground">
                {action.shortcut}
              </kbd>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}

function AgentSummary() {
  const { t } = useTranslation();
  const { data: agents, isLoading } = useQuery({
    queryKey: ["agents"],
    queryFn: () => invoke<string>("list_agents").then((r) => JSON.parse(r) as AgentInfo[]),
    refetchInterval: 30000,
  });

  const categories = [
    { id: "development", label: "Dev", color: "bg-blue-500" },
    { id: "ml", label: "AI/ML", color: "bg-purple-500" },
    { id: "security", label: "Security", color: "bg-red-500" },
    { id: "automation", label: "Auto", color: "bg-green-500" },
    { id: "search", label: "Search", color: "bg-yellow-500" },
    { id: "plugin", label: "Plugins", color: "bg-pink-500" },
  ];

  if (isLoading) return null;

  return (
    <div className="rounded-xl border border-border/60 bg-card p-4">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Brain className="h-3.5 w-3.5 text-primary" />
          <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("dashboard.agent_fleet")}</h3>
        </div>
        <span className="text-xs text-muted-foreground">{t("dashboard.total", { count: agents?.length ?? 0 })}</span>
      </div>
      <div className="flex flex-wrap gap-2">
        {categories.map((cat) => {
          const count = agents?.filter((a) => a.id.includes(cat.id)).length ?? 0;
          if (count === 0) return null;
          return (
            <div key={cat.id} className="flex items-center gap-1.5 rounded-md bg-muted/50 px-2 py-1">
              <div className={`h-2 w-2 rounded-full ${cat.color}`} />
              <span className="text-[10px] font-medium text-foreground">{cat.label}</span>
              <span className="text-[10px] text-muted-foreground">{count}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Mock data for browser mode (when Tauri backend is unavailable)
const BROWSER_MODE_STATE: SystemState = {
  uptime_secs: 3742,
  memory_used_mb: 256,
  cpu_usage_pct: 12.4,
  active_skills: 145,
  active_connections: 12,
  version: "0.1.0",
};

export function DashboardMode() {
  const { t } = useTranslation();
  const { data: backendState, error } = useQuery({
    queryKey: ["systemState"],
    queryFn: () => invoke<SystemState>("get_system_state"),
    refetchInterval: 10000,
    retry: 1,
  });

  // Use real data if available, fallback to mock data for browser mode
  const state = backendState ?? (error ? BROWSER_MODE_STATE : null);
  const isBrowserMode = !backendState && !!error;

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="rounded-xl bg-primary/10 p-3 gold-glow dark:gold-glow">
          <LayoutDashboard className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("dashboard.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("dashboard.description")}</p>
        </div>
      </div>

      {isBrowserMode && (
        <div className="flex items-center gap-2 rounded-md bg-yellow-500/10 border border-yellow-500/20 px-3 py-2 text-xs text-yellow-600 dark:text-yellow-400">
          <AlertCircle className="h-3.5 w-3.5 shrink-0" />
          <p>{t("dashboard.browser_mode")}</p>
        </div>
      )}

      {state ? (
        <>
          {/* Metrics grid */}
          <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
            <MetricCard label={t("dashboard.uptime")} value={formatDuration(state.uptime_secs)} icon={Clock} trend="up" />
            <MetricCard label={t("dashboard.memory")} value={`${state.memory_used_mb} MB`} icon={Brain} trend="stable" />
            <MetricCard label={t("dashboard.cpu")} value={`${state.cpu_usage_pct.toFixed(1)}%`} icon={Activity} trend={state.cpu_usage_pct > 50 ? "down" : "up"} />
            <MetricCard label={t("dashboard.active_skills")} value={String(state.active_skills)} icon={Puzzle} trend="up" />
          </div>

          {/* System health */}
          <div className="rounded-xl border border-border/60 bg-card p-5">
            <div className="flex items-center gap-2 mb-4">
              <Activity className="h-3.5 w-3.5 text-primary" />
              <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("dashboard.health")}</h3>
            </div>
            <div className="space-y-3">
              <HealthBar label={t("dashboard.memory_usage")} value={state.memory_used_mb} max={state.memory_used_mb * 2} />
              <HealthBar label={t("dashboard.cpu_load")} value={state.cpu_usage_pct} max={100} />
            </div>
            <div className="mt-4 grid gap-2 text-xs md:grid-cols-3">
              <div className="flex items-center justify-between rounded-lg bg-muted/40 px-3 py-2">
                <span className="text-muted-foreground">{t("dashboard.version")}</span>
                <span className="font-medium text-foreground">{state.version}</span>
              </div>
              <div className="flex items-center justify-between rounded-lg bg-muted/40 px-3 py-2">
                <span className="text-muted-foreground">{t("dashboard.connections")}</span>
                <span className="font-medium text-foreground">{state.active_connections}</span>
              </div>
              <div className="flex items-center justify-between rounded-lg bg-muted/40 px-3 py-2">
                <span className="text-muted-foreground">{t("dashboard.status")}</span>
                <span className="flex items-center gap-1 font-medium text-green-500">
                  <div className="relative h-1.5 w-1.5">
                    <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400/60" />
                    <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-green-500" />
                  </div>
                  Online
                </span>
              </div>
            </div>
          </div>

          {/* Agent Summary */}
          <AgentSummary />

          {/* Phi Brain Status */}
          <PhiBrainStatus />

          {/* Bottom grid */}
          <div className="grid gap-4 md:grid-cols-2">
            <EventsFeed />
            <QuickActions />
          </div>
        </>
      ) : (
        <div className="flex items-center justify-center py-24">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}
    </div>
  );
}
