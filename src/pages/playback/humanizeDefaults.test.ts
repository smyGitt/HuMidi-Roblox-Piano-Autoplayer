import { describe, expect, it } from "vitest";
import { HANDS_AND_IMPERFECTION_DEFAULTS, TIMING_AND_FEEL_DEFAULTS } from "./humanizeDefaults";
import { DEFAULT_CONFIG } from "./types";

describe("humanize card defaults", () => {
  it("Timing & Feel resets exactly its switches and sliders to their defaults", () => {
    expect(TIMING_AND_FEEL_DEFAULTS).toEqual({
      vary_timing: DEFAULT_CONFIG.vary_timing,
      timing_variance: DEFAULT_CONFIG.timing_variance,
      vary_articulation: DEFAULT_CONFIG.vary_articulation,
      articulation: DEFAULT_CONFIG.articulation,
      enable_tempo_sway: DEFAULT_CONFIG.enable_tempo_sway,
      tempo_sway_intensity: DEFAULT_CONFIG.tempo_sway_intensity,
      invert_tempo_sway: DEFAULT_CONFIG.invert_tempo_sway,
    });
  });

  it("Hands & Imperfection resets exactly its switches and sliders to their defaults", () => {
    expect(HANDS_AND_IMPERFECTION_DEFAULTS).toEqual({
      enable_drift_correction: DEFAULT_CONFIG.enable_drift_correction,
      drift_decay_factor: DEFAULT_CONFIG.drift_decay_factor,
      enable_mistakes: DEFAULT_CONFIG.enable_mistakes,
      mistake_chance: DEFAULT_CONFIG.mistake_chance,
    });
  });

  it("the two cards share no setting, so one reset never touches the other", () => {
    const timing = Object.keys(TIMING_AND_FEEL_DEFAULTS);
    const hands = Object.keys(HANDS_AND_IMPERFECTION_DEFAULTS);
    expect(timing.filter((key) => hands.includes(key))).toEqual([]);
  });
});
