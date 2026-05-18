import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { PluginInfo } from "@/types";

interface PluginState {
  plugins: PluginInfo[];
  loading: boolean;
  error: string | null;
  fetchPlugins: () => Promise<void>;
  enablePlugin: (id: string) => Promise<void>;
  disablePlugin: (id: string) => Promise<void>;
  installPlugin: (path: string) => Promise<void>;
  uninstallPlugin: (id: string) => Promise<void>;
}

export const usePluginStore = create<PluginState>((set) => ({
  plugins: [],
  loading: false,
  error: null,

  fetchPlugins: async () => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<string>("list_plugins");
      const plugins = JSON.parse(result) as PluginInfo[];
      set({ plugins, loading: false });
    } catch (e) {
      const msg = `plugins: ${e}`;
      set({ error: msg, loading: false });
    }
  },

  enablePlugin: async (id) => {
    try {
      await invoke("plugin_enable", { id });
      set((s) => ({
        plugins: s.plugins.map((p) => (p.id === id ? { ...p, enabled: true } : p)),
      }));
    } catch (e) {
      const msg = `enable: ${e}`;
      set({ error: msg });
    }
  },

  disablePlugin: async (id) => {
    try {
      await invoke("plugin_disable", { id });
      set((s) => ({
        plugins: s.plugins.map((p) => (p.id === id ? { ...p, enabled: false } : p)),
      }));
    } catch (e) {
      const msg = `disable: ${e}`;
      set({ error: msg });
    }
  },

  installPlugin: async (path) => {
    try {
      await invoke("plugin_install", { path });
      await set((s) => ({ plugins: [...s.plugins] }));
    } catch (e) {
      const msg = `install: ${e}`;
      set({ error: msg });
    }
  },

  uninstallPlugin: async (id) => {
    try {
      await invoke("plugin_uninstall", { id });
      set((s) => ({ plugins: s.plugins.filter((p) => p.id !== id) }));
    } catch (e) {
      const msg = `uninstall: ${e}`;
      set({ error: msg });
    }
  },
}));
