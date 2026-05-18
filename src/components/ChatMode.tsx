import { useState, useRef, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage, AgentInfo } from "@/types";
import { useModelStore } from "@/stores/useModelStore";
import { useConfigStore } from "@/stores/useConfigStore";
import { useChatStore } from "@/stores/useChatStore";
import { usePhiBrainStore } from "@/stores/usePhiBrainStore";
import {
  Send,
  Bot,
  User,
  Loader2,
  AlertCircle,
  Sparkles,
  History,
  Cpu,
  PanelRightOpen,
  PanelRightClose,
  MessageSquarePlus,
} from "lucide-react";

function formatTime(ts: number) {
  return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function MarkdownBlock({ content }: { content: string }) {
  return (
    <div className="prose prose-sm dark:prose-invert max-w-none">
      {content.split("```").map((part, i) => {
        if (i % 2 === 0) {
          return part.split("\n").map((line, j) => {
            if (line.startsWith("# ")) return <h1 key={j} className="text-lg font-bold mt-3 mb-1">{line.slice(2)}</h1>;
            if (line.startsWith("## ")) return <h2 key={j} className="text-base font-bold mt-2 mb-1">{line.slice(3)}</h2>;
            if (line.startsWith("### ")) return <h3 key={j} className="text-sm font-bold mt-2 mb-1">{line.slice(4)}</h3>;
            if (line.startsWith("- ")) return <li key={j} className="text-sm ml-4 list-disc">{line.slice(2)}</li>;
            if (line.startsWith("1. ")) return <li key={j} className="text-sm ml-4 list-decimal">{line.slice(3)}</li>;
            if (line.startsWith("> ")) return <blockquote key={j} className="border-l-2 border-muted-foreground/30 pl-3 text-sm italic text-muted-foreground">{line.slice(2)}</blockquote>;
            if (line.match(/^```/)) return null;
            return line.trim() ? <p key={j} className="text-sm mb-1">{line}</p> : null;
          });
        }
        return (
          <pre key={i} className="my-2 overflow-x-auto rounded-lg bg-muted p-3 text-xs font-mono">
            <code>{part}</code>
          </pre>
        );
      })}
    </div>
  );
}

export function ChatMode() {
  const { t } = useTranslation();
  const sessions = useChatStore((s) => s.sessions);
  const activeSessionId = useChatStore((s) => s.activeSessionId);
  const createSession = useChatStore((s) => s.createSession);
  const setActiveSession = useChatStore((s) => s.setActiveSession);
  const addMessage = useChatStore((s) => s.addMessage);
  const messages = useChatStore((s) => {
    const session = s.sessions.find((ss) => ss.id === s.activeSessionId);
    return session?.messages || [];
  });
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [contextOpen, setContextOpen] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const initialized = useRef(false);

  // Ensure at least one session exists (runs once on mount)
  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    if (!activeSessionId && sessions.length === 0) {
      createSession(t("chat.new_conversation"));
    } else if (!activeSessionId && sessions.length > 0) {
      setActiveSession(sessions[0].id);
    }
  }, []);

  // Listen for Phi Brain corrections
  const addHealthAlert = usePhiBrainStore((s) => s.addHealthAlert);
  useEffect(() => {
    const unlisten = listen("phi-correction", (event) => {
      const data = event.payload as { corrections: string[]; hallucination_score: number };
      if (data.corrections?.length > 0) {
        addHealthAlert({
          level: "info",
          message: `Phi Brain: ${data.corrections.length} corrections applied`,
          advice: `Hallucination score: ${(data.hallucination_score * 100).toFixed(0)}%`,
          auto_action: null,
          timestamp: Date.now(),
        });
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [addHealthAlert]);

  const providers = useModelStore((s) => s.providers);
  const [selectedProvider, setSelectedProvider] = useState("");

  const { data: agents } = useQuery({
    queryKey: ["agents"],
    queryFn: async () => {
      const raw = await invoke<string>("list_agents");
      return JSON.parse(raw) as AgentInfo[];
    },
  });

  const verifiedProviders = useConfigStore((s) => s.verifiedProviders);
  const loadConfig = useConfigStore((s) => s.loadConfig);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const activeProvider = selectedProvider || "auto";
  const enabledProviders = providers.filter((p) => p.enabled && verifiedProviders && verifiedProviders.includes(p.id));

  const sendMessage = useCallback(async () => {
    if (!input.trim() || loading) return;
    const sid = activeSessionId || createSession(input.trim());
    const userMsg: ChatMessage = { role: "user", content: input, timestamp: Date.now() };
    addMessage(sid, userMsg);
    setInput("");
    setLoading(true);
    setError(null);
    try {
      const currentMessages = useChatStore.getState().activeMessages();
      const response = await invoke<string>("ai_chat", {
        messages: currentMessages,
        model: activeProvider || "default",
      });
      const assistantMsg: ChatMessage = { role: "assistant", content: response, timestamp: Date.now() };
      addMessage(sid, assistantMsg);
    } catch (e: any) {
      if (typeof e === "string") {
        setError(e);
      } else if (e && typeof e === "object" && "message" in e && typeof e.message === "string") {
        setError(e.message);
      } else if (e instanceof Error) {
        setError(e.message);
      } else {
        setError(JSON.stringify(e) || "Chat request failed");
      }
    } finally {
      setLoading(false);
      inputRef.current?.focus();
    }
  }, [input, loading, activeProvider, activeSessionId, addMessage, createSession]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  return (
    <div className="flex h-full">
      {/* Main chat area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-6 py-3 bg-card/30">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="relative">
                <Bot className="h-4 w-4 text-primary" />
                <span className="absolute -top-0.5 -right-0.5 h-1.5 w-1.5 rounded-full bg-green-500" />
              </div>
              <h2 className="text-sm font-semibold text-foreground">{t("chat.title")}</h2>
            </div>
            <select
              value={activeProvider}
              onChange={(e) => setSelectedProvider(e.target.value)}
              className="rounded-lg border border-input bg-background px-2.5 py-1.5 text-xs text-muted-foreground focus-visible:ring-1 focus-visible:ring-ring font-medium"
            >
              <option value="auto">🤖 {t("models.mode_auto") || "Smart Auto"}</option>
              {enabledProviders.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.label}
                </option>
              ))}
            </select>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => {
                const sid = createSession();
                setActiveSession(sid);
              }}
              className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
              title={t("chat.new_conversation")}
            >
              <MessageSquarePlus className="h-3.5 w-3.5" />
              {t("chat.new")}
            </button>
            {sessions.length > 1 && (
              <select
                value={activeSessionId || ""}
                onChange={(e) => setActiveSession(e.target.value)}
                className="rounded-md border border-input bg-background px-2 py-1 text-xs text-muted-foreground"
              >
                {sessions.map((s) => (
                  <option key={s.id} value={s.id}>{s.title.slice(0, 20)}</option>
                ))}
              </select>
            )}
            <button
              onClick={() => setContextOpen(!contextOpen)}
              className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-muted-foreground hover:text-foreground"
            >
              {contextOpen ? <PanelRightClose className="h-3.5 w-3.5" /> : <PanelRightOpen className="h-3.5 w-3.5" />}
              {contextOpen ? t("chat.hide_context") : t("chat.show_context")}
            </button>
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-6 space-y-4 scrollbar-thin">
          {messages.length === 0 && !error && (
            <div className="flex h-full flex-col items-center justify-center text-center">
              <div className="mb-6 relative">
                <div className="rounded-full bg-primary/10 p-5 gold-glow">
                  <Bot className="h-10 w-10 text-primary" />
                </div>
                <span className="absolute -bottom-1 -right-1 flex h-3 w-3">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400/60" />
                  <span className="relative inline-flex rounded-full h-3 w-3 bg-green-500" />
                </span>
              </div>
              <h3 className="text-lg font-semibold text-foreground mb-1">{t("chat.welcome")}</h3>
              <p className="text-sm text-muted-foreground max-w-md leading-relaxed">
                {t("chat.description")}
              </p>
              <div className="mt-8 flex flex-wrap justify-center gap-2">
                {[t("chat.hint.refactor"), t("chat.hint.test"), t("chat.hint.explain"), t("chat.hint.bug")].map((hint) => (
                  <button
                    key={hint}
                    onClick={() => { setInput(hint); inputRef.current?.focus(); }}
                    className="rounded-full border border-border/60 bg-card px-3.5 py-1.5 text-xs text-muted-foreground hover:border-primary/30 hover:bg-primary/5 hover:text-foreground transition-all"
                  >
                    {hint}
                  </button>
                ))}
              </div>
            </div>
          )}

          {error && (
            <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
              <div>
                <p className="font-medium">{t("chat.error")}</p>
                <p className="mt-0.5 text-destructive/80">{error}</p>
              </div>
            </div>
          )}

          {messages.map((msg, i) => (
            <div key={i} className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}>
              <div className={`flex gap-3 max-w-[80%] ${msg.role === "user" ? "flex-row-reverse" : ""}`}>
                <div
                  className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full border ${
                    msg.role === "user"
                      ? "bg-primary text-primary-foreground border-primary/20"
                      : "bg-muted text-muted-foreground border-border/50"
                  }`}
                >
                  {msg.role === "user" ? <User className="h-4 w-4" /> : <Bot className="h-4 w-4" />}
                </div>
                <div>
                  <div
                    className={`rounded-xl px-4 py-2.5 ${
                      msg.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "bg-card border border-border shadow-sm"
                    }`}
                  >
                    {msg.role === "assistant" ? (
                      <MarkdownBlock content={msg.content} />
                    ) : (
                      <p className="text-sm whitespace-pre-wrap leading-relaxed">{msg.content}</p>
                    )}
                  </div>
                  <p className={`mt-1 text-[10px] text-muted-foreground/40 ${msg.role === "user" ? "text-right" : ""}`}>
                    {formatTime(msg.timestamp)}
                  </p>
                </div>
              </div>
            </div>
          ))}

          {loading && (
            <div className="flex justify-start">
              <div className="flex gap-3 max-w-[80%]">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
                  <Bot className="h-4 w-4" />
                </div>
                <div className="rounded-lg border border-border bg-card px-4 py-3">
                  <div className="flex items-center gap-2">
                    <div className="h-2 w-2 animate-bounce rounded-full bg-foreground/40" />
                    <div className="h-2 w-2 animate-bounce rounded-full bg-foreground/40" style={{ animationDelay: "0.15s" }} />
                    <div className="h-2 w-2 animate-bounce rounded-full bg-foreground/40" style={{ animationDelay: "0.3s" }} />
                  </div>
                </div>
              </div>
            </div>
          )}

          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div className="border-t border-border bg-card/20 p-4">
          <form
            onSubmit={(e) => { e.preventDefault(); sendMessage(); }}
            className="flex gap-3"
          >
            <div className="relative flex-1">
              <input
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={t("chat.placeholder")}
                disabled={loading}
                className="w-full rounded-xl border border-input bg-background px-4 py-2.5 pr-10 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-primary/30 disabled:opacity-50 transition-all"
              />
              {loading && (
                <div className="absolute right-3 top-1/2 -translate-y-1/2">
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                </div>
              )}
            </div>
            <button
              type="submit"
              disabled={loading || !input.trim()}
              className="inline-flex items-center justify-center gap-2 rounded-xl bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 transition-all shadow-sm hover:shadow-md"
            >
              <Send className="h-4 w-4" />
              <span className="hidden sm:inline">{t("chat.send")}</span>
            </button>
          </form>
        </div>
      </div>

      {/* Context sidebar */}
      {contextOpen && (
        <aside className="w-64 border-l border-border bg-card/50 flex flex-col">
          <div className="border-b border-border px-4 py-3">
            <div className="flex items-center gap-2">
              <Sparkles className="h-4 w-4 text-primary" />
              <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("chat.context")}</h3>
            </div>
          </div>

          {/* Active agents */}
          <div className="px-4 py-3">
            <div className="mb-2 flex items-center gap-1.5">
              <Cpu className="h-3.5 w-3.5 text-muted-foreground" />
              <h4 className="text-xs font-medium text-muted-foreground uppercase">{t("chat.agents")}</h4>
            </div>
            <div className="space-y-1">
              {agents?.slice(0, 5).map((a) => (
                <div key={a.id} className="flex items-center gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-accent/50 cursor-pointer">
                  <div className="h-1.5 w-1.5 rounded-full bg-green-500" />
                  <span className="text-foreground truncate">{a.name}</span>
                </div>
              ))}
              {agents && agents.length > 5 && (
                <p className="px-2 text-[10px] text-muted-foreground">{t("chat.more", { count: agents.length - 5 })}</p>
              )}
            </div>
          </div>

          {/* Recent history */}
          <div className="px-4 py-3 border-t border-border">
            <div className="mb-2 flex items-center gap-1.5">
              <History className="h-3.5 w-3.5 text-muted-foreground" />
              <h4 className="text-xs font-medium text-muted-foreground uppercase">{t("chat.history")}</h4>
            </div>
            {messages.length === 0 ? (
              <p className="text-xs text-muted-foreground/50 px-2">{t("chat.no_messages")}</p>
            ) : (
              <div className="space-y-1">
                {messages.slice(-3).map((m, i) => (
                  <div key={i} className="truncate rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent/50">
                    {m.content.slice(0, 40)}{m.content.length > 40 ? "..." : ""}
                  </div>
                ))}
              </div>
            )}
          </div>
        </aside>
      )}
    </div>
  );
}
