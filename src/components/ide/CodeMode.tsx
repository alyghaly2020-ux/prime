import { useMemo, useState, lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { useIdeStore } from "@/stores/useIdeStore";
import { invoke } from "@tauri-apps/api/core";
import { FileExplorer } from "@/components/ide/FileExplorer";
import { EditorTabs } from "@/components/ide/EditorTabs";
import { WorkspaceManager } from "@/components/ide/WorkspaceManager";
import { ImportWizard } from "@/components/ide/ImportWizard";
import { Terminal } from "@/components/ide/Terminal";
import { EmbeddedBrowser } from "@/components/ide/EmbeddedBrowser";
import {
  Loader2, Sparkles, PanelRightClose, PanelRightOpen,
  Search, Terminal as TerminalIcon, GitBranch, FolderOpen,
  ChevronLeft, ChevronRight, Globe,
} from "lucide-react";

const MonacoEditor = lazy(() =>
  import("@/components/MonacoEditor").then((m) => ({ default: m.MonacoEditor }))
);

export function CodeMode() {
  const { t } = useTranslation();
  const activeFile = useIdeStore((s) => s.activeFile);
  const openFiles = useIdeStore((s) => s.openFiles);
  const workspace = useIdeStore((s) => s.workspace);
  const terminalOpen = useIdeStore((s) => s.terminalOpen);
  const showBrowser = useIdeStore((s) => s.showBrowser);
  const cursorPosition = useIdeStore((s) => s.cursorPosition);
  const setTerminalOpen = useIdeStore((s) => s.setTerminalOpen);
  const setWorkspaceDialogOpen = useIdeStore((s) => s.setWorkspaceDialogOpen);
  const [aiSidebarOpen, setAiSidebarOpen] = useState(true);
  const [aiInput, setAiInput] = useState("");
  const [chatMessages, setChatMessages] = useState<{role: string, content: string}[]>([]);
  const [isThinking, setIsThinking] = useState(false);
  const [sidebarView, setSidebarView] = useState<"explorer" | "search" | "projects">("explorer");
  const [sidebarVisible, setSidebarVisible] = useState(true);

  const currentFile = useMemo(
    () => openFiles.find((f) => f.path === activeFile),
    [openFiles, activeFile]
  );

  const handleAiChat = async (prompt: string, includeFileContext: boolean = false) => {
    if (!prompt.trim()) return;
    
    let fullPrompt = prompt;
    if (includeFileContext && currentFile) {
      fullPrompt = `Context from ${currentFile.name}:\n` +
        `\`\`\`${currentFile.language}\n${currentFile.content}\n\`\`\`\n\n` + 
        prompt;
    }

    const newUserMsg = { role: "user", content: prompt };
    const messagesForApi = [...chatMessages, { role: "user", content: fullPrompt }];
    
    setChatMessages((prev) => [...prev, newUserMsg]);
    setIsThinking(true);
    setAiInput("");

    try {
      const result: any = await invoke("ai_chat", { 
        messages: messagesForApi,
        model: "default" 
      });
      setChatMessages((prev) => [...prev, { role: "assistant", content: result.content || String(result) }]);
    } catch (err) {
      console.error("AI Chat error:", err);
      setChatMessages((prev) => [...prev, { role: "assistant", content: `Error: ${err}` }]);
    } finally {
      setIsThinking(false);
    }
  };

  return (
    <div className="flex h-full overflow-hidden">
      {sidebarVisible && (
        <aside
          className="border-r border-border bg-card/30 flex flex-col"
          style={{ width: useIdeStore.getState().sidebarWidth }}
        >
          <div className="flex border-b border-border">
            {[
              { id: "explorer" as const, icon: FolderOpen },
              { id: "search" as const, icon: Search },
              { id: "projects" as const, icon: GitBranch },
            ].map((tab) => (
              <button
                key={tab.id}
                onClick={() => setSidebarView(tab.id)}
                className={`flex-1 py-2 text-xs transition-colors ${
                  sidebarView === tab.id
                    ? "text-primary border-b-2 border-primary bg-accent/20"
                    : "text-muted-foreground hover:text-foreground hover:bg-accent/10"
                }`}
              >
                <tab.icon className="h-4 w-4 mx-auto" />
              </button>
            ))}
          </div>

          <div className="flex-1 overflow-hidden">
            {sidebarView === "explorer" && <FileExplorer />}
            {sidebarView === "search" && (
              <div className="p-2">
                <input
                  placeholder="Search files..."
                  className="w-full rounded-md border border-input bg-background px-2 py-1 text-xs"
                  onChange={(e) => useIdeStore.getState().setSearchQuery(e.target.value)}
                />
              </div>
            )}
            {sidebarView === "projects" && <WorkspaceManager />}
          </div>

          {sidebarView === "projects" && (
            <div className="border-t border-border p-2">
              <ImportWizard />
            </div>
          )}
        </aside>
      )}

      <button
        onClick={() => setSidebarVisible(!sidebarVisible)}
        className="absolute left-0 top-1/2 z-10 -translate-y-1/2 rounded-r-md border border-border bg-card p-0.5 text-muted-foreground hover:text-foreground"
        style={{ left: sidebarVisible ? useIdeStore.getState().sidebarWidth : 0 }}
      >
        {sidebarVisible ? <ChevronLeft className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
      </button>

      <div className="flex flex-1 flex-col min-w-0">
        {openFiles.length > 0 && <EditorTabs />}

          <div className="flex-1 overflow-hidden">
            {showBrowser ? (
              <EmbeddedBrowser />
            ) : currentFile ? (
              <Suspense fallback={<div className="flex h-full items-center justify-center text-muted-foreground"><Loader2 className="h-5 w-5 animate-spin" /></div>}>
                <MonacoEditor
                  key={currentFile.path}
                  value={currentFile.content}
                  language={currentFile.language}
                  path={currentFile.path}
                  onChange={(value) => {
                    if (value !== undefined) {
                      useIdeStore.getState().updateFileContent(currentFile.path, value);
                    }
                  }}
                />
              </Suspense>
            ) : (
              <div className="flex h-full items-center justify-center">
                <div className="text-center max-w-md">
                  <Sparkles className="h-12 w-12 mx-auto mb-4 text-primary/30" />
                  <h2 className="text-lg font-semibold text-foreground mb-2">
                    {workspace ? "Select a file" : "Welcome to Prime IDE"}
                  </h2>
                  <p className="text-sm text-muted-foreground mb-6">
                    {workspace
                      ? "Choose a file from the explorer to start editing"
                      : "Open a project folder to explore files, edit code, and run commands"}
                  </p>
                  {!workspace && (
                    <button
                      onClick={() => {
                        setSidebarVisible(true);
                        setSidebarView("projects");
                        setWorkspaceDialogOpen(true);
                      }}
                      className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
                    >
                      Open Folder
                    </button>
                  )}
                </div>
              </div>
            )}
          </div>

          {!showBrowser && terminalOpen && <Terminal />}

        <div className="flex items-center justify-between border-t border-border bg-card/30 px-3 py-1">
          <div className="flex items-center gap-3">
            <button
              onClick={() => setTerminalOpen(!terminalOpen)}
              className={`flex items-center gap-1 text-xs ${
                terminalOpen ? "text-primary" : "text-muted-foreground"
              } hover:text-foreground`}
            >
              <TerminalIcon className="h-3 w-3" />
              Terminal
            </button>
            <button
              onClick={() => useIdeStore.getState().setShowBrowser(!showBrowser)}
              className={`flex items-center gap-1 text-xs ${
                showBrowser ? "text-primary" : "text-muted-foreground"
              } hover:text-foreground`}
            >
              <Globe className="h-3 w-3" />
              Browser
            </button>
            {workspace && (
              <span className="text-xs text-muted-foreground">{workspace.path}</span>
            )}
          </div>
          <div className="flex items-center gap-3 text-xs text-muted-foreground">
            {currentFile && (
              <>
                <span>{currentFile.language}</span>
                <span>Ln {cursorPosition?.lineNumber || 1}, Col {cursorPosition?.column || 1}</span>
              </>
            )}
            <span>UTF-8</span>
            <button
              onClick={() => setAiSidebarOpen(!aiSidebarOpen)}
              className="text-muted-foreground hover:text-foreground"
            >
              {aiSidebarOpen ? <PanelRightClose className="h-3 w-3" /> : <PanelRightOpen className="h-3 w-3" />}
            </button>
          </div>
        </div>
      </div>

      {aiSidebarOpen && (
        <aside className="w-64 border-l border-border bg-card/30 flex flex-col">
          <div className="flex items-center gap-2 border-b border-border px-3 py-2">
            <Sparkles className="h-4 w-4 text-primary" />
            <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">
              {t("code.assistant")}
            </h3>
          </div>

          <div className="flex-1 overflow-y-auto p-3 space-y-3">
            {chatMessages.length === 0 ? (
              <>
                <div className="rounded-lg bg-gradient-to-br from-primary/5 to-primary/10 border border-primary/20 p-3">
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {currentFile
                      ? `Ask me about ${currentFile.name}`
                      : t("code.description")}
                  </p>
                </div>

                <div className="space-y-1">
                  <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider px-1">
                    {t("code.quick_actions")}
                  </p>
                  {[
                    { label: t("code.action.explain"), icon: Sparkles, prompt: "Explain the selected code or file." },
                    { label: t("code.action.bugs"), icon: Search, prompt: "Find bugs in this code." },
                    { label: t("code.action.tests"), icon: TerminalIcon, prompt: "Write tests for this code." },
                    { label: t("code.action.refactor"), icon: GitBranch, prompt: "Refactor this code to be cleaner." },
                  ].map((action) => (
                    <button
                      key={action.label}
                      onClick={() => handleAiChat(action.prompt, true)}
                      className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground transition-colors text-left"
                    >
                      <action.icon className="h-3.5 w-3.5 shrink-0" />
                      <span>{action.label}</span>
                    </button>
                  ))}
                </div>
              </>
            ) : (
              <div className="space-y-3">
                {chatMessages.map((msg, i) => (
                  <div key={i} className={`p-2 rounded-md text-xs ${msg.role === 'user' ? 'bg-primary/20 text-foreground ml-4' : 'bg-accent/30 text-foreground mr-4'}`}>
                    <span className="font-semibold block mb-1 text-[10px] opacity-70">
                      {msg.role === 'user' ? 'You' : 'Assistant'}
                    </span>
                    <div className="whitespace-pre-wrap">{msg.content}</div>
                  </div>
                ))}
                {isThinking && (
                  <div className="p-2 rounded-md bg-accent/30 text-foreground mr-4 text-xs flex items-center gap-2">
                    <Loader2 className="h-3 w-3 animate-spin" /> Thinking...
                  </div>
                )}
              </div>
            )}
          </div>

          <div className="border-t border-border p-3">
            <div className="flex gap-2">
              <input
                value={aiInput}
                onChange={(e) => setAiInput(e.target.value)}
                placeholder={t("code.placeholder")}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && aiInput.trim()) {
                    handleAiChat(aiInput, !!currentFile);
                  }
                }}
                className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
              <button
                onClick={() => handleAiChat(aiInput, !!currentFile)}
                className="rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90"
              >
                {t("code.ask")}
              </button>
            </div>
          </div>
        </aside>
      )}
    </div>
  );
}
