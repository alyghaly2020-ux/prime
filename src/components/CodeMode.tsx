import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MonacoEditor } from "@/components/MonacoEditor";
import {
  FileCode,
  FolderOpen,
  Search,
  Sparkles,
  GitBranch,
  AlertCircle,
  Loader2,
  File,
  Terminal,
  Plus,
  PanelRightClose,
  PanelRightOpen,
} from "lucide-react";

const FILES = {
  src: {
    label: "src",
    files: ["App.tsx", "main.tsx", "index.css"],
    dirs: {
      components: ["ChatMode.tsx", "CodeMode.tsx", "DashboardMode.tsx", "MonacoEditor.tsx", "LauncherScreen.tsx"],
      stores: ["useAppStore.ts", "useViewMode.ts", "useModelStore.ts", "useMcpStore.ts", "useToolsStore.ts"],
      hooks: ["useTheme.ts"],
      types: ["index.ts"],
      lib: ["utils.ts"],
    },
  },
};

function FileExplorer() {
  const [expanded, setExpanded] = useState<Set<string>>(new Set(["src", "src/components", "src/stores"]));
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  const toggle = (path: string) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const FileItem = ({ name, path }: { name: string; path: string }) => (
    <button
      onClick={() => setSelectedFile(path)}
      className={`flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs transition-colors ${
        selectedFile === path
          ? "bg-primary/10 text-primary font-medium"
          : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
      }`}
    >
      <File className="h-3.5 w-3.5 shrink-0" />
      <span className="truncate">{name}</span>
    </button>
  );

  return (
    <div className="space-y-0.5">
      {Object.entries(FILES).map(([dir, contents]) => (
        <div key={dir}>
          <button
            onClick={() => toggle(dir)}
            className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs hover:bg-accent/50"
          >
            <FolderOpen className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <span className="font-medium text-foreground">{dir}</span>
          </button>
          {expanded.has(dir) && (
            <div className="ml-3 space-y-0.5 border-l border-border pl-2">
              {contents.files?.map((f) => (
                <FileItem key={`${dir}/${f}`} name={f} path={`${dir}/${f}`} />
              ))}
              {contents.dirs && Object.entries(contents.dirs).map(([sub, files]) => (
                <div key={`${dir}/${sub}`}>
                  <button
                    onClick={() => toggle(`${dir}/${sub}`)}
                    className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs hover:bg-accent/50"
                  >
                    <FolderOpen className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                    <span className="text-foreground">{sub}</span>
                  </button>
                  {expanded.has(`${dir}/${sub}`) && (
                    <div className="ml-3 space-y-0.5 border-l border-border pl-2">
                      {files.map((f) => (
                        <FileItem key={`${dir}/${sub}/${f}`} name={f} path={`${dir}/${sub}/${f}`} />
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export function CodeMode() {
  const { t } = useTranslation();
  const [aiInput, setAiInput] = useState("");
  const [aiSidebarOpen, setAiSidebarOpen] = useState(true);

  return (
    <div className="flex h-full">
      {/* File explorer */}
      <aside className="w-52 border-r border-border bg-card/30 flex flex-col">
        <div className="flex items-center justify-between border-b border-border px-3 py-2 bg-card/30">
          <div className="flex items-center gap-2">
            <FolderOpen className="h-3.5 w-3.5 text-primary" />
            <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("code.explorer")}</h3>
          </div>
          <div className="flex items-center gap-1">
            <button className="rounded p-1 text-muted-foreground hover:text-foreground">
              <Plus className="h-3.5 w-3.5" />
            </button>
            <button className="rounded p-1 text-muted-foreground hover:text-foreground">
              <Search className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
        <div className="flex-1 overflow-y-auto p-2">
          <FileExplorer />
        </div>
        <div className="border-t border-border p-2">
          <div className="flex items-center gap-2 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent/50 cursor-pointer">
            <GitBranch className="h-3.5 w-3.5" />
            <span>{t("code.branch")}</span>
          </div>
        </div>
      </aside>

      {/* Editor area */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Tab bar */}
        <div className="flex items-center justify-between border-b border-border bg-card/30">
          <div className="flex items-center">
            <div className="flex items-center gap-1.5 border-r border-border px-3 py-1.5 text-xs text-foreground bg-background/80">
              <FileCode className="h-3.5 w-3.5 text-primary" />
              <span>{t("code.tab.app")}</span>
            </div>
            <div className="flex items-center gap-1.5 px-3 py-1.5 text-xs text-muted-foreground">
              <FileCode className="h-3.5 w-3.5" />
              <span>{t("code.tab.main")}</span>
            </div>
          </div>
          <div className="flex items-center gap-1 pr-2">
            <button className="rounded p-1 text-muted-foreground hover:text-foreground">
              <Terminal className="h-3.5 w-3.5" />
            </button>
            <button
              onClick={() => setAiSidebarOpen(!aiSidebarOpen)}
              className="rounded p-1 text-muted-foreground hover:text-foreground"
            >
              {aiSidebarOpen ? <PanelRightClose className="h-3.5 w-3.5" /> : <PanelRightOpen className="h-3.5 w-3.5" />}
            </button>
          </div>
        </div>

        {/* Monaco Editor */}
        <div className="flex-1 overflow-hidden">
          <MonacoEditor />
        </div>
      </div>

      {/* AI sidebar */}
      {aiSidebarOpen && (
        <aside className="w-64 border-l border-border bg-card/30 flex flex-col">
          <div className="flex items-center gap-2 border-b border-border px-3 py-2">
            <Sparkles className="h-4 w-4 text-primary" />
            <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">{t("code.assistant")}</h3>
          </div>

          <div className="flex-1 overflow-y-auto p-3 space-y-3">
            <div className="rounded-lg bg-gradient-to-br from-primary/5 to-primary/10 border border-primary/20 p-3">
              <p className="text-xs text-muted-foreground leading-relaxed">
                {t("code.description")}
              </p>
            </div>

            <div className="space-y-1">
              <p className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider px-1">{t("code.quick_actions")}</p>
              {[
                { label: t("code.action.explain"), icon: Sparkles, desc: t("code.action.explain_desc") },
                { label: t("code.action.bugs"), icon: AlertCircle, desc: t("code.action.bugs_desc") },
                { label: t("code.action.tests"), icon: FileCode, desc: t("code.action.tests_desc") },
                { label: t("code.action.refactor"), icon: Loader2, desc: t("code.action.refactor_desc") },
              ].map((action) => (
                <button
                  key={action.label}
                  className="flex w-full items-start gap-2 rounded-md px-2 py-2 text-xs text-muted-foreground hover:bg-accent hover:text-foreground transition-colors text-left"
                >
                  <action.icon className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                  <div>
                    <p className="font-medium">{action.label}</p>
                    <p className="text-[10px] text-muted-foreground/60">{action.desc}</p>
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* AI input */}
          <div className="border-t border-border p-3">
            <div className="flex gap-2">
              <input
                value={aiInput}
                onChange={(e) => setAiInput(e.target.value)}
                placeholder={t("code.placeholder")}
                className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-xs ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              />
              <button className="rounded-md bg-primary px-2.5 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90">
                {t("code.ask")}
              </button>
            </div>
          </div>
        </aside>
      )}
    </div>
  );
}
