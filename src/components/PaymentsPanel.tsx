import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  usePaymentStore,
  type PaymentMethodSummary,
  type WalletPlatform,
  type ConnectMethod,
  PLATFORM_META,
  CONNECT_METHOD_LABELS,
  CONNECT_METHOD_SIMPLES,
} from "@/stores/usePaymentStore";
import {
  Sparkles,
  Zap,
  Loader2,
  Plug,
  Unplug,
  CheckCircle2,
  Settings,
  Link2,
  QrCode,
  Smartphone,
  Key,
  Globe,
  Cable,
  MousePointerClick,
} from "lucide-react";

const METHOD_ICONS: Record<ConnectMethod, React.ReactNode> = {
  extension: <MousePointerClick className="h-3.5 w-3.5" />,
  qr_code: <QrCode className="h-3.5 w-3.5" />,
  api_key: <Key className="h-3.5 w-3.5" />,
  oauth: <Globe className="h-3.5 w-3.5" />,
  usb: <Cable className="h-3.5 w-3.5" />,
  manual: <Smartphone className="h-3.5 w-3.5" />,
};

function PlatformIcon({ platform, className }: { platform: WalletPlatform; className?: string }) {
  const meta = PLATFORM_META[platform];
  return <span className={className}>{meta?.icon || "💳"}</span>;
}

function QrModal({ data, onClose }: { data: string; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="rounded-xl bg-card p-6 shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <div className="mb-4 text-center">
          <p className="text-sm font-medium text-foreground">{t("payments.scan_qr")}</p>
          <p className="text-xs text-muted-foreground mt-1">{t("payments.scan_qr")}</p>
        </div>
        <div className="mx-auto flex h-48 w-48 items-center justify-center rounded-lg bg-white p-4">
          <div className="text-center">
            <QrCode className="mx-auto h-32 w-32 text-black" />
            <p className="mt-2 text-[10px] text-gray-500 break-all">{data.slice(0, 40)}...</p>
          </div>
        </div>
        <button
          onClick={onClose}
          className="mt-4 w-full rounded-lg bg-primary py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90"
        >
          {t("common.close")}
        </button>
      </div>
    </div>
  );
}

export function PaymentsPanel() {
  const { t } = useTranslation();
  const { mode, methods, setMode, setMethods, setActiveMethodId } = usePaymentStore();
  const [loading, setLoading] = useState(true);
  const [connecting, setConnecting] = useState<Record<string, boolean>>({});
  const [qrData, setQrData] = useState<string | null>(null);

  const loadMethods = useCallback(async () => {
    try {
      setLoading(true);
      const result = await invoke<PaymentMethodSummary[]>("list_payment_methods");
      setMethods(result);
      const active = result.find((m) => m.is_active);
      if (active) setActiveMethodId(active.id);
    } catch (e) {
      console.error("Failed to load payment methods:", e);
    } finally {
      setLoading(false);
    }
  }, [setMethods, setActiveMethodId]);

  useEffect(() => { loadMethods(); }, [loadMethods]);

  const connectedCount = methods.filter((m) => m.is_active).length;

  const toggleMethod = useCallback(async (id: string) => {
    setConnecting((c) => ({ ...c, [id]: true }));
    try {
      const method = methods.find((m) => m.id === id);
      if (method?.is_active) {
        await invoke("disconnect_wallet", { id });
      } else {
        await invoke("set_active_payment_method", { id });
        if (method?.connection_method === "qr_code") {
          setQrData(method.connection_data || `wc:${method.platform}:${id}`);
        }
        await invoke("connect_wallet", { id });
      }
      await loadMethods();
    } catch (e) {
      console.error("Failed to toggle payment method:", e);
    } finally {
      setConnecting((c) => ({ ...c, [id]: false }));
    }
  }, [methods, loadMethods]);

  const togglePaymentMode = useCallback(async () => {
    const result = await invoke<string>("toggle_payment_mode");
    const newMode = JSON.parse(result);
    setMode(newMode === "auto" ? "auto" : "manual");
  }, [setMode]);

  const connectAll = useCallback(async () => {
    for (const method of methods) {
      try {
        await invoke("set_active_payment_method", { id: method.id });
        await invoke("connect_wallet", { id: method.id });
      } catch { /* ignore */ }
    }
    await loadMethods();
  }, [methods, loadMethods]);

  const disconnectAll = useCallback(async () => {
    for (const method of methods) {
      try {
        await invoke("disconnect_wallet", { id: method.id });
      } catch { /* ignore */ }
    }
    await loadMethods();
  }, [methods, loadMethods]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      {qrData && <QrModal data={qrData} onClose={() => setQrData(null)} />}

      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("payments.title")}</h1>
          <p className="text-sm text-muted-foreground">
            {methods.length > 0
              ? t("payments.count", { connected: connectedCount, total: methods.length })
              : t("payments.no_methods")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1 rounded-lg border border-border bg-muted/40 p-0.5">
            <button
              onClick={() => togglePaymentMode()}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${mode === "auto" ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
            >
              <Sparkles className="mr-1 inline h-3 w-3" />
              {t("payments.mode_auto")}
            </button>
            <button
              onClick={() => togglePaymentMode()}
              className={`rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${mode === "manual" ? "bg-card text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`}
            >
              {t("payments.mode_manual")}
            </button>
          </div>
          {methods.length > 0 && (
            <>
              <button
                onClick={connectAll}
                disabled={connectedCount === methods.length}
                className="inline-flex items-center gap-1 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
              >
                <Plug className="h-3.5 w-3.5" />
                {t("payments.connect_all")}
              </button>
              <button
                onClick={disconnectAll}
                disabled={connectedCount === 0}
                className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-3 py-1.5 text-xs text-foreground hover:bg-accent disabled:opacity-50"
              >
                <Unplug className="h-3.5 w-3.5" />
                {t("payments.disconnect_all")}
              </button>
            </>
          )}
        </div>
      </div>

      {mode === "auto" && (
        <div className="rounded-lg border border-primary/20 bg-primary/5 p-3">
          <div className="flex items-start gap-3">
            <Zap className="mt-0.5 h-5 w-5 text-primary shrink-0" />
            <div>
              <p className="text-sm font-medium text-foreground">{t("payments.smart_title")}</p>
              <p className="text-xs text-muted-foreground mt-0.5">{t("payments.smart_desc")}</p>
            </div>
          </div>
        </div>
      )}

      {methods.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-center text-muted-foreground">
          <Settings className="mb-3 h-12 w-12 text-muted-foreground/30" />
          <p className="text-sm font-medium">{t("payments.no_methods")}</p>
          <p className="text-xs mt-1">{t("payments.no_methods_hint")}</p>
        </div>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {methods.map((method) => {
            const isConnecting = connecting[method.id];
            const meta = PLATFORM_META[method.platform];
            const isSimple = CONNECT_METHOD_SIMPLES.includes(method.connection_method);
            const methodLabel = CONNECT_METHOD_LABELS[method.connection_method];
            return (
              <div
                key={method.id}
                className={`rounded-lg border p-4 transition-all ${
                  method.is_active
                    ? "border-green-500/30 bg-green-500/5"
                    : "border-border bg-card"
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-start gap-3 min-w-0 flex-1">
                    <div className={`mt-1 flex h-9 w-9 items-center justify-center rounded-lg ${
                      method.is_active ? "bg-green-500/10" : "bg-muted"
                    }`}>
                      <PlatformIcon platform={method.platform} className="text-lg" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h3 className="font-medium text-card-foreground">{method.label}</h3>
                        {method.is_active ? (
                          <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-[10px] font-medium text-green-600">
                            <CheckCircle2 className="h-2.5 w-2.5" />
                            {t("app.status.active")}
                          </span>
                        ) : (
                          <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">
                            {t("app.status.inactive")}
                          </span>
                        )}
                        {method.agent_controlled && (
                          <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] font-medium text-blue-600">
                            <Link2 className="h-2.5 w-2.5" />
                            {t("payments.agent_controlled")}
                          </span>
                        )}
                      </div>
                      <div className="mt-1 flex flex-wrap gap-1 items-center">
                        <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                          {method.chain}
                        </span>
                        {isSimple && (
                          <span className="inline-flex items-center gap-1 rounded bg-green-500/10 px-1.5 py-0.5 text-[10px] font-medium text-green-600">
                            {METHOD_ICONS[method.connection_method]}
                            {methodLabel}
                          </span>
                        )}
                        {!isSimple && (
                          <span className="inline-flex items-center gap-1 rounded bg-amber-500/10 px-1.5 py-0.5 text-[10px] text-amber-600">
                            {METHOD_ICONS[method.connection_method]}
                            {methodLabel}
                          </span>
                        )}
                        {meta && (
                          <span className="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                            {meta.label}
                          </span>
                        )}
                      </div>
                      <p className="mt-1 text-sm font-semibold text-foreground">{method.balance}</p>
                    </div>
                  </div>
                  <button
                    onClick={() => {
                      if (!method.is_active && method.connection_method === "qr_code") {
                        setQrData(method.connection_data || `wc:${method.platform}:${method.id}`);
                      }
                      toggleMethod(method.id);
                    }}
                    disabled={isConnecting}
                    className={`ml-3 shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                      method.is_active
                        ? "border border-destructive/30 text-destructive hover:bg-destructive/10"
                        : "bg-primary text-primary-foreground hover:bg-primary/90"
                    } disabled:opacity-50`}
                  >
                    {isConnecting ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : method.is_active ? (
                      t("payments.deactivate")
                    ) : isSimple ? (
                      <span className="inline-flex items-center gap-1">
                        {METHOD_ICONS[method.connection_method]}
                        {method.connection_method === "qr_code" ? t("payments.scan_qr") : t("payments.quick_connect")}
                      </span>
                    ) : (
                      t("payments.connect")
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
