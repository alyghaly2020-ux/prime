import { create } from "zustand";

export interface OpenFile {
  path: string;
  name: string;
  content: string;
  language: string;
  modified: string;
  dirty: boolean;
}

export interface Workspace {
  id: string;
  name: string;
  path: string;
  last_opened: string;
}

interface IdeState {
  workspace: Workspace | null;
  workspaces: Workspace[];
  openFiles: OpenFile[];
  activeFile: string | null;
  searchQuery: string;
  showHidden: boolean;
  sidebarWidth: number;
  terminalOpen: boolean;
  showBrowser: boolean;
  importDialogOpen: boolean;
  workspaceDialogOpen: boolean;
  cursorPosition: { lineNumber: number; column: number };

  setWorkspace: (ws: Workspace | null) => void;
  setShowBrowser: (show: boolean) => void;
  setWorkspaces: (wss: Workspace[]) => void;
  openFile: (file: OpenFile) => void;
  closeFile: (path: string) => void;
  setActiveFile: (path: string | null) => void;
  updateFileContent: (path: string, content: string) => void;
  setSearchQuery: (q: string) => void;
  setShowHidden: (s: boolean) => void;
  setSidebarWidth: (w: number) => void;
  setTerminalOpen: (o: boolean) => void;
  setImportDialogOpen: (o: boolean) => void;
  setWorkspaceDialogOpen: (o: boolean) => void;
  setCursorPosition: (pos: { lineNumber: number; column: number }) => void;
}

export const useIdeStore = create<IdeState>((set) => ({
  workspace: null,
  workspaces: [],
  openFiles: [],
  activeFile: null,
  searchQuery: "",
  showHidden: false,
  sidebarWidth: 208,
  terminalOpen: false,
  showBrowser: false,
  importDialogOpen: false,
  workspaceDialogOpen: false,
  cursorPosition: { lineNumber: 1, column: 1 },

  setWorkspace: (ws) => set({ workspace: ws }),
  setWorkspaces: (wss) => set({ workspaces: wss }),
  openFile: (file) =>
    set((state) => {
      const exists = state.openFiles.find((f) => f.path === file.path);
      if (exists) {
        return { activeFile: file.path };
      }
      return {
        openFiles: [...state.openFiles, file],
        activeFile: file.path,
      };
    }),
  closeFile: (path) =>
    set((state) => {
      const files = state.openFiles.filter((f) => f.path !== path);
      let active = state.activeFile;
      if (active === path) {
        const idx = state.openFiles.findIndex((f) => f.path === path);
        active = files[Math.min(idx, files.length - 1)]?.path ?? null;
      }
      return { openFiles: files, activeFile: active };
    }),
  setActiveFile: (path) => set({ activeFile: path }),
  updateFileContent: (path, content) =>
    set((state) => ({
      openFiles: state.openFiles.map((f) =>
        f.path === path ? { ...f, content, dirty: true } : f
      ),
    })),
  setSearchQuery: (q) => set({ searchQuery: q }),
  setShowHidden: (s) => set({ showHidden: s }),
  setSidebarWidth: (w) => set({ sidebarWidth: w }),
  setTerminalOpen: (o) => set({ terminalOpen: o }),
  setShowBrowser: (show) => set({ showBrowser: show }),
  setImportDialogOpen: (o) => set({ importDialogOpen: o }),
  setWorkspaceDialogOpen: (o) => set({ workspaceDialogOpen: o }),
  setCursorPosition: (pos) => set({ cursorPosition: pos }),
}));
