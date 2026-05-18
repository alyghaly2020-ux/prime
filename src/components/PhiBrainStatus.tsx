import { useTranslation } from "react-i18next";
import { usePhiBrainStore } from "@/stores/usePhiBrainStore";
import { Brain, AlertTriangle, CheckCircle, Loader2, Shield } from "lucide-react";

export function PhiBrainStatus() {
  const { t } = useTranslation();
  const { available, profileMaturity, healthAlerts, enabled, proofreadingEnabled, guardianEnabled } =
    usePhiBrainStore();

  const maturityPercent = Math.round(profileMaturity * 100);
  const maturityLevel =
    maturityPercent < 10
      ? "🌱"
      : maturityPercent < 30
        ? "🌿"
        : maturityPercent < 55
          ? "🌳"
          : maturityPercent < 80
            ? "🌲"
            : "🏆";

  return (
    <div className="rounded-lg border border-border bg-card p-4 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Brain className="h-4 w-4 text-primary" />
          <span className="text-sm font-semibold">{t("phi_brain.title", "Phi Brain")}</span>
        </div>
        <div className="flex items-center gap-1.5">
          {available ? (
            <CheckCircle className="h-3.5 w-3.5 text-green-500" />
          ) : (
            <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
          )}
          <span className="text-xs text-muted-foreground">
            {available ? t("phi_brain.online", "Online") : t("phi_brain.offline", "Offline")}
          </span>
        </div>
      </div>

      {/* Status indicators */}
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="flex items-center gap-1.5">
          <Shield className="h-3 w-3 text-blue-500" />
          <span className="text-muted-foreground">
            {proofreadingEnabled ? t("phi_brain.proofread_on", "Proofread ON") : t("phi_brain.proofread_off", "Proofread OFF")}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <AlertTriangle className="h-3 w-3 text-amber-500" />
          <span className="text-muted-foreground">
            {guardianEnabled ? t("phi_brain.guardian_on", "Guardian ON") : t("phi_brain.guardian_off", "Guardian OFF")}
          </span>
        </div>
      </div>

      {/* Maturity */}
      <div className="space-y-1">
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">
            {t("phi_brain.maturity", "Learning Progress")}
          </span>
          <span className="font-medium">
            {maturityLevel} {maturityPercent}%
          </span>
        </div>
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <div
            className="h-full rounded-full bg-gradient-to-r from-primary to-primary/60 transition-all duration-500"
            style={{ width: `${maturityPercent}%` }}
          />
        </div>
      </div>

      {/* Recent alerts */}
      {healthAlerts.length > 0 && (
        <div className="space-y-1">
          <span className="text-xs font-medium text-muted-foreground">
            {t("phi_brain.recent_alerts", "Recent Alerts")}
          </span>
          {healthAlerts.slice(0, 3).map((alert, i) => (
            <div
              key={i}
              className={`rounded px-2 py-1 text-xs ${
                alert.level === "emergency"
                  ? "bg-red-500/10 text-red-400 border border-red-500/20"
                  : alert.level === "critical"
                    ? "bg-orange-500/10 text-orange-400 border border-orange-500/20"
                    : "bg-yellow-500/10 text-yellow-400 border border-yellow-500/20"
              }`}
            >
              <span className="font-medium">{alert.message}</span>
              <p className="mt-0.5 text-muted-foreground">{alert.advice}</p>
            </div>
          ))}
        </div>
      )}

      {/* Status message */}
      {!enabled && (
        <div className="rounded bg-muted/50 px-2 py-1.5 text-xs text-muted-foreground">
          {t("phi_brain.disabled", "Phi Brain is disabled. Enable it for smart routing and proofreading.")}
        </div>
      )}
    </div>
  );
}
