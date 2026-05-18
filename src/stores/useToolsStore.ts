import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ToolInfo, ToolsListResult } from "@/types";

interface ToolsState {
  tools: ToolInfo[];
  byCategory: { category: string; count: number }[];
  total: number;
  loading: boolean;
  error: string | null;
  searchQuery: string;
  filterCategory: string | null;
  fetchTools: () => Promise<void>;
  toggleTool: (id: string, enabled: boolean) => Promise<void>;
  enableCategory: (category: string) => Promise<void>;
  searchTools: (query: string) => Promise<void>;
  setFilterCategory: (cat: string | null) => void;
}

export const useToolsStore = create<ToolsState>((set, get) => ({
  tools: [],
  byCategory: [],
  total: 0,
  loading: false,
  error: null,
  searchQuery: "",
  filterCategory: null,

  fetchTools: async () => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<string>("list_all_tools");
      const parsed = JSON.parse(result) as ToolsListResult;
      set({
        tools: parsed.tools,
        byCategory: parsed.by_category,
        total: parsed.total,
        loading: false,
      });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  toggleTool: async (id, enabled) => {
    try {
      await invoke("toggle_tool", { id, enabled });
      set((s) => ({
        tools: s.tools.map((t) => (t.id === id ? { ...t, enabled } : t)),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  enableCategory: async (category) => {
    try {
      await invoke("enable_tool_category", { category });
      set((s) => ({
        tools: s.tools.map((t) =>
          t.category === category ? { ...t, enabled: true } : t,
        ),
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  searchTools: async (query) => {
    set({ searchQuery: query, loading: true });
    if (!query.trim()) {
      const state = get();
      await state.fetchTools();
      return;
    }
    try {
      const result = await invoke<string>("search_tools", { query });
      const tools = JSON.parse(result) as ToolInfo[];
      set({ tools, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  setFilterCategory: (cat) => set({ filterCategory: cat }),
}));
