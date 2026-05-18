import { create } from "zustand";
import { persist, createJSONStorage } from "zustand/middleware";

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

interface HealthAlert {
  level: string;
  message: string;
  advice: string;
  auto_action: string | null;
  timestamp: number;
}

interface PhiBrainState {
  available: boolean;
  profileMaturity: number;
  healthAlerts: HealthAlert[];
  enabled: boolean;
  proofreadingEnabled: boolean;
  guardianEnabled: boolean;

  setAvailable: (available: boolean) => void;
  setProfileMaturity: (maturity: number) => void;
  addHealthAlert: (alert: HealthAlert) => void;
  clearAlerts: () => void;
  toggleEnabled: () => void;
  toggleProofreading: () => void;
  toggleGuardian: () => void;
}

export const usePhiBrainStore = create<PhiBrainState>()(
  persist(
    (set) => ({
      available: false,
      profileMaturity: 0.0,
      healthAlerts: [],
      enabled: true,
      proofreadingEnabled: true,
      guardianEnabled: true,

      setAvailable: (available) => set({ available }),
      setProfileMaturity: (profileMaturity) => set({ profileMaturity }),
      addHealthAlert: (alert) =>
        set((s) => ({
          healthAlerts: [alert, ...s.healthAlerts].slice(0, 50),
        })),
      clearAlerts: () => set({ healthAlerts: [] }),
      toggleEnabled: () => set((s) => ({ enabled: !s.enabled })),
      toggleProofreading: () =>
        set((s) => ({ proofreadingEnabled: !s.proofreadingEnabled })),
      toggleGuardian: () =>
        set((s) => ({ guardianEnabled: !s.guardianEnabled })),
    }),
    {
      name: "phi-brain-store",
      storage: createJSONStorage(() => safeStorage),
    }
  )
);
