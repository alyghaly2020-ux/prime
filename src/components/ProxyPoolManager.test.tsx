import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { ProxyPoolManager } from "./ProxyPoolManager";

vi.mock("@tanstack/react-query", () => {
  const mockProxies = [
    { id: "p1", host: "us-east.proxy.example", port: 8080, protocol: "https", status: "online", latency_ms: 45, country: "US", last_used: "2m ago" },
    { id: "p2", host: "eu-west.proxy.example", port: 3128, protocol: "http", status: "online", latency_ms: 89, country: "DE", last_used: "5m ago" },
    { id: "p3", host: "ap-southeast.proxy.example", port: 8080, protocol: "socks5", status: "online", latency_ms: 152, country: "SG", last_used: "1m ago" },
    { id: "p4", host: "sa-east.proxy.example", port: 8080, protocol: "https", status: "error", latency_ms: null, country: "BR", last_used: "15m ago" },
    { id: "p5", host: "us-west.proxy.example", port: 3128, protocol: "http", status: "offline", latency_ms: null, country: "US", last_used: "1h ago" },
    { id: "p6", host: "eu-central.proxy.example", port: 8080, protocol: "https", status: "online", latency_ms: 67, country: "FR", last_used: "3m ago" },
  ];
  return {
    useQuery: vi.fn((opts) => {
      if (opts?.queryKey?.[0] === "proxy_active_count") {
        return { data: 6, error: null, isLoading: false, refetch: vi.fn() };
      }
      return { data: mockProxies, error: null, isLoading: false, refetch: vi.fn() };
    }),
    QueryClientProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
    QueryClient: vi.fn(),
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => []),
}));

describe("ProxyPoolManager", () => {
  it("renders title and stats", () => {
    render(<ProxyPoolManager />);
    expect(screen.getByText("proxy.title")).toBeInTheDocument();
    expect(screen.getByText("proxy.total")).toBeInTheDocument();
    expect(screen.getByText("proxy.online")).toBeInTheDocument();
    expect(screen.getByText("proxy.avg_latency")).toBeInTheDocument();
  });

  it("shows proxy count", () => {
    render(<ProxyPoolManager />);
    expect(screen.getByText("6")).toBeInTheDocument();
  });

  it("shows proxy entries", () => {
    render(<ProxyPoolManager />);
    expect(screen.getByText(/us-east.proxy.example/)).toBeInTheDocument();
    expect(screen.getByText(/eu-west.proxy.example/)).toBeInTheDocument();
  });

  it("shows rotation settings", () => {
    render(<ProxyPoolManager />);
    expect(screen.getByText("proxy.rotation_settings")).toBeInTheDocument();
    const roundRobins = screen.getAllByText("proxy.round_robin");
    expect(roundRobins.length).toBeGreaterThanOrEqual(2);
  });
});
