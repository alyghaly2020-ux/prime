import { useTranslation } from "react-i18next";
import { useUpdateStore } from "@/stores/useUpdateStore";
import { Download, X, Loader2, RefreshCw } from "lucide-react";

export function UpdateBanner() {
  const { t } = useTranslation();
  const { status, info, check, install, dismiss } = useUpdateStore();

  if (status !== "available") return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 max-w-sm animate-in slide-in-from-right-4 fade-in">
      <div className="rounded-xl border border-primary/30 bg-gradient-to-br from-primary/10 via-card to-card p-4 shadow-2xl backdrop-blur-sm">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-start gap-2.5">
            <div className="mt-0.5 flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
              <Download className="h-4 w-4 text-primary" />
            </div>
            <div>
              <p className="text-sm font-medium text-foreground">
                {t("update.available", { version: info?.version ?? "" })}
              </p>
              <p className="mt-0.5 text-xs text-muted-foreground">
                {t("update.available_desc")}
              </p>
            </div>
          </div>
          <button
            onClick={dismiss}
            className="shrink-0 rounded-lg p-1 text-muted-foreground hover:text-foreground hover:bg-accent transition-all"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
        <div className="mt-3 flex items-center gap-2">
          <button
            onClick={install}
            className="inline-flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-all"
          >
            <Download className="h-3.5 w-3.5" />
            {t("update.install")}
          </button>
          <button
            onClick={() => check()}
            className="inline-flex items-center gap-1.5 rounded-lg border border-border px-3 py-1.5 text-xs text-foreground hover:bg-accent transition-all"
          >
            {t("update.later")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function UpdateChecker() {
  const { status, check } = useUpdateStore();

  return (
    <button
      onClick={() => check()}
      disabled={status === "checking"}
      className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] text-muted-foreground hover:text-foreground hover:bg-accent transition-all disabled:opacity-50"
      title="Check for updates"
    >
      <RefreshCw className={`h-3 w-3 ${status === "checking" ? "animate-spin" : ""}`} />
      {status === "checking" ? (
        <Loader2 className="h-2.5 w-2.5 animate-spin" />
      ) : status === "uptodate" ? (
        <span className="text-green-500">&#9679;</span>
      ) : status === "error" ? (
        <span className="text-destructive">&#9679;</span>
      ) : null}
    </button>
  );
}
