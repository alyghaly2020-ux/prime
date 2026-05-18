import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { BrowserAutomation } from "./BrowserAutomation";

vi.mock("@tanstack/react-query", () => ({
  useQuery: vi.fn(() => ({
    data: false,
    error: null,
    isLoading: false,
    refetch: vi.fn(),
  })),
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  QueryClient: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => false),
}));

describe("BrowserAutomation", () => {
  it("renders header", () => {
    render(<BrowserAutomation />);
    expect(screen.getByText("browser.title")).toBeInTheDocument();
    expect(screen.getByText("browser.engine")).toBeInTheDocument();
  });

  it("shows disconnected state", () => {
    render(<BrowserAutomation />);
    const disconnected = screen.getAllByText("browser.disconnected");
    expect(disconnected.length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("browser.not_connected")).toBeInTheDocument();
  });

  it("shows view tabs", () => {
    render(<BrowserAutomation />);
    expect(screen.getByText("browser.view_preview")).toBeInTheDocument();
    expect(screen.getByText("browser.view_dom")).toBeInTheDocument();
    expect(screen.getByText("browser.view_a11y")).toBeInTheDocument();
  });

  it("shows connect button", () => {
    render(<BrowserAutomation />);
    expect(screen.getByText("browser.connect_btn")).toBeInTheDocument();
  });
});
