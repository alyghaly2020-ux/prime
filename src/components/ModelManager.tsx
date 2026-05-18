import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useModelStore, type ModelProvider } from "@/stores/useModelStore";
import { useConfigStore } from "@/stores/useConfigStore";
import {
  Cpu,
  Brain,
  ToggleLeft,
  ToggleRight,
  Globe,
  Zap,
  DollarSign,
  TrendingDown,
  Sparkles,
  Loader2,
  Settings,
} from "lucide-react";

const PROVIDER_LOGOS: Record<string, typeof Cpu> = {
  openai: Brain,
  anthropic: Brain,
  deepseek: Zap,
  google: Globe,
  ollama: Cpu,
  mistral: Brain,
  groq: Zap,
  openrouter: Globe,
  localai: Cpu,
  custom_openai: Cpu,
};

const SPECIALTY_COLORS: Record<string, string> = {
  planning: "bg-purple-500/10 text-purple-600 dark:text-purple-400",
  coding: "bg-blue-500/10 text-blue-600 dark:text-blue-400",
  chat: "bg-green-500/10 text-green-600 dark:text-green-400",
  ui: "bg-pink-500/10 text-pink-600 dark:text-pink-400",
  debugging: "bg-orange-500/10 text-orange-600 dark:text-orange-400",
  research: "bg-cyan-500/10 text-cyan-600 dark:text-cyan-400",
};

function ProviderCard({ provider }: {
  provider: { id: ModelProvider; label: string; enabled: boolean; selectedModel?: string };
}) {
  const { t } = useTranslation();
  const { toggleProvider, routingMode } = useModelStore();
  const profiles = useModelStore((s) => s.getProfiles());
  const profile = profiles.find((p) => p.provider === provider.id);
  const Logo = PROVIDER_LOGOS[provider.id] || Cpu;
  const tasks = useModelStore((s) => s.getTasksForProvider(provider.id));

  return (
    <div className={`rounded-xl border p-4 transition-all duration-300 ${
      provider.enabled 
        ? "border-green-500/25 bg-green-500/5 shadow-[0_0_15px_rgba(34,197,94,0.05)]" 
        : "border-border bg-card/60 hover:bg-accent/40"
    }`}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3 min-w-0 flex-1">
          <div className={`relative flex h-10 w-10 shrink-0 items-center justify-center rounded-lg transition-all duration-300 ${
            provider.enabled 
              ? "bg-green-500/10 text-green-500 ring-2 ring-green-500/20 shadow-[0_0_8px_rgba(34,197,94,0.2)]" 
              : "bg-muted text-muted-foreground"
          }`}>
            <Logo className="h-5 w-5" />
            {provider.enabled && (
              <span className="absolute -top-0.5 -right-0.5 flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
              </span>
            )}
          </div>
          
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="font-semibold text-sm text-foreground leading-none">{provider.label}</h3>
              {provider.selectedModel && (
                <span className="rounded bg-primary/10 border border-primary/20 px-1.5 py-0.5 text-[10px] font-medium text-primary">
                  {provider.selectedModel}
                </span>
              )}
              {profile && (
                <span className="rounded bg-muted/80 border border-border/40 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground font-mono">
                  {profile.quality}/10 · {profile.speed} · {profile.cost}
                </span>
              )}
              <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[9px] font-medium ${
                provider.enabled 
                  ? "bg-green-500/10 text-green-600 dark:text-green-400" 
                  : "bg-muted text-muted-foreground"
              }`}>
                {provider.enabled ? "Active" : "Standby"}
              </span>
            </div>
            
            <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
              {routingMode === "auto" ? (
                <span className="text-[10px] text-muted-foreground/75 italic">
                  Available for Smart Routing
                </span>
              ) : (
                <span className={`text-[10px] font-medium transition-colors ${provider.enabled ? "text-green-600/80 dark:text-green-400/80" : "text-muted-foreground/75"}`}>
                  {provider.enabled ? "Selected for Swarm" : "Ready for Swarm"}
                </span>
              )}
              {tasks.length > 0 && (
                <>
                  <span className="text-muted-foreground/30 text-[10px]">•</span>
                  {tasks.map((task) => (
                    <span key={task} className={`rounded px-1.5 py-0.5 text-[9px] font-semibold tracking-wider uppercase ${SPECIALTY_COLORS[task] || "bg-muted text-muted-foreground"}`}>
                      {t(`models.task_${task}`)}
                    </span>
                  ))}
                </>
              )}
            </div>
          </div>
        </div>
        
        <button
          onClick={() => toggleProvider(provider.id)}
          className="rounded-lg p-2 text-muted-foreground hover:bg-accent shrink-0 ml-4 transition-all active:scale-95"
          title={provider.enabled ? t("plugins.disable") : t("plugins.enable")}
        >
          {provider.enabled ? (
            <ToggleRight className="h-6 w-6 text-green-500" />
          ) : (
            <ToggleLeft className="h-6 w-6 text-muted-foreground/45" />
          )}
        </button>
      </div>
    </div>
  );
}

export function ModelManager() {
  const { t } = useTranslation();
  const { providers, routingMode, setRoutingMode, costToday, costSaved } = useModelStore();
  const { verifiedProviders, loading, loadConfig, verifyAll } = useConfigStore();

  useEffect(() => {
    loadConfig().then(() => verifyAll());
  }, [loadConfig, verifyAll]);

  // A provider is strictly connected/configured if it is successfully tested and verified
  const configuredProviders = providers.filter((p) => {
    return verifiedProviders && verifiedProviders.includes(p.id);
  });

  const enabledCount = configuredProviders.filter((p) => p.enabled).length;

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("models.title")}</h2>
          <p className="text-sm text-muted-foreground">
            {t("models.count", { count: configuredProviders.length, online: enabledCount })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 rounded-lg border border-border bg-muted/40 p-0.5">
            <button
              onClick={() => setRoutingMode("auto")}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${routingMode === "auto" ? "bg-card text-foreground shadow-sm animate-pulse" : "text-muted-foreground hover:text-foreground"}`}
            >
              <Sparkles className="mr-1 inline h-3 w-3 text-gold" />
              {t("models.mode_auto")}
            </button>
            <button
              onClick={() => setRoutingMode("manual")}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${routingMode === "manual" ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
            >
              {t("models.mode_manual")}
            </button>
          </div>
        </div>
      </div>

      {routingMode === "auto" && (
        <div className="rounded-lg border border-primary/20 bg-primary/5 p-3">
          <div className="flex items-start gap-3">
            <Brain className="mt-0.5 h-5 w-5 text-primary shrink-0 animate-pulse" />
            <div>
              <p className="text-sm font-medium text-foreground">{t("models.orchestrator_title")}</p>
              <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                Task routing is fully automated: simple chats run on local/free models, complex reasoning goes to advanced models, and tool-assisted tasks target tool-capable models.
              </p>
              <div className="mt-2 flex items-center gap-4 text-xs font-mono">
                <span className="flex items-center gap-1 text-muted-foreground">
                  <DollarSign className="h-3 w-3" />
                  {t("models.cost_today") || "Today's Cost"}: ${costToday.toFixed(2)}
                </span>
                <span className="flex items-center gap-1 text-green-500">
                  <TrendingDown className="h-3 w-3" />
                  {t("models.cost_saved") || "Saved vs Single"}: ${costSaved.toFixed(2)}
                </span>
              </div>
            </div>
          </div>
        </div>
      )}

      {configuredProviders.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 px-6 text-center border border-dashed border-border rounded-xl bg-card/25 max-w-md mx-auto">
          <div className="rounded-full bg-amber-500/10 p-4 mb-4 text-amber-500 animate-pulse">
            <Settings className="h-8 w-8" />
          </div>
          <h3 className="text-sm font-semibold text-foreground mb-1">No Configured Models Found</h3>
          <p className="text-xs text-muted-foreground max-w-sm mb-4 leading-relaxed">
            All AI models are currently offline because no API keys or local host connections have been entered. Go to Settings (⚙️ in the top right menu) to connect your models!
          </p>
        </div>
      ) : (
        <div className="grid gap-3">
          {configuredProviders.map((p) => (
            <ProviderCard key={p.id} provider={p} />
          ))}
        </div>
      )}
    </div>
  );
}
