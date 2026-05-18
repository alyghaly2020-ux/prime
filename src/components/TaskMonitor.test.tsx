import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { TaskMonitor } from "./TaskMonitor";

const MOCK_TASKS = [
  { id: "Index workspace files", metadata: { category: "indexing" }, status: "Running", created_at: "2024-01-15T10:28:00Z", duration_ms: 120000, error: null },
  { id: "Code review: src/App.tsx", metadata: { category: "review" }, status: "Running", created_at: "2024-01-15T11:15:00Z", duration_ms: 45000, error: null },
  { id: "Deep research: Rust vs Go", metadata: { category: "research" }, status: "Completed", created_at: "2024-01-15T10:00:00Z", duration_ms: 523000, error: null },
  { id: "Build frontend bundle", metadata: { category: "build" }, status: "Completed", created_at: "2024-01-15T11:20:00Z", duration_ms: 8900, error: null },
  { id: "Deploy to staging", metadata: { category: "deploy" }, status: "Completed", created_at: "2024-01-15T09:55:00Z", duration_ms: 34000, error: null },
  { id: "MCP server: github", metadata: { category: "mcp" }, status: "Pending", created_at: "2024-01-15T11:30:00Z", duration_ms: null, error: null },
  { id: "MCP server: memory", metadata: { category: "mcp" }, status: "Pending", created_at: "2024-01-15T11:31:00Z", duration_ms: null, error: null },
  { id: "Test: integration suite", metadata: { category: "testing" }, status: "Failed", created_at: "2024-01-15T09:20:00Z", duration_ms: 125000, error: "Assertion failed in test_mcp_connection" },
  { id: "Plugin: install tailwind", metadata: { category: "plugins" }, status: "Completed", created_at: "2024-01-15T08:00:00Z", duration_ms: 15000, error: null },
  { id: "Memory compaction", metadata: { category: "memory" }, status: "Failed", created_at: "2024-01-15T07:00:00Z", duration_ms: 45000, error: "Out of memory budget" },
  { id: "AI chat: code generation", metadata: { category: "ai" }, status: "Completed", created_at: "2024-01-15T06:30:00Z", duration_ms: 12000, error: null },
  { id: "Sync git remotes", metadata: { category: "git" }, status: "Completed", created_at: "2024-01-15T04:00:00Z", duration_ms: 6700, error: null },
];

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(() => ({
    data: {
      tasks: MOCK_TASKS,
      summary: { running: 2, pending: 2, completed: 6, failed: 2, total: 12, avg_duration_ms: 44100 },
    },
    error: null,
    isLoading: false,
    refetch: vi.fn(),
  })),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  QueryClient: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => []),
}));

describe("TaskMonitor", () => {
  it("renders title", () => {
    render(<TaskMonitor />);
    expect(screen.getByText("Task Monitor")).toBeInTheDocument();
  });

  it("shows summary counts", () => {
    render(<TaskMonitor />);
    expect(screen.getByText("completed")).toBeInTheDocument();
    expect(screen.getByText("running")).toBeInTheDocument();
    expect(screen.getByText("failed")).toBeInTheDocument();
    expect(screen.getByText("pending")).toBeInTheDocument();
  });

  it("shows task names from mock data", () => {
    render(<TaskMonitor />);
    expect(screen.getByText("Index workspace files")).toBeInTheDocument();
    expect(screen.getByText("Code review: src/App.tsx")).toBeInTheDocument();
  });

  it("shows filter tabs", () => {
    render(<TaskMonitor />);
    expect(screen.getByText("All (12)")).toBeInTheDocument();
    expect(screen.getByText("running (2)")).toBeInTheDocument();
  });

  it("shows success rate", () => {
    render(<TaskMonitor />);
    expect(screen.getByText(/Success rate:/)).toBeInTheDocument();
  });
});
