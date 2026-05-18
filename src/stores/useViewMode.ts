import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

export type ViewMode = "chat" | "code" | "dashboard" | "browser" | "models" | "connections" | "payments";

interface ViewModeState {
  mode: ViewMode;
  previousMode: ViewMode;
  setMode: (mode: ViewMode) => void;
}

const safeStorage = {
  getItem: (name: string): string | null => {
    try {
      return localStorage.getItem(name);
    } catch {
      return null;
    }
  },
  setItem: (name: string, value: string): void => {
    try {
      localStorage.setItem(name, value);
    } catch {
      // Storage full or unavailable
    }
  },
  removeItem: (name: string): void => {
    try {
      localStorage.removeItem(name);
    } catch {
      // Storage unavailable
    }
  },
};

export const useViewMode = create<ViewModeState>()(
  persist(
    (set) => ({
      mode: "dashboard",
      previousMode: "dashboard",
      setMode: (mode) =>
        set((s) => ({
          mode,
          previousMode: s.mode,
        })),
    }),
    { name: "prime-view-mode", storage: createJSONStorage(() => safeStorage) },
  ),
);
