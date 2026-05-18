import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Workflow } from "@/types";

interface WorkflowState {
  workflows: Workflow[];
  selectedWorkflow: Workflow | null;
  loading: boolean;
  error: string | null;
  fetchWorkflows: () => Promise<void>;
  selectWorkflow: (workflow: Workflow | null) => void;
  startWorkflow: (id: string) => Promise<void>;
  cancelWorkflow: (id: string) => Promise<void>;
  pauseWorkflow: (id: string) => Promise<void>;
  resumeWorkflow: (id: string) => Promise<void>;
}

export const useWorkflowStore = create<WorkflowState>((set) => ({
  workflows: [],
  selectedWorkflow: null,
  loading: false,
  error: null,

  fetchWorkflows: async () => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<string>("list_workflows");
      const workflows = JSON.parse(result) as Workflow[];
      set({ workflows, loading: false });
    } catch (e) {
      const msg = `workflows: ${e}`;
      set({ error: msg, loading: false });
    }
  },

  selectWorkflow: (workflow) => set({ selectedWorkflow: workflow }),

  startWorkflow: async (id) => {
    try {
      await invoke("workflow_start", { id });
      await set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id ? { ...w, status: "running" } : w,
        ),
      }));
    } catch (e) {
      const msg = `startWf: ${e}`;
      set({ error: msg });
    }
  },

  cancelWorkflow: async (id) => {
    try {
      await invoke("workflow_cancel", { id });
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id ? { ...w, status: "cancelled" } : w,
        ),
      }));
    } catch (e) {
      const msg = `cancelWf: ${e}`;
      set({ error: msg });
    }
  },

  pauseWorkflow: async (id) => {
    try {
      await invoke("workflow_pause", { id });
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id ? { ...w, status: "paused" } : w,
        ),
      }));
    } catch (e) {
      const msg = `pauseWf: ${e}`;
      set({ error: msg });
    }
  },

  resumeWorkflow: async (id) => {
    try {
      await invoke("workflow_resume", { id });
      set((s) => ({
        workflows: s.workflows.map((w) =>
          w.id === id ? { ...w, status: "running" } : w,
        ),
      }));
    } catch (e) {
      const msg = `resumeWf: ${e}`;
      set({ error: msg });
    }
  },
}));
