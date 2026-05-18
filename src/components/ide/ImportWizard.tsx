import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useIdeStore } from "@/stores/useIdeStore";
import { X, Download, ExternalLink, Check, Loader2 } from "lucide-react";

interface IdeHistory {
  source: string;
  projects: { name: string; path: string; last_opened: string }[];
}

export function ImportWizard() {
  const importDialogOpen = useIdeStore((s) => s.importDialogOpen);
  const setImportDialogOpen = useIdeStore((s) => s.setImportDialogOpen);
  const [history, setHistory] = useState<IdeHistory[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [imported, setImported] = useState<Set<string>>(new Set());

  const scan = async () => {
    setLoading(true);
    try {
      const result: IdeHistory[] = await invoke("import_ide_history");
      setHistory(result);
    } catch (e) {
      console.error("Import scan failed:", e);
    }
    setLoading(false);
  };

  const importProject = async (name: string, path: string) => {
    try {
      await invoke("add_workspace", { name, path });
      setImported((prev) => new Set(prev).add(path));
    } catch { /* workspace import failed silently */ }
  };

  return (
    <>
      {importDialogOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-[500px] max-h-[600px] rounded-lg border border-border bg-card p-6 shadow-lg flex flex-col">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-sm font-semibold">Import from IDE</h2>
              <button
                onClick={() => setImportDialogOpen(false)}
                className="rounded p-1 text-muted-foreground hover:text-foreground"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {!history ? (
              <div className="flex flex-col items-center justify-center py-12 gap-3">
                <Download className="h-10 w-10 text-muted-foreground/40" />
                <p className="text-xs text-muted-foreground text-center max-w-xs">
                  Import your projects from VS Code, JetBrains, or any IDE on this machine.
                  One click to bring all your history.
                </p>
                <button
                  onClick={scan}
                  disabled={loading}
                  className="rounded-md bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50 flex items-center gap-2"
                >
                  {loading && <Loader2 className="h-3 w-3 animate-spin" />}
                  {loading ? "Scanning..." : "Scan My IDEs"}
                </button>
              </div>
            ) : history.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 gap-2">
                <p className="text-sm text-muted-foreground">No IDE projects found</p>
                <button
                  onClick={scan}
                  className="text-xs text-primary hover:underline"
                >
                  Try again
                </button>
              </div>
            ) : (
              <div className="flex-1 overflow-y-auto space-y-4">
                {history.map((ide) => (
                  <div key={ide.source}>
                    <h3 className="text-xs font-semibold text-foreground mb-2 flex items-center gap-1.5">
                      <ExternalLink className="h-3 w-3" />
                      {ide.source}
                      <span className="text-muted-foreground font-normal">
                        ({ide.projects.length} projects)
                      </span>
                    </h3>
                    <div className="space-y-1">
                      {ide.projects.map((proj) => (
                        <div
                          key={proj.path}
                          className="flex items-center gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-accent group"
                        >
                          <div className="flex-1 min-w-0">
                            <p className="truncate font-medium">{proj.name || proj.path.split("/").pop()}</p>
                            <p className="truncate text-[10px] text-muted-foreground/60">{proj.path}</p>
                          </div>
                          {imported.has(proj.path) ? (
                            <span className="text-green-500 flex items-center gap-1">
                              <Check className="h-3 w-3" />
                              Imported
                            </span>
                          ) : (
                            <button
                              onClick={() => importProject(proj.name || proj.path.split("/").pop() || "project", proj.path)}
                              className="rounded-md bg-primary/10 px-2 py-1 text-primary hover:bg-primary/20"
                            >
                              Import
                            </button>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      <button
        onClick={() => {
          setImportDialogOpen(true);
          if (!history) scan();
        }}
        className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
      >
        <Download className="h-3.5 w-3.5" />
        Import from IDE
      </button>
    </>
  );
}
