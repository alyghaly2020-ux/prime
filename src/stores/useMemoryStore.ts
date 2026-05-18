import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { MemoryEntry, MemoryStats, MemoryType } from "@/types";

interface MemoryState {
  entries: MemoryEntry[];
  stats: MemoryStats | null;
  selectedType: MemoryType;
  searchQuery: string;
  loading: boolean;
  error: string | null;
  setSelectedType: (type: MemoryType) => void;
  setSearchQuery: (query: string) => void;
  fetchEntries: () => Promise<void>;
  fetchStats: () => Promise<void>;
  deleteEntry: (id: string) => Promise<void>;
  clearMemory: (type: MemoryType) => Promise<void>;
}

export const useMemoryStore = create<MemoryState>((set, get) => ({
  entries: [],
  stats: null,
  selectedType: "working",
  searchQuery: "",
  loading: false,
  error: null,

  setSelectedType: (type) => set({ selectedType: type }),
  setSearchQuery: (query) => set({ searchQuery: query }),

  fetchEntries: async () => {
    set({ loading: true, error: null });
    try {
      const { selectedType, searchQuery } = get();
      const result = await invoke<string>("query_memory", {
        query: searchQuery || "*",
        memoryType: selectedType,
      });
      const entries = JSON.parse(result) as MemoryEntry[];
      set({ entries, loading: false });
    } catch (e) {
      const msg = `memory: ${e}`;
      set({ error: msg, loading: false });
    }
  },

  fetchStats: async () => {
    try {
      const result = await invoke<string>("get_memory_stats");
      const stats = JSON.parse(result) as MemoryStats;
      set({ stats });
    } catch {
      // Silently fail for stats
    }
  },

  deleteEntry: async (id) => {
    try {
      await invoke("delete_memory_entry", { id });
      set((s) => ({ entries: s.entries.filter((e) => e.id !== id) }));
    } catch (e) {
      const msg = `deleteEntry: ${e}`;
      set({ error: msg });
    }
  },

  clearMemory: async (type) => {
    try {
      await invoke("clear_memory", { memoryType: type });
      set({ entries: [] });
    } catch (e) {
      const msg = `clearMemory: ${e}`;
      set({ error: msg });
    }
  },
}));
