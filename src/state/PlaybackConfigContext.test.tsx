import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PlaybackConfigProvider, usePlaybackConfig } from "./PlaybackConfigContext";
import { DEFAULT_CONFIG } from "../pages/playback/types";
import type { ReactNode } from "react";

const SAVE_DELAY_MS = 300;

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(),
  saveAppConfig: vi.fn<(patch: Record<string, unknown>) => Promise<void>>(),
}));

vi.mock("../lib/tauri", () => tauriMocks);

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  tauriMocks.loadAppConfig.mockReset().mockResolvedValue({});
  tauriMocks.saveAppConfig.mockReset().mockResolvedValue(undefined);
});

function wrapper({ children }: { children: ReactNode }) {
  return <PlaybackConfigProvider>{children}</PlaybackConfigProvider>;
}

describe("usePlaybackConfig", () => {
  it("starts at DEFAULT_CONFIG", () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    expect(result.current.config).toEqual(DEFAULT_CONFIG);
  });

  it("updateConfig shallow-merges a partial patch without touching other fields", () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    act(() => result.current.updateConfig({ tempo: 150 }));
    expect(result.current.config.tempo).toBe(150);
    expect(result.current.config.transpose).toBe(DEFAULT_CONFIG.transpose);
    expect(result.current.config.pedal_style).toBe(DEFAULT_CONFIG.pedal_style);
  });

  it("updateConfig can be called multiple times, accumulating changes", () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    act(() => result.current.updateConfig({ use_88_key_layout: true }));
    act(() => result.current.updateConfig({ simulate_hands: true }));
    expect(result.current.config.use_88_key_layout).toBe(true);
    expect(result.current.config.simulate_hands).toBe(true);
  });

  it("setConfig replaces the whole config object", () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    const replacement = { ...DEFAULT_CONFIG, tempo: 500, transpose: 12 };
    act(() => result.current.setConfig(replacement));
    expect(result.current.config).toEqual(replacement);
  });

  it("throws when used outside a PlaybackConfigProvider", () => {
    expect(() => renderHook(() => usePlaybackConfig())).toThrow(
      "usePlaybackConfig must be used within PlaybackConfigProvider",
    );
  });
});

describe("usePlaybackConfig persistence (desktop app)", () => {
  beforeEach(() => {
    tauriMocks.isTauri.mockReturnValue(true);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  async function flush() {
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
  }

  it("restores the settings saved in the config file, reading the original names", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({
      tempo: 140,
      transpose: -2,
      enable_vary_timing: true,
      value_timing_variance: 0.05,
      enable_hand_drift: true,
      pedal_style: "rhythmic",
      countdown: false,
    });
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    await flush();
    expect(result.current.config).toEqual({
      ...DEFAULT_CONFIG,
      tempo: 140,
      transpose: -2,
      vary_timing: true,
      timing_variance: 0.05,
      enable_drift_correction: true,
      pedal_style: "rhythmic",
      countdown: false,
    });
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("keeps the defaults when nothing was saved yet, without writing anything", async () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    await flush();
    expect(result.current.config).toEqual(DEFAULT_CONFIG);
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("saves every change of every setting, under the original names", async () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    await flush();
    act(() => result.current.updateConfig({ vary_articulation: true, articulation: 80, mistake_chance: 2 }));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledTimes(1);
    const saved = tauriMocks.saveAppConfig.mock.calls[0][0];
    expect(saved).toMatchObject({
      enable_vary_articulation: true,
      value_articulation: 80,
      value_mistake_chance: 2,
      tempo: DEFAULT_CONFIG.tempo,
    });
  });

  it("does not save the per-song fields: the loaded file and the pedal threshold curves", async () => {
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    await flush();
    act(() => result.current.updateConfig({ midi_file: "song.mid", pedal_curve_on: [{ time: 0, value: 0.7 }], pedal_curve_off: [{ time: 0, value: 0.2 }] }));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
  });

  it("saves a reset back to the default so the saved value follows it", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ transpose: 5 });
    const { result } = renderHook(() => usePlaybackConfig(), { wrapper });
    await flush();
    expect(result.current.config.transpose).toBe(5);
    act(() => result.current.updateConfig({ transpose: DEFAULT_CONFIG.transpose }));
    act(() => void vi.advanceTimersByTime(SAVE_DELAY_MS * 2));
    expect(tauriMocks.saveAppConfig.mock.calls[0][0]).toMatchObject({ transpose: DEFAULT_CONFIG.transpose });
  });
});
