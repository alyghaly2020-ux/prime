import { useState, useEffect } from "react";
import { useMcpStore } from "@/stores/useMcpStore";
import type { McpServerInfo } from "@/types";
import { useTranslation } from "react-i18next";
import {
  Server,
  Play,
  Square,
  RotateCcw,
  RefreshCw,
  Loader2,
  AlertCircle,
  Plus,
  Terminal,
  FileJson,
  Globe,
  Database,
  GitBranch,
  Search,
  BookOpen,
  Monitor,
  MessageSquare,
  Cpu,
  Wifi,
  WifiOff,
} from "lucide-react";

const SERVER_ICONS: Record<string, typeof Server> = {
  filesystem: FileJson,
  git: GitBranch,
  terminal: Terminal,
  browser: Globe,
  memory: Cpu,
  search: Search,
  docs: BookOpen,
  database: Database,
  os: Monitor,
  telegram: MessageSquare,
  discord: MessageSquare,
  whatsapp: MessageSquare,
};

export function McpManager() {
  const { t } = useTranslation();
  const { servers, loading, error, fetchServers, startServer, stopServer, restartServer } = useMcpStore();
  useEffect(() => { fetchServers(); }, [fetchServers]);

  const runningCount = servers.filter((s) => s.running).length;

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("mcp.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("mcp.count", { count: servers.length, running: runningCount })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent">
            <Plus className="h-4 w-4" />
            {t("mcp.add")}
          </button>
          <button
            onClick={() => fetchServers()}
            disabled={loading}
            className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
            {t("mcp.refresh")}
          </button>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{error}</p>
        </div>
      )}

      {/* Loading */}
      {loading && servers.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {/* Empty */}
      {!loading && servers.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Server className="mb-2 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">{t("mcp.empty")}</p>
          <button className="mt-3 inline-flex items-center gap-1 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">
            <Plus className="h-4 w-4" />
            {t("mcp.empty_hint")}
          </button>
        </div>
      )}

      {/* Server list */}
      <div className="grid gap-3">
        {servers.map((server) => {
          const Icon = SERVER_ICONS[server.id] || Server;
          return (
            <ServerCard
              key={server.id}
              server={server}
              icon={Icon}
              onStart={startServer}
              onStop={stopServer}
              onRestart={restartServer}
            />
          );
        })}
      </div>
    </div>
  );
}

function ServerCard({
  server,
  icon: Icon,
  onStart,
  onStop,
  onRestart,
}: {
  server: McpServerInfo;
  icon: typeof Server;
  onStart: (id: string) => Promise<void>;
  onStop: (id: string) => Promise<void>;
  onRestart: (id: string) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const handleAction = async (action: string, fn: () => Promise<void>) => {
    setActionLoading(action);
    try { await fn(); } finally { setActionLoading(null); }
  };

  return (
    <div className="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent/50">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <div className={`flex h-10 w-10 items-center justify-center rounded-lg ${
            server.running ? "bg-green-500/10 text-green-600" : "bg-muted text-muted-foreground"
          }`}>
            <Icon className="h-5 w-5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h3 className="font-medium text-card-foreground capitalize">{server.name}</h3>
              <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                {t("mcp.version", { version: server.version })}
              </span>
              <span className="text-[10px] font-mono text-muted-foreground/50">{t("mcp.id", { id: server.id })}</span>
            </div>
            <div className="mt-1 flex items-center gap-2">
              {server.running ? (
                <span className="flex items-center gap-1 text-xs text-green-600">
                  <Wifi className="h-3 w-3" />
                  {t("mcp.running")}
                </span>
              ) : (
                <span className="flex items-center gap-1 text-xs text-muted-foreground">
                  <WifiOff className="h-3 w-3" />
                  {t("mcp.stopped")}
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-1">
          {server.running ? (
            <button
              onClick={() => handleAction("stop", () => onStop(server.id))}
              disabled={actionLoading !== null}
              className="rounded p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-50"
              title={t("mcp.stop")}
            >
              {actionLoading === "stop" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Square className="h-4 w-4" />}
            </button>
          ) : (
            <button
              onClick={() => handleAction("start", () => onStart(server.id))}
              disabled={actionLoading !== null}
              className="rounded p-1.5 text-muted-foreground hover:bg-green-500/10 hover:text-green-600 disabled:opacity-50"
              title={t("mcp.start")}
            >
              {actionLoading === "start" ? <Loader2 className="h-4 w-4 animate-spin" /> : <Play className="h-4 w-4" />}
            </button>
          )}
          <button
            onClick={() => handleAction("restart", () => onRestart(server.id))}
            disabled={actionLoading !== null}
            className="rounded p-1.5 text-muted-foreground hover:bg-accent disabled:opacity-50"
            title={t("mcp.restart")}
          >
            {actionLoading === "restart" ? <Loader2 className="h-4 w-4 animate-spin" /> : <RotateCcw className="h-4 w-4" />}
          </button>
        </div>
      </div>
    </div>
  );
}
