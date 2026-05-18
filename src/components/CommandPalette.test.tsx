import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { CommandPalette } from "./CommandPalette";

describe("CommandPalette", () => {
  it("renders nothing when closed", () => {
    const { container } = render(
      <CommandPalette open={false} onClose={vi.fn()} onNavigate={vi.fn()} />
    );
    expect(container.innerHTML).toBe("");
  });

  it("renders search input when open", () => {
    render(<CommandPalette open={true} onClose={vi.fn()} onNavigate={vi.fn()} />);
    expect(screen.getByPlaceholderText("command_palette.placeholder")).toBeDefined();
  });

  it("shows all items when query is empty", () => {
    render(<CommandPalette open={true} onClose={vi.fn()} onNavigate={vi.fn()} />);
    expect(screen.getByText("command_palette.chat")).toBeDefined();
    expect(screen.getByText("command_palette.code")).toBeDefined();
    expect(screen.getByText("command_palette.dashboard")).toBeDefined();
    expect(screen.getByText("command_palette.agents")).toBeDefined();
    expect(screen.getByText("command_palette.browser")).toBeDefined();
  });

  it("filters items by query", () => {
    render(<CommandPalette open={true} onClose={vi.fn()} onNavigate={vi.fn()} />);
    const input = screen.getByPlaceholderText("command_palette.placeholder");
    fireEvent.change(input, { target: { value: "chat" } });
    expect(screen.getByText("command_palette.chat")).toBeDefined();
    expect(screen.queryByText("command_palette.code")).toBeNull();
  });

  it("calls onNavigate and onClose on item click", () => {
    const onClose = vi.fn();
    const onNavigate = vi.fn();
    render(<CommandPalette open={true} onClose={onClose} onNavigate={onNavigate} />);
    fireEvent.click(screen.getByText("command_palette.chat"));
    expect(onNavigate).toHaveBeenCalledWith("chat");
    expect(onClose).toHaveBeenCalled();
  });

  it("calls onClose on Escape", () => {
    const onClose = vi.fn();
    render(<CommandPalette open={true} onClose={onClose} onNavigate={vi.fn()} />);
    fireEvent.keyDown(screen.getByPlaceholderText("command_palette.placeholder"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("handles arrow key navigation and Enter", () => {
    const onNavigate = vi.fn();
    const onClose = vi.fn();
    render(<CommandPalette open={true} onClose={onClose} onNavigate={onNavigate} />);

    const input = screen.getByPlaceholderText("command_palette.placeholder");

    // Arrow down twice
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });

    // Arrow up once (back to first)
    fireEvent.keyDown(input, { key: "ArrowUp" });

    // Enter
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onNavigate).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("shows empty state when no results match", () => {
    render(<CommandPalette open={true} onClose={vi.fn()} onNavigate={vi.fn()} />);
    const input = screen.getByPlaceholderText("command_palette.placeholder");
    fireEvent.change(input, { target: { value: "zzzzzxyz" } });
    expect(screen.getByText("common.no_results")).toBeDefined();
  });

  it("closes when clicking backdrop", () => {
    const onClose = vi.fn();
    const { container } = render(<CommandPalette open={true} onClose={onClose} onNavigate={vi.fn()} />);
    const backdrop = container.querySelector(".fixed.inset-0");
    if (backdrop) fireEvent.click(backdrop);
    expect(onClose).toHaveBeenCalled();
  });
});
