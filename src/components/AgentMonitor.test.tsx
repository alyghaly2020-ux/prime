import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { AgentMonitor } from "./AgentMonitor";

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(() => ({
    data: [
      { id: "rust-expert-agent", name: "Rust Expert", role: "Systems programming", model: "gpt-4" },
      { id: "coding-agent", name: "Coding Agent", role: "General implementation", model: "gpt-4" },
    ],
    error: null,
    isLoading: false,
    refetch: vi.fn(),
  })),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  QueryClient: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => JSON.stringify([])),
}));

describe("AgentMonitor", () => {
  it("renders title with agent count", () => {
    render(<AgentMonitor />);
    expect(screen.getByText("agents.title")).toBeInTheDocument();
    expect(screen.getByText(/agents\.count/)).toBeInTheDocument();
  });

  it("shows filter chips with agent category counts", () => {
    render(<AgentMonitor />);
    expect(screen.getByText("agents.all")).toBeInTheDocument();
  });

  it("shows expandable category sections", () => {
    render(<AgentMonitor />);
    expect(screen.getByText("agents.category.development")).toBeInTheDocument();
    expect(screen.getByText("agents.category.rust")).toBeInTheDocument();
  });

  it("shows workflow templates", () => {
    render(<AgentMonitor />);
    expect(screen.getByText("agents.workflows")).toBeInTheDocument();
    expect(screen.getByText("agents.workflow.code_review")).toBeInTheDocument();
    expect(screen.getByText("agents.workflow.deep_research")).toBeInTheDocument();
  });
});
