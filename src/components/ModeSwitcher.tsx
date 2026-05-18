import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useViewMode, type ViewMode } from "@/stores/useViewMode";
import { useModelStore } from "@/stores/useModelStore";
import { useAgentActivityStore } from "@/stores/useAgentActivityStore";
import { listen } from "@tauri-apps/api/event";
import { MessageSquare, Code2, LayoutDashboard, Globe, Cpu, Wifi, Wallet } from "lucide-react";

export function ModeSwitcher() {
  const { t } = useTranslation();
  const { mode, setMode } = useViewMode();
  const activeProvider = useModelStore((s) => s.activeProvider);
  const routingMode = useModelStore((s) => s.routingMode);
  const providers = useModelStore((s) => s.providers);
  const currentProvider = providers.find((p) => p.id === activeProvider);

  const { chatActive, codeActive, browserActive, paymentsActive, setActivity } = useAgentActivityStore();

  useEffect(() => {
    const unlisten = listen<{ tab: string; active: boolean }>("agent-activity", (event) => {
      console.info("Agent activity event received:", event.payload);
      setActivity(event.payload.tab, event.payload.active);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setActivity]);

  const MODES: { id: ViewMode; label: string; icon: typeof MessageSquare; shortcut: string }[] = [
    { id: "chat", label: t("mode_switcher.chat"), icon: MessageSquare, shortcut: "F1" },
    { id: "code", label: t("mode_switcher.code"), icon: Code2, shortcut: "F2" },
    { id: "dashboard", label: t("mode_switcher.dashboard"), icon: LayoutDashboard, shortcut: "F3" },
    { id: "browser", label: t("mode_switcher.browser"), icon: Globe, shortcut: "F4" },
    { id: "models", label: t("mode_switcher.models"), icon: Cpu, shortcut: "F5" },
    { id: "connections", label: t("mode_switcher.connections"), icon: Wifi, shortcut: "F6" },
    { id: "payments", label: t("mode_switcher.payments"), icon: Wallet, shortcut: "F7" },
  ];

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "F1") { e.preventDefault(); setMode("chat"); }
      if (e.key === "F2") { e.preventDefault(); setMode("code"); }
      if (e.key === "F3") { e.preventDefault(); setMode("dashboard"); }
      if (e.key === "F4") { e.preventDefault(); setMode("browser"); }
      if (e.key === "F5") { e.preventDefault(); setMode("models"); }
      if (e.key === "F6") { e.preventDefault(); setMode("connections"); }
      if (e.key === "F7") { e.preventDefault(); setMode("payments"); }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setMode]);

  return (
    <div className="flex items-center gap-1 rounded-xl bg-muted/40 p-0.5 border border-border/30">
      {MODES.map((m) => {
        const Icon = m.icon;
        const isActive = mode === m.id;
        const isActivityActive = 
          (m.id === "chat" && chatActive) ||
          (m.id === "code" && codeActive) ||
          (m.id === "browser" && browserActive) ||
          (m.id === "payments" && paymentsActive);

        return (
          <button
            key={m.id}
            onClick={() => setMode(m.id)}
            className={`relative flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm font-medium transition-all duration-300 ${
              isActive
                ? "bg-card text-foreground shadow-sm dark:shadow-gold/5 border border-primary/20"
                : isActivityActive
                ? "bg-green-500/10 text-green-500 border border-green-500/35 shadow-[0_0_8px_rgba(34,197,94,0.15)] animate-pulse"
                : "text-muted-foreground hover:text-foreground hover:bg-accent/50 border border-transparent"
            }`}
          >
            <Icon className={`h-4 w-4 ${isActive ? "text-primary" : isActivityActive ? "text-green-500 animate-spin-slow" : ""}`} />
            <span>{m.label}</span>
            {m.id === "models" && currentProvider && !isActive && (
              <span className="text-[10px] text-muted-foreground/60 hidden lg:inline">
                {routingMode === "auto" ? "A" : currentProvider.label}
              </span>
            )}
            {m.id === "payments" && !isActive && !isActivityActive && (
              <span className="text-[10px] text-muted-foreground/60 hidden lg:inline">
                💳
              </span>
            )}
            <kbd className="ml-0.5 hidden rounded border border-border/40 bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground/60 md:inline">
              {m.shortcut}
            </kbd>

            {isActivityActive && (
              <span className="absolute -top-1 -right-1 flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
}
