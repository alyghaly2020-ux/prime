import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Shield, ShieldCheck, ShieldAlert, Lock, Eye, FileSearch, Wifi, HardDrive, Terminal, AlertTriangle, CheckCircle2, XCircle, Loader2, AlertCircle } from "lucide-react";

interface SecurityFeature {
  id: string;
  label: string;
  description: string;
  icon: typeof Shield;
  enabled: boolean;
  severity: "high" | "medium" | "low";
}

interface AuditEntryIPC {
  id: number;
  timestamp: string;
  action: string;
  subject: string;
  resource: string;
  result: "Allow" | "Deny";
  reason: string | null;
}

export function SecurityDashboard() {
  const { t } = useTranslation();
  const FEATURE_DEFS: (Omit<SecurityFeature, "enabled">)[] = [
    { id: "sandbox", label: t("security.feature.sandbox"), description: t("security.feature.sandbox_desc"), icon: Terminal, severity: "high" },
    { id: "encryption", label: t("security.feature.encryption"), description: t("security.feature.encryption_desc"), icon: Lock, severity: "high" },
    { id: "permissions", label: t("security.feature.permissions"), description: t("security.feature.permissions_desc"), icon: Eye, severity: "high" },
    { id: "audit", label: t("security.feature.audit"), description: t("security.feature.audit_desc"), icon: FileSearch, severity: "medium" },
    { id: "network", label: t("security.feature.network"), description: t("security.feature.network_desc"), icon: Wifi, severity: "medium" },
    { id: "filesystem", label: t("security.feature.filesystem"), description: t("security.feature.filesystem_desc"), icon: HardDrive, severity: "medium" },
    { id: "telemetry", label: t("security.feature.telemetry"), description: t("security.feature.telemetry_desc"), icon: AlertTriangle, severity: "low" },
  ];

  const [features, setFeatures] = useState<SecurityFeature[]>(
    FEATURE_DEFS.map((f) => ({ ...f, enabled: true }))
  );

  const { data: auditData, isLoading, error } = useQuery({
    queryKey: ["audit_query"],
    queryFn: () => invoke<AuditEntryIPC[]>("audit_query", { limit: 10 }),
    refetchInterval: 15000,
  });

  useQuery({
    queryKey: ["security_get_policy"],
    queryFn: async () => {
      const policy = await invoke<{
        sandbox_enabled: boolean;
        encryption_at_rest: boolean;
        permission_model: string;
      }>("security_get_policy");
      setFeatures(FEATURE_DEFS.map((f) => ({
        ...f,
        enabled: f.id === "sandbox" ? policy.sandbox_enabled
          : f.id === "encryption" ? policy.encryption_at_rest
          : f.id === "permissions" ? policy.permission_model !== "Permissive"
          : f.id === "network" ? policy.permission_model === "Permissive"
          : f.id === "filesystem" ? policy.permission_model === "Permissive"
          : true,
      })));
      return policy;
    },
    refetchInterval: 30000,
  });

  const toggleFeature = (id: string) => {
    setFeatures((prev) => prev.map((f) => f.id === id ? { ...f, enabled: !f.enabled } : f));
  };

  const enabledCount = features.filter((f) => f.enabled).length;
  const overallScore = Math.round((enabledCount / features.length) * 100);

  return (
    <div className="h-full overflow-y-auto p-6 space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-bold text-foreground">{t("security.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("security.description")}</p>
        </div>
      </div>

      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
          <p>{t("security.failed", { error: String(error) })}</p>
        </div>
      )}

      {/* Score card */}
      <div className="rounded-xl border border-border bg-card p-5">
        <div className="flex items-center gap-4">
          <div className={`rounded-full p-3 ${overallScore >= 70 ? "bg-green-500/10" : overallScore >= 40 ? "bg-yellow-500/10" : "bg-red-500/10"}`}>
            {overallScore >= 70 ? <ShieldCheck className="h-8 w-8 text-green-500" /> : overallScore >= 40 ? <ShieldAlert className="h-8 w-8 text-yellow-500" /> : <Shield className="h-8 w-8 text-red-500" />}
          </div>
          <div>
            <p className="text-2xl font-bold text-foreground">{t("security.score", { score: overallScore })}</p>
            <p className="text-xs text-muted-foreground">{t("security.score_label")}</p>
          </div>
          <div className="ml-auto flex items-center gap-4 text-xs text-muted-foreground">
            <span className="flex items-center gap-1"><div className="h-2 w-2 rounded-full bg-green-500" /> {features.filter((f) => f.enabled && f.severity === "high").length} {t("security.severity_high")}</span>
            <span className="flex items-center gap-1"><div className="h-2 w-2 rounded-full bg-yellow-500" /> {features.filter((f) => f.enabled && f.severity === "medium").length} {t("security.severity_medium")}</span>
            <span className="flex items-center gap-1"><div className="h-2 w-2 rounded-full bg-blue-500" /> {features.filter((f) => f.enabled && f.severity === "low").length} {t("security.severity_low")}</span>
          </div>
        </div>
        <div className="mt-4 h-2 overflow-hidden rounded-full bg-muted">
          <div className={`h-full rounded-full transition-all duration-500 ${overallScore >= 70 ? "bg-green-500" : overallScore >= 40 ? "bg-yellow-500" : "bg-red-500"}`} style={{ width: `${overallScore}%` }} />
        </div>
      </div>

      {/* Features grid */}
      <div className="grid gap-3 md:grid-cols-2">
        {features.map((feature) => {
          const Icon = feature.icon;
          return (
            <div key={feature.id} className="flex items-start gap-3 rounded-xl border border-border bg-card p-4 transition-colors hover:bg-accent/50">
              <div className={`rounded-lg p-2 ${feature.enabled ? "bg-primary/10" : "bg-muted"}`}>
                <Icon className={`h-4 w-4 ${feature.enabled ? "text-primary" : "text-muted-foreground/50"}`} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between">
                  <p className="text-sm font-medium text-card-foreground">{feature.label}</p>
                  <div className={`flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] ${feature.severity === "high" ? "bg-red-500/10 text-red-500" : feature.severity === "medium" ? "bg-yellow-500/10 text-yellow-500" : "bg-blue-500/10 text-blue-500"}`}>
                    {feature.severity === "high" ? t("security.severity_high") : feature.severity === "medium" ? t("security.severity_medium") : t("security.severity_low")}
                  </div>
                </div>
                <p className="mt-1 text-xs text-muted-foreground">{feature.description}</p>
                <div className="mt-2 flex items-center gap-2">
                  <button
                    onClick={() => toggleFeature(feature.id)}
                    className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors ${feature.enabled ? "bg-primary" : "bg-muted-foreground/30"}`}
                  >
                    <span className={`inline-block h-3.5 w-3.5 rounded-full bg-white transition-all ${feature.enabled ? "translate-x-[18px]" : "translate-x-[2px]"}`} />
                  </button>
                  <span className="text-[10px] text-muted-foreground">{feature.enabled ? t("common.enabled") : t("common.disabled")}</span>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Recent events */}
      <div className="rounded-xl border border-border bg-card p-4">
        <h3 className="mb-3 text-xs font-semibold text-foreground uppercase tracking-wider">
          {t("security.events")} {isLoading && <Loader2 className="inline h-3.5 w-3.5 animate-spin ml-1" />}
        </h3>
        <div className="space-y-2">
          {auditData && auditData.length > 0 ? auditData.slice(0, 4).map((entry) => (
            <div key={entry.id} className="flex items-center gap-3 rounded-md bg-muted/30 px-3 py-2">
              {entry.result === "Deny" ? <XCircle className="h-3.5 w-3.5 text-red-500" /> : <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />}
              <span className="text-[10px] text-muted-foreground shrink-0">{entry.action}</span>
              <span className="text-xs text-foreground truncate">{entry.resource}</span>
              {entry.reason && <span className="text-[10px] text-muted-foreground ml-auto">{entry.reason}</span>}
            </div>
          )) : (
            <div className="flex items-center justify-center py-8 text-xs text-muted-foreground">
              {t("security.events_empty")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
