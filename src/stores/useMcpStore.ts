import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { McpServerInfo, McpServerConfig } from "@/types";

interface McpState {
  servers: McpServerInfo[];
  configs: McpServerConfig[];
  loading: boolean;
  error: string | null;
  fetchServers: () => Promise<void>;
  startServer: (id: string) => Promise<void>;
  stopServer: (id: string) => Promise<void>;
  restartServer: (id: string) => Promise<void>;
  toggleServer: (id: string, enabled: boolean) => Promise<void>;
  addConfig: (config: McpServerConfig) => Promise<void>;
  removeConfig: (id: string) => Promise<void>;
}

export const useMcpStore = create<McpState>((set) => ({
  servers: [],
  configs: [],
  loading: false,
  error: null,

  fetchServers: async () => {
    set({ loading: true, error: null });
    try {
      const servers = await invoke<McpServerInfo[]>("list_mcp_servers");
      set({ servers, loading: false });
    } catch (e) {
      const msg = String(e);
      set({ error: msg, loading: false });
    }
  },

  startServer: async (id) => {
    try {
      await invoke("mcp_start_server", { id });
      await set((s) => ({
        servers: s.servers.map((sv) => (sv.id === id ? { ...sv, running: true } : sv)),
      }));
    } catch (e) {
      const msg = `start: ${e}`;
      set({ error: msg });
    }
  },

  stopServer: async (id) => {
    try {
      await invoke("mcp_stop_server", { id });
      set((s) => ({
        servers: s.servers.map((sv) => (sv.id === id ? { ...sv, running: false } : sv)),
      }));
    } catch (e) {
      const msg = `stop: ${e}`;
      set({ error: msg });
    }
  },

  restartServer: async (id) => {
    try {
      await invoke("mcp_restart_server", { id });
    } catch (e) {
      const msg = `restart: ${e}`;
      set({ error: msg });
    }
  },

  toggleServer: async (id, enabled) => {
    try {
      await invoke("mcp_toggle_server", { id, enabled });
    } catch (e) {
      const msg = `toggle: ${e}`;
      set({ error: msg });
    }
  },

  addConfig: async (config) => {
    try {
      await invoke("mcp_add_config", { config });
      set((s) => ({ configs: [...s.configs, config] }));
    } catch (e) {
      const msg = `addConfig: ${e}`;
      set({ error: msg });
    }
  },

  removeConfig: async (id) => {
    try {
      await invoke("mcp_remove_config", { id });
      set((s) => ({ configs: s.configs.filter((c) => c.id !== id) }));
    } catch (e) {
      const msg = `removeConfig: ${e}`;
      set({ error: msg });
    }
  },
}));
