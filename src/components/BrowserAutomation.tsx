import { useState, useCallback } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Globe, ArrowLeft, ArrowRight, RotateCw, Loader2, Terminal, Eye, Code, Play, ExternalLink, AlertCircle, Bot, Plus, X } from "lucide-react";

interface Tab {
  id: string;
  url: string;
  title: string;
  screenshot?: string;
  html?: string;
  a11yTree?: string;
}

export function BrowserAutomation() {
  const { t } = useTranslation();
  const [tabs, setTabs] = useState<Tab[]>([{ id: '1', url: 'https://duckduckgo.com', title: 'New Tab' }]);
  const [activeTabId, setActiveTabId] = useState('1');
  const [urlInput, setUrlInput] = useState("https://duckduckgo.com");
  const [view, setView] = useState<"preview" | "dom" | "a11y">("preview");
  const [connecting, setConnecting] = useState(false);
  const [navigating, setNavigating] = useState(false);
  const [aiControlled, setAiControlled] = useState(false);

  const activeTab = tabs.find(t => t.id === activeTabId);

  const { data: connected, isLoading, error, refetch } = useQuery({
    queryKey: ["browser_is_connected"],
    queryFn: () => invoke<boolean>("browser_is_connected"),
    refetchInterval: 5000,
  });

  const isConnected = connected === true;

  const refreshSnapshot = useCallback(async () => {
    if (!isConnected) return;
    try {
      const snapshot = await invoke<any>("browser_snapshot");
      setTabs(prev => prev.map(t => t.id === activeTabId ? { 
        ...t, 
        url: snapshot.url || t.url, 
        title: snapshot.title || t.title,
        screenshot: snapshot.screenshot,
        html: snapshot.html,
        a11yTree: snapshot.a11y_tree
      } : t));
      if (snapshot.url) {
        setUrlInput(snapshot.url);
      }
    } catch (e) {
      console.error("Failed to refresh snapshot:", e);
    }
  }, [activeTabId, isConnected]);

  const handleConnect = useCallback(async () => {
    setConnecting(true);
    try {
      await invoke("browser_connect");
      await refetch();
      setTimeout(() => {
        refreshSnapshot();
      }, 1200);
    } catch (e) {
      console.error("Failed to connect browser:", e);
    } finally {
      setConnecting(false);
    }
  }, [refetch, refreshSnapshot]);

  const handleNavigate = useCallback(async () => {
    if (!urlInput || !activeTab) return;
    let finalUrl = urlInput;
    if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
      finalUrl = 'https://' + finalUrl;
    }
    setTabs(prev => prev.map(t => t.id === activeTabId ? { ...t, url: finalUrl, title: finalUrl } : t));
    if (isConnected) {
      setNavigating(true);
      try {
        const snapshot = await invoke<any>("browser_navigate", { url: finalUrl });
        setTabs(prev => prev.map(t => t.id === activeTabId ? { 
          ...t, 
          url: snapshot.url, 
          title: snapshot.title || snapshot.url,
          screenshot: snapshot.screenshot,
          html: snapshot.html,
          a11yTree: snapshot.a11y_tree
        } : t));
      } catch (e) {
        console.error("Navigation failed:", e);
      } finally {
        setNavigating(false);
      }
    }
  }, [urlInput, activeTab, activeTabId, isConnected]);

  const handleDisconnect = useCallback(async () => {
    try {
      await invoke("browser_disconnect");
      await refetch();
      setTabs(prev => prev.map(t => t.id === activeTabId ? { ...t, screenshot: undefined, html: undefined, a11yTree: undefined } : t));
    } catch (e) {
      console.error("Failed to disconnect:", e);
    }
  }, [refetch, activeTabId]);

  const addTab = () => {
    const id = Date.now().toString();
    setTabs([...tabs, { id, url: 'https://duckduckgo.com', title: 'New Tab' }]);
    setActiveTabId(id);
    setUrlInput('https://duckduckgo.com');
  };

  const closeTab = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (tabs.length === 1) return;
    const newTabs = tabs.filter(t => t.id !== id);
    setTabs(newTabs);
    if (activeTabId === id) {
      const newActive = newTabs[newTabs.length - 1];
      setActiveTabId(newActive.id);
      setUrlInput(newActive.url);
    }
  };

  const switchTab = (id: string) => {
    const tab = tabs.find(t => t.id === id);
    if (tab) {
      setActiveTabId(id);
      setUrlInput(tab.url);
    }
  };

  return (
    <div className={`h-full flex flex-col overflow-hidden transition-all duration-300 ${aiControlled ? 'border-4 border-green-500/50 shadow-[inset_0_0_20px_rgba(34,197,94,0.2)]' : ''}`}>
      {/* Header */}
      <div className="border-b border-border bg-card px-4 py-2">
        <div className="flex items-center gap-2">
          <h1 className="text-sm font-semibold text-foreground shrink-0">{t("browser.title")}</h1>
          <span className="text-xs text-muted-foreground">{t("browser.engine")}</span>
          <div className={`ml-2 flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] ${isConnected ? "bg-green-500/10 text-green-500" : "bg-muted text-muted-foreground"}`}>
            <div className={`h-1.5 w-1.5 rounded-full ${isConnected ? "bg-green-500" : "bg-muted-foreground/50"}`} />
            {isConnected ? t("browser.connected") : t("browser.disconnected")}
          </div>
          <div className="flex-1" />
          <button
            onClick={() => setAiControlled(!aiControlled)}
            className={`flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-md font-medium transition-colors ${aiControlled ? 'bg-green-500/20 text-green-500 border border-green-500/30' : 'bg-accent/50 text-muted-foreground hover:text-foreground'}`}
          >
            <Bot className="h-3.5 w-3.5" />
            AI Auto
          </button>
        </div>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 border-b border-border bg-destructive/10 px-4 py-2 text-xs text-destructive">
          <AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
          <p>{t("browser.error", { error: String(error) })}</p>
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center bg-card/50 border-b border-border overflow-x-auto no-scrollbar px-2 pt-2">
        {tabs.map(tab => (
          <div
            key={tab.id}
            onClick={() => switchTab(tab.id)}
            className={`group flex items-center gap-2 px-3 py-1.5 text-xs max-w-[200px] min-w-[120px] cursor-pointer rounded-t-md border-t border-x ${activeTabId === tab.id ? 'bg-background border-border text-foreground' : 'bg-transparent border-transparent text-muted-foreground hover:bg-accent/30'}`}
          >
            <span className="truncate flex-1">{tab.title}</span>
            <button onClick={(e) => closeTab(tab.id, e)} className="opacity-0 group-hover:opacity-100 hover:bg-accent rounded p-0.5">
              <X className="h-3 w-3" />
            </button>
          </div>
        ))}
        <button onClick={addTab} className="ml-1 p-1 text-muted-foreground hover:text-foreground hover:bg-accent rounded-md">
          <Plus className="h-4 w-4" />
        </button>
      </div>

      {/* URL bar */}
      <div className="flex items-center gap-2 border-b border-border bg-background px-3 py-2">
        <div className="flex items-center gap-1">
          <button className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled><ArrowLeft className="h-4 w-4" /></button>
          <button className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled><ArrowRight className="h-4 w-4" /></button>
          <button 
            onClick={refreshSnapshot} 
            disabled={!isConnected || navigating}
            className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30"
          >
            {navigating ? <Loader2 className="h-4 w-4 animate-spin text-primary" /> : <RotateCw className="h-4 w-4" />}
          </button>
        </div>
        <div className="relative flex-1">
          <Globe className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground/50" />
          <input
            type="text"
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            onKeyDown={(e) => { if (e.key === "Enter") handleNavigate(); }}
            placeholder={isConnected ? t("browser.placeholder_connected") : t("browser.placeholder_disconnected")}
            disabled={!isConnected || navigating}
            className="w-full rounded-md border border-input bg-background py-1.5 pl-8 pr-3 text-sm text-foreground placeholder:text-muted-foreground/50 disabled:opacity-50"
          />
        </div>
        <button
          onClick={handleNavigate}
          disabled={!isConnected || !urlInput || navigating}
          className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          {navigating ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Play className="h-3.5 w-3.5" />}
          {t("browser.go")}
        </button>
        <button
          onClick={isConnected ? handleDisconnect : handleConnect}
          disabled={isLoading || connecting}
          className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-xs text-foreground hover:bg-accent"
        >
          {connecting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <ExternalLink className="h-3.5 w-3.5" />}
          {isConnected ? t("browser.disconnect") : t("browser.connect")}
        </button>
      </div>

      {/* View tabs */}
      <div className="flex gap-1 border-b border-border bg-muted/30 px-3 py-1.5">
        {(["preview", "dom", "a11y"] as const).map((v) => (
          <button
            key={v}
            onClick={() => setView(v)}
            className={`inline-flex items-center gap-1 rounded-md px-2.5 py-1 text-xs font-medium ${
              view === v ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {v === "preview" && <Eye className="h-3 w-3" />}
            {v === "dom" && <Code className="h-3 w-3" />}
            {v === "a11y" && <Terminal className="h-3 w-3" />}
            {v === "preview" ? t("browser.view_preview") : v === "dom" ? t("browser.view_dom") : t("browser.view_a11y")}
          </button>
        ))}
      </div>

      {/* Content area */}
      <div className="flex-1 overflow-hidden">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        ) : view === "preview" ? (
          activeTab?.screenshot ? (
            <div className="relative w-full h-full bg-accent/10 overflow-auto flex items-center justify-center p-4">
              <img
                src={activeTab.screenshot}
                alt={activeTab.title}
                className="max-w-full max-h-full object-contain rounded-lg border border-border shadow-lg"
              />
            </div>
          ) : isConnected ? (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground bg-background">
              <Loader2 className="mb-4 h-8 w-8 animate-spin text-primary" />
              <p className="text-sm font-medium">Loading live browser snapshot...</p>
              <button
                onClick={refreshSnapshot}
                className="mt-4 inline-flex items-center gap-1.5 rounded-md bg-primary/10 text-primary border border-primary/20 px-3.5 py-1.5 text-xs font-semibold hover:bg-primary/20 active:scale-95 transition-all"
              >
                <RotateCw className="h-3.5 w-3.5" />
                Fetch Current View
              </button>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
              <Globe className="mb-4 h-16 w-16 text-muted-foreground/20" />
              <p className="text-lg font-medium">{t("browser.not_connected")}</p>
              <p className="mt-1 text-sm">{t("browser.connect_hint")}</p>
              <button
                onClick={handleConnect}
                disabled={connecting}
                className="mt-4 inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {connecting ? <Loader2 className="h-4 w-4 animate-spin" /> : <ExternalLink className="h-4 w-4" />}
                {t("browser.connect_btn")}
              </button>
            </div>
          )
        ) : !isConnected ? (
          <div className="flex flex-col items-center justify-center h-full p-6 text-muted-foreground">
            <Globe className="mb-4 h-16 w-16 text-muted-foreground/20" />
            <p className="text-lg font-medium">{t("browser.not_connected")}</p>
            <p className="mt-1 text-sm">{t("browser.connect_hint")}</p>
            <button
              onClick={handleConnect}
              disabled={connecting}
              className="mt-4 inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {connecting ? <Loader2 className="h-4 w-4 animate-spin" /> : <ExternalLink className="h-4 w-4" />}
              {t("browser.connect_btn")}
            </button>
          </div>
        ) : view === "dom" ? (
          <div className="h-full overflow-auto p-4 bg-muted/20 font-mono text-xs">
            {activeTab?.html ? (
              <pre className="whitespace-pre-wrap break-all text-muted-foreground bg-card p-3 rounded-lg border border-border max-h-[85vh] overflow-y-auto">
                {activeTab.html}
              </pre>
            ) : (
              <p className="text-muted-foreground text-center py-12">No DOM content loaded yet. Navigate to a URL to fetch DOM structure.</p>
            )}
          </div>
        ) : (
          <div className="h-full overflow-auto p-4 bg-muted/20 font-mono text-xs">
            {activeTab?.a11yTree ? (
              <pre className="whitespace-pre-wrap break-all text-muted-foreground bg-card p-3 rounded-lg border border-border max-h-[85vh] overflow-y-auto">
                {activeTab.a11yTree}
              </pre>
            ) : (
              <p className="text-muted-foreground text-center py-12">No accessibility tree loaded yet. Navigate to a URL to fetch a11y tree.</p>
            )}
          </div>
        )}
      </div>

      {/* Status bar */}
      <div className="flex items-center gap-3 border-t border-border bg-card px-4 py-1.5 text-[10px] text-muted-foreground">
        <span>{t("browser.status_bar")}</span>
        <span className="text-border">|</span>
        <span className={isConnected ? "text-green-500 font-semibold" : ""}>
          {isConnected ? t("browser.connected") : t("browser.disconnected")}
        </span>
        <span className="text-border">|</span>
        <span>{t("browser.mcp")}</span>
      </div>
    </div>
  );
}
