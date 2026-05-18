import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

export interface SystemSettings {
  sandbox: boolean;
  network: boolean;
  filesystem: boolean;
  permission_prompts: boolean;
  audit_logging: boolean;
  headless_enable: boolean;
  payment_default_chain: string;
  payment_audit: boolean;
}

export interface ConnectionConfig {
  enabled: boolean;
  label: string;
  fields: Record<string, string>;
}

interface ConfigStore {
  apiKeys: Record<string, string>;
  connectionConfigs: Record<string, ConnectionConfig>;
  systemSettings: SystemSettings;
  verifiedProviders: string[];
  loading: boolean;
  loadConfig: () => Promise<void>;
  setApiKeyLocal: (provider: string, key: string) => void;
  saveApiKey: (provider: string, key: string) => Promise<void>;
  saveConnection: (id: string, config: ConnectionConfig) => Promise<void>;
  updateSystemSetting: <K extends keyof SystemSettings>(key: K, value: SystemSettings[K]) => Promise<void>;
  addVerifiedProvider: (provider: string) => void;
  verifyAll: () => Promise<void>;
}

const DEFAULT_SYSTEM_SETTINGS: SystemSettings = {
  sandbox: true,
  network: true,
  filesystem: false,
  permission_prompts: true,
  audit_logging: true,
  headless_enable: false,
  payment_default_chain: "Ethereum",
  payment_audit: true,
};

export const useConfigStore = create<ConfigStore>((set, get) => ({
  apiKeys: {},
  connectionConfigs: {},
  systemSettings: DEFAULT_SYSTEM_SETTINGS,
  verifiedProviders: [],
  loading: true,
  loadConfig: async () => {
    try {
      const raw = await invoke<string>("get_config");
      const cfg = JSON.parse(raw);
      set({ 
        apiKeys: cfg.api_keys || {}, 
        connectionConfigs: cfg.connection_configs || {},
        systemSettings: { ...DEFAULT_SYSTEM_SETTINGS, ...(cfg.system_settings || {}) },
        verifiedProviders: cfg.verified_providers || [],
        loading: false 
      });
    } catch (e) {
      console.error("Failed to load config", e);
      set({ loading: false });
    }
  },
  setApiKeyLocal: (provider, key) => {
    set((s) => ({ apiKeys: { ...s.apiKeys, [provider]: key } }));
  },
  saveApiKey: async (provider, key) => {
    try {
      await invoke("save_api_key", { provider, key });
      set((s) => ({ 
        apiKeys: { ...s.apiKeys, [provider]: key },
        verifiedProviders: s.verifiedProviders.filter((p) => p !== provider)
      }));
    } catch (e) {
      console.error("Failed to save api key", e);
    }
  },
  saveConnection: async (id, config) => {
    try {
      await invoke("save_connection_config", { id, config_json: JSON.stringify(config) });
      set((s) => ({ connectionConfigs: { ...s.connectionConfigs, [id]: config } }));
    } catch (e) {
      console.error("Failed to save connection config", e);
    }
  },
  updateSystemSetting: async (key, value) => {
    const newSettings = { ...get().systemSettings, [key]: value };
    set({ systemSettings: newSettings });
    try {
      await invoke("save_system_settings", { settings_json: JSON.stringify(newSettings) });
    } catch (e) {
      console.error("Failed to save system settings", e);
    }
  },
  addVerifiedProvider: async (provider) => {
    set((s) => {
      const newProviders = s.verifiedProviders.includes(provider) ? s.verifiedProviders : [...s.verifiedProviders, provider];
      invoke("save_verified_providers", { providers: newProviders }).catch(e => console.error("Failed to save verified providers", e));
      return { verifiedProviders: newProviders };
    });
  },
  verifyAll: async () => {
    const { apiKeys, addVerifiedProvider } = get();
    const providersToTest = Object.keys(apiKeys).filter(k => apiKeys[k] && apiKeys[k].trim() !== "");
    if (!providersToTest.includes("ollama")) {
      providersToTest.push("ollama");
    }

    // Run connections tests concurrently
    await Promise.all(
      providersToTest.map(async (id) => {
        try {
          await invoke<string>("model_test_connection", { id });
          addVerifiedProvider(id);
        } catch { /* skip unresponsive providers on startup */ }
      })
    );
  },
}));
