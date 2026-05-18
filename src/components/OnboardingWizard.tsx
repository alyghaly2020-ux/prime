import { useState, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "@/stores/useAppStore";
import {
  Rocket,
  Server,
  Puzzle,
  CheckCircle2,
  ArrowRight,
  ArrowLeft,
  Loader2,
} from "lucide-react";

export function OnboardingWizard() {
  const { t } = useTranslation();
  const { onboardingStep, setOnboardingStep, setOnboardingCompleted } = useAppStore();
  const [loading, setLoading] = useState(false);

  const STEPS = useMemo(
    () => [
      {
        id: "welcome",
        title: t("onboarding.welcome_title"),
        description: t("onboarding.welcome_desc"),
        icon: Rocket,
      },
      {
        id: "mcp",
        title: t("onboarding.mcp_title"),
        description: t("onboarding.mcp_desc"),
        icon: Server,
      },
      {
        id: "skills",
        title: t("onboarding.skills_title"),
        description: t("onboarding.skills_desc"),
        icon: Puzzle,
      },
      {
        id: "ready",
        title: t("onboarding.ready_title"),
        description: t("onboarding.ready_desc"),
        icon: CheckCircle2,
      },
    ],
    [t],
  );

  const step = STEPS[onboardingStep];
  const isFirst = onboardingStep === 0;
  const isLast = onboardingStep === STEPS.length - 1;
  const Icon = step.icon;

  const handleNext = async () => {
    if (isLast) {
      setLoading(true);
      // Simulate final setup
      await new Promise((r) => setTimeout(r, 1000));
      setOnboardingCompleted(true);
      return;
    }
    setOnboardingStep(onboardingStep + 1);
  };

  const handleSkip = () => {
    setOnboardingCompleted(true);
  };

  return (
    <div className="flex h-screen items-center justify-center bg-background">
      <div className="w-full max-w-lg rounded-xl border border-border bg-card p-8 shadow-lg">
        {/* Progress */}
        <div className="mb-8 flex items-center gap-2">
          {STEPS.map((s, i) => (
            <div key={s.id} className="flex items-center gap-2">
              <div
                className={`flex h-8 w-8 items-center justify-center rounded-full text-sm font-medium transition-colors ${
                  i <= onboardingStep
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted text-muted-foreground"
                }`}
              >
                {i + 1}
              </div>
              {i < STEPS.length - 1 && (
                <div
                  className={`h-0.5 w-8 transition-colors ${
                    i < onboardingStep ? "bg-primary" : "bg-muted"
                  }`}
                />
              )}
            </div>
          ))}
        </div>

        {/* Content */}
        <div className="mb-8 text-center">
          <div className="mb-4 inline-flex rounded-full bg-primary/10 p-3">
            <Icon className="h-8 w-8 text-primary" />
          </div>
          <h2 className="mb-2 text-2xl font-bold text-card-foreground">
            {step.title}
          </h2>
          <p className="text-sm text-muted-foreground">{step.description}</p>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-between">
          <button
            onClick={handleSkip}
            className="text-sm text-muted-foreground hover:text-foreground"
          >
            {t("onboarding.skip")}
          </button>
          <div className="flex items-center gap-2">
            {!isFirst && (
              <button
                onClick={() => setOnboardingStep(onboardingStep - 1)}
                className="inline-flex items-center gap-1 rounded-md border border-input bg-background px-4 py-2 text-sm hover:bg-accent"
              >
                <ArrowLeft className="h-4 w-4" />
                {t("onboarding.back")}
              </button>
            )}
            <button
              onClick={handleNext}
              disabled={loading}
              className="inline-flex items-center gap-1 rounded-md bg-primary px-6 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {loading ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {t("onboarding.setting_up")}
                </>
              ) : isLast ? (
                <>
                  {t("onboarding.get_started")}
                  <Rocket className="h-4 w-4" />
                </>
              ) : (
                <>
                  {t("onboarding.next")}
                  <ArrowRight className="h-4 w-4" />
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
