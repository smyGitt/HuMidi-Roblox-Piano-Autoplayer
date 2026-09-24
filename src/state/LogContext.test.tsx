import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LogProvider, useLog } from "./LogContext";
import type { ReactNode } from "react";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(() => Promise.resolve({})),
  saveAppConfig: vi.fn(() => Promise.resolve()),
}));

vi.mock("../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri")>("../lib/tauri");
  return { ...actual, ...tauriMocks };
});

function wrapper({ children }: { children: ReactNode }) {
  return <LogProvider>{children}</LogProvider>;
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  tauriMocks.loadAppConfig.mockReset().mockReturnValue(Promise.resolve({}));
  tauriMocks.saveAppConfig.mockReset().mockReturnValue(Promise.resolve());
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("useLog", () => {
  it("classifies a WARN-marker message as WARN even if it looks otherwise neutral", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("Load failed: file not found"));
    expect(result.current.entries[0].level).toBe("WARN");
  });

  it("classifies an OK-marker message as OK", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("Parsed 3 track(s) successful"));
    expect(result.current.entries[0].level).toBe("OK");
  });

  it("classifies a leading-bracket message as DEBUG", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("[pedal] model warmed up"));
    expect(result.current.entries[0].level).toBe("DEBUG");
  });

  it("classifies anything else as INFO", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("Waiting for a MIDI file"));
    expect(result.current.entries[0].level).toBe("INFO");
  });

  it("WARN takes precedence over OK when both markers are present", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("Export failed but the previous step was successful"));
    expect(result.current.entries[0].level).toBe("WARN");
  });

  it("drops empty or whitespace-only messages instead of storing a blank entry", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("   "));
    expect(result.current.entries).toHaveLength(0);
  });

  it("prefixes every stored line with a [HH:MM:SS] LEVEL header", () => {
    const { result } = renderHook(() => useLog(), { wrapper });
    act(() => result.current.appendLog("hello"));
    expect(result.current.entries[0].line).toMatch(/^\[\d{2}:\d{2}:\d{2}\] INFO hello$/);
  });

  describe("path redaction", () => {
    it("collapses an absolute Windows path down to its final component when enabled (default on)", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      expect(result.current.redactPaths).toBe(true);
      act(() => result.current.appendLog("Loaded C:\\Users\\me\\Documents\\song.mid"));
      expect(result.current.entries[0].line).toContain("song.mid");
      expect(result.current.entries[0].line).not.toContain("Users");
    });

    it("collapses a bare directory path to its final component too", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.appendLog("Save dir: C:\\Users\\me\\HuMidiSaves"));
      expect(result.current.entries[0].line).toContain("HuMidiSaves");
      expect(result.current.entries[0].line).not.toContain("Users");
    });

    it("collapses a forward-slash Windows path too (regression: was backslash-only before)", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.appendLog("Loaded C:/Users/me/Documents/song.mid"));
      expect(result.current.entries[0].line).toContain("song.mid");
      expect(result.current.entries[0].line).not.toContain("Users");
    });

    it("leaves paths untouched once redaction is turned off", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.setRedactPaths(false));
      act(() => result.current.appendLog("Loaded C:\\Users\\me\\Documents\\song.mid"));
      expect(result.current.entries[0].line).toContain("C:\\Users\\me\\Documents\\song.mid");
    });

    it("does not retroactively redact entries logged before the toggle changed", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.appendLog("Loaded C:\\Users\\me\\Documents\\song.mid"));
      const firstLine = result.current.entries[0].line;
      expect(firstLine).toContain("song.mid");
      expect(firstLine).not.toContain("Users");
      act(() => result.current.setRedactPaths(false));
      expect(result.current.entries[0].line).toBe(firstLine);
    });
  });

  describe("persistence (Tauri mode)", () => {
    beforeEach(() => {
      tauriMocks.isTauri.mockReturnValue(true);
    });

    it("hydrates redactPaths from load_app_config on mount", async () => {
      tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ redact_debug_paths: false }));
      const { result } = renderHook(() => useLog(), { wrapper });
      await waitFor(() => expect(result.current.redactPaths).toBe(false));
    });

    it("an empty persisted config leaves redactPaths at its default (true)", async () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      await waitFor(() => expect(tauriMocks.loadAppConfig).toHaveBeenCalled());
      expect(result.current.redactPaths).toBe(true);
    });

    it("setRedactPaths persists under the exact Python key name", async () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      await act(async () => result.current.setRedactPaths(false));
      expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ redact_debug_paths: false });
    });
  });

  describe("session snapshot", () => {
    it("starts with every field null", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      expect(result.current.snapshot).toEqual({
        file: null,
        source: null,
        tracks: null,
        notes: null,
        duration: null,
        pedal: null,
        tempo: null,
        pedal_style: null,
      });
    });

    it("updateSnapshot applies a partial patch without touching other fields", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.updateSnapshot({ file: "song.mid", source: "MIDI file" }));
      expect(result.current.snapshot.file).toBe("song.mid");
      expect(result.current.snapshot.source).toBe("MIDI file");
      expect(result.current.snapshot.tracks).toBeNull();

      act(() => result.current.updateSnapshot({ tracks: 3 }));
      expect(result.current.snapshot.tracks).toBe("3");
      expect(result.current.snapshot.file).toBe("song.mid");
    });

    it("updateSnapshot coerces numeric values to strings", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.updateSnapshot({ tracks: 5, notes: 214 }));
      expect(result.current.snapshot.tracks).toBe("5");
      expect(result.current.snapshot.notes).toBe("214");
    });

    it("a null value resets that field back to the empty placeholder", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.updateSnapshot({ pedal: "12 presses" }));
      expect(result.current.snapshot.pedal).toBe("12 presses");
      act(() => result.current.updateSnapshot({ pedal: null }));
      expect(result.current.snapshot.pedal).toBeNull();
    });

    it("clearSnapshot resets every field to null", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() =>
        result.current.updateSnapshot({ file: "song.mid", source: "MIDI file", tracks: 2, notes: 100 }),
      );
      act(() => result.current.clearSnapshot());
      expect(result.current.snapshot).toEqual({
        file: null,
        source: null,
        tracks: null,
        notes: null,
        duration: null,
        pedal: null,
        tempo: null,
        pedal_style: null,
      });
    });
  });

  describe("tallies and clearing", () => {
    it("clear() empties the entry list", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => result.current.appendLog("one"));
      act(() => result.current.appendLog("two"));
      expect(result.current.entries).toHaveLength(2);
      act(() => result.current.clear());
      expect(result.current.entries).toHaveLength(0);
    });
  });

  describe("ring buffer cap", () => {
    it("caps stored entries at 5000, dropping the oldest first", () => {
      const { result } = renderHook(() => useLog(), { wrapper });
      act(() => {
        for (let i = 0; i < 5010; i++) result.current.appendLog(`entry ${i}`);
      });
      expect(result.current.entries).toHaveLength(5000);
      expect(result.current.entries[0].line).toContain("entry 10");
      expect(result.current.entries[result.current.entries.length - 1].line).toContain("entry 5009");
    });
  });
});
