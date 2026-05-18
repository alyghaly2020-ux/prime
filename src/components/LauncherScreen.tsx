import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore, type LauncherStage } from "@/stores/useAppStore";
import { Cpu, Code2, Globe, Wrench, Puzzle, Loader2, Rocket, CheckCircle2 } from "lucide-react";

const STAGE_ORDER: LauncherStage[] = ["chat", "tools", "mcp", "editor", "skills"];

function ParticleField() {
  return (
    <div className="absolute inset-0 overflow-hidden pointer-events-none">
      {Array.from({ length: 30 }).map((_, i) => (
        <div
          key={i}
          className="absolute h-1 w-1 rounded-full bg-primary/20"
          style={{
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
            animation: `pulse-drift ${3 + Math.random() * 4}s ease-in-out infinite`,
            animationDelay: `${Math.random() * 3}s`,
            opacity: 0.3 + Math.random() * 0.4,
          }}
        />
      ))}
    </div>
  );
}

export function LauncherScreen() {
  const { t } = useTranslation();
  const { launcherStages, setLauncherStage, setLauncherCompleted } = useAppStore();

  const STAGES: { id: LauncherStage; label: string; icon: typeof Cpu; description: string }[] = [
    { id: "chat", label: t("launcher.feature.chat"), icon: Globe, description: t("launcher.feature.chat_desc") },
    { id: "tools", label: t("launcher.feature.tools"), icon: Wrench, description: t("launcher.feature.tools_desc") },
    { id: "mcp", label: t("launcher.feature.mcp"), icon: Code2, description: t("launcher.feature.mcp_desc") },
    { id: "editor", label: t("launcher.feature.editor"), icon: Cpu, description: t("launcher.feature.editor_desc") },
    { id: "skills", label: t("launcher.feature.wasm"), icon: Puzzle, description: t("launcher.feature.wasm_desc") },
  ];

  const [progress, setProgress] = useState(0);
  const [stageIndex, setStageIndex] = useState(0);
  const [launchReady, setLaunchReady] = useState(false);
  const [launching, setLaunching] = useState(false);

  // Simulate stage progression
  useEffect(() => {
    if (launchReady) return;

    const currentStage = STAGE_ORDER[stageIndex];
    if (!currentStage || stageIndex >= STAGE_ORDER.length) {
      setLaunchReady(true);
      return;
    }

    setLauncherStage(currentStage, "downloading");

    let p = 0;
    const interval = setInterval(() => {
      p += 2 + Math.random() * 4;
      if (p >= 100) {
        p = 100;
        clearInterval(interval);
        setLauncherStage(currentStage, "ready");
        setProgress(0);
        setTimeout(() => {
          setStageIndex((i) => i + 1);
        }, 300);
      }
      setProgress(p);
    }, 80);

    return () => clearInterval(interval);
  }, [stageIndex, launchReady, setLauncherStage]);

  // All stages ready
  useEffect(() => {
    if (stageIndex >= STAGE_ORDER.length) {
      setLaunchReady(true);
    }
  }, [stageIndex]);

  const handleLaunch = () => {
    setLaunching(true);
    setTimeout(() => {
      setLauncherCompleted(true);
    }, 600);
  };

  const totalProgress = STAGE_ORDER.reduce((acc, s) => {
    const status = launcherStages[s];
    if (status === "ready") return acc + 20;
    if (status === "downloading" && s === STAGE_ORDER[stageIndex]) {
      return acc + (progress / 100) * 20;
    }
    return acc;
  }, 0);

  return (
    <div className="relative flex h-screen flex-col items-center justify-center bg-gradient-to-b from-background via-background to-background/95 overflow-hidden">
      <ParticleField />

      {/* Ambient glow */}
      <div className="absolute top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 h-96 w-96 rounded-full bg-primary/5 blur-3xl" />

      {/* Logo */}
      <div className="relative z-10 mb-12 flex flex-col items-center">
        <div className="mb-4 flex h-24 w-24 items-center justify-center">
          <img src="/prime.png" alt={t("launcher.title")} className="h-full w-full object-contain drop-shadow-2xl" />
        </div>
        <h1 className="text-4xl font-bold tracking-tight text-foreground">{t("launcher.title")}</h1>
        <p className="mt-2 text-sm text-muted-foreground">{t("launcher.subtitle")}</p>
      </div>

      {/* Stage list */}
      <div className="relative z-10 w-full max-w-md space-y-3">
        {STAGES.map((stage, i) => {
          const status = launcherStages[stage.id];
          const isActive = stageIndex === i && status === "downloading";
          const isReady = status === "ready";

          return (
            <div
              key={stage.id}
              className={`flex items-center gap-4 rounded-xl border px-4 py-3 transition-all duration-500 ${
                isActive
                  ? "border-primary/40 bg-primary/5 shadow-lg shadow-primary/5"
                  : isReady
                    ? "border-border/60 bg-card/50"
                    : "border-border/30 bg-card/30 opacity-40"
              }`}
            >
              {/* Icon */}
              <div
                className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg transition-colors ${
                  isReady
                    ? "bg-primary/10 text-primary"
                    : isActive
                      ? "bg-primary/10 text-primary"
                      : "bg-muted text-muted-foreground"
                }`}
              >
                {isReady ? (
                  <CheckCircle2 className="h-5 w-5" />
                ) : (
                  <stage.icon className="h-5 w-5" />
                )}
              </div>

              {/* Info */}
              <div className="flex-1 min-w-0">
                <p
                  className={`text-sm font-medium ${
                    isReady ? "text-foreground" : isActive ? "text-foreground" : "text-muted-foreground"
                  }`}
                >
                  {stage.label}
                </p>
                <p className="text-xs text-muted-foreground truncate">{stage.description}</p>
              </div>

              {/* Status */}
              <div className="shrink-0">
                {isActive && (
                  <div className="flex items-center gap-2">
                    <Loader2 className="h-4 w-4 animate-spin text-primary" />
                    <span className="text-xs font-medium text-primary">{Math.round(progress)}%</span>
                  </div>
                )}
                {isReady && (
                  <span className="text-xs font-medium text-green-500">{t("app.status.online")}</span>
                )}
                {status === "pending" && (
                  <span className="text-xs text-muted-foreground">Waiting</span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Progress bar */}
      <div className="relative z-10 mt-8 w-full max-w-md">
        <div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <div
            className="h-full rounded-full bg-gradient-to-r from-primary to-primary/60 transition-all duration-300 ease-out"
            style={{ width: `${totalProgress}%` }}
          />
        </div>
        <p className="mt-2 text-center text-xs text-muted-foreground">
          {launchReady ? "All systems ready" : `Initializing... ${Math.round(totalProgress)}%`}
        </p>
      </div>

      {/* Launch button */}
      <div className="relative z-10 mt-8">
        <button
          onClick={handleLaunch}
          disabled={!launchReady || launching}
          className="group relative inline-flex items-center gap-2 rounded-xl bg-primary px-8 py-3 text-sm font-semibold text-primary-foreground shadow-lg shadow-primary/20 transition-all hover:bg-primary/90 hover:shadow-xl hover:shadow-primary/30 disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {launching ? (
            <>
              <Loader2 className="h-5 w-5 animate-spin" />
              Launching Prime...
            </>
          ) : (
            <>
              <Rocket className="h-5 w-5 transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5" />
              Launch Prime
            </>
          )}
        </button>
        {!launchReady && (
          <p className="mt-2 text-center text-xs text-muted-foreground">
            Waiting for Stage 1 to complete...
          </p>
        )}
      </div>
    </div>
  );
}
