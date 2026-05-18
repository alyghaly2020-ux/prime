import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useIdeStore } from "@/stores/useIdeStore";
import {
  File, Folder, FolderOpen, ChevronRight, ChevronDown,
  RefreshCw, Plus, RotateCcw,
} from "lucide-react";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: string;
}

function TreeNode({
  path,
  name,
  depth,
  onSelect,
  searchQuery,
}: {
  path: string;
  name: string;
  depth: number;
  onSelect: (path: string, name: string) => void;
  searchQuery: string;
}) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<FileEntry[] | null>(null);
  const [loading, setLoading] = useState(false);
  const activeFile = useIdeStore((s) => s.activeFile);

  const loadChildren = useCallback(async () => {
    setLoading(true);
    try {
      const entries: FileEntry[] = await invoke("list_dir", { path });
      setChildren(entries);
    } catch { setChildren([]); }
    setLoading(false);
  }, [path]);

  useEffect(() => {
    if (expanded && !children) { loadChildren(); }
  }, [expanded, children, loadChildren]);

  const toggle = () => {
    setExpanded(!expanded);
  };

  const handleClick = async () => {
    if (name.startsWith(".") && !searchQuery) return;
    try {
      const result: { content: string; language: string; modified: string } =
        await invoke("read_file", { path });
      useIdeStore.getState().openFile({
        path,
        name,
        content: result.content,
        language: result.language,
        modified: result.modified,
        dirty: false,
      });
    } catch { /* file open failed silently */ }
  };

  return (
    <div>
      <button
        onClick={name.startsWith(".") && !searchQuery ? undefined : toggle}
        onDoubleClick={handleClick}
        className={`flex w-full items-center gap-1 rounded-md px-1 py-0.5 text-xs transition-colors ${
          activeFile === path
            ? "bg-primary/10 text-primary font-medium"
            : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
        }`}
        style={{ paddingLeft: `${depth * 12 + 4}px` }}
      >
        {children && children.length > 0 ? (
          expanded ? <ChevronDown className="h-3 w-3 shrink-0" /> : <ChevronRight className="h-3 w-3 shrink-0" />
        ) : (
          <span className="w-3 shrink-0" />
        )}
        {expanded ? (
          <FolderOpen className="h-3.5 w-3.5 shrink-0 text-amber-500" />
        ) : (
          <Folder className="h-3.5 w-3.5 shrink-0 text-amber-500" />
        )}
        <span className="truncate">{name}</span>
      </button>
      {expanded && children && (
        <div>
          {children
            .filter((e) => searchQuery || !e.name.startsWith("."))
            .map((child) =>
              child.is_dir ? (
                <TreeNode
                  key={child.path}
                  path={child.path}
                  name={child.name}
                  depth={depth + 1}
                  onSelect={onSelect}
                  searchQuery={searchQuery}
                />
              ) : (
                <button
                  key={child.path}
                  onClick={() => onSelect(child.path, child.name)}
                  className={`flex w-full items-center gap-1 rounded-md px-1 py-0.5 text-xs transition-colors ${
                    activeFile === child.path
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-muted-foreground hover:bg-accent/50 hover:text-foreground"
                  }`}
                  style={{ paddingLeft: `${(depth + 1) * 12 + 4}px` }}
                >
                  <span className="w-3 shrink-0" />
                  <File className="h-3.5 w-3.5 shrink-0 text-blue-400" />
                  <span className="truncate">{child.name}</span>
                  {searchQuery && child.path.toLowerCase().includes(searchQuery.toLowerCase()) && (
                    <span className="ml-auto text-[9px] text-primary/70">{child.modified}</span>
                  )}
                </button>
              )
            )}
        </div>
      )}
      {expanded && loading && (
        <div
          className="flex items-center gap-1 py-0.5 text-xs text-muted-foreground"
          style={{ paddingLeft: `${(depth + 1) * 12 + 4}px` }}
        >
          <RefreshCw className="h-3 w-3 animate-spin" />
          Loading...
        </div>
      )}
    </div>
  );
}

export function FileExplorer() {
  const [root, setRoot] = useState<string>("");
  const searchQuery = useIdeStore((s) => s.searchQuery) || "";
  const [searchResults, setSearchResults] = useState<FileEntry[]>([]);
  const [searching, setSearching] = useState(false);
  const workspace = useIdeStore((s) => s.workspace);

  useEffect(() => {
    if (workspace?.path) setRoot(workspace.path);
  }, [workspace]);

  const handleFileSelect = async (path: string, name: string) => {
    try {
      const result: { content: string; language: string; modified: string } =
        await invoke("read_file", { path });
      useIdeStore.getState().openFile({
        path, name: name || path.split("/").pop() || name,
        content: result.content,
        language: result.language,
        modified: result.modified,
        dirty: false,
      });
    } catch { /* file open failed silently */ }
  };

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim() || !root) return;
    setSearching(true);
    try {
      const results: FileEntry[] = await invoke("search_files", {
        path: root,
        query: searchQuery,
      });
      setSearchResults(results);
    } catch { setSearchResults([]); }
    setSearching(false);
  }, [searchQuery, root]);

  useEffect(() => {
    if (searchQuery.length > 1) handleSearch();
    else setSearchResults([]);
  }, [searchQuery, handleSearch]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between border-b border-border px-2 py-1.5 bg-card/30">
        <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
          {workspace?.name || "Explorer"}
        </span>
        <div className="flex items-center gap-0.5">
          <button
            onClick={() => useIdeStore.getState().setWorkspaceDialogOpen(true)}
            className="rounded p-0.5 text-muted-foreground hover:text-foreground"
          >
            <Plus className="h-3 w-3" />
          </button>
          <button
            onClick={() => {
              if (root) invoke("list_dir", { path: root }).catch(() => {});
            }}
            className="rounded p-0.5 text-muted-foreground hover:text-foreground"
          >
            <RotateCcw className="h-3 w-3" />
          </button>
        </div>
      </div>

      {searchQuery.length > 0 ? (
        <div className="flex-1 overflow-y-auto p-1">
          {searching ? (
            <div className="flex items-center justify-center py-4">
              <RefreshCw className="h-4 w-4 animate-spin text-muted-foreground" />
            </div>
          ) : searchResults.length === 0 ? (
            <div className="py-4 text-center text-xs text-muted-foreground">No results</div>
          ) : (
            searchResults.map((r) => (
              <button
                key={r.path}
                onClick={() => handleFileSelect(r.path, r.name)}
                className="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
              >
                <File className="h-3 w-3 shrink-0" />
                <span className="truncate">{r.path.replace(root + "/", "")}</span>
              </button>
            ))
          )}
        </div>
      ) : root ? (
        <div className="flex-1 overflow-y-auto p-1">
          <TreeNode
            path={root}
            name={root.split("/").pop() || root}
            depth={0}
            onSelect={handleFileSelect}
            searchQuery=""
          />
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center flex-1 gap-2 p-4 text-center">
          <FolderOpen className="h-8 w-8 text-muted-foreground/40" />
          <p className="text-xs text-muted-foreground">
            Open a project to explore files
          </p>
          <button
            onClick={() => useIdeStore.getState().setWorkspaceDialogOpen(true)}
            className="rounded-md bg-primary px-3 py-1 text-xs text-primary-foreground hover:bg-primary/90"
          >
            Open Folder
          </button>
        </div>
      )}
    </div>
  );
}
