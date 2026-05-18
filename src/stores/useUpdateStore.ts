import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "installing" | "uptodate" | "error";

export interface UpdateInfo {
  version: string;
  date: string;
  body: string;
}

interface UpdateStoreState {
  status: UpdateStatus;
  info: UpdateInfo | null;
  error: string | null;
  lastCheck: number | null;
  dismissedVersion: string | null;

  check: () => Promise<void>;
  install: () => Promise<void>;
  dismiss: () => void;
}

export const useUpdateStore = create<UpdateStoreState>()((set, get) => ({
  status: "idle",
  info: null,
  error: null,
  lastCheck: null,
  dismissedVersion: null,

  check: async () => {
    if (get().status === "checking") return;
    set({ status: "checking", error: null });
    try {
      const result = await invoke<{ shouldUpdate: boolean; manifest: UpdateInfo | null }>("update_check");
      if (result.shouldUpdate && result.manifest) {
        const dv = get().dismissedVersion;
        if (dv === result.manifest.version) {
          set({ status: "idle", info: null, lastCheck: Date.now() });
          return;
        }
        set({ status: "available", info: result.manifest, lastCheck: Date.now() });
      } else {
        set({ status: "uptodate", info: null, lastCheck: Date.now() });
      }
    } catch (e) {
      set({ status: "error", error: String(e), lastCheck: Date.now() });
    }
  },

  install: async () => {
    try {
      set({ status: "downloading" });
      await invoke("update_install");
      set({ status: "installing" });
      await invoke("update_relaunch");
    } catch (e) {
      set({ status: "error", error: String(e) });
    }
  },

  dismiss: () => {
    const info = get().info;
    set({ status: "idle", info: null, dismissedVersion: info?.version ?? null });
  },
}));
