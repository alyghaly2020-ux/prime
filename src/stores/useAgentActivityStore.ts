import { create } from "zustand";

interface AgentActivityState {
  chatActive: boolean;
  codeActive: boolean;
  browserActive: boolean;
  paymentsActive: boolean;
  setActivity: (tab: string, active: boolean) => void;
}

export const useAgentActivityStore = create<AgentActivityState>((set) => ({
  chatActive: false,
  codeActive: false,
  browserActive: false,
  paymentsActive: false,
  setActivity: (tab, active) => {
    if (tab === "chat") set({ chatActive: active });
    if (tab === "code") set({ codeActive: active });
    if (tab === "browser") set({ browserActive: active });
    if (tab === "payments") set({ paymentsActive: active });
  },
}));
