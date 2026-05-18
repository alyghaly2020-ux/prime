import { useState, useCallback, useRef, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { useChatStore } from "@/stores/useChatStore";
import type { ChatMessage } from "@/types";
import { 
  Globe, ArrowLeft, ArrowRight, RotateCw, Loader2, ExternalLink, 
  Send, AlertCircle, Sparkles, Home, Search, Heart,
  PanelRightClose, PanelRightOpen, FileText, Eye, EyeOff, Copy, Check
} from "lucide-react";

export function BrowserModeFull() {
  const { t } = useTranslation();
  const sessions = useChatStore((s) => s.sessions);
  const addMessage = useChatStore((s) => s.addMessage);
  const renameSession = useChatStore((s) => s.renameSession);
  const createSession = useChatStore((s) => s.createSession);
  const [urlInput, setUrlInput] = useState("");
  const [chatInput, setChatInput] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [navigating, setNavigating] = useState(false);
  const [screenshot, setScreenshot] = useState<string>("");
  const [loading, setLoading] = useState(false);
  const [chatError, setChatError] = useState<string | null>(null);
  const chatRef = useRef<HTMLInputElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Layout customization states
  const [showSidebar, setShowSidebar] = useState(true);
  const [cleanMode, setCleanMode] = useState(false);

  // Tab State: "direct" (Embedded Direct Iframe Browser) or "copilot" (AI Playwright Browser)
  const [browserMode, setBrowserMode] = useState<"direct" | "copilot">("direct");

  // Direct Browser Navigation History & States
  const [directHistory, setDirectHistory] = useState<string[]>(["about:home"]);
  const [directHistoryIndex, setDirectHistoryIndex] = useState(0);
  const [directUrlInput, setDirectUrlInput] = useState("");
  const [iframeReloadKey, setIframeReloadKey] = useState(0);

  const currentDirectUrl = directHistory[directHistoryIndex];

  // DOM OCR / Content Extraction states
  const [ocrModalOpen, setOcrModalOpen] = useState(false);
  const [ocrText, setOcrText] = useState("");
  const [ocrLoading, setOcrLoading] = useState(false);
  const [ocrError, setOcrError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  // Helper to handle URL formatting and search fallback
  const navigateDirect = useCallback((url: string) => {
    let targetUrl = url.trim();
    if (!targetUrl) return;

    // Check if it looks like a URL, otherwise default to search
    const isUrl = /^(https?:\/\/)?([\da-z.-]+)\.([a-z.]{2,6})([/\w .-]*)*\/?$/.test(targetUrl);
    if (!isUrl) {
      targetUrl = `https://www.bing.com/search?q=${encodeURIComponent(targetUrl)}`;
    } else {
      if (!/^https?:\/\//i.test(targetUrl)) {
        targetUrl = `https://${targetUrl}`;
      }
    }

    const newHistory = directHistory.slice(0, directHistoryIndex + 1);
    newHistory.push(targetUrl);
    setDirectHistory(newHistory);
    setDirectHistoryIndex(newHistory.length - 1);
    setDirectUrlInput(targetUrl);
  }, [directHistory, directHistoryIndex]);

  const goBackDirect = useCallback(() => {
    if (directHistoryIndex > 0) {
      setDirectHistoryIndex(directHistoryIndex - 1);
      const url = directHistory[directHistoryIndex - 1];
      setDirectUrlInput(url === "about:home" ? "" : url);
    }
  }, [directHistoryIndex, directHistory]);

  const goForwardDirect = useCallback(() => {
    if (directHistoryIndex < directHistory.length - 1) {
      setDirectHistoryIndex(directHistoryIndex + 1);
      const url = directHistory[directHistoryIndex + 1];
      setDirectUrlInput(url === "about:home" ? "" : url);
    }
  }, [directHistoryIndex, directHistory]);

  const goHomeDirect = useCallback(() => {
    const newHistory = directHistory.slice(0, directHistoryIndex + 1);
    newHistory.push("about:home");
    setDirectHistory(newHistory);
    setDirectHistoryIndex(newHistory.length - 1);
    setDirectUrlInput("");
  }, [directHistory, directHistoryIndex]);

  const reloadDirect = useCallback(() => {
    setIframeReloadKey(prev => prev + 1);
  }, []);

  // Create or reuse a browser session (never conflicts with ChatMode)
  const chatStoreLoading = useChatStore((s) => s.loading);
  const browserSessionIdRef = useRef<string | null>(null);
  const browserCheckedRef = useRef(false);
  
  const getBrowserSession = useCallback(() => {
    const existing = sessions.find((s) => s.id.startsWith("browser-"));
    if (existing) {
      browserSessionIdRef.current = existing.id;
      return existing;
    }
    return null;
  }, [sessions]);

  useEffect(() => {
    if (chatStoreLoading) return;
    if (browserCheckedRef.current) return;
    browserCheckedRef.current = true;

    if (!getBrowserSession()) {
      const id = createSession("Browser Chat", "auto", "auto", "browser-chat");
      renameSession(id, "Browser Chat");
      browserSessionIdRef.current = id;
    }
  }, [chatStoreLoading, getBrowserSession, createSession, renameSession]);

  const messages = (getBrowserSession()?.messages) || [];

  // Smooth scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, loading]);

  const { data: connected, isLoading, error, refetch } = useQuery({
    queryKey: ["browser_is_connected"],
    queryFn: () => invoke<boolean>("browser_is_connected"),
    refetchInterval: 5000,
  });

  const isConnected = connected === true;

  const refreshSnapshot = useCallback(async () => {
    if (!isConnected) return;
    try {
      const result = await invoke<any>("browser_snapshot");
      if (result.screenshot) {
        setScreenshot(result.screenshot);
      }
      if (result.url) {
        setUrlInput(result.url);
      }
    } catch (e) {
      console.error("Failed to refresh snapshot:", e);
    }
  }, [isConnected]);

  // Listen to live browser updates from backend agent actions
  useEffect(() => {
    const unlisten = listen("browser-updated", () => {
      console.info("Browser updated event received, refreshing snapshot...");
      refreshSnapshot();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refreshSnapshot]);

  // Listen to view mode changes to auto-refresh snapshot when user enters browser mode
  useEffect(() => {
    if (isConnected) {
      refreshSnapshot();
    }
  }, [isConnected, refreshSnapshot]);

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
    if (!urlInput || !isConnected) return;
    setNavigating(true);
    try {
      const result = await invoke<any>("browser_navigate", { url: urlInput });
      if (result.screenshot) {
        setScreenshot(result.screenshot);
      }
      if (result.url) {
        setUrlInput(result.url);
      }
    } catch (e) {
      console.error("Navigation failed:", e);
    } finally {
      setNavigating(false);
    }
  }, [urlInput, isConnected]);

  const handleChatSend = useCallback(async () => {
    if (!chatInput.trim() || loading) return;
    const sid = browserSessionIdRef.current;
    if (!sid) return;

    const userMsg: ChatMessage = { role: "user", content: chatInput, timestamp: Date.now() };
    addMessage(sid, userMsg);
    setChatInput("");
    setLoading(true);
    setChatError(null);

    try {
      const currentMessages = useChatStore.getState().sessions.find(s => s.id === sid)?.messages || [];
      const response = await invoke<string>("ai_chat", {
        messages: currentMessages,
        model: "auto",
      });
      const assistantMsg: ChatMessage = { role: "assistant", content: response, timestamp: Date.now() };
      addMessage(sid, assistantMsg);

      setTimeout(() => {
        refreshSnapshot();
      }, 1500);

    } catch (e: any) {
      if (typeof e === "string") {
        setChatError(e);
      } else if (e && typeof e === "object" && "message" in e && typeof e.message === "string") {
        setChatError(e.message);
      } else {
        setChatError("فشل إرسال الرسالة، يرجى المحاولة لاحقاً.");
      }
    } finally {
      setLoading(false);
      chatRef.current?.focus();
    }
  }, [chatInput, loading, refreshSnapshot, addMessage]);

  // DOM OCR Logic with backend synchronization
  const handleOcr = useCallback(async () => {
    setOcrLoading(true);
    setOcrModalOpen(true);
    setOcrError(null);
    setOcrText("");
    
    try {
      let activeUrl = currentDirectUrl;
      if (browserMode === "copilot") {
        activeUrl = urlInput;
      }

      if (!activeUrl || activeUrl === "about:home") {
        throw new Error("يرجى الانتقال إلى موقع ويب أولاً لاستخراج النصوص منه.");
      }

      // Ensure playwright is connected
      if (!isConnected) {
        setOcrText("جارٍ تشغيل محرك تصفح المساعد آلياً وتوصيله بالإنترنت...");
        await invoke("browser_connect");
        await refetch();
      }

      // Navigate the headless browser to the active URL to sync DOM
      setOcrText("جارٍ تحميل الصفحة ومزامنة البنية البرمجية للـ DOM...");
      await invoke("browser_navigate", { url: activeUrl });

      // Get the full text of the synced active page
      setOcrText("جارٍ استخراج وتصفية النصوص البرمجية ومحتوى الصفحة...");
      const text = await invoke<string>("browser_get_text");
      if (!text || text.trim() === "") {
        throw new Error("لم يتم العثور على نصوص قابلة للقراءة في الصفحة الحالية.");
      }
      setOcrText(text);
    } catch (e: any) {
      console.error("DOM OCR failed:", e);
      setOcrError(e?.message || String(e) || "فشل استخراج النصوص من الصفحة.");
    } finally {
      setOcrLoading(false);
    }
  }, [currentDirectUrl, browserMode, urlInput, isConnected, refetch]);

  const handleCopyOcr = useCallback(() => {
    navigator.clipboard.writeText(ocrText);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [ocrText]);

  const handleSendOcrToAi = useCallback(() => {
    const sid = browserSessionIdRef.current;
    if (!sid || !ocrText) return;

    const content = `لقد قمت باستخراج النصوص التالية من الصفحة باستخدام أداة DOM OCR المدمجة. يرجى قراءتها، وتلخيص أهم النقاط فيها، وتحليلها بشكل احترافي ومفصل:\n\n\`\`\`text\n${ocrText.slice(0, 10000)}\n\`\`\`\n${ocrText.length > 10000 ? "... (تم اقتطاع بقية النص لطوله الزائد)" : ""}`;
    const userMsg: ChatMessage = { role: "user", content, timestamp: Date.now() };
    
    addMessage(sid, userMsg);
    setOcrModalOpen(false);
    setBrowserMode("copilot"); // Switch to copilot view so they see the chat response streaming!
    setShowSidebar(true); // Ensure sidebar is visible
    setLoading(true);
    setChatError(null);

    invoke<string>("ai_chat", {
      messages: [...messages, userMsg],
      model: "auto",
    })
      .then((response) => {
        const assistantMsg: ChatMessage = { role: "assistant", content: response, timestamp: Date.now() };
        addMessage(sid, assistantMsg);
      })
      .catch((e: any) => {
        console.error("AI analysis of OCR failed:", e);
      })
      .finally(() => {
        setLoading(false);
      });
  }, [ocrText, messages, addMessage]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        chatRef.current?.blur();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const quickLinks = [
    { name: "Google", url: "https://www.google.com", color: "from-blue-500 to-indigo-500", desc: "محرك بحث جوجل العالمي" },
    { name: "Bing", url: "https://www.bing.com", color: "from-teal-500 to-cyan-500", desc: "محرك بحث مايكروسوفت بينج" },
    { name: "Wikipedia", url: "https://www.wikipedia.org", color: "from-slate-600 to-zinc-800", desc: "الموسوعة الحرة ويكيبيديا" },
    { name: "GitHub", url: "https://github.com", color: "from-purple-600 to-indigo-800", desc: "منصة المطورين ومستودعات البرمجة" },
    { name: "StackOverflow", url: "https://stackoverflow.com", color: "from-orange-500 to-amber-600", desc: "مجتمع الأسئلة والأجوبة البرمجية" },
    { name: "Hugging Face", url: "https://huggingface.co", color: "from-yellow-400 to-orange-500", desc: "نماذج الذكاء الاصطناعي المفتوحة" },
  ];

  return (
    <div className="flex h-full w-full bg-background overflow-hidden flex-col relative">
      
      {/* 📄 DOM OCR MODAL DIALOG */}
      {ocrModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-background/80 backdrop-blur-md animate-in fade-in duration-300">
          <div className="relative w-full max-w-3xl h-[80vh] flex flex-col bg-card/90 border border-border rounded-2xl shadow-2xl overflow-hidden animate-in zoom-in-95 duration-300">
            
            {/* Header */}
            <div className="flex items-center justify-between border-b border-border bg-card px-6 py-4">
              <div className="flex items-center gap-2">
                <FileText className="h-5 w-5 text-primary" />
                <h3 className="text-sm font-bold text-foreground">مستخرج النصوص الذكي (DOM OCR)</h3>
              </div>
              <button 
                onClick={() => setOcrModalOpen(false)}
                className="text-xs text-muted-foreground hover:text-foreground bg-accent/40 hover:bg-accent/60 px-3 py-1.5 rounded-lg transition-all"
              >
                إغلاق ×
              </button>
            </div>

            {/* Body */}
            <div className="flex-1 p-6 overflow-y-auto space-y-4">
              {ocrLoading ? (
                <div className="flex flex-col items-center justify-center h-full text-center space-y-3">
                  <Loader2 className="h-10 w-10 animate-spin text-primary" />
                  <p className="text-xs font-semibold text-foreground/80">{ocrText || "جارٍ استخراج النصوص وتحليل DOM..."}</p>
                </div>
              ) : ocrError ? (
                <div className="flex flex-col items-center justify-center h-full text-center max-w-md mx-auto space-y-3">
                  <AlertCircle className="h-12 w-12 text-destructive animate-pulse" />
                  <p className="text-xs font-bold text-foreground">حدث خطأ أثناء استخراج النصوص</p>
                  <p className="text-[11px] text-muted-foreground leading-relaxed">{ocrError}</p>
                  <button 
                    onClick={handleOcr}
                    className="bg-primary/10 text-primary border border-primary/20 hover:bg-primary/20 px-4 py-2 rounded-xl text-xs font-semibold"
                  >
                    إعادة المحاولة 🔄
                  </button>
                </div>
              ) : (
                <div className="h-full flex flex-col space-y-3">
                  <div className="flex items-center justify-between text-xs bg-accent/20 px-3 py-2 rounded-lg">
                    <span className="text-muted-foreground">تم استخراج النصوص بنجاح من الصفحة النشطة.</span>
                    <div className="flex items-center gap-2">
                      <button 
                        onClick={handleCopyOcr}
                        className="flex items-center gap-1 hover:text-primary transition-colors font-semibold text-xs"
                      >
                        {copied ? <span className="text-green-500 flex items-center gap-0.5"><Check className="h-3.5 w-3.5" /> تم النسخ!</span> : <span className="flex items-center gap-0.5"><Copy className="h-3.5 w-3.5" /> نسخ النص</span>}
                      </button>
                    </div>
                  </div>
                  <textarea
                    readOnly
                    value={ocrText}
                    className="flex-1 w-full bg-accent/10 border border-border/60 rounded-xl p-4 text-xs font-mono leading-relaxed resize-none focus:outline-none focus:ring-1 focus:ring-primary shadow-inner"
                  />
                </div>
              )}
            </div>

            {/* Footer */}
            {!ocrLoading && !ocrError && (
              <div className="border-t border-border bg-card px-6 py-4 flex items-center justify-between shrink-0">
                <button
                  onClick={handleSendOcrToAi}
                  className="bg-primary text-primary-foreground hover:bg-primary/95 px-4 py-2 rounded-xl text-xs font-bold flex items-center gap-1.5 active:scale-95 transition-all shadow-lg shadow-primary/10"
                >
                  <Sparkles className="h-3.5 w-3.5" />
                  إرسال إلى مساعد الذكاء الاصطناعي للتحليل 🤖
                </button>
                <div className="text-[10px] text-muted-foreground">
                  الحجم: {ocrText.length} حرف
                </div>
              </div>
            )}

          </div>
        </div>
      )}

      {/* Top Universal Mode Switcher Header (Hidden in Clean Mode if Clean Mode is active) */}
      {!cleanMode && (
        <div className="flex items-center justify-between border-b border-border bg-card/65 px-4 py-2 shrink-0 backdrop-blur-md animate-in fade-in duration-300">
          <div className="flex items-center gap-1.5 bg-accent/20 p-1 rounded-xl">
            <button
              onClick={() => setBrowserMode("direct")}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-300 ${
                browserMode === "direct"
                  ? "bg-primary text-primary-foreground shadow-md shadow-primary/10"
                  : "text-muted-foreground hover:bg-accent/40 hover:text-foreground"
              }`}
            >
              <Globe className="h-3.5 w-3.5" />
              <span>متصفح تفاعلي مباشر</span>
            </button>
            <button
              onClick={() => setBrowserMode("copilot")}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all duration-300 ${
                browserMode === "copilot"
                  ? "bg-primary text-primary-foreground shadow-md shadow-primary/10"
                  : "text-muted-foreground hover:bg-accent/40 hover:text-foreground"
              }`}
            >
              <Sparkles className="h-3.5 w-3.5 animate-pulse text-yellow-400" />
              <span>مساعد الذكاء الاصطناعي (Copilot)</span>
            </button>
          </div>
          
          <div className="text-[10px] text-muted-foreground bg-accent/40 px-3 py-1 rounded-full font-mono flex items-center gap-1.5">
            <span className={`h-1.5 w-1.5 rounded-full ${isConnected ? "bg-green-500" : "bg-amber-500 animate-pulse"}`} />
            {isConnected ? "Playwright Engine: Active" : "Playwright Engine: Headless Offline"}
          </div>
        </div>
      )}

      <div className="flex-1 flex h-full w-full overflow-hidden">
        {/* Left side: Browser viewport depending on active mode */}
        <div className="flex-1 flex flex-col border-r border-border overflow-hidden">
          
          {browserMode === "direct" ? (
            /* ================================================================= */
            /* DIRECT EMBEDDED IFRAME BROWSER */
            /* ================================================================= */
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* Navigation Header */}
              <div className="flex items-center gap-2 border-b border-border bg-card/85 px-3 py-1.5 shrink-0">
                <div className="flex items-center gap-1">
                  <button 
                    onClick={goBackDirect}
                    disabled={directHistoryIndex === 0}
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent disabled:opacity-30 transition-colors animate-in fade-in"
                    title="للخلف"
                  >
                    <ArrowLeft className="h-4 w-4" />
                  </button>
                  <button 
                    onClick={goForwardDirect}
                    disabled={directHistoryIndex === directHistory.length - 1}
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent disabled:opacity-30 transition-colors animate-in fade-in"
                    title="للأمام"
                  >
                    <ArrowRight className="h-4 w-4" />
                  </button>
                  <button 
                    onClick={reloadDirect} 
                    disabled={currentDirectUrl === "about:home"}
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent disabled:opacity-30 transition-colors animate-in fade-in"
                    title="إعادة تحميل"
                  >
                    <RotateCw className="h-4 w-4" />
                  </button>
                  <button 
                    onClick={goHomeDirect} 
                    className="rounded p-1.5 text-muted-foreground hover:bg-accent transition-colors"
                    title="الرئيسية"
                  >
                    <Home className="h-4 w-4" />
                  </button>
                </div>
                
                <div className="relative flex-1">
                  <Globe className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground/50" />
                  <input
                    type="text"
                    value={directUrlInput}
                    onChange={(e) => setDirectUrlInput(e.target.value)}
                    onKeyDown={(e) => { if (e.key === "Enter") navigateDirect(directUrlInput); }}
                    placeholder="أدخل عنوان موقع (URL) أو اكتب للبحث..."
                    className="w-full rounded-lg border border-input bg-background/50 py-1.5 pl-8 pr-3 text-sm text-foreground placeholder:text-muted-foreground/50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus:bg-background transition-all"
                  />
                  {directUrlInput && (
                    <button 
                      onClick={() => navigateDirect(directUrlInput)}
                      className="absolute right-2 top-1/2 -translate-y-1/2 text-xs font-semibold text-primary hover:text-primary/80 px-2 py-0.5 rounded bg-accent/40"
                    >
                      اذهب
                    </button>
                  )}
                </div>

                {/* Direct Action controls: DOM OCR, Clean Mode, Sidebar Toggle */}
                <div className="flex items-center gap-1.5">
                  
                  {currentDirectUrl !== "about:home" && (
                    <button
                      onClick={handleOcr}
                      className="flex items-center gap-1 rounded-lg border border-primary/30 bg-primary/10 text-primary px-2.5 py-1.5 text-xs font-bold hover:bg-primary/20 transition-all active:scale-95 animate-in slide-in-from-right-4 duration-300 shrink-0"
                      title="استخراج النصوص الذكي (DOM OCR)"
                    >
                      <FileText className="h-3.5 w-3.5 animate-pulse" />
                      <span className="hidden sm:inline">DOM OCR</span>
                    </button>
                  )}

                  <button
                    onClick={() => setCleanMode(prev => !prev)}
                    className={`rounded-lg border px-2.5 py-1.5 text-xs font-semibold transition-all active:scale-95 shrink-0 ${
                      cleanMode 
                        ? "border-amber-500/30 bg-amber-500/10 text-amber-500 shadow-md shadow-amber-500/5 animate-pulse" 
                        : "border-border bg-background/50 hover:bg-accent text-muted-foreground"
                    }`}
                    title={cleanMode ? "عرض الأدوات والتحذيرات السفلية" : "تفعيل الوضع النظيف (المبسط)"}
                  >
                    {cleanMode ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                  </button>

                  <button
                    onClick={() => setShowSidebar(prev => !prev)}
                    className="rounded-lg border border-border bg-background/50 hover:bg-accent p-1.5 text-muted-foreground transition-all active:scale-95 shrink-0"
                    title={showSidebar ? "إخفاء مساعد الذكاء الاصطناعي" : "إظهار مساعد الذكاء الاصطناعي"}
                  >
                    {showSidebar ? <PanelRightClose className="h-4 w-4" /> : <PanelRightOpen className="h-4 w-4" />}
                  </button>

                  {currentDirectUrl !== "about:home" && (
                    <a 
                      href={currentDirectUrl}
                      target="_blank" 
                      rel="noopener noreferrer"
                      className="inline-flex items-center gap-1 rounded-lg border border-border bg-background/50 p-1.5 text-xs text-muted-foreground hover:bg-accent shrink-0 transition-colors"
                      title="فتح نافذة خارجية مستقلة"
                    >
                      <ExternalLink className="h-4 w-4" />
                    </a>
                  )}

                </div>
              </div>

              {/* Viewport content */}
              <div className="flex-1 relative bg-accent/5 dark:bg-card/20 flex flex-col items-center justify-center overflow-hidden">
                {currentDirectUrl === "about:home" ? (
                  /* GORGEOUS START PAGE */
                  <div className="w-full max-w-2xl px-6 py-12 flex flex-col items-center justify-center text-center space-y-8 overflow-y-auto max-h-full scrollbar-none animate-in fade-in zoom-in-95 duration-500">
                    <div className="space-y-3">
                      <div className="inline-flex p-4 rounded-3xl bg-gradient-to-tr from-primary/10 to-primary/30 border border-primary/20 shadow-xl shadow-primary/5 animate-bounce">
                        <Globe className="h-12 w-12 text-primary" />
                      </div>
                      <h2 className="text-3xl font-extrabold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-foreground to-foreground/75">Prime Web Browser</h2>
                      <p className="text-sm text-muted-foreground max-w-md mx-auto">تصفح الويب بسرعة وكفاءة من داخل التطبيق مباشرة بمرونة تامة وبدون قيود.</p>
                    </div>

                    {/* Start Page Search */}
                    <div className="w-full relative max-w-lg shadow-2xl rounded-2xl overflow-hidden bg-card/40 border border-border backdrop-blur-xl p-1.5 focus-within:ring-2 focus-within:ring-primary transition-all">
                      <div className="flex items-center gap-2">
                        <Search className="h-5 w-5 text-muted-foreground/60 ml-3 shrink-0" />
                        <input
                          type="text"
                          placeholder="ابحث في الويب أو أدخل عنواناً..."
                          onKeyDown={(e) => {
                            if (e.key === "Enter") navigateDirect((e.target as HTMLInputElement).value);
                          }}
                          className="w-full bg-transparent py-2.5 px-1 text-sm text-foreground focus:outline-none placeholder:text-muted-foreground/45"
                        />
                      </div>
                    </div>

                    {/* Quick Link Grid */}
                    <div className="w-full space-y-4">
                      <div className="flex items-center justify-between px-1">
                        <span className="text-xs font-bold text-muted-foreground tracking-wider uppercase">روابط سريعة تفاعلية</span>
                        <span className="h-px flex-1 bg-border/40 mx-4" />
                      </div>
                      <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                        {quickLinks.map((link) => (
                          <button
                            key={link.name}
                            onClick={() => navigateDirect(link.url)}
                            className="group flex flex-col items-start p-3.5 rounded-xl border border-border/60 bg-card/30 hover:bg-card hover:border-primary/40 hover:shadow-xl transition-all duration-300 active:scale-95 text-right relative overflow-hidden"
                          >
                            <div className={`absolute top-0 right-0 w-1.5 h-full bg-gradient-to-b ${link.color}`} />
                            <span className="text-xs font-bold text-foreground group-hover:text-primary transition-colors">{link.name}</span>
                            <span className="text-[10px] text-muted-foreground/80 mt-1 line-clamp-1">{link.desc}</span>
                          </button>
                        ))}
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5 text-[10px] text-muted-foreground/75 font-medium pt-8">
                      <span>صُنع بحب لأجلك</span>
                      <Heart className="h-3 w-3 text-destructive fill-destructive" />
                      <span>ضمن حزمة برايم الذكية</span>
                    </div>
                  </div>
                ) : (
                  /* IFRAME BROWSER VIEWPORT */
                  <div className="w-full h-full relative flex flex-col">
                    <iframe
                      key={iframeReloadKey}
                      src={currentDirectUrl}
                      title="Prime Embedded Browser"
                      className="w-full h-full border-none bg-background rounded-lg shadow-inner"
                      sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
                    />
                    
                    {/* Security Frame warning and action (Hidden in Clean Mode) */}
                    {!cleanMode && (
                      <div className="absolute bottom-4 left-4 right-4 bg-card/90 backdrop-blur border border-border rounded-xl p-3 shadow-2xl flex items-center justify-between text-xs animate-in fade-in slide-in-from-bottom-5">
                        <div className="flex items-center gap-2">
                          <AlertCircle className="h-4 w-4 text-primary shrink-0 animate-pulse" />
                          <span className="text-muted-foreground leading-normal text-right">
                            إذا لم يتم تحميل الموقع، فقد يرجع ذلك إلى سياسة أمان الموقع الخارجي. يمكنك فتحه في نافذة خارجية مباشرة.
                          </span>
                        </div>
                        <a 
                          href={currentDirectUrl} 
                          target="_blank" 
                          rel="noopener noreferrer"
                          className="bg-primary text-primary-foreground hover:bg-primary/95 px-3 py-1.5 rounded-lg flex items-center gap-1 font-semibold active:scale-95 transition-all shrink-0 ml-4"
                        >
                          <ExternalLink className="h-3.5 w-3.5" />
                          فتح خارجي
                        </a>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          ) : (
            /* ================================================================= */
            /* ORIGINAL AI COPILOT HEADLESS BROWSER */
            /* ================================================================= */
            <div className="flex-1 flex flex-col overflow-hidden">
              {/* URL bar */}
              <div className="flex items-center gap-2 border-b border-border bg-card/80 px-3 py-1.5 shrink-0">
                <div className="flex items-center gap-1">
                  <button className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled><ArrowLeft className="h-3.5 w-3.5" /></button>
                  <button className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled><ArrowRight className="h-3.5 w-3.5" /></button>
                  <button 
                    onClick={refreshSnapshot} 
                    disabled={!isConnected || navigating}
                    className="rounded p-1 text-muted-foreground hover:bg-accent disabled:opacity-30"
                  >
                    {navigating ? <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" /> : <RotateCw className="h-3.5 w-3.5" />}
                  </button>
                </div>
                <div className="relative flex-1">
                  <Globe className="absolute left-2.5 top-1/2 h-3 w-3 -translate-y-1/2 text-muted-foreground/50" />
                  <input
                    type="text"
                    value={urlInput}
                    onChange={(e) => setUrlInput(e.target.value)}
                    onKeyDown={(e) => { if (e.key === "Enter") handleNavigate(); }}
                    placeholder={isConnected ? t("browser.placeholder_connected") : t("browser.placeholder_disconnected")}
                    disabled={!isConnected || navigating}
                    className="w-full rounded-md border border-input bg-background py-1.5 pl-8 pr-3 text-sm text-foreground placeholder:text-muted-foreground/50 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  />
                </div>

                <div className="flex items-center gap-1.5">
                  {urlInput && urlInput !== "about:home" && (
                    <button
                      onClick={handleOcr}
                      className="flex items-center gap-1 rounded-lg border border-primary/30 bg-primary/10 text-primary px-2.5 py-1.5 text-xs font-bold hover:bg-primary/20 transition-all active:scale-95 shrink-0"
                      title="استخراج النصوص الذكي (DOM OCR)"
                    >
                      <FileText className="h-3.5 w-3.5 animate-pulse" />
                      <span className="hidden sm:inline">DOM OCR</span>
                    </button>
                  )}

                  <button
                    onClick={() => setShowSidebar(prev => !prev)}
                    className="rounded-lg border border-border bg-background hover:bg-accent p-1.5 text-muted-foreground transition-all active:scale-95 shrink-0"
                    title={showSidebar ? "إخفاء مساعد الذكاء الاصطناعي" : "إظهار مساعد الذكاء الاصطناعي"}
                  >
                    {showSidebar ? <PanelRightClose className="h-4 w-4" /> : <PanelRightOpen className="h-4 w-4" />}
                  </button>

                  <button
                    onClick={isConnected ? undefined : handleConnect}
                    disabled={isLoading || connecting}
                    className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-xs text-foreground hover:bg-accent shrink-0"
                  >
                    {connecting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <ExternalLink className="h-3.5 w-3.5" />}
                    {isConnected ? t("browser.connected") : t("browser.connect")}
                  </button>
                </div>
              </div>

              {/* Error */}
              {error && (
                <div role="alert" className="flex items-start gap-2 border-b border-border bg-destructive/10 px-4 py-2 text-xs text-destructive shrink-0">
                  <AlertCircle className="mt-0.5 h-3 w-3 shrink-0" />
                  <p>{t("browser.error", { error: String(error) })}</p>
                </div>
              )}

              {/* Viewport */}
              <div className="flex-1 overflow-auto bg-accent/5 dark:bg-card/30 flex items-center justify-center p-4">
                {isLoading ? (
                  <div className="flex items-center justify-center h-full">
                    <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                  </div>
                ) : !isConnected ? (
                  <div className="flex flex-col items-center justify-center h-full text-muted-foreground text-center">
                    <Globe className="mb-4 h-16 w-16 text-muted-foreground/20" />
                    <p className="text-lg font-medium">{t("browser.not_connected")}</p>
                    <p className="mt-1 text-sm">{t("browser.connect_hint")}</p>
                    <button
                      onClick={handleConnect}
                      disabled={connecting}
                      className="mt-4 inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/95 disabled:opacity-50 transition-all active:scale-95"
                    >
                      {connecting ? <Loader2 className="h-4 w-4 animate-spin" /> : <ExternalLink className="h-4 w-4" />}
                      {t("browser.connect_btn")}
                    </button>
                  </div>
                ) : screenshot ? (
                  <div className="relative max-w-full max-h-full overflow-auto flex items-center justify-center">
                    <img
                      src={screenshot}
                      alt="Browser Viewport"
                      className="max-w-full max-h-full object-contain rounded-lg border border-border shadow-xl bg-background"
                    />
                    {navigating && (
                      <div className="absolute inset-0 bg-background/40 backdrop-blur-[1px] flex items-center justify-center rounded-lg">
                        <Loader2 className="h-8 w-8 animate-spin text-primary" />
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center h-full text-muted-foreground text-center">
                    <Loader2 className="mb-4 h-8 w-8 animate-spin text-primary" />
                    <p className="text-sm font-medium">جارٍ جلب لقطة شاشة للمتصفح النشط...</p>
                    <button
                      onClick={refreshSnapshot}
                      className="mt-4 inline-flex items-center gap-1.5 rounded-md bg-primary/10 text-primary border border-primary/20 px-3.5 py-1.5 text-xs font-semibold hover:bg-primary/20 active:scale-95 transition-all"
                    >
                      <RotateCw className="h-3.5 w-3.5" />
                      تحديث اللقطة
                    </button>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Right side: Chat Sidebar (Prime Copilot) */}
        {showSidebar && (
          <div className="w-[340px] shrink-0 flex flex-col bg-card/50 border-l border-border overflow-hidden animate-in slide-in-from-left duration-300">
            {/* Sidebar Header */}
            <div className="flex items-center justify-between border-b border-border bg-card px-4 py-3 shrink-0">
              <div className="flex items-center gap-2">
                <div className="flex h-6 w-6 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <Sparkles className="h-3.5 w-3.5" />
                </div>
                <span className="text-xs font-semibold tracking-tight text-foreground">Prime Copilot</span>
              </div>
              <div className="flex items-center gap-1.5">
                <span className="inline-flex h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                <span className="text-[10px] text-muted-foreground font-medium">نشط</span>
              </div>
            </div>

            {/* Sidebar Messages */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4">
              {messages.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-center text-muted-foreground/60 p-4">
                  <Globe className="h-8 w-8 mb-2 text-muted-foreground/20 animate-pulse" />
                  <p className="text-xs font-semibold text-foreground/80">مرحباً بك في متصفح برايم المدمج!</p>
                  <p className="text-[11px] mt-1 leading-relaxed text-right">
                    يمكنك الدردشة معي للتحكم بالمتصفح بشكل آلي. جرب كتابة "افتح يوتيوب" أو "ابحث في جوجل عن..." وسأقوم بتوجيه المتصفح وحفظ اللقطات لك فوراً!
                  </p>
                </div>
              ) : (
                messages.map((msg, i) => (
                  <div
                    key={i}
                    className={`flex flex-col max-w-[85%] ${msg.role === "user" ? "ml-auto items-end" : "mr-auto items-start"}`}
                  >
                    <div
                      className={`rounded-2xl px-3 py-2 text-xs leading-relaxed shadow-sm ${
                        msg.role === "user"
                          ? "bg-primary text-primary-foreground rounded-tr-none"
                          : "bg-muted text-foreground rounded-tl-none"
                      }`}
                    >
                      <p className="whitespace-pre-wrap">{msg.content}</p>
                    </div>
                  </div>
                ))
              )}
              {loading && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground/75 px-1">
                  <Loader2 className="h-3 w-3 animate-spin text-primary" />
                  <span>برايم يفكر...</span>
                </div>
              )}
              {chatError && (
                <div className="flex items-start gap-1.5 bg-destructive/10 border border-destructive/20 text-destructive rounded-lg p-2.5 text-[11px]">
                  <AlertCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
                  <p className="flex-1 leading-relaxed">{chatError}</p>
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>

            {/* Sidebar Input */}
            <div className="border-t border-border bg-card p-3 shrink-0">
              <div className="flex items-center gap-2">
                <input
                  ref={chatRef}
                  type="text"
                  value={chatInput}
                  onChange={(e) => setChatInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleChatSend();
                    }
                  }}
                  placeholder="اكتب رسالة للتحكم بالمتصفح..."
                  disabled={loading}
                  className="flex-1 rounded-lg border border-input bg-background px-3 py-2 text-xs text-foreground placeholder:text-muted-foreground/45 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50"
                />
                <button
                  onClick={handleChatSend}
                  disabled={!chatInput.trim() || loading}
                  className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground hover:bg-primary/95 disabled:opacity-50 transition-all active:scale-95 shrink-0"
                >
                  <Send className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
