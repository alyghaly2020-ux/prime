import { useState, useEffect, useRef, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Search, Monitor, Brain, Server, Puzzle, Workflow, Activity, Terminal, Wrench, Cpu, ShieldCheck, Globe, Wifi, Settings, LayoutDashboard, Wallet } from "lucide-react";

interface PaletteAction {
  id: string;
  label: string;
  category: "panel" | "mode" | "action";
  icon: typeof Monitor;
  shortcut?: string;
  action: () => void;
}

export function CommandPalette({
  open,
  onClose,
  onNavigate,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (panel: string) => void;
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const actions: PaletteAction[] = [
    { id: "chat", label: t("command_palette.chat"), category: "mode", icon: Brain, shortcut: "F1", action: () => onNavigate("chat") },
    { id: "code", label: t("command_palette.code"), category: "mode", icon: Terminal, shortcut: "F2", action: () => onNavigate("code") },
    { id: "dashboard", label: t("command_palette.dashboard"), category: "mode", icon: LayoutDashboard, shortcut: "F3", action: () => onNavigate("dashboard") },
    { id: "agents", label: t("command_palette.agents"), category: "panel", icon: Brain, action: () => onNavigate("agents") },
    { id: "browser", label: t("command_palette.browser"), category: "panel", icon: Globe, action: () => onNavigate("browser") },
    { id: "events", label: t("command_palette.events"), category: "panel", icon: Activity, action: () => onNavigate("events") },
    { id: "logs", label: t("command_palette.logs"), category: "panel", icon: Terminal, action: () => onNavigate("logs") },
    { id: "mcp", label: t("command_palette.mcp"), category: "panel", icon: Server, action: () => onNavigate("mcp") },
    { id: "memory", label: t("command_palette.memory"), category: "panel", icon: Brain, action: () => onNavigate("memory") },
    { id: "models", label: t("command_palette.models"), category: "panel", icon: Cpu, action: () => onNavigate("models") },
    { id: "payments", label: t("command_palette.payments"), category: "panel", icon: Wallet, action: () => onNavigate("payments") },
    { id: "plugins", label: t("command_palette.plugins"), category: "panel", icon: Puzzle, action: () => onNavigate("plugins") },
    { id: "proxy", label: t("command_palette.proxy"), category: "panel", icon: Wifi, action: () => onNavigate("proxy") },
    { id: "security", label: t("command_palette.security"), category: "panel", icon: ShieldCheck, action: () => onNavigate("security") },
    { id: "tasks", label: t("command_palette.tasks"), category: "panel", icon: Monitor, action: () => onNavigate("observability") },
    { id: "tools", label: t("command_palette.tools"), category: "panel", icon: Wrench, action: () => onNavigate("tools") },
    { id: "workflows", label: t("command_palette.workflows"), category: "panel", icon: Workflow, action: () => onNavigate("workflows") },
    { id: "settings", label: t("command_palette.settings"), category: "panel", icon: Settings, action: () => onNavigate("settings") },
  ];

  const filtered = query.trim() === ""
    ? actions
    : actions.filter((a) => {
        const q = query.toLowerCase();
        return a.label.toLowerCase().includes(q) || a.id.toLowerCase().includes(q);
      });

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIdx(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const execute = useCallback((idx: number) => {
    if (filtered[idx]) {
      filtered[idx].action();
      onClose();
    }
  }, [filtered, onClose]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") { onClose(); return; }
    if (e.key === "ArrowDown") { e.preventDefault(); setSelectedIdx((i) => Math.min(i + 1, filtered.length - 1)); return; }
    if (e.key === "ArrowUp") { e.preventDefault(); setSelectedIdx((i) => Math.max(i - 1, 0)); return; }
    if (e.key === "Enter") { execute(selectedIdx); return; }
  };

  if (!open) return null;

  const categories = ["mode", "panel"] as const;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]" onClick={onClose}>
      <div className="fixed inset-0 bg-black/50" />
      <div className="relative w-full max-w-lg rounded-xl border border-border bg-card shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-3 border-b border-border px-4 py-3">
          <Search className="h-4 w-4 text-muted-foreground" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => { setQuery(e.target.value); setSelectedIdx(0); }}
            onKeyDown={handleKeyDown}
            placeholder={t("command_palette.placeholder")}
            className="flex-1 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/50"
          />
          <kbd className="rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{t("command_palette.esc")}</kbd>
        </div>
        <div className="max-h-80 overflow-y-auto p-2">
          {filtered.length === 0 && (
            <div className="py-8 text-center text-sm text-muted-foreground">
              {t("common.no_results", { query })}
            </div>
          )}
          {categories.map((cat) => {
            const items = filtered.filter((a) => a.category === cat);
            if (items.length === 0) return null;
            return (
              <div key={cat}>
                <p className="px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  {cat === "mode" ? t("command_palette.modes") : t("command_palette.panels")}
                </p>
                {items.map((item, _i) => {
                  const idx = filtered.indexOf(item);
                  const Icon = item.icon;
                  return (
                    <button
                      key={item.id}
                      onClick={() => execute(idx)}
                      className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                        idx === selectedIdx ? "bg-accent text-accent-foreground" : "text-foreground hover:bg-accent/50"
                      }`}
                    >
                      <Icon className="h-4 w-4 text-muted-foreground" />
                      <span className="flex-1">{item.label}</span>
                      {item.shortcut && (
                        <kbd className="rounded border border-border bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{item.shortcut}</kbd>
                      )}
                    </button>
                  );
                })}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
