import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SecurityDashboard } from "./SecurityDashboard";

const MOCK_AUDIT = [
  { id: 1, timestamp: "2024-01-15T10:00:00Z", action: "permission.request", subject: "plugin-x", resource: "filesystem", result: "Deny", reason: "Not granted" },
  { id: 2, timestamp: "2024-01-15T09:55:00Z", action: "sandbox.violation", subject: "plugin-y", resource: "process.spawn", result: "Deny", reason: "Blocked" },
  { id: 3, timestamp: "2024-01-15T09:00:00Z", action: "audit.rotate", subject: "system", resource: "audit.log", result: "Allow", reason: null },
  { id: 4, timestamp: "2024-01-15T08:00:00Z", action: "encryption.rotate", subject: "system", resource: "keys", result: "Allow", reason: null },
];

const MOCK_POLICY = {
  sandbox_enabled: true,
  encryption_at_rest: true,
  permission_model: "Moderate",
  max_cpu_cores: 4.0,
  max_memory_mb: 1024,
  max_timeout_secs: 60,
  allowed_networks: [],
};

let callCount = 0;
vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(() => {
    callCount++;
    if (callCount === 1) return { data: MOCK_AUDIT, error: null, isLoading: false, refetch: vi.fn() };
    return { data: MOCK_POLICY, error: null, isLoading: false, refetch: vi.fn() };
  }),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  QueryClient: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => []),
}));

describe("SecurityDashboard", () => {
  it("renders title and score", () => {
    render(<SecurityDashboard />);
    expect(screen.getByText("security.title")).toBeInTheDocument();
    expect(screen.getByText("security.score_label")).toBeInTheDocument();
  });

  it("shows all feature toggles", () => {
    render(<SecurityDashboard />);
    expect(screen.getByText("security.feature.sandbox")).toBeInTheDocument();
    expect(screen.getByText("security.feature.encryption")).toBeInTheDocument();
    expect(screen.getByText("security.feature.permissions")).toBeInTheDocument();
    expect(screen.getByText("security.feature.audit")).toBeInTheDocument();
  });

  it("shows toggle states", () => {
    render(<SecurityDashboard />);
    const enabled = screen.getAllByText("common.enabled");
    expect(enabled.length).toBeGreaterThanOrEqual(4);
  });

  it("shows security events section", () => {
    render(<SecurityDashboard />);
    expect(screen.getByText("security.events")).toBeInTheDocument();
  });
});
