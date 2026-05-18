import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useIdeStore } from "@/stores/useIdeStore";
import {
  Folder, FolderOpen, Plus, Trash2, ExternalLink, X,
} from "lucide-react";

export function WorkspaceManager() {
  const workspaces = useIdeStore((s) => s.workspaces);
  const setWorkspaces = useIdeStore((s) => s.setWorkspaces);
  const setWorkspace = useIdeStore((s) => s.setWorkspace);
  const workspaceDialogOpen = useIdeStore((s) => s.workspaceDialogOpen);
  const setWorkspaceDialogOpen = useIdeStore((s) => s.setWorkspaceDialogOpen);
  const [newName, setNewName] = useState("");
  const [newPath, setNewPath] = useState("");
  const [loading, setLoading] = useState(false);

  const loadWorkspaces = useCallback(async () => {
    try {
      const wss = await invoke("list_workspaces");
      setWorkspaces(wss as any[]);
      
      // Auto-open first workspace if none is currently active
      const currentWs = useIdeStore.getState().workspace;
      if (!currentWs && Array.isArray(wss) && wss.length > 0) {
        const first = wss[0];
        await invoke("open_workspace", { id: first.id });
        setWorkspace(first);
      }
    } catch { /* load failed silently */ }
  }, [setWorkspaces, setWorkspace]);

  useEffect(() => {
    loadWorkspaces();
  }, [loadWorkspaces]);

  const addWorkspace = async () => {
    if (!newPath.trim()) return;
    setLoading(true);
    try {
      await invoke("add_workspace", { name: newName || newPath.split("/").pop(), path: newPath });
      setNewName("");
      setNewPath("");
      await loadWorkspaces();
      setWorkspaceDialogOpen(false);
    } catch { /* add workspace failed silently */ }
    setLoading(false);
  };

  const removeWorkspace = async (id: string) => {
    try {
      await invoke("remove_workspace", { id });
      await loadWorkspaces();
    } catch { /* remove failed silently */ }
  };

  const openWorkspace = async (ws: any) => {
    try {
      await invoke("open_workspace", { id: ws.id });
      setWorkspace(ws as any);
      setWorkspaceDialogOpen(false);
    } catch { /* open failed silently */ }
  };

  return (
    <>
      <div className="flex items-center justify-between border-b border-border px-3 py-2 bg-card/30">
        <div className="flex items-center gap-2">
          <FolderOpen className="h-3.5 w-3.5 text-primary" />
          <h3 className="text-xs font-semibold text-foreground uppercase tracking-wider">
            Projects
          </h3>
        </div>
        <button
          onClick={() => setWorkspaceDialogOpen(true)}
          className="rounded p-1 text-muted-foreground hover:text-foreground"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {workspaces.length === 0 && (
          <div className="py-6 text-center text-xs text-muted-foreground">
            No projects yet
          </div>
        )}
        {workspaces.map((ws) => (
          <div
            key={ws.id}
            className="group flex items-center gap-2 rounded-md px-2 py-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground cursor-pointer"
            onClick={() => openWorkspace(ws)}
          >
            <Folder className="h-3.5 w-3.5 shrink-0 text-primary/70" />
            <div className="flex-1 min-w-0">
              <p className="truncate font-medium">{ws.name}</p>
              <p className="truncate text-[10px] text-muted-foreground/60">{ws.path}</p>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                removeWorkspace(ws.id);
              }}
              className="rounded p-0.5 opacity-0 group-hover:opacity-100 hover:bg-destructive/20 hover:text-destructive"
            >
              <Trash2 className="h-3 w-3" />
            </button>
          </div>
        ))}
      </div>

      {workspaceDialogOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
          <div className="w-96 rounded-lg border border-border bg-card p-6 shadow-lg">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-sm font-semibold">Open Project</h2>
              <button
                onClick={() => setWorkspaceDialogOpen(false)}
                className="rounded p-1 text-muted-foreground hover:text-foreground"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {workspaces.length > 0 && (
              <div className="mb-4">
                <p className="text-xs text-muted-foreground mb-2">Recent projects</p>
                <div className="space-y-1 max-h-40 overflow-y-auto">
                  {workspaces.map((ws) => (
                    <button
                      key={ws.id}
                      onClick={() => openWorkspace(ws)}
                      className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-left hover:bg-accent"
                    >
                      <Folder className="h-3.5 w-3.5 shrink-0" />
                      <span className="truncate flex-1">{ws.path}</span>
                      <ExternalLink className="h-3 w-3 shrink-0 text-muted-foreground" />
                    </button>
                  ))}
                </div>
                <div className="my-3 border-t border-border" />
              </div>
            )}

            <div className="space-y-3">
              <div>
                <label className="text-xs text-muted-foreground mb-1 block">
                  Display Name (optional)
                </label>
                <input
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="My Project"
                  className="w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-xs"
                />
              </div>
              <div>
                <label className="text-xs text-muted-foreground mb-1 block">
                  Project Path
                </label>
                <div className="flex items-center gap-2">
                  <input
                    value={newPath}
                    onChange={(e) => setNewPath(e.target.value)}
                    placeholder="/home/user/projects/my-app"
                    className="flex-1 rounded-md border border-input bg-background px-2.5 py-1.5 text-xs"
                  />
                  <button
                    onClick={async () => {
                      try {
                        const { open } = await import("@tauri-apps/plugin-dialog");
                        const selected = await open({
                          directory: true,
                          multiple: false,
                          title: "Select Project Folder",
                        });
                        if (selected) setNewPath(selected as string);
                      } catch { /* dialog failed silently */ }
                    }}
                    className="rounded-md border border-input bg-background px-2 py-1.5 text-xs hover:bg-accent"
                  >
                    Browse
                  </button>
                </div>
              </div>
              <button
                onClick={addWorkspace}
                disabled={loading || !newPath.trim()}
                className="w-full rounded-md bg-primary px-3 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                {loading ? "Opening..." : "Open Project"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
