import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

// =============================================================================
// Mutable mock state — allows per-test overrides
// =============================================================================
let mockLauncherCompleted = true;
let mockOnboardingCompleted = true;
let mockViewMode = "dashboard";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) => {
    switch (cmd) {
      case "get_system_state":
        return { version: "0.1.0", uptime_secs: 3600, active_skills: 12, active_connections: 3, memory_used_mb: 256, cpu_usage_pct: 23.5 };
      case "ping": return "pong";
      case "list_agents": return JSON.stringify([]);
      default: return null;
    }
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(({ queryKey }: { queryKey: string[] }) => {
    if (queryKey[0] === "systemState" || queryKey[0] === "ping") {
      return { data: queryKey[0] === "ping" ? "pong" : { version: "0.1.0", uptime_secs: 3600, active_skills: 12, active_connections: 3, memory_used_mb: 256, cpu_usage_pct: 23.5 }, error: null, isLoading: false };
    }
    return { data: [], error: null, isLoading: false, refetch: vi.fn() };
  }),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  QueryClient: vi.fn(),
}));

vi.mock("@/stores/useAppStore", () => ({
  useAppStore: Object.assign(
    (selector?: (s: Record<string, unknown>) => unknown) => {
      const state = {
        sidebarCollapsed: false,
        activePanel: "dashboard",
        onboardingCompleted: mockOnboardingCompleted,
        launcherCompleted: mockLauncherCompleted,
        launcherStages: { chat: "ready" as const, tools: "ready" as const, mcp: "ready" as const, editor: "ready" as const, skills: "ready" as const },
        setActivePanel: vi.fn(),
        toggleSidebar: vi.fn(),
        setOnboardingCompleted: vi.fn(),
        setLauncherCompleted: vi.fn(),
        setTheme: vi.fn(),
        theme: "dark" as const,
      };
      if (selector) return selector(state as unknown as Record<string, unknown>);
      return state;
    },
    { getState: () => ({}) },
  ),
}));

vi.mock("@/stores/useModelStore", () => ({
  useModelStore: (selector?: (s: Record<string, unknown>) => unknown) => {
    if (selector) return selector({ models: [] });
    return { models: [] };
  },
}));

vi.mock("@/stores/useViewMode", () => ({
  useViewMode: (selector?: (s: Record<string, unknown>) => unknown) => {
    const state = { mode: mockViewMode, previousMode: "dashboard", setMode: vi.fn() };
    if (selector) return selector(state as unknown as Record<string, unknown>);
    return state;
  },
}));

vi.mock("@/stores/useUpdateStore", () => ({
  useUpdateStore: (selector?: (s: Record<string, unknown>) => unknown) => {
    const state = { status: "idle", info: null, check: vi.fn(), install: vi.fn(), dismiss: vi.fn() };
    if (selector) return selector(state as unknown as Record<string, unknown>);
    return state;
  },
}));

vi.mock("@/hooks/useTheme", () => ({
  useTheme: () => ({ theme: "dark" as const, setTheme: vi.fn() }),
}));

vi.mock("@/components/ModeSwitcher", () => ({
  ModeSwitcher: () => <div data-testid="mode-switcher">ModeSwitcher</div>,
}));
vi.mock("@/components/ChatMode", () => ({
  ChatMode: () => <div data-testid="chat-mode">Chat</div>,
}));
vi.mock("@/components/ide/CodeMode", () => ({
  CodeMode: () => <div data-testid="code-mode">Code</div>,
}));
vi.mock("@/components/DashboardMode", () => ({
  DashboardMode: () => <div data-testid="dashboard-mode">Dashboard</div>,
}));
vi.mock("@/components/LauncherScreen", () => ({
  LauncherScreen: () => <div data-testid="launcher-screen">Launcher</div>,
}));
vi.mock("@/components/OnboardingWizard", () => ({
  OnboardingWizard: () => <div data-testid="onboarding-wizard">Onboarding</div>,
}));

// =============================================================================
// Tests
// =============================================================================

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLauncherCompleted = true;
    mockOnboardingCompleted = true;
    mockViewMode = "dashboard";
  });

  it("renders without crashing", () => {
    const { container } = render(<App />);
    expect(container).toBeTruthy();
  });

  it("renders ModeSwitcher", () => {
    render(<App />);
    expect(screen.getByTestId("mode-switcher")).toBeInTheDocument();
  });

  it("shows dashboard mode by default", () => {
    render(<App />);
    expect(screen.getByTestId("dashboard-mode")).toBeInTheDocument();
  });

  it("renders theme toggle button", () => {
    render(<App />);
    const themeBtn = screen.getByTitle("app.theme.toggle");
    expect(themeBtn).toBeInTheDocument();
  });

  it("shows Running status", () => {
    render(<App />);
    expect(screen.getByText("app.status.running")).toBeInTheDocument();
  });

  it("shows launcher screen when not completed", () => {
    mockLauncherCompleted = false;
    render(<App />);
    expect(screen.getByTestId("launcher-screen")).toBeInTheDocument();
  });

  it("shows onboarding wizard when not completed", () => {
    mockLauncherCompleted = true;
    mockOnboardingCompleted = false;
    render(<App />);
    expect(screen.getByTestId("onboarding-wizard")).toBeInTheDocument();
  });
});
