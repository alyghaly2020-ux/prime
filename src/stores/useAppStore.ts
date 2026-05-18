import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";
import type { Theme } from "@/types";

export type LauncherStage = "chat" | "tools" | "mcp" | "editor" | "skills";
export type StageStatus = "pending" | "downloading" | "ready";

interface AppState {
  theme: Theme;
  sidebarCollapsed: boolean;
  sidebarWidth: number;
  activePanel: string;
  onboardingCompleted: boolean;
  onboardingStep: number;
  launcherCompleted: boolean;
  launcherStages: Record<LauncherStage, StageStatus>;
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setSidebarWidth: (width: number) => void;
  setActivePanel: (panel: string) => void;
  setOnboardingCompleted: (completed: boolean) => void;
  setOnboardingStep: (step: number) => void;
  setLauncherCompleted: (completed: boolean) => void;
  setLauncherStage: (stage: LauncherStage, status: StageStatus) => void;
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

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      theme: "system",
      sidebarCollapsed: false,
      sidebarWidth: 260,
      activePanel: "dashboard",
      onboardingCompleted: true,
      onboardingStep: 0,
      launcherCompleted: false,
      launcherStages: { chat: "pending", tools: "pending", mcp: "pending", editor: "pending", skills: "pending" },
      setTheme: (theme) => set({ theme }),
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
      setSidebarWidth: (width) => set({ sidebarWidth: width }),
      setActivePanel: (panel) => set({ activePanel: panel }),
      setOnboardingCompleted: (completed) => set({ onboardingCompleted: completed }),
      setOnboardingStep: (step) => set({ onboardingStep: step }),
      setLauncherCompleted: (completed) => set({ launcherCompleted: completed }),
      setLauncherStage: (stage, status) =>
        set((s) => ({ launcherStages: { ...s.launcherStages, [stage]: status } })),
    }),
    { name: "prime-app-store", storage: createJSONStorage(() => safeStorage) },
  ),
);
