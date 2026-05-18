import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Plus, XCircle, Wifi, WifiOff, RotateCw, Loader2, AlertCircle } from "lucide-react";

interface ProxyEntryIPC {
  id: string;
  host: string;
  port: number;
  protocol: string;
  status: string;
  latency_ms: number | null;
  country: string;
  last_used: string | null;
}

export function ProxyPoolManager() {
  const { t } = useTranslation();
  const { data: proxies, isLoading, error, refetch } = useQuery({
    queryKey: ["proxy_list"],
    queryFn: () => invoke<ProxyEntryIPC[]>("proxy_list"),
    refetchInterval: 30000,
  });

  const { data: activeCount } = useQuery({
    queryKey: ["proxy_active_count"],
    queryFn: () => invoke<number>("proxy_active_count"),
    refetchInterval: 30000,
  });

  const items = proxies ?? [];
  const online = items.filter((p) => p.status === "online" || p.status === "Online");
  const avgLatency = online.length > 0
    ? online.reduce((a, p) => a + (p.latency_ms ?? 0), 0) / online.length
    : 0;

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("proxy.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("proxy.description")}</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => refetch()} className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent">
            <RefreshCw className="h-4 w-4" />
            {t("proxy.test_all")}
          </button>
          <button className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90">
            <Plus className="h-4 w-4" />
            {t("proxy.add")}
          </button>
        </div>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{t("proxy.failed", { error: String(error) })}</p>
        </div>
      )}

      {isLoading && (
        <div className="flex items-center justify-center py-24">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {!isLoading && !error && (
        <>
          {/* Stats */}
          <div className="grid grid-cols-4 gap-3">
            <div className="rounded-lg border border-border bg-card p-3">
              <p className="text-xs text-muted-foreground">{t("proxy.total")}</p>
              <p className="mt-1 text-xl font-bold text-foreground">{activeCount ?? items.length}</p>
            </div>
            <div className="rounded-lg border border-border bg-card p-3">
              <p className="text-xs text-muted-foreground">{t("proxy.online")}</p>
              <p className="mt-1 text-xl font-bold text-green-500">{online.length}</p>
            </div>
            <div className="rounded-lg border border-border bg-card p-3">
              <p className="text-xs text-muted-foreground">{t("proxy.avg_latency")}</p>
              <p className="mt-1 text-xl font-bold text-foreground">{Math.round(avgLatency)}ms</p>
            </div>
            <div className="rounded-lg border border-border bg-card p-3">
              <p className="text-xs text-muted-foreground">{t("proxy.rotation")}</p>
              <p className="mt-1 text-xl font-bold text-foreground">{t("proxy.round_robin")}</p>
            </div>
          </div>

          {/* Proxy list */}
          <div className="space-y-2">
            {items.map((proxy) => {
              const isOnline = proxy.status === "online" || proxy.status === "Online";
              const isError = proxy.status === "error" || proxy.status === "Error" || proxy.status === "offline" || proxy.status === "Offline";
              return (
                <div key={proxy.id} className="flex items-center gap-4 rounded-xl border border-border bg-card p-4 transition-colors hover:bg-accent/50">
                  <div className={`rounded-lg p-2 ${isOnline ? "bg-green-500/10" : isError ? "bg-red-500/10" : "bg-muted"}`}>
                    {isOnline ? <Wifi className="h-4 w-4 text-green-500" /> : isError ? <XCircle className="h-4 w-4 text-red-500" /> : <WifiOff className="h-4 w-4 text-muted-foreground/50" />}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-medium text-foreground">{proxy.host}:{proxy.port}</p>
                      <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{proxy.protocol}</span>
                      {proxy.country && <span className="text-[10px] text-muted-foreground">{proxy.country}</span>}
                    </div>
                    <div className="mt-1 flex items-center gap-3 text-xs text-muted-foreground">
                      {proxy.latency_ms != null && <span className="text-green-500">{proxy.latency_ms}ms</span>}
                      {proxy.last_used && <span>{t("proxy.last_used", { value: proxy.last_used })}</span>}
                    </div>
                  </div>
                  <div className="flex items-center gap-1">
                    <button className="rounded-md border border-border bg-background p-1.5 text-muted-foreground hover:bg-accent">
                      <RotateCw className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              );
            })}
            {items.length === 0 && (
              <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                <WifiOff className="mb-3 h-10 w-10 text-muted-foreground/30" />
                <p className="text-sm font-medium">{t("proxy.empty")}</p>
              </div>
            )}
          </div>

          {/* Rotation config */}
          <div className="rounded-xl border border-border bg-card p-4">
            <h3 className="mb-3 text-xs font-semibold text-foreground uppercase tracking-wider">{t("proxy.rotation_settings")}</h3>
            <div className="grid gap-3 md:grid-cols-3">
              <div className="rounded-lg bg-muted/30 p-3">
                <p className="text-xs text-muted-foreground">{t("proxy.strategy")}</p>
                <p className="mt-1 text-sm font-medium text-foreground">{t("proxy.round_robin")}</p>
              </div>
              <div className="rounded-lg bg-muted/30 p-3">
                <p className="text-xs text-muted-foreground">{t("proxy.interval")}</p>
                <p className="mt-1 text-sm font-medium text-foreground">{t("proxy.every_request")}</p>
              </div>
              <div className="rounded-lg bg-muted/30 p-3">
                <p className="text-xs text-muted-foreground">{t("proxy.health_check")}</p>
                <p className="mt-1 text-sm font-medium text-foreground">{t("proxy.every_60s")}</p>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
