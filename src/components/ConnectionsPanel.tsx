import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  WifiOff,
  Loader2,
  MessageCircle,
  Phone,
  Mail,
  Bot,
  Plug,
  Unplug,
  CheckCircle2,
  Settings,
} from "lucide-react";

interface ConnectionInfo {
  id: string;
  label: string;
  icon: typeof MessageCircle;
  fields: { key: string; label: string; secret: boolean }[];
}

const CONNECTION_DEFS: ConnectionInfo[] = [
  { id: "telegram", label: "Telegram", icon: MessageCircle, fields: [{ key: "api_id", label: "API ID", secret: false }, { key: "api_hash", label: "API Hash", secret: true }] },
  { id: "telegram_bot", label: "Telegram Bot", icon: Bot, fields: [{ key: "bot_token", label: "Bot Token", secret: true }] },
  { id: "whatsapp", label: "WhatsApp", icon: Phone, fields: [{ key: "access_token", label: "Meta Access Token", secret: true }, { key: "phone_number_id", label: "Phone Number ID", secret: false }] },
  { id: "discord", label: "Discord", icon: Bot, fields: [{ key: "bot_token", label: "Bot Token", secret: true }] },
  { id: "slack", label: "Slack", icon: Bot, fields: [{ key: "bot_token", label: "Bot Token", secret: true }, { key: "signing_secret", label: "Signing Secret", secret: true }] },
  { id: "email", label: "Email (SMTP)", icon: Mail, fields: [{ key: "smtp_host", label: "SMTP Host", secret: false }, { key: "smtp_port", label: "Port", secret: false }, { key: "username", label: "Username", secret: false }, { key: "password", label: "Password", secret: true }] },
  { id: "wechat", label: "WeChat (WeCom Bot)", icon: MessageCircle, fields: [{ key: "webhook_url", label: "Webhook URL", secret: true }] },
  { id: "signal", label: "Signal", icon: MessageCircle, fields: [{ key: "phone_number", label: "Phone Number", secret: false }] },
  { id: "matrix", label: "Matrix", icon: MessageCircle, fields: [{ key: "homeserver", label: "Homeserver URL", secret: false }, { key: "access_token", label: "Access Token", secret: true }] },
  { id: "irc", label: "IRC", icon: MessageCircle, fields: [{ key: "server", label: "Server", secret: false }, { key: "port", label: "Port", secret: false }, { key: "nick", label: "Nickname", secret: false }] },
];

export function ConnectionsPanel() {
  const { t } = useTranslation();
  const [connections, setConnections] = useState<Record<string, { enabled: boolean; fields: Record<string, string> }>>({});
  const [loading, setLoading] = useState(true);
  const [connecting, setConnecting] = useState<Record<string, boolean>>({});

  const loadConfig = useCallback(async () => {
    try {
      setLoading(true);
      const raw = await invoke<string>("get_config");
      const cfg = JSON.parse(raw);
      setConnections(cfg.connection_configs || {});
    } catch (e) {
      console.error("Failed to load connections:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadConfig(); }, [loadConfig]);

  const configuredDefs = CONNECTION_DEFS.filter((def) => {
    const cfg = connections[def.id];
    return cfg && (cfg.enabled || def.fields.some((f) => cfg.fields[f.key]));
  });

  const connectedCount = configuredDefs.filter((def) => connections[def.id]?.enabled).length;

  const toggleConnection = useCallback(async (id: string) => {
    setConnecting((c) => ({ ...c, [id]: true }));
    try {
      const prev = connections[id] || { enabled: false, fields: {} };
      const updated = { ...prev, enabled: !prev.enabled };
      await invoke("save_connection_config", { id, configJson: JSON.stringify(updated) });
      setConnections((c) => ({ ...c, [id]: updated }));
    } catch (e) {
      console.error("Failed to toggle connection:", e);
    } finally {
      setConnecting((c) => ({ ...c, [id]: false }));
    }
  }, [connections]);

  const connectAll = useCallback(async () => {
    const updated = { ...connections };
    for (const def of configuredDefs) {
      updated[def.id] = { ...(updated[def.id] || { fields: {} }), enabled: true };
    }
    for (const [id, cfg] of Object.entries(updated)) {
      try {
        await invoke("save_connection_config", { id, configJson: JSON.stringify(cfg) });
      } catch { /* ignore individual failures */ }
    }
    setConnections(updated);
  }, [connections, configuredDefs]);

  const disconnectAll = useCallback(async () => {
    const updated = { ...connections };
    for (const id of Object.keys(updated)) {
      updated[id] = { ...updated[id], enabled: false };
      try {
        await invoke("save_connection_config", { id, configJson: JSON.stringify(updated[id]) });
      } catch { /* ignore */ }
    }
    setConnections(updated);
  }, [connections]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("connections.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {configuredDefs.length > 0
              ? t("connections.count", { connected: connectedCount, total: configuredDefs.length })
              : t("connections.empty")}
          </p>
        </div>
        {configuredDefs.length > 0 && (
          <div className="flex items-center gap-2">
            <button
              onClick={connectAll}
              disabled={connectedCount === configuredDefs.length}
              className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              <Plug className="h-3.5 w-3.5" />
              {t("connections.connect_all")}
            </button>
            <button
              onClick={disconnectAll}
              disabled={connectedCount === 0}
              className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-xs text-foreground hover:bg-accent disabled:opacity-50"
            >
              <Unplug className="h-3.5 w-3.5" />
              {t("connections.disconnect_all")}
            </button>
          </div>
        )}
      </div>

      {configuredDefs.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
          <Settings className="mb-3 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm font-medium">{t("connections.empty")}</p>
          <p className="text-xs mt-1">{t("connections.empty_hint")}</p>
        </div>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {configuredDefs.map((def) => {
            const cfg = connections[def.id] || { enabled: false, fields: {} };
            const Icon = def.icon;
            const isConnecting = connecting[def.id];
            return (
              <div
                key={def.id}
                className={`rounded-lg border p-4 transition-all ${
                  cfg.enabled
                    ? "border-green-500/30 bg-green-500/5"
                    : "border-border bg-card"
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-start gap-3 min-w-0 flex-1">
                    <div className={`mt-1 flex h-9 w-9 items-center justify-center rounded-lg ${
                      cfg.enabled ? "bg-green-500/10" : "bg-muted"
                    }`}>
                      <Icon className={`h-4 w-4 ${cfg.enabled ? "text-green-500" : "text-muted-foreground"}`} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <h3 className="font-medium text-card-foreground">{def.label}</h3>
                        {cfg.enabled ? (
                          <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-[10px] font-medium text-green-600">
                            <CheckCircle2 className="h-2.5 w-2.5" />
                            {t("app.status.connected")}
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
                            <WifiOff className="h-2.5 w-2.5" />
                            {t("app.status.disconnected")}
                          </span>
                        )}
                      </div>
                      {def.fields.map((f) => cfg.fields[f.key] && (
                        <p key={f.key} className="mt-0.5 text-xs text-muted-foreground">
                          {f.label}: {f.secret ? "••••••••" : cfg.fields[f.key]}
                        </p>
                      ))}
                    </div>
                  </div>
                  <button
                    onClick={() => toggleConnection(def.id)}
                    disabled={isConnecting}
                    className={`ml-3 shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                      cfg.enabled
                        ? "border border-destructive/30 text-destructive hover:bg-destructive/10"
                        : "bg-primary text-primary-foreground hover:bg-primary/90"
                    } disabled:opacity-50`}
                  >
                    {isConnecting ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : cfg.enabled ? (
                      t("connections.disconnect")
                    ) : (
                      t("connections.connect")
                    )}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
