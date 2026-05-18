import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAppStore } from "@/stores/useAppStore";
import { useTheme } from "@/hooks/useTheme";
import { useViewMode, type ViewMode } from "@/stores/useViewMode";
import { ModeSwitcher } from "@/components/ModeSwitcher";
import { ChatMode } from "@/components/ChatMode";
import { CodeMode } from "@/components/ide/CodeMode";
import { DashboardMode } from "@/components/DashboardMode";
import { BrowserModeFull } from "@/components/BrowserModeFull";
import { LauncherScreen } from "@/components/LauncherScreen";
import { WorkflowPanel } from "@/components/WorkflowPanel";
import { PluginManager } from "@/components/PluginManager";
import { McpManager } from "@/components/McpManager";
import { AgentMonitor } from "@/components/AgentMonitor";
import { TaskMonitor } from "@/components/TaskMonitor";
import { BrowserAutomation } from "@/components/BrowserAutomation";
import { SecurityDashboard } from "@/components/SecurityDashboard";
import { ProxyPoolManager } from "@/components/ProxyPoolManager";
import { CommandPalette } from "@/components/CommandPalette";
import { UpdateBanner } from "@/components/UpdateBanner";
import { useUpdateStore } from "@/stores/useUpdateStore";
import { useChatStore } from "@/stores/useChatStore";
import { usePhiBrainStore } from "@/stores/usePhiBrainStore";
import { MemoryViewer } from "@/components/MemoryViewer";
import { EventTimeline } from "@/components/EventTimeline";
import { LogsViewer } from "@/components/LogsViewer";
import { ModelManager } from "@/components/ModelManager";
import { SettingsPanel } from "@/components/SettingsPanel";
import { ToolsRegistry } from "@/components/ToolsRegistry";
import { OnboardingWizard } from "@/components/OnboardingWizard";
import { LanguageSwitcher } from "@/components/LanguageSwitcher";
import { ConnectionsPanel } from "@/components/ConnectionsPanel";
import { PaymentsPanel } from "@/components/PaymentsPanel";
import {
  Wrench,
  Loader2,
  Sun,
  Moon,
  Monitor,
  ChevronDown,
  Brain,
  Server,
  Puzzle,
  Activity,
  Terminal,
  LayoutDashboard,
  ShieldCheck,
  Wifi,
  Cpu,
  Globe,
  Settings,
  Sparkles,
  X,
} from "lucide-react";

type Panel =
  | "memory" | "mcp" | "plugins" | "workflows" | "events" | "logs"
  | "agents" | "tools" | "models" | "browser" | "security" | "proxy"
  | "observability" | "settings";

const PANEL_ITEMS: { id: Panel; label: string; icon: typeof Brain }[] = [
  { id: "agents", label: "sidebar.agents", icon: LayoutDashboard },
  { id: "memory", label: "sidebar.memory", icon: Brain },
  { id: "mcp", label: "sidebar.mcp", icon: Server },
  { id: "plugins", label: "sidebar.plugins", icon: Puzzle },
  { id: "tools", label: "sidebar.tools", icon: Sparkles },
  { id: "events", label: "sidebar.events", icon: Activity },
  { id: "logs", label: "sidebar.logs", icon: Terminal },
  { id: "workflows", label: "sidebar.workflows", icon: Wrench },
  { id: "browser", label: "sidebar.browser", icon: Globe },
  { id: "security", label: "sidebar.security", icon: ShieldCheck },
  { id: "proxy", label: "sidebar.proxy", icon: Wifi },
  { id: "observability", label: "sidebar.tasks", icon: Cpu },
  { id: "settings", label: "sidebar.settings", icon: Settings },
];

function DiagPing() {
  const { t } = useTranslation();
  const { data, error, isLoading } = useQuery({
    queryKey: ["ping"],
    queryFn: () => invoke<string>("ping"),
    retry: 1,
    staleTime: 60000,
  });
  if (isLoading) return <Loader2 className="ml-2 h-3 w-3 animate-spin text-muted-foreground" />;
  if (error) return <span className="ml-2 text-[10px] text-muted-foreground/50 font-medium">{t("app.browser_only")}</span>;
  return <span className="ml-1 text-xs text-green-500">{data}</span>;
}

function SysPanelDropdown({ activePanel, onSelect, isOpen, onToggle }: { activePanel: string; onSelect: (p: string) => void; isOpen: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setMenuOpen(false);
    };
    if (menuOpen) document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [menuOpen]);

  const handleSelect = useCallback((p: string) => {
    onSelect(p);
    setMenuOpen(false);
  }, [onSelect]);

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => {
          if (isOpen) {
            onToggle();
            setMenuOpen(false);
          } else {
            setMenuOpen(true);
          }
        }}
        className={`inline-flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
          isOpen
            ? "bg-primary/15 text-primary shadow-sm dark:shadow-gold/10"
            : "text-muted-foreground hover:text-foreground hover:bg-accent"
        }`}
      >
        <Wrench className="h-3.5 w-3.5" />
        <span className="hidden sm:inline">{t("app.system")}</span>
        <ChevronDown className={`h-3 w-3 transition-transform duration-200 ${menuOpen ? "rotate-180" : ""}`} />
      </button>
      {menuOpen && (
        <div className="absolute right-0 top-full z-30 mt-1 w-44 rounded-lg border border-border bg-card py-1 shadow-lg">
          <div className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/50">
            {t("sidebar.title")}
          </div>
          {PANEL_ITEMS.map((item) => {
            const Icon = item.icon;
            const isActive = activePanel === item.id;
            return (
              <button
                key={item.id}
                onClick={() => handleSelect(item.id)}
                className={`flex w-full items-center gap-2 px-3 py-1.5 text-xs transition-colors hover:bg-accent ${
                  isActive ? "text-primary font-medium" : "text-card-foreground"
                }`}
              >
                <Icon className="h-3.5 w-3.5 shrink-0" />
                {t(item.label)}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function App() {
  const { activePanel, setActivePanel, onboardingCompleted, launcherCompleted } = useAppStore();
  const { theme, setTheme } = useTheme();
  const { mode, setMode } = useViewMode();
  const { t } = useTranslation();
  const [sysPanelOpen, setSysPanelOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        setPaletteOpen(true);
      }
      if (e.key === "Escape" && sysPanelOpen) {
        setSysPanelOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [sysPanelOpen]);

  useEffect(() => {
    const unlisten = listen<string>("change-view-mode", (event) => {
      console.info("View mode change requested by backend:", event.payload);
      setMode(event.payload as ViewMode);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setMode]);

  const handlePaletteNavigate = (target: string) => {
    if (["chat", "code", "dashboard", "browser", "models", "connections", "payments"].includes(target)) {
      setMode(target as ViewMode);
    } else {
      setActivePanel(target);
    }
  };

  const handleSysPanelSelect = (p: string) => {
    setActivePanel(p);
    setSysPanelOpen(true);
  };

  // Phi Brain event listeners (health alerts + corrections) + chat load
  useEffect(() => {
    useChatStore.getState().loadSessions();

    const unlistenHealth = listen("phi-health-alert", (event) => {
      const alert = event.payload as {
        level: string;
        message: string;
        advice: string;
        auto_action: string | null;
        timestamp: number;
      };
      usePhiBrainStore.getState().addHealthAlert(alert);
    });

    const unlistenCorrection = listen("phi-correction", (event) => {
      const data = event.payload as {
        corrections: Array<{ kind: string; original: string; fixed: string; confidence: number }>;
        hallucination_score: number;
      };
      console.info(
        `Phi Brain: ${data.corrections.length} corrections applied (hallucination_score=${data.hallucination_score})`
      );
    });

    return () => {
      unlistenHealth.then((fn) => fn());
      unlistenCorrection.then((fn) => fn());
    };
  }, []);

  // Daily auto-update check
  const updateCheck = useUpdateStore((s) => s.check);
  useEffect(() => {
    const DAY_MS = 86_400_000;
    updateCheck();
    const interval = setInterval(updateCheck, DAY_MS);
    return () => clearInterval(interval);
  }, [updateCheck]);

  if (!launcherCompleted) {
    return <LauncherScreen />;
  }

  if (!onboardingCompleted) {
    return <OnboardingWizard />;
  }

  const renderSysPanel = () => {
    switch (activePanel) {
      case "memory": return <MemoryViewer />;
      case "mcp": return <McpManager />;
      case "plugins": return <PluginManager />;
      case "workflows": return <WorkflowPanel />;
      case "events": return <EventTimeline />;
      case "logs": return <LogsViewer />;
      case "agents": return <AgentMonitor />;
      case "tools": return <ToolsRegistry />;
      case "browser": return <BrowserAutomation />;
      case "security": return <SecurityDashboard />;
      case "proxy": return <ProxyPoolManager />;
      case "observability": return <TaskMonitor />;
      case "settings": return <SettingsPanel />;
      default: return null;
    }
  };

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* Top bar */}
      <header className="flex items-center justify-between border-b border-border px-4 py-2 bg-card/30 shrink-0">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-3">
            <div className="relative">
              <img src="/prime_no_text.png" alt={t("app.title")} className="h-7 w-7 object-contain" />
              <span className="absolute -top-0.5 -right-0.5 flex h-2 w-2">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-gold/60 opacity-75" />
                <span className="relative inline-flex rounded-full h-2 w-2 bg-gold" />
              </span>
            </div>
            <span className="text-sm font-semibold text-foreground hidden sm:inline tracking-tight">
              {t("app.title")}
            </span>
          </div>

          <ModeSwitcher />
        </div>

        <div className="flex items-center gap-2">
          {/* Sys panel toggle with dropdown */}
          <SysPanelDropdown activePanel={activePanel} onSelect={handleSysPanelSelect} isOpen={sysPanelOpen} onToggle={() => setSysPanelOpen(!sysPanelOpen)} />

          {/* Language Switcher */}
          <LanguageSwitcher />

          {/* Status */}
          <div className="flex items-center gap-1.5 px-2">
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400/60" />
              <span className="relative inline-flex rounded-full h-2 w-2 bg-green-500" />
            </span>
            <span className="text-xs text-muted-foreground hidden sm:inline">{t("app.status.running")}</span>
            <DiagPing />
          </div>

          {/* Theme toggle */}
          <button
            onClick={() => {
              const order: ("light" | "dark" | "system")[] = ["light", "dark", "system"];
              const idx = order.indexOf(theme);
              setTheme(order[(idx + 1) % order.length]);
            }}
            className="rounded-lg p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent transition-all"
            title={t("app.theme.toggle")}
          >
            {theme === "light" ? (
              <Sun className="h-4 w-4 text-sunrise" />
            ) : theme === "dark" ? (
              <Moon className="h-4 w-4 text-gold" />
            ) : (
              <Monitor className="h-4 w-4" />
            )}
          </button>
        </div>
      </header>

      {/* Sys panel dropdown content */}
      {sysPanelOpen && activePanel && (
        <div className="border-b border-border bg-card/60 backdrop-blur-sm shrink-0">
          <div className="px-4 py-3">
            <div className="flex items-center justify-between mb-3">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/50">{activePanel}</span>
              <button onClick={() => setSysPanelOpen(false)} className="rounded p-1 text-muted-foreground hover:text-foreground hover:bg-accent transition-all">
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
            {renderSysPanel()}
          </div>
        </div>
      )}

      {/* Mode content */}
      <div className="flex-1 overflow-hidden">
        {mode === "chat" && <ChatMode />}
        {mode === "code" && <CodeMode />}
        {mode === "dashboard" && <DashboardMode />}
        {mode === "browser" && <BrowserModeFull />}
        {mode === "models" && <ModelManager />}
        {mode === "connections" && <ConnectionsPanel />}
        {mode === "payments" && <PaymentsPanel />}
        <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} onNavigate={handlePaletteNavigate} />
        <UpdateBanner />
      </div>
    </div>
  );
}

export default App;
