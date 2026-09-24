import { describe, expect, it } from "vitest";
import { formatSaveTime } from "./formatSaveTime";

describe("formatSaveTime", () => {
  it("formats an afternoon timestamp on a 12 hour clock with PM", () => {
    expect(formatSaveTime("2026-09-23T15:45:12")).toBe("Sep 23, 2026, 3:45 PM");
  });

  it("formats a morning timestamp with AM", () => {
    expect(formatSaveTime("2026-01-05T09:05:00")).toBe("Jan 5, 2026, 9:05 AM");
  });

  it("formats midnight and noon as 12 AM and 12 PM", () => {
    expect(formatSaveTime("2026-03-01T00:00:00")).toBe("Mar 1, 2026, 12:00 AM");
    expect(formatSaveTime("2026-03-01T12:00:00")).toBe("Mar 1, 2026, 12:00 PM");
  });

  it("returns the input unchanged when it is not a date", () => {
    expect(formatSaveTime("old-value")).toBe("old-value");
    expect(formatSaveTime("")).toBe("");
  });
});
