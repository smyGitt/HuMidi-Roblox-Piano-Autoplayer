import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { LogProvider, useLog } from "./LogContext";
import { parseBoolean, parseNumberInRange, parseOneOf } from "./persistedParsers";
import { usePersistedState } from "./usePersistedState";

const SAVE_DELAY_MS = 300;
const KEY = "ui_example";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(),
  saveAppConfig: vi.fn<(patch: Record<string, unknown>) => Promise<void>>(),
}));

vi.mock("../lib/tauri", () => tauriMocks);

function wrapper({ children }: { children: ReactNode }) {
  return <LogProvider>{children}</LogProvider>;
}

async function flush() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.loadAppConfig.mockReset().mockResolvedValue({});
  tauriMocks.saveAppConfig.mockReset().mockResolvedValue(undefined);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("usePersistedState", () => {
  it("never touches the backend outside the desktop app", async () => {
    tauriMocks.isTauri.mockReturnValue(false);
    const { result } = renderHook(() => usePersistedState(KEY, "a", { parse: (raw) => (raw as string) ?? undefined }), { wrapper });
    await flush();
    act(() => result.current[1]("b"));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(result.current[0]).toBe("b");
    expect(tauriMocks.loadAppConfig).not.toHaveBeenCalled();
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("loads the stored value and does not write it straight back", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ [KEY]: true });
    const { result } = renderHook(() => usePersistedState(KEY, false, { parse: parseBoolean }), { wrapper });
    await flush();
    expect(result.current[0]).toBe(true);
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("keeps the fallback when the stored value is invalid or missing, without writing", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ [KEY]: "not a number" });
    const { result } = renderHook(() => usePersistedState(KEY, 5, { parse: parseNumberInRange(0, 10) }), { wrapper });
    await flush();
    expect(result.current[0]).toBe(5);
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("saves a change under its key after a short delay, once, with the latest value", async () => {
    const { result } = renderHook(() => usePersistedState(KEY, "One", { parse: parseOneOf(["One", "Two", "Three"] as const) }), { wrapper });
    await flush();
    act(() => result.current[1]("Two"));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS / 2));
    act(() => result.current[1]("Three"));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledTimes(1);
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ [KEY]: "Three" });
  });

  it("does not save when the value is set back to what is already stored", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ [KEY]: "Two" });
    const { result } = renderHook(() => usePersistedState(KEY, "One", { parse: parseOneOf(["One", "Two"] as const) }), { wrapper });
    await flush();
    act(() => result.current[1]("One"));
    act(() => result.current[1]("Two"));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("saves the serialized form and not the whole value", async () => {
    const { result } = renderHook(
      () =>
        usePersistedState(KEY, { keep: 1, skip: 1 }, {
          parse: () => undefined,
          serialize: (value) => ({ keep: value.keep }),
        }),
      { wrapper },
    );
    await flush();
    act(() => result.current[1]({ keep: 2, skip: 9 }));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ [KEY]: { keep: 2 } });
    act(() => result.current[1]({ keep: 2, skip: 10 }));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledTimes(1);
  });

  it("writes a failed save to the app log instead of swallowing it", async () => {
    tauriMocks.saveAppConfig.mockRejectedValue(new Error("disk full"));
    const { result } = renderHook(
      () => ({ state: usePersistedState(KEY, 1, { parse: parseNumberInRange(0, 9) }), log: useLog() }),
      { wrapper },
    );
    await flush();
    act(() => result.current.state[1](2));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    await flush();
    expect(result.current.log.entries.some((entry) => entry.line.includes(`Saving ${KEY} failed: Error: disk full`))).toBe(true);
  });
});

describe("persisted parsers", () => {
  it("accept only valid values", () => {
    expect(parseBoolean(true)).toBe(true);
    expect(parseBoolean("true")).toBeUndefined();
    expect(parseNumberInRange(1, 3)(2)).toBe(2);
    expect(parseNumberInRange(1, 3)(4)).toBeUndefined();
    expect(parseNumberInRange(1, 3)(Number.NaN)).toBeUndefined();
    expect(parseOneOf(["a", "b"] as const)("b")).toBe("b");
    expect(parseOneOf(["a", "b"] as const)("c")).toBeUndefined();
  });
});
