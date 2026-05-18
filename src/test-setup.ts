import "@testing-library/jest-dom";
import { vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (params) return key.replace(/\{(\w+)\}/g, (_, k) => String(params[k] ?? `{${k}}`));
      return key;
    },
    i18n: { changeLanguage: vi.fn(), language: "en", dir: "ltr" },
  }),
  initReactI18next: { type: "3rdParty", init: vi.fn() },
}));
