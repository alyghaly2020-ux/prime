import { useState, useRef, useEffect, useCallback } from 'react';
import { ChevronLeft, ChevronRight, RefreshCw, Bot, ShieldAlert, Plus, X, Loader2 } from 'lucide-react';
import { invoke } from "@tauri-apps/api/core";

interface Tab {
  id: string;
  url: string;
  title: string;
}

export function EmbeddedBrowser() {
  const [tabs, setTabs] = useState<Tab[]>([{ id: '1', url: 'https://duckduckgo.com', title: 'Search' }]);
  const [activeTabId, setActiveTabId] = useState('1');
  const [inputUrl, setInputUrl] = useState('https://duckduckgo.com');
  const [tabHtmls, setTabHtmls] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [aiControlled, setAiControlled] = useState(false);
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const activeTab = tabs.find(t => t.id === activeTabId);

  const loadUrl = useCallback(async (tabId: string, url: string) => {
    setLoading(true);
    try {
      let finalUrl = url;
      if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
        finalUrl = 'https://' + finalUrl;
      }
      
      const rawHtml = await invoke<string>("fetch_web_page", { url: finalUrl });
      
      // Inject base URL to resolve relative images and styling perfectly!
      const parser = new DOMParser();
      const doc = parser.parseFromString(rawHtml, 'text/html');
      let baseEl = doc.querySelector('base');
      if (!baseEl) {
        baseEl = doc.createElement('base');
        doc.head.insertBefore(baseEl, doc.head.firstChild);
      }
      baseEl.setAttribute('href', finalUrl);
      
      // Open links inside the iframe itself instead of breaking out
      const baseTarget = doc.querySelector('base[target]');
      if (!baseTarget) {
        baseEl.setAttribute('target', '_self');
      }
      
      const absoluteHtml = doc.documentElement.outerHTML;
      setTabHtmls(prev => ({ ...prev, [tabId]: absoluteHtml }));
    } catch (e) {
      console.error("Failed to fetch web page:", e);
      setTabHtmls(prev => ({ 
        ...prev, 
        [tabId]: `<html><body style="font-family: sans-serif; padding: 2rem; color: #ef4444; background: #fee2e2; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 80vh; text-align: center;">
          <h3 style="margin-bottom: 8px;">Web Connection Failure</h3>
          <p style="font-size: 13px; color: #7f1d1d; max-width: 400px; margin-bottom: 12px;">Could not load page. Please verify your internet connection or check the destination URL.</p>
          <p style="font-size: 10px; color: #b91c1c; font-family: monospace; background: rgba(0,0,0,0.05); padding: 4px 8px; border-radius: 4px;">Error details: ${String(e)}</p>
        </body></html>` 
      }));
    } finally {
      setLoading(false);
    }
  }, []);

  // Fetch initial tab url
  useEffect(() => {
    if (activeTab && !tabHtmls[activeTabId]) {
      loadUrl(activeTabId, activeTab.url);
    }
  }, [activeTabId, activeTab, tabHtmls, loadUrl]);

  const navigate = (e: React.FormEvent) => {
    e.preventDefault();
    let finalUrl = inputUrl;
    if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://')) {
      finalUrl = 'https://' + finalUrl;
    }
    setTabs(tabs.map(t => t.id === activeTabId ? { ...t, url: finalUrl, title: finalUrl } : t));
    loadUrl(activeTabId, finalUrl);
  };

  const addTab = () => {
    const id = Date.now().toString();
    const defaultUrl = 'https://duckduckgo.com';
    setTabs([...tabs, { id, url: defaultUrl, title: 'New Tab' }]);
    setActiveTabId(id);
    setInputUrl(defaultUrl);
    loadUrl(id, defaultUrl);
  };

  const closeTab = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (tabs.length === 1) return;
    const newTabs = tabs.filter(t => t.id !== id);
    setTabs(newTabs);
    
    // Clean html cache
    setTabHtmls(prev => {
      const copy = { ...prev };
      delete copy[id];
      return copy;
    });

    if (activeTabId === id) {
      const remainingTab = newTabs[newTabs.length - 1];
      setActiveTabId(remainingTab.id);
      setInputUrl(remainingTab.url);
    }
  };

  return (
    <div className={`flex flex-col h-full w-full bg-background transition-all duration-300 ${aiControlled ? 'border-4 border-green-500/50 shadow-[inset_0_0_20px_rgba(34,197,94,0.2)]' : ''}`}>
      {/* Tabs */}
      <div className="flex items-center bg-card/50 border-b border-border overflow-x-auto no-scrollbar px-2 pt-2">
        {tabs.map(tab => (
          <div
            key={tab.id}
            onClick={() => { setActiveTabId(tab.id); setInputUrl(tab.url); }}
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

      {/* Toolbar */}
      <div className="flex items-center gap-2 p-2 bg-background border-b border-border">
        <button className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent rounded-md">
          <ChevronLeft className="h-4 w-4" />
        </button>
        <button className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent rounded-md">
          <ChevronRight className="h-4 w-4" />
        </button>
        <button 
          onClick={() => activeTab && loadUrl(activeTabId, activeTab.url)} 
          disabled={loading}
          className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-accent rounded-md disabled:opacity-50"
        >
          {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
        </button>
        
        <form onSubmit={navigate} className="flex-1 flex">
          <input
            value={inputUrl}
            onChange={(e) => setInputUrl(e.target.value)}
            disabled={loading}
            className="flex-1 bg-accent/30 border border-border rounded-md px-3 py-1 text-xs focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-75"
            placeholder="Search or enter web address"
          />
        </form>

        <button 
          onClick={() => setAiControlled(!aiControlled)}
          className={`flex items-center gap-1.5 px-2.5 py-1 text-xs rounded-md font-medium transition-colors ${aiControlled ? 'bg-green-500/20 text-green-500 border border-green-500/30' : 'bg-accent/50 text-muted-foreground hover:text-foreground'}`}
          title="Toggle AI Control Mode (DOM/OCR Access)"
        >
          <Bot className="h-3.5 w-3.5" />
          AI Auto
        </button>
        <button className="p-1.5 text-amber-500/70 hover:text-amber-500 hover:bg-accent rounded-md" title="Stealth Mode Active">
          <ShieldAlert className="h-4 w-4" />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 relative bg-white">
        {loading && !tabHtmls[activeTabId] ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center bg-background/80 gap-2">
            <Loader2 className="h-8 w-8 animate-spin text-primary" />
            <p className="text-xs text-muted-foreground font-medium">Bypassing Web Sandbox Security...</p>
          </div>
        ) : (
          activeTab && (
            <iframe
              ref={iframeRef}
              srcDoc={tabHtmls[activeTabId] || `<html><body style="font-family: sans-serif; display: flex; align-items: center; justify-content: center; height: 80vh; color: #888;">Connecting securely...</body></html>`}
              className="absolute inset-0 w-full h-full border-0"
              sandbox="allow-same-origin allow-scripts allow-popups allow-forms"
            />
          )
        )}
      </div>
    </div>
  );
}
