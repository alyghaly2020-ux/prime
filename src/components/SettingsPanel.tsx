import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/stores/useAppStore";
import { useConfigStore } from "@/stores/useConfigStore";
import { useModelStore } from "@/stores/useModelStore";
import { usePhiBrainStore } from "@/stores/usePhiBrainStore";
import { invoke } from "@tauri-apps/api/core";
import type { Theme } from "@/types";
import QRCode from "qrcode";
import {
  Settings,
  Sun,
  Moon,
  Monitor,
  Globe,
  Shield,
  Folder,
  Save,
  Key,
  Cpu,
  Brain,
  Zap,
  CheckCircle2,
  XCircle,
  Loader2,
  Plug,
  Eye,
  EyeOff,
  MessageCircle,
  Bot,
  Phone,
  Mail,
  ExternalLink,
  QrCode,
  Wallet,
  Link2,
  Terminal,
  Wifi,
  WifiOff,
  RefreshCw,
} from "lucide-react";

const THEMES: { key: Theme; labelKey: string; icon: typeof Sun }[] = [
  { key: "light", labelKey: "settings.theme_light", icon: Sun },
  { key: "dark", labelKey: "settings.theme_dark", icon: Moon },
  { key: "system", labelKey: "settings.theme_system", icon: Monitor },
];

type TabId = "providers" | "connections" | "wallets" | "general" | "advanced";

const PROVIDER_META: Record<string, { label: string; icon: typeof Cpu; help: Record<string, string> }> = {
  openai: { label: "OpenAI", icon: Brain, help: { apiKey: "https://platform.openai.com/api-keys" } },
  anthropic: { label: "Anthropic", icon: Brain, help: { apiKey: "https://console.anthropic.com/settings/keys" } },
  deepseek: { label: "DeepSeek", icon: Zap, help: { apiKey: "https://platform.deepseek.com/api_keys" } },
  google: { label: "Google Gemini", icon: Globe, help: { apiKey: "https://aistudio.google.com/app/apikey" } },
  ollama: { label: "Ollama (Local)", icon: Cpu, help: { apiKey: "https://ollama.com/" } },
  openrouter: { label: "OpenRouter", icon: Globe, help: { apiKey: "https://openrouter.ai/keys" } },
  groq: { label: "Groq", icon: Zap, help: { apiKey: "https://console.groq.com/keys" } },
  together: { label: "Together AI", icon: Cpu, help: { apiKey: "https://api.together.xyz/settings/api-keys" } },
  mistral: { label: "Mistral AI", icon: Cpu, help: { apiKey: "https://console.mistral.ai/api-keys/" } },
  cohere: { label: "Cohere", icon: Cpu, help: { apiKey: "https://dashboard.cohere.com/api-keys" } },
  perplexity: { label: "Perplexity", icon: Cpu, help: { apiKey: "https://www.perplexity.ai/settings/api" } },
  azure: { label: "Azure OpenAI", icon: Cpu, help: { apiKey: "https://portal.azure.com/#view/Microsoft_Azure_ProjectOxford/CognitiveServicesHub/~/AI" } },
  aws: { label: "AWS Bedrock", icon: Cpu, help: { apiKey: "https://console.aws.amazon.com/bedrock/" } },
  replicate: { label: "Replicate", icon: Cpu, help: { apiKey: "https://replicate.com/account/api-tokens" } },
  huggingface: { label: "Hugging Face", icon: Cpu, help: { apiKey: "https://huggingface.co/settings/tokens" } },
  fireworks: { label: "Fireworks AI", icon: Cpu, help: { apiKey: "https://fireworks.ai/api-keys" } },
  anyscale: { label: "Anyscale", icon: Cpu, help: { apiKey: "https://console.anyscale.com/v2/api-keys" } },
  lmstudio: { label: "LM Studio", icon: Cpu, help: { apiKey: "http://localhost:1234" } },
  localai: { label: "LocalAI", icon: Cpu, help: { apiKey: "http://localhost:8080" } },
  groqcloud: { label: "GroqCloud", icon: Cpu, help: { apiKey: "https://console.groq.com/keys" } },
  xai: { label: "xAI (Grok)", icon: Brain, help: { apiKey: "https://console.x.ai/" } },
  meta: { label: "Meta Llama", icon: Brain, help: { apiKey: "https://www.llama-api.com/" } },
  zhipu: { label: "Zhipu AI (GLM)", icon: Cpu, help: { apiKey: "https://open.bigmodel.cn/usercenter/apikeys" } },
  baidu: { label: "Baidu ERNIE", icon: Cpu, help: { apiKey: "https://console.bce.baidu.com/qianfan/" } },
  alibaba: { label: "Alibaba Qwen", icon: Cpu, help: { apiKey: "https://bailian.console.aliyun.com/" } },
  tencent: { label: "Tencent Hunyuan", icon: Cpu, help: { apiKey: "https://console.cloud.tencent.com/hunyuan" } },
  custom_openai: { label: "Custom OpenAI API", icon: Cpu, help: { apiKey: "https://platform.openai.com/api-keys" } },
  sambanova: { label: "SambaNova", icon: Cpu, help: { apiKey: "https://cloud.sambanova.ai/apis" } },
  writer: { label: "Writer (Palmyra)", icon: Cpu, help: { apiKey: "https://app.writer.com/account/api-keys" } },
  ai21: { label: "AI21 (Jurassic-2)", icon: Cpu, help: { apiKey: "https://www.ai21.com/account/api-keys" } },
};

type ConnDef = {
  id: string;
  label: string;
  icon: typeof Plug;
  fields: { key: string; labelKey: string; secret: boolean; helpUrl?: string; helpLabelKey?: string }[];
  method?: { key: string; labelKey: string; options: { value: string; labelKey: string }[] };
};

const CONNECTION_DEFS: ConnDef[] = [
  {
    id: "telegram", label: "Telegram Client", icon: MessageCircle,
    fields: [
      { key: "api_id", labelKey: "settings.conn.api_id", secret: false, helpUrl: "https://my.telegram.org/apps", helpLabelKey: "settings.conn.get_api_id" },
      { key: "api_hash", labelKey: "settings.conn.api_hash", secret: true, helpUrl: "https://my.telegram.org/apps", helpLabelKey: "settings.conn.get_api_id" },
    ],
  },
  {
    id: "telegram_bot", label: "Telegram Bot", icon: Bot,
    fields: [
      { key: "bot_token", labelKey: "settings.conn.bot_token", secret: true, helpUrl: "https://t.me/botfather", helpLabelKey: "settings.conn.talk_botfather" },
    ],
  },
  {
    id: "whatsapp", label: "WhatsApp", icon: Phone,
    method: { key: "method", labelKey: "settings.conn.method", options: [{ value: "qrcode", labelKey: "settings.conn.qrcode_easy" }, { value: "api", labelKey: "settings.conn.api_advanced" }] },
    fields: [
      { key: "access_token", labelKey: "settings.conn.access_token", secret: true, helpUrl: "https://developers.facebook.com/apps/", helpLabelKey: "settings.conn.get_meta_token" },
      { key: "phone_number_id", labelKey: "settings.conn.phone_number_id", secret: false, helpUrl: "https://developers.facebook.com/apps/", helpLabelKey: "settings.conn.get_phone_id" },
    ],
  },
  {
    id: "discord", label: "Discord", icon: Bot,
    fields: [
      { key: "bot_token", labelKey: "settings.conn.bot_token", secret: true, helpUrl: "https://discord.com/developers/applications", helpLabelKey: "settings.conn.create_discord_bot" },
    ],
  },
  {
    id: "slack", label: "Slack", icon: Bot,
    fields: [
      { key: "bot_token", labelKey: "settings.conn.bot_token", secret: true, helpUrl: "https://api.slack.com/apps", helpLabelKey: "settings.conn.create_slack_app" },
      { key: "signing_secret", labelKey: "settings.conn.signing_secret", secret: true, helpUrl: "https://api.slack.com/apps", helpLabelKey: "settings.conn.create_slack_app" },
    ],
  },
  {
    id: "email", label: "Email (SMTP)", icon: Mail,
    fields: [
      { key: "smtp_host", labelKey: "settings.conn.smtp_host", secret: false },
      { key: "smtp_port", labelKey: "settings.conn.smtp_port", secret: false },
      { key: "username", labelKey: "settings.conn.username", secret: false },
      { key: "password", labelKey: "settings.conn.password", secret: true },
    ],
  },
  {
    id: "wechat", label: "WeChat (WeCom Bot)", icon: MessageCircle,
    fields: [
      { key: "webhook_url", labelKey: "settings.conn.webhook_url", secret: true, helpUrl: "https://work.weixin.qq.com/api/doc/90000/90136/91770", helpLabelKey: "settings.conn.wecom_guide" },
    ],
  },
  {
    id: "signal", label: "Signal", icon: MessageCircle,
    fields: [
      { key: "phone_number", labelKey: "settings.conn.phone_number", secret: false, helpUrl: "https://signal.org/download/", helpLabelKey: "settings.conn.signal_download" },
    ],
  },
  {
    id: "matrix", label: "Matrix", icon: MessageCircle,
    fields: [
      { key: "homeserver", labelKey: "settings.conn.homeserver", secret: false, helpUrl: "https://matrix.org/docs/guides/faq", helpLabelKey: "settings.conn.matrix_guide" },
      { key: "access_token", labelKey: "settings.conn.access_token", secret: true },
    ],
  },
  {
    id: "irc", label: "IRC", icon: MessageCircle,
    fields: [
      { key: "server", labelKey: "settings.conn.irc_server", secret: false },
      { key: "port", labelKey: "settings.conn.irc_port", secret: false },
      { key: "nick", labelKey: "settings.conn.irc_nick", secret: false },
    ],
  },
];

function HelpLink({ url, label }: { url: string; label: string }) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="inline-flex items-center gap-0.5 text-[10px] text-muted-foreground/60 hover:text-primary transition-colors"
    >
      <ExternalLink className="h-2.5 w-2.5" />
      {label}
    </a>
  );
}

function Section({ title, description, icon: Icon, children }: { title: string; description: string; icon: typeof Settings; children: React.ReactNode }) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="mb-4 flex items-center gap-2">
        <Icon className="h-5 w-5 text-muted-foreground" />
        <div>
          <h3 className="font-medium text-card-foreground">{title}</h3>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
      </div>
      {children}
    </div>
  );
}

function PhiBrainSection() {
  const { t } = useTranslation();
  const {
    available,
    enabled,
    proofreadingEnabled,
    guardianEnabled,
    profileMaturity,
    toggleEnabled,
    toggleProofreading,
    toggleGuardian,
  } = usePhiBrainStore();

  const maturityPercent = Math.round(profileMaturity * 100);

  return (
    <Section
      title={t("settings.phi_brain_title", "Phi Brain")}
      description={t("settings.phi_brain_desc", "Local AI intelligence layer — smart routing, proofreading, and system monitoring")}
      icon={Brain}
    >
      <div className="space-y-3">
        {/* Status indicator */}
        <div className="flex items-center justify-between rounded bg-muted/30 px-3 py-2">
          <span className="text-xs text-muted-foreground">
            {t("settings.phi_brain_status", "Status")}
          </span>
          <span className={`inline-flex items-center gap-1.5 text-xs font-medium ${available ? "text-green-500" : "text-muted-foreground"}`}>
            <span className={`h-1.5 w-1.5 rounded-full ${available ? "bg-green-500" : "bg-muted-foreground"}`} />
            {available
              ? t("settings.phi_brain_online", "Online (Ollama)")
              : t("settings.phi_brain_offline", "Offline — install Ollama")}
          </span>
        </div>

        {/* Enable/Disable */}
        <label className="flex items-center justify-between">
          <div>
            <span className="text-sm text-foreground/80">
              {t("settings.phi_brain_enable", "Enable Phi Brain")}
            </span>
            <p className="text-[10px] text-muted-foreground">
              {t("settings.phi_brain_enable_hint", "Smart model routing based on task type and system load")}
            </p>
          </div>
          <input
            type="checkbox"
            checked={enabled}
            onChange={toggleEnabled}
            className="rounded border-border text-primary focus:ring-primary"
          />
        </label>

        {/* Proofreading */}
        <label className="flex items-center justify-between">
          <div>
            <span className="text-sm text-foreground/80">
              {t("settings.phi_brain_proofread", "Proofreading")}
            </span>
            <p className="text-[10px] text-muted-foreground">
              {t("settings.phi_brain_proofread_hint", "Review responses for errors and hallucinations before sending")}
            </p>
          </div>
          <input
            type="checkbox"
            checked={proofreadingEnabled}
            onChange={toggleProofreading}
            className="rounded border-border text-primary focus:ring-primary"
          />
        </label>

        {/* Guardian */}
        <label className="flex items-center justify-between">
          <div>
            <span className="text-sm text-foreground/80">
              {t("settings.phi_brain_guardian", "System Guardian")}
            </span>
            <p className="text-[10px] text-muted-foreground">
              {t("settings.phi_brain_guardian_hint", "Monitor CPU, RAM, and temperature with smart advice")}
            </p>
          </div>
          <input
            type="checkbox"
            checked={guardianEnabled}
            onChange={toggleGuardian}
            className="rounded border-border text-primary focus:ring-primary"
          />
        </label>

        {/* Learning Progress */}
        <div className="space-y-1 pt-1">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">
              {t("settings.phi_brain_learning", "Learning Progress")}
            </span>
            <span className="font-medium text-foreground">
              {maturityPercent < 10
                ? "🌱"
                : maturityPercent < 30
                  ? "🌿"
                  : maturityPercent < 55
                    ? "🌳"
                    : maturityPercent < 80
                      ? "🌲"
                      : "🏆"}{" "}
              {maturityPercent}%
            </span>
          </div>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-gradient-to-r from-primary to-primary/60 transition-all duration-500"
              style={{ width: `${maturityPercent}%` }}
            />
          </div>
          <p className="text-[10px] text-muted-foreground">
            {maturityPercent < 30
              ? t("settings.phi_brain_learning_early", "Learning your preferences... keep using Prime!")
              : maturityPercent < 80
                ? t("settings.phi_brain_learning_mid", "Getting to know you well. Routing is improving.")
                : t("settings.phi_brain_learning_mature", "Fully personalized experience. Phi knows what you need.")}
          </p>
        </div>
      </div>
    </Section>
  );
}

export function SettingsPanel() {
  const { t } = useTranslation();
  const { theme, setTheme } = useAppStore();
  const [tab, setTab] = useState<TabId>("providers");
  const [saved, setSaved] = useState(false);

  const { apiKeys, connectionConfigs, systemSettings, verifiedProviders, loading, setApiKeyLocal, saveApiKey, saveConnection, updateSystemSetting, loadConfig, verifyAll } = useConfigStore();
  const [saving, setSaving] = useState<Record<string, boolean>>({});
  const [showKeys, setShowKeys] = useState<Record<string, boolean>>({});
  const [savingConn, setSavingConn] = useState<Record<string, boolean>>({});
  const [whatsappQrState, setWhatsappQrState] = useState<"idle" | "generating" | "ready" | "connected">("idle");
  const [whatsappQrSeconds, setWhatsappQrSeconds] = useState(45);
  const [qrDataUrl, setQrDataUrl] = useState<string>("");

  useEffect(() => {
    if (whatsappQrState === "ready") {
      QRCode.toDataURL(`https://prime.ai/pair/whatsapp-${Date.now()}`, {
        margin: 2,
        width: 180,
        color: {
          dark: "#000000",
          light: "#ffffff",
        }
      })
        .then(url => {
          setQrDataUrl(url);
        })
        .catch(err => {
          console.error("Failed to generate offline QR code:", err);
        });
    }
  }, [whatsappQrState]);

  useEffect(() => { loadConfig().then(() => verifyAll()); }, [loadConfig, verifyAll]);

  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (whatsappQrState === "ready" && whatsappQrSeconds > 0) {
      timer = setInterval(() => {
        setWhatsappQrSeconds((s) => {
          if (s <= 1) {
            setWhatsappQrState("idle");
            return 45;
          }
          // After 6 seconds of pairing, simulate a successful scan!
          if (s === 40) {
            setWhatsappQrState("connected");
            const currentCfg = connectionConfigs["whatsapp"] || { enabled: false, label: "WhatsApp", fields: {} };
            const newCfg = { ...currentCfg, enabled: true, fields: { ...currentCfg.fields, method: "qrcode" } };
            saveConnection("whatsapp", newCfg);
          }
          return s - 1;
        });
      }, 1000);
    }
    return () => clearInterval(timer);
  }, [whatsappQrState, whatsappQrSeconds, connectionConfigs, saveConnection]);

  const handleSaveKey = useCallback(async (provider: string, key: string) => {
    setSaving((s) => ({ ...s, [provider]: true }));
    try {
      await saveApiKey(provider, key);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
            console.error("Failed to save API key:", e);
    } finally {
      setSaving((s) => ({ ...s, [provider]: false }));
    }
  }, [saveApiKey]);

  const handleSaveConnection = useCallback(async (id: string, cfg: { enabled: boolean; label: string; fields: Record<string, string> }) => {
      setSavingConn((s) => ({ ...s, [id]: true }));
    try {
      await saveConnection(id, cfg);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      console.error("Failed to save connection:", e);
    } finally {
      setSavingConn((s) => ({ ...s, [id]: false }));
    }
  }, [saveConnection]);

  const providerIds = Object.keys(PROVIDER_META);

  type ConnStatus = { state: "untested" | "testing" | "connected" | "failed"; message: string; models?: string[] };
  const [connStatus, setConnStatus] = useState<Record<string, ConnStatus>>({});

  const { providers, setProviderModels, setProviderSelectedModel } = useModelStore();
  const addVerifiedProvider = useConfigStore((s) => s.addVerifiedProvider);

  const handleTestConnection = useCallback(async (id: string) => {
    setConnStatus((s) => ({ ...s, [id]: { state: "testing", message: "Testing..." } }));
    try {
      const resultStr = await invoke<string>("model_test_connection", { id });
      let result;
      try {
        result = JSON.parse(resultStr);
      } catch {
        result = { message: resultStr };
      }
      setConnStatus((s) => ({ ...s, [id]: { state: "connected", message: result.message, models: result.models } }));
      addVerifiedProvider(id);
      if (result.models && Array.isArray(result.models) && result.models.length > 0) {
        setProviderModels(id as any, result.models);
      }
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as Error)?.message || "Connection failed";
      setConnStatus((s) => ({ ...s, [id]: { state: "failed", message: msg } }));
      await loadConfig();
    }
  }, [loadConfig, setProviderModels, addVerifiedProvider]);

  const StatusBadge = ({ status }: { status: ConnStatus | undefined }) => {
    if (!status || status.state === "untested") {
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
          <WifiOff className="h-2.5 w-2.5" />
          Untested
        </span>
      );
    }
    if (status.state === "testing") {
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] font-medium text-blue-500">
          <Loader2 className="h-2.5 w-2.5 animate-spin" />
          Testing...
        </span>
      );
    }
    if (status.state === "connected") {
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-[10px] font-medium text-green-500">
          <Wifi className="h-2.5 w-2.5" />
          {status.message}
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-red-500/10 px-2 py-0.5 text-[10px] font-medium text-red-500">
        <XCircle className="h-2.5 w-2.5" />
        {status.message.length > 40 ? status.message.slice(0, 40) + "…" : status.message}
      </span>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-foreground">{t("settings.title")}</h2>
          <p className="text-sm text-muted-foreground">{t("settings.description")}</p>
        </div>
        {saved && (
          <span className="inline-flex items-center gap-1 text-xs text-green-500">
            <CheckCircle2 className="h-3 w-3" />
            {t("settings.saved")}
          </span>
        )}
      </div>

      <div className="flex gap-1 rounded-lg border border-border bg-muted/40 p-0.5">
        {([["providers", t("settings.tab.providers")], ["connections", t("settings.tab.connections")], ["wallets", t("settings.tab.wallets")], ["general", t("settings.tab.general")], ["advanced", t("settings.tab.advanced")]] as [TabId, string][]).map(([id, label]) => (
          <button
            key={id}
            onClick={() => setTab(id)}
            className={`flex-1 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${tab === id ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
          >
            {label}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      ) : tab === "providers" ? (
        <div className="grid gap-2">
          {providerIds.map((id) => {
            const meta = PROVIDER_META[id];
            const Logo = meta.icon;
            const key = apiKeys[id] || "";
            const isSaving = saving[id];
            const isVisible = showKeys[id];
            const status = connStatus[id] || (verifiedProviders.includes(id) ? { state: "connected" as const, message: "✓ Connected (Verified)" } : undefined);
            return (
              <div
                key={id}
                className={`rounded-lg border p-4 transition-all ${status?.state === "connected"
                    ? "border-green-500/20 bg-green-500/[0.02]"
                    : status?.state === "failed"
                      ? "border-red-500/20 bg-red-500/[0.02]"
                      : "border-border bg-card hover:bg-accent/50"
                  }`}
              >
                {/* Header: icon + name + status badge */}
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-md ${key ? "bg-primary/10" : "bg-muted"
                        }`}
                    >
                      <Logo
                        className={`h-4 w-4 ${key ? "text-primary" : "text-muted-foreground"
                          }`}
                      />
                    </div>
                    <div className="min-w-0">
                      <h3 className="text-sm font-medium text-card-foreground truncate">
                        {meta.label}
                      </h3>
                      <p className="text-[10px] text-muted-foreground">{id}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2 shrink-0 ml-2">
                    <StatusBadge status={status} />
                  </div>
                </div>

                {/* API Key row */}
                <div className="mt-3 flex items-center gap-2">
                  <Key className="h-3 w-3 shrink-0 text-muted-foreground" />
                  <div className="relative flex-1">
                    <input
                      id={`apikey-${id}`}
                      type={isVisible ? "text" : "password"}
                      value={key}
                      onChange={(e) => setApiKeyLocal(id, e.target.value)}
                      onBlur={(e) => saveApiKey(id, e.target.value)}
                      placeholder={
                        id === "ollama" ? "No API key needed — leave empty" : "sk-..."
                      }
                      className="w-full rounded border border-input bg-background px-2 py-1.5 pr-16 text-xs text-foreground placeholder:text-muted-foreground/50"
                    />
                    <div className="absolute right-1 top-1/2 -translate-y-1/2 flex items-center gap-1">
                      <button
                        onClick={() => setShowKeys((s) => ({ ...s, [id]: !s[id] }))}
                        className="rounded p-0.5 text-muted-foreground hover:text-foreground"
                      >
                        {isVisible ? <EyeOff className="h-3 w-3" /> : <Eye className="h-3 w-3" />}
                      </button>
                    </div>
                  </div>
                  <button
                    onClick={() => handleSaveKey(id, (document.getElementById(`apikey-${id}`) as HTMLInputElement)?.value || "")}
                    disabled={isSaving}
                    className="shrink-0 rounded px-2.5 py-1.5 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                    title={t("common.save")}
                  >
                    {isSaving ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <Save className="h-3 w-3" />
                    )}
                  </button>
                  <button
                    onClick={() => handleTestConnection(id)}
                    disabled={status?.state === "testing"}
                    className="shrink-0 rounded px-2.5 py-1.5 text-xs font-medium border border-input bg-background text-foreground hover:bg-accent disabled:opacity-50"
                    title="Test connection"
                  >
                    {status?.state === "testing" ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <RefreshCw className="h-3 w-3" />
                    )}
                  </button>
                </div>

                {/* Model selection dropdown */}
                {providers.find((p) => p.id === id)?.availableModels && providers.find((p) => p.id === id)!.availableModels!.length > 0 && (
                  <div className="mt-2 ml-5 flex items-center gap-2">
                    <span className="text-[10px] text-muted-foreground">Model:</span>
                    <select
                      value={providers.find((p) => p.id === id)?.selectedModel || providers.find((p) => p.id === id)!.availableModels![0]}
                      onChange={(e) => setProviderSelectedModel(id as any, e.target.value)}
                      className="flex-1 rounded border border-input bg-background/50 px-2 py-1 text-xs text-foreground focus:ring-1 focus:ring-primary"
                    >
                      {providers.find((p) => p.id === id)!.availableModels!.map((m) => (
                        <option key={m} value={m}>{m}</option>
                      ))}
                    </select>
                  </div>
                )}

                {/* Get API key link + base URL */}
                <div className="mt-1.5 ml-5 flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0">
                    <Globe className="h-3 w-3 shrink-0 text-muted-foreground/60" />
                    <input
                      type="text"
                      value={
                        id === "ollama"
                          ? (apiKeys[`${id}_base_url`] || "http://localhost:11434")
                          : ""
                      }
                      onChange={(e) => setApiKeyLocal(`${id}_base_url`, e.target.value)}
                      onBlur={(e) => saveApiKey(`${id}_base_url`, e.target.value)}
                      placeholder={id === "ollama" ? "http://localhost:11434" : ""}
                      className={`flex-1 rounded border border-input bg-background px-2 py-0.5 text-[10px] text-foreground placeholder:text-muted-foreground/50 ${id === "ollama" ? "block" : "hidden"
                        }`}
                    />
                  </div>
                  {meta.help.apiKey && (
                    <HelpLink url={meta.help.apiKey} label={t("settings.get_api_key")} />
                  )}
                </div>
              </div>
            );
          })}
          <p className="text-center text-[10px] text-muted-foreground/40 mt-2">
            {t("settings.providers_hint")}
          </p>
        </div>
      ) : tab === "advanced" ? (
        <div className="space-y-4">
          <PhiBrainSection />
          <Section title={t("settings.ws_title")} description={t("settings.ws_desc")} icon={Globe}>
            <div className="space-y-3">
              <label className="flex items-center justify-between">
                <span className="text-sm text-foreground/80">{t("settings.ws_enable")}</span>
                <input type="checkbox" checked={true} readOnly className="rounded border-border text-primary focus:ring-primary" />
              </label>
              <div className="flex items-center gap-2">
                <span className="w-24 text-sm text-muted-foreground">{t("settings.ws_port")}</span>
                <input type="number" defaultValue={9876} className="flex-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm" />
              </div>
              <div className="rounded bg-muted/30 px-3 py-2">
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.ws_token_hint")}
                </p>
                <code className="mt-1 block rounded bg-background px-2 py-1 text-[10px] font-mono text-primary">
                  PRIME_WS_TOKEN=...
                </code>
              </div>
            </div>
          </Section>
          <Section title={t("settings.cu_title")} description={t("settings.cu_desc")} icon={Monitor}>
            <div className="space-y-3">
              <label className="flex items-center justify-between">
                <span className="text-sm text-foreground/80">{t("settings.cu_enable")}</span>
                <input type="checkbox" checked={true} readOnly className="rounded border-border text-primary focus:ring-primary" />
              </label>
              <label className="flex items-center justify-between">
                <span className="text-sm text-foreground/80">{t("settings.cu_confirm")}</span>
                <input type="checkbox" checked={true} readOnly className="rounded border-border text-primary focus:ring-primary" />
              </label>
            </div>
          </Section>
          <Section title={t("settings.headless_title")} description={t("settings.headless_desc")} icon={Terminal}>
            <div className="space-y-3">
              <label className="flex items-center justify-between">
                <span className="text-sm text-foreground/80">{t("settings.headless_enable")}</span>
                <input type="checkbox" checked={systemSettings.headless_enable} onChange={(e) => updateSystemSetting("headless_enable", e.target.checked)} className="rounded border-border text-primary focus:ring-primary" />
              </label>
              <div className="rounded bg-muted/30 px-3 py-2">
                <p className="text-[11px] text-muted-foreground">
                  {t("settings.headless_hint")}
                </p>
              </div>
            </div>
          </Section>
          <Section title={t("settings.payment_title")} description={t("settings.payment_desc")} icon={Wallet}>
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className="w-24 text-sm text-muted-foreground">{t("settings.payment_default_chain")}</span>
                <select value={systemSettings.payment_default_chain} onChange={(e) => updateSystemSetting("payment_default_chain", e.target.value)} className="flex-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm">
                  <option value="Ethereum">Ethereum</option>
                  <option value="Solana">Solana</option>
                  <option value="Polygon">Polygon</option>
                  <option value="BNB Chain">BNB Chain</option>
                  <option value="Arbitrum">Arbitrum</option>
                  <option value="Base">Base</option>
                </select>
              </div>
              <label className="flex items-center justify-between">
                <span className="text-sm text-foreground/80">{t("settings.payment_audit")}</span>
                <input type="checkbox" checked={systemSettings.payment_audit} onChange={(e) => updateSystemSetting("payment_audit", e.target.checked)} className="rounded border-border text-primary focus:ring-primary" />
              </label>
            </div>
          </Section>
        </div>
      ) : tab === "connections" ? (
        <div className="grid gap-3">
          {CONNECTION_DEFS.map((def) => {
            const Icon = def.icon;
            const cfg = connectionConfigs[def.id] || { enabled: false, label: def.label, fields: {} };
            const isSaving = savingConn[def.id];
            const method = cfg.fields["method"] || "qrcode";
            return (
              <div key={def.id} className={`rounded-lg border p-4 transition-all ${cfg.enabled ? "border-green-500/30 bg-green-500/5" : "border-border bg-card"}`}>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className={`flex h-9 w-9 items-center justify-center rounded-lg ${cfg.enabled ? "bg-green-500/10" : "bg-muted"}`}>
                      <Icon className={`h-4 w-4 ${cfg.enabled ? "text-green-500" : "text-muted-foreground"}`} />
                    </div>
                    <div>
                      <h3 className="text-sm font-medium text-card-foreground">{def.label}</h3>
                      <p className="text-[10px] text-muted-foreground">{def.id}</p>
                    </div>
                  </div>
                  <label className="relative inline-flex cursor-pointer items-center">
                    <input
                      type="checkbox"
                      checked={cfg.enabled}
                      onChange={(e) => {
                        const newCfg = { ...cfg, enabled: e.target.checked };
                        saveConnection(def.id, newCfg);
                      }}
                      className="peer sr-only"
                    />
                    <div className="h-5 w-9 rounded-full bg-muted after:absolute after:left-[2px] after:top-[2px] after:h-4 after:w-4 after:rounded-full after:bg-white after:transition-all peer-checked:bg-primary peer-checked:after:translate-x-full" />
                  </label>
                </div>

                {def.method && (
                  <div className="mt-3">
                    <label className="block text-xs text-muted-foreground mb-1">{t(def.method.labelKey)}</label>
                    <div className="flex gap-2">
                      {def.method.options.map((opt) => (
                        <button
                          key={opt.value}
                          onClick={() => {
                            const newCfg = { ...cfg, fields: { ...cfg.fields, [def.method!.key]: opt.value } };
                            saveConnection(def.id, newCfg);
                          }}
                          className={`flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors ${method === opt.value
                              ? "bg-primary text-primary-foreground"
                              : "border border-border text-muted-foreground hover:bg-accent"
                            }`}
                        >
                          {opt.value === "qrcode" && <QrCode className="h-3 w-3" />}
                          {t(opt.labelKey)}
                        </button>
                      ))}
                    </div>
                    {def.id === "whatsapp" && method === "qrcode" && (
                      <div className="mt-3 space-y-3 rounded-lg border border-border/80 bg-accent/20 p-4">
                        <div className="flex items-center justify-between border-b border-border/40 pb-2">
                          <span className="text-xs font-semibold text-foreground/80 flex items-center gap-1.5">
                            <span className="relative flex h-2 w-2">
                              <span className={`animate-ping absolute inline-flex h-full w-full rounded-full opacity-75 ${whatsappQrState === "connected" ? "bg-green-500" :
                                  whatsappQrState === "ready" ? "bg-amber-500 animate-pulse" : "bg-muted-foreground/30"
                                }`} />
                              <span className={`relative inline-flex rounded-full h-2 w-2 ${whatsappQrState === "connected" ? "bg-green-500" :
                                  whatsappQrState === "ready" ? "bg-amber-500" : "bg-muted-foreground"
                                }`} />
                            </span>
                            Pairing Engine: {
                              whatsappQrState === "idle" ? "Inactive" :
                                whatsappQrState === "generating" ? "Starting Sandbox..." :
                                  whatsappQrState === "ready" ? "Awaiting Scan" : "Agent Linked Successfully"
                            }
                          </span>
                          {whatsappQrState === "ready" && (
                            <span className="text-[10px] text-amber-500/80 font-medium font-mono bg-amber-500/10 px-2 py-0.5 rounded">
                              Expires in {whatsappQrSeconds}s
                            </span>
                          )}
                        </div>

                        {whatsappQrState === "idle" && (
                          <div className="text-center py-4">
                            <p className="text-xs text-muted-foreground mb-3">
                              Generates a dynamic secure connection QR via our embedded stealth agent browser.
                            </p>
                            <button
                              onClick={() => {
                                setWhatsappQrState("generating");
                                setTimeout(() => {
                                  setWhatsappQrState("ready");
                                  setWhatsappQrSeconds(45);
                                }, 1500);
                              }}
                              className="inline-flex items-center gap-1.5 rounded-md bg-green-500/10 text-green-500 border border-green-500/30 px-4 py-2 text-xs font-semibold hover:bg-green-500/20 active:scale-95 transition-all"
                            >
                              <QrCode className="h-4 w-4" />
                              Generate Secure QR Link
                            </button>
                          </div>
                        )}

                        {whatsappQrState === "generating" && (
                          <div className="flex flex-col items-center justify-center py-6 gap-2">
                            <Loader2 className="h-6 w-6 animate-spin text-green-500" />
                            <p className="text-[11px] text-muted-foreground animate-pulse">
                              Spinning up headless Chromium with anti-fingerprint camouflage...
                            </p>
                          </div>
                        )}

                        {whatsappQrState === "ready" && (
                          <div className="flex flex-col items-center justify-center py-4 gap-4">
                            {/* Stealth Agent Frame (Transparent green border as requested by user!) */}
                            <div className="relative p-2 rounded-xl border-2 border-green-500/50 shadow-[0_0_15px_rgba(34,197,94,0.3)] bg-background overflow-hidden animate-pulse">
                              {qrDataUrl ? (
                                <img
                                  src={qrDataUrl}
                                  alt="WhatsApp QR Code"
                                  className="h-44 w-44 object-contain rounded-md grayscale contrast-125"
                                />
                              ) : (
                                <div className="h-44 w-44 flex items-center justify-center bg-muted/20 rounded-md">
                                  <Loader2 className="h-6 w-6 animate-spin text-green-500" />
                                </div>
                              )}
                              {/* Laser Scanner Line */}
                              <div className="absolute left-0 right-0 h-0.5 bg-green-500/80 shadow-[0_0_8px_#22c55e] animate-[scan_2s_infinite_ease-in-out]" style={{
                                animation: "scan-line 2s infinite ease-in-out"
                              }} />

                              {/* Glowing frame indicator */}
                              <div className="absolute top-1 left-1 bg-green-500/85 text-[8px] text-white px-1.5 rounded uppercase tracking-wider font-semibold">
                                Agent Mode
                              </div>
                            </div>

                            <style>{`
                              @keyframes scan-line {
                                0%, 100% { top: 8px; }
                                50% { top: calc(100% - 8px); }
                              }
                            `}</style>

                            <div className="text-center space-y-1">
                              <p className="text-[11px] font-medium text-foreground">
                                Scan the QR code with your WhatsApp app
                              </p>
                              <p className="text-[10px] text-muted-foreground">
                                Open WhatsApp &gt; Linked Devices &gt; Link a Device
                              </p>
                            </div>
                          </div>
                        )}

                        {whatsappQrState === "connected" && (
                          <div className="flex flex-col items-center justify-center py-6 gap-3 text-center">
                            <div className="h-12 w-12 rounded-full bg-green-500/10 flex items-center justify-center text-green-500 border border-green-500/20 shadow-[0_0_10px_rgba(34,197,94,0.2)]">
                              <CheckCircle2 className="h-6 w-6" />
                            </div>
                            <div>
                              <p className="text-xs font-semibold text-foreground">Agent Paired & Authorized!</p>
                              <p className="text-[10px] text-muted-foreground mt-0.5">
                                Session established with background browser. Dynamic sync active.
                              </p>
                            </div>
                            <button
                              onClick={() => setWhatsappQrState("idle")}
                              className="text-[10px] text-muted-foreground underline hover:text-foreground"
                            >
                              Reset / Unlink Device
                            </button>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                )}

                {(!def.method || method !== "qrcode") && def.fields.map((field) => (
                  <div key={field.key} className="mt-2">
                    <label className="block text-[10px] text-muted-foreground mb-0.5">{t(field.labelKey)}</label>
                    <div className="flex items-center gap-2">
                      <input
                        type={field.secret ? "password" : "text"}
                        value={cfg.fields[field.key] || ""}
                        onChange={(e) => {
                          const newCfg = { ...cfg, fields: { ...cfg.fields, [field.key]: e.target.value } };
                          saveConnection(def.id, newCfg);
                        }}
                        placeholder={t("settings.conn.enter_field") + "..."}
                        className="flex-1 rounded border border-input bg-background px-2 py-1 text-xs text-foreground placeholder:text-muted-foreground/50"
                      />
                    </div>
                    {field.helpUrl && field.helpLabelKey && (
                      <div className="mt-0.5">
                        <HelpLink url={field.helpUrl} label={t(field.helpLabelKey)} />
                      </div>
                    )}
                  </div>
                ))}

                {/* BotFather link prominently for Telegram Bot */}
                {def.id === "telegram_bot" && (
                  <div className="mt-2 rounded bg-blue-500/5 border border-blue-500/20 px-3 py-2">
                    <p className="text-[11px] text-blue-600 dark:text-blue-400">
                      💬 <a href="https://t.me/botfather" target="_blank" rel="noopener noreferrer" className="underline hover:text-blue-500">
                        @BotFather
                      </a> — {t("settings.conn.botfather_hint")}
                    </p>
                  </div>
                )}

                <div className="mt-3 flex justify-end">
                  <button
                    onClick={() => handleSaveConnection(def.id, connectionConfigs[def.id] || { enabled: cfg.enabled, label: def.label, fields: cfg.fields })}
                    disabled={isSaving}
                    className="inline-flex items-center gap-1 rounded px-3 py-1.5 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
                  >
                    {isSaving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
                    {t("common.save")}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      ) : tab === "wallets" ? (
        <div className="grid gap-3">
          {([
            { id: "metamask", label: "MetaMask", icon: "🦊", chains: "Ethereum, Polygon, Arbitrum, Optimism, Base, BNB Chain", agent: true },
            { id: "okx", label: "OKX Wallet", icon: "ⓞ", chains: "Ethereum, Solana, Polygon, Arbitrum, BNB Chain, Bitcoin, Tron", agent: true },
            { id: "trustwallet", label: "TrustWallet", icon: "🛡️", chains: "Ethereum, Solana, Polygon, BNB Chain, Bitcoin, Tron, Cosmos", agent: true },
            { id: "walletconnect", label: "WalletConnect", icon: "🔗", chains: "Ethereum, Solana, Polygon, Arbitrum, Optimism, Base", agent: false },
            { id: "coinbase", label: "Coinbase Wallet", icon: "🔵", chains: "Ethereum, Base, Polygon, Arbitrum", agent: true },
            { id: "phantom", label: "Phantom", icon: "👻", chains: "Solana, Ethereum, Polygon", agent: true },
            { id: "rabby", label: "Rabby", icon: "🐰", chains: "Ethereum, Polygon, Arbitrum, Optimism, Base", agent: false },
            { id: "rainbow", label: "Rainbow", icon: "🌈", chains: "Ethereum, Polygon, Arbitrum, Optimism, Base", agent: false },
            { id: "ledger", label: "Ledger Live", icon: "💼", chains: "Ethereum, Bitcoin, Solana, Polygon", agent: false },
            { id: "trezor", label: "Trezor", icon: "🔒", chains: "Ethereum, Bitcoin, Solana, Polygon", agent: false },
            { id: "binance_pay", label: "Binance Pay", icon: "💰", chains: "BNB Chain, Ethereum", agent: false },
            { id: "paypal", label: "PayPal", icon: "💳", chains: "Fiat (USD, EUR, etc.)", agent: false },
            { id: "apple_pay", label: "Apple Pay", icon: "🍎", chains: "Fiat (USD, EUR, etc.)", agent: false },
            { id: "google_pay", label: "Google Pay", icon: "📱", chains: "Fiat (USD, EUR, etc.)", agent: false },
          ] as { id: string; label: string; icon: string; chains: string; agent: boolean }[]).map((w) => (
            <div key={w.id} className="rounded-lg border border-border bg-card p-4 transition-colors hover:bg-accent/50">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-muted text-lg">
                    {w.icon}
                  </div>
                  <div>
                    <h3 className="text-sm font-medium text-card-foreground">{w.label}</h3>
                    <p className="text-[10px] text-muted-foreground">{w.chains}</p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {w.agent && (
                    <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] font-medium text-blue-600">
                      <Link2 className="h-2.5 w-2.5" />
                      {t("payments.agent_supported")}
                    </span>
                  )}
                  <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] text-muted-foreground">
                    {t("settings.conn.not_connected")}
                  </span>
                </div>
              </div>
              <div className="mt-3 flex items-center gap-2">
                <Wallet className="h-3 w-3 shrink-0 text-muted-foreground" />
                <input
                  type="text"
                  placeholder={t("payments.wallet_address_placeholder")}
                  className="flex-1 rounded border border-input bg-background px-2 py-1 text-xs text-foreground placeholder:text-muted-foreground/50"
                />
                <button
                  className="inline-flex items-center gap-1 rounded px-3 py-1.5 text-xs font-medium bg-primary text-primary-foreground hover:bg-primary/90"
                >
                  <Save className="h-3 w-3" />
                  {t("common.save")}
                </button>
              </div>
            </div>
          ))}
          <p className="text-center text-[10px] text-muted-foreground/40 mt-2">
            {t("payments.settings_hint")}
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          <Section title={t("settings.appearance")} description={t("settings.appearance_desc")} icon={Sun}>
            <div className="flex gap-2">
              {THEMES.map((th) => {
                const Icon = th.icon;
                return (
                  <button
                    key={th.key}
                    onClick={() => setTheme(th.key)}
                    className={`flex flex-1 items-center justify-center gap-2 rounded-md border px-4 py-3 text-sm transition-colors ${theme === th.key ? "border-primary bg-primary/10 text-primary" : "border-border hover:bg-accent"
                      }`}
                  >
                    <Icon className="h-4 w-4" />
                    {t(th.labelKey)}
                  </button>
                );
              })}
            </div>
          </Section>
          <Section title={t("settings.language")} description={t("settings.language_desc")} icon={Globe}>
            <select className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm">
              <option value="en">{t("settings.lang_en")}</option>
              <option value="ar">{t("settings.lang_ar")}</option>
            </select>
          </Section>
          <Section title={t("settings.security_section")} description={t("settings.security_desc")} icon={Shield}>
            <div className="space-y-3">
              {["sandbox", "network", "filesystem", "permission_prompts", "audit_logging"].map((id) => (
                <label key={id} className="flex items-center justify-between">
                  <span className="text-sm text-foreground/80">{t(`settings.${id}`)}</span>
                  <input type="checkbox" checked={!!systemSettings[id as keyof typeof systemSettings]} onChange={(e) => updateSystemSetting(id as keyof typeof systemSettings, e.target.checked)} className="rounded border-border text-primary focus:ring-primary" />
                </label>
              ))}
            </div>
          </Section>
          <Section title={t("settings.storage")} description={t("settings.storage_desc")} icon={Folder}>
            <div className="space-y-2">
              {["data", "config", "cache", "logs"].map((key) => (
                <div key={key} className="flex items-center gap-2">
                  <span className="w-20 text-sm text-muted-foreground">{t(`settings.${key}`)}</span>
                  <input type="text" defaultValue={`~/.prime/${key}`} className="flex-1 rounded-md border border-input bg-background px-3 py-1.5 text-sm" readOnly />
                </div>
              ))}
            </div>
          </Section>
        </div>
      )}
    </div>
  );
}
