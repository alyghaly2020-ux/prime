import { create } from "zustand";
import { persist } from "zustand/middleware";

export type ModelProvider =
  | "openai" | "anthropic" | "deepseek" | "google" | "ollama"
  | "openrouter" | "groq" | "together" | "mistral" | "cohere"
  | "perplexity" | "azure" | "aws" | "replicate" | "huggingface"
  | "fireworks" | "anyscale" | "lmstudio" | "localai" | "groqcloud";

export interface ProviderConfig {
  id: ModelProvider;
  label: string;
  apiKey: string;
  baseUrl: string;
  enabled: boolean;
  selectedModel?: string;
  availableModels?: string[];
}

export interface ModelProfile {
  provider: ModelProvider;
  quality: number;
  cost: "low" | "medium" | "high";
  speed: "fast" | "medium" | "slow";
  specialties: string[];
}

export type RoutingMode = "auto" | "manual";

export interface TaskRoute {
  task: string;
  provider: ModelProvider;
}

interface ModelStoreState {
  providers: ProviderConfig[];
  routingMode: RoutingMode;
  activeProvider: ModelProvider;
  manualProvider: ModelProvider;
  taskRoutes: TaskRoute[];
  costToday: number;
  costSaved: number;

  setProviderKey: (id: ModelProvider, key: string) => void;
  setProviderBaseUrl: (id: ModelProvider, url: string) => void;
  setProviderModels: (id: ModelProvider, models: string[]) => void;
  setProviderSelectedModel: (id: ModelProvider, model: string) => void;
  toggleProvider: (id: ModelProvider) => void;
  addProvider: (id: ModelProvider) => void;
  setRoutingMode: (mode: RoutingMode) => void;
  setManualProvider: (provider: ModelProvider) => void;
  setTaskRoute: (task: string, provider: ModelProvider) => void;
  getProfiles: () => ModelProfile[];
  getTasksForProvider: (provider: ModelProvider) => string[];
  getProviderForTask: (task: string) => ModelProvider;
}

const DEFAULT_PROFILES: ModelProfile[] = [
  { provider: "openai", quality: 9, cost: "high", speed: "medium", specialties: ["planning", "coding", "chat"] },
  { provider: "anthropic", quality: 10, cost: "high", speed: "medium", specialties: ["planning", "coding", "research"] },
  { provider: "deepseek", quality: 9, cost: "low", speed: "fast", specialties: ["coding", "debugging"] },
  { provider: "google", quality: 7, cost: "low", speed: "fast", specialties: ["ui", "chat", "research"] },
  { provider: "ollama", quality: 6, cost: "low", speed: "medium", specialties: ["chat", "simple"] },
];

const PROVIDER_LABELS: Record<ModelProvider, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  deepseek: "DeepSeek",
  google: "Google Gemini",
  ollama: "Ollama (Local)",
  openrouter: "OpenRouter",
  groq: "Groq",
  together: "Together AI",
  mistral: "Mistral AI",
  cohere: "Cohere",
  perplexity: "Perplexity",
  azure: "Azure OpenAI",
  aws: "AWS Bedrock",
  replicate: "Replicate",
  huggingface: "Hugging Face",
  fireworks: "Fireworks AI",
  anyscale: "Anyscale",
  lmstudio: "LM Studio",
  localai: "LocalAI",
  groqcloud: "GroqCloud",
};

const PROVIDER_BASE_URLS: Record<ModelProvider, string> = {
  openai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  deepseek: "https://api.deepseek.com",
  google: "https://generativelanguage.googleapis.com",
  ollama: "http://localhost:11434",
  openrouter: "https://openrouter.ai/api",
  groq: "https://api.groq.com/openai",
  together: "https://api.together.xyz",
  mistral: "https://api.mistral.ai",
  cohere: "https://api.cohere.ai",
  perplexity: "https://api.perplexity.ai",
  azure: "https://YOUR_RESOURCE.openai.azure.com",
  aws: "https://bedrock-runtime.YOUR_REGION.amazonaws.com",
  replicate: "https://api.replicate.com",
  huggingface: "https://api-inference.huggingface.co",
  fireworks: "https://api.fireworks.ai",
  anyscale: "https://api.endpoints.anyscale.com",
  lmstudio: "http://localhost:1234",
  localai: "http://localhost:8080",
  groqcloud: "https://api.groq.com/openai",
};

export const useModelStore = create<ModelStoreState>()(
  persist(
    (set, get) => ({
      providers: [
        { id: "openai" as ModelProvider, label: "OpenAI", apiKey: "", baseUrl: PROVIDER_BASE_URLS.openai, enabled: true },
        { id: "anthropic" as ModelProvider, label: "Anthropic", apiKey: "", baseUrl: PROVIDER_BASE_URLS.anthropic, enabled: false },
        { id: "deepseek" as ModelProvider, label: "DeepSeek", apiKey: "", baseUrl: PROVIDER_BASE_URLS.deepseek, enabled: false },
        { id: "google" as ModelProvider, label: "Google Gemini", apiKey: "", baseUrl: PROVIDER_BASE_URLS.google, enabled: false },
        { id: "ollama" as ModelProvider, label: "Ollama (Local)", apiKey: "", baseUrl: PROVIDER_BASE_URLS.ollama, enabled: false },
        { id: "mistral" as ModelProvider, label: "Mistral AI", apiKey: "", baseUrl: PROVIDER_BASE_URLS.mistral, enabled: false },
        { id: "groq" as ModelProvider, label: "Groq", apiKey: "", baseUrl: PROVIDER_BASE_URLS.groq, enabled: false },
        { id: "openrouter" as ModelProvider, label: "OpenRouter", apiKey: "", baseUrl: PROVIDER_BASE_URLS.openrouter, enabled: false },
        { id: "localai" as ModelProvider, label: "LocalAI", apiKey: "", baseUrl: PROVIDER_BASE_URLS.localai, enabled: false },
        { id: "custom_openai" as ModelProvider, label: "Custom OpenAI API", apiKey: "", baseUrl: "", enabled: false },
      ],
      routingMode: "auto",
      activeProvider: "openai",
      manualProvider: "openai",
      taskRoutes: [
        { task: "planning", provider: "anthropic" },
        { task: "coding", provider: "deepseek" },
        { task: "chat", provider: "openai" },
        { task: "ui", provider: "google" },
        { task: "debugging", provider: "deepseek" },
        { task: "research", provider: "anthropic" },
      ],
      costToday: 0.42,
      costSaved: 2.15,

      setProviderKey: (id: ModelProvider, key: string) =>
        set((s: any) => ({
          providers: s.providers.map((p: any) => (p.id === id ? { ...p, apiKey: key } : p)),
        })),

      setProviderBaseUrl: (id: ModelProvider, url: string) =>
        set((s: any) => ({
          providers: s.providers.map((p: any) => (p.id === id ? { ...p, baseUrl: url } : p)),
        })),

      setProviderModels: (id: ModelProvider, models: string[]) =>
        set((s: any) => ({
          providers: s.providers.map((p: any) => (p.id === id ? { ...p, availableModels: models, selectedModel: p.selectedModel || models[0] } : p)),
        })),

      setProviderSelectedModel: (id: ModelProvider, model: string) =>
        set((s: any) => ({
          providers: s.providers.map((p: any) => (p.id === id ? { ...p, selectedModel: model } : p)),
        })),

      toggleProvider: (id: ModelProvider) =>
        set((s: any) => ({
          providers: s.providers.map((p: any) => (p.id === id ? { ...p, enabled: !p.enabled } : p)),
        })),

      addProvider: (id: ModelProvider) =>
        set((s: any) => {
          if (s.providers.find((p: any) => p.id === id)) return s;
          return {
            providers: [...s.providers, { id, label: PROVIDER_LABELS[id], apiKey: "", baseUrl: PROVIDER_BASE_URLS[id], enabled: true }],
          };
        }),

      setRoutingMode: (mode: RoutingMode) =>
        set({ routingMode: mode }),

      setManualProvider: (provider: ModelProvider) =>
        set({ manualProvider: provider, activeProvider: provider }),

      setTaskRoute: (task: string, provider: ModelProvider) =>
        set((s: any) => ({
          taskRoutes: s.taskRoutes.map((r: any) => (r.task === task ? { ...r, provider } : r)),
        })),

      getProfiles: () => DEFAULT_PROFILES,

      getTasksForProvider: (provider: ModelProvider) => {
        const routes = (get() as any).taskRoutes;
        return routes.filter((r: any) => r.provider === provider).map((r: any) => r.task);
      },

      getProviderForTask: (task: string) => {
        const s = get() as any;
        if (s.routingMode === "manual") return s.manualProvider;
        const route = s.taskRoutes.find((r: any) => r.task === task);
        return route?.provider ?? s.activeProvider;
      },
    }),
    { 
      name: "prime-model-store",
      merge: (persistedState: any, currentState: any) => {
        const mergedProviders = [...currentState.providers];
        if (persistedState && Array.isArray(persistedState.providers)) {
          persistedState.providers.forEach((p: any) => {
            const idx = mergedProviders.findIndex((mp) => mp.id === p.id);
            if (idx !== -1) {
              mergedProviders[idx] = { ...mergedProviders[idx], ...p };
            }
          });
        }
        return {
          ...currentState,
          ...persistedState,
          providers: mergedProviders,
        };
      }
    },
  ),
);
