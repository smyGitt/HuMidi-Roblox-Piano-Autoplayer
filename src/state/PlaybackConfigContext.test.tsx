import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PlaybackConfigProvider, usePlaybackConfig } from "./PlaybackConfigContext";
import { DEFAULT_CONFIG } from "../pages/playback/types";
import type { ReactNode } from "react";

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
