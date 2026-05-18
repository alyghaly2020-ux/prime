import { useEffect } from "react";
import { usePluginStore } from "@/stores/usePluginStore";
import { useTranslation } from "react-i18next";
import type { PluginInfo } from "@/types";
import {
  RefreshCw,
  Loader2,
  AlertCircle,
  Puzzle,
  ToggleLeft,
  ToggleRight,
  Trash2,
  Shield,
} from "lucide-react";

function PluginCard({ plugin }: { plugin: PluginInfo }) {
  const { t } = useTranslation();
  const { enablePlugin, disablePlugin, uninstallPlugin } = usePluginStore();

  const handleToggle = () => {
    if (plugin.enabled) {
      disablePlugin(plugin.id);
    } else {
      enablePlugin(plugin.id);
    }
  };

  const statusColor = plugin.enabled
    ? "bg-green-500"
    : plugin.status === "error"
      ? "bg-red-500"
      : "bg-gray-400";

  return (
    <div className="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent/50">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3">
          <div className={`mt-1.5 h-2.5 w-2.5 shrink-0 rounded-full ${statusColor}`} />
          <div>
            <div className="flex items-center gap-2">
              <h3 className="font-medium text-card-foreground">{plugin.name}</h3>
              <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                {t("plugins.version", { version: plugin.version })}
              </span>
            </div>
            <p className="mt-0.5 text-sm text-muted-foreground">
              {plugin.description}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
                {plugin.type}
              </span>
              {plugin.permissions.length > 0 && (
                <span
                  className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground"
                  title={plugin.permissions.join(", ")}
                >
                  <Shield className="h-3 w-3" />
                  {t("plugins.permissions", { count: plugin.permissions.length })}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={handleToggle}
            className="rounded p-1.5 text-muted-foreground hover:bg-accent"
            title={plugin.enabled ? t("plugins.disable") : t("plugins.enable")}
          >
            {plugin.enabled ? (
              <ToggleRight className="h-5 w-5 text-green-500" />
            ) : (
              <ToggleLeft className="h-5 w-5" />
            )}
          </button>
          <button
            onClick={() => uninstallPlugin(plugin.id)}
            className="rounded p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
            title={t("plugins.uninstall")}
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
}

export function PluginManager() {
  const { t } = useTranslation();
  const { plugins, loading, error, fetchPlugins } = usePluginStore();

  useEffect(() => {
    fetchPlugins();
  }, [fetchPlugins]);

  const enabledCount = plugins.filter((p) => p.enabled).length;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("plugins.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("plugins.count", { count: plugins.length, active: enabledCount })}
          </p>
        </div>
        <button
          onClick={() => fetchPlugins()}
          disabled={loading}
          className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm hover:bg-accent"
        >
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          {t("plugins.refresh")}
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

      {loading && plugins.length === 0 && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      )}

      {!loading && plugins.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-12 text-center">
          <Puzzle className="mb-2 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm text-muted-foreground">{t("plugins.empty")}</p>
        </div>
      )}

      <div className="grid gap-3">
        {plugins.map((p) => (
          <PluginCard key={p.id} plugin={p} />
        ))}
      </div>
    </div>
  );
}
