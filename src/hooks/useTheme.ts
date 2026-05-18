import { useEffect, useCallback } from "react";
import { useAppStore } from "@/stores/useAppStore";
import type { Theme } from "@/types";

export function useTheme(): {
  theme: Theme;
  effectiveTheme: "light" | "dark";
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
} {
  const { theme, setTheme } = useAppStore();

  const getSystemTheme = useCallback((): "light" | "dark" => {
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
    }
    return "light";
  }, []);

  const effectiveTheme =
    theme === "system" ? getSystemTheme() : (theme as "light" | "dark");

  const applyTheme = useCallback((t: "light" | "dark") => {
    const root = document.documentElement;
    root.classList.remove("light", "dark");
    root.classList.add(t);
    root.style.colorScheme = t;
    document.body.classList.remove("light", "dark");
    document.body.classList.add(t);
  }, []);

  useEffect(() => {
    applyTheme(effectiveTheme);
  }, [effectiveTheme, applyTheme]);

  // Listen for system theme changes
  useEffect(() => {
    if (theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      const sysTheme = mq.matches ? "dark" : "light";
      applyTheme(sysTheme);
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme, applyTheme]);

  const toggleTheme = useCallback(() => {
    const order: Theme[] = ["light", "dark", "system"];
    const idx = order.indexOf(theme);
    setTheme(order[(idx + 1) % order.length]);
  }, [theme, setTheme]);

  return { theme, effectiveTheme, setTheme, toggleTheme };
}
