import { describe, it, expect } from "vitest";
import { cn, formatBytes, formatTime, truncate } from "./utils";

// =============================================================================
// cn() — class name concatenation with tailwind-merge
// =============================================================================

describe("cn", () => {
  it("returns empty string for no inputs", () => {
    expect(cn()).toBe("");
  });

  it("handles a single string class", () => {
    expect(cn("foo")).toBe("foo");
  });

  it("concatenates multiple string classes", () => {
    expect(cn("foo", "bar")).toBe("foo bar");
  });

  it("filters out falsy values", () => {
    const falsy = false;
    expect(cn("foo", falsy && "bar", "baz")).toBe("foo baz");
    expect(cn("foo", null, undefined, "bar")).toBe("foo bar");
    expect(cn("foo", 0 as unknown as string, "bar")).toBe("foo bar");
  });

  it("handles conditional classes via ternary", () => {
    const isActive = true;
    expect(cn("base", isActive ? "active" : "inactive")).toBe("base active");
    const isInactive = false;
    expect(cn("base", isInactive ? "active" : "inactive")).toBe("base inactive");
  });

  it("merges Tailwind classes correctly (later wins)", () => {
    // twMerge should resolve conflicts with the last value winning
    expect(cn("px-4", "px-2")).toBe("px-2");
    expect(cn("text-red-500", "text-blue-500")).toBe("text-blue-500");
    expect(cn("p-4", "p-2")).toBe("p-2");
  });

  it("handles object syntax via clsx", () => {
    expect(cn({ foo: true, bar: false })).toBe("foo");
    expect(cn({ foo: true }, { bar: true })).toBe("foo bar");
  });

  it("handles array syntax via clsx", () => {
    expect(cn(["foo", "bar"])).toBe("foo bar");
  });

  it("combines strings, objects, and arrays", () => {
    expect(cn("base", { conditional: true }, ["extra1", "extra2"])).toBe(
      "base conditional extra1 extra2",
    );
  });
});

// =============================================================================
// formatBytes()
// =============================================================================

describe("formatBytes", () => {
  it('returns "0 B" for zero', () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("formats bytes without unit conversion", () => {
    expect(formatBytes(500)).toBe("500 B");
  });

  it("formats kilobytes", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(2048)).toBe("2 KB");
    expect(formatBytes(1536)).toBe("1.5 KB");
  });

  it("formats megabytes", () => {
    expect(formatBytes(1048576)).toBe("1 MB");
    expect(formatBytes(3145728)).toBe("3 MB");
  });

  it("formats gigabytes", () => {
    expect(formatBytes(1073741824)).toBe("1 GB");
  });

  it("formats terabytes", () => {
    expect(formatBytes(1099511627776)).toBe("1 TB");
  });

  it("rounds to 2 decimal places", () => {
    expect(formatBytes(1234)).toBe("1.21 KB");
    expect(formatBytes(1234567)).toBe("1.18 MB");
  });
});

// =============================================================================
// formatTime()
// =============================================================================

describe("formatTime", () => {
  it('returns "0ms" for zero', () => {
    expect(formatTime(0)).toBe("0ms");
  });

  it("formats milliseconds", () => {
    expect(formatTime(500)).toBe("500ms");
    expect(formatTime(999)).toBe("999ms");
  });

  it("formats seconds", () => {
    expect(formatTime(1000)).toBe("1.0s");
    expect(formatTime(1500)).toBe("1.5s");
    expect(formatTime(59000)).toBe("59.0s");
  });

  it("formats minutes and seconds", () => {
    expect(formatTime(60000)).toBe("1m 0s");
    expect(formatTime(90000)).toBe("1m 30s");
    expect(formatTime(3661000)).toBe("61m 1s");
  });
});

// =============================================================================
// truncate()
// =============================================================================

describe("truncate", () => {
  it("returns the original string if shorter than limit", () => {
    expect(truncate("hello", 10)).toBe("hello");
  });

  it("returns the original string if equal to limit", () => {
    expect(truncate("hello", 5)).toBe("hello");
  });

  it("truncates and appends ellipsis if longer than limit", () => {
    expect(truncate("hello world", 5)).toBe("hello...");
    expect(truncate("abcdefghij", 3)).toBe("abc...");
  });

  it("handles empty string", () => {
    expect(truncate("", 5)).toBe("");
  });

  it("handles zero limit", () => {
    expect(truncate("hello", 0)).toBe("...");
  });
});
