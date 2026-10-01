import { describe, expect, it } from "vitest";
import { configFromSaved, configToPatch, PERSISTED_NAMES } from "./persistedConfig";
import { DEFAULT_CONFIG } from "./types";

const SAVED_TEMPO = 140;
const SAVED_THRESHOLD = 0.62;
const SAVED_VARIANCE = 0.05;

describe("configToPatch", () => {
  it("writes every user setting under the original config file names and nothing per-song", () => {
    const patch = configToPatch({
      ...DEFAULT_CONFIG,
      midi_file: "song.mid",
      tempo: SAVED_TEMPO,
      vary_timing: true,
      pedal_threshold_on: SAVED_THRESHOLD,
      pedal_threshold_off: SAVED_THRESHOLD,
    });
    expect(patch.tempo).toBe(SAVED_TEMPO);
    expect(patch.enable_vary_timing).toBe(true);
    expect(patch.value_timing_variance).toBe(DEFAULT_CONFIG.timing_variance);
    expect(patch.enable_hand_drift).toBe(false);
    expect(patch.value_hand_drift_decay).toBe(DEFAULT_CONFIG.drift_decay_factor);
    expect(patch).not.toHaveProperty("midi_file");
    expect(patch).not.toHaveProperty("pedal_threshold_on");
    expect(patch).not.toHaveProperty("pedal_threshold_off");
    expect(Object.keys(patch).sort()).toEqual(Object.values(PERSISTED_NAMES).sort());
  });

  it("covers every config field except the per-song ones", () => {
    const covered = new Set(Object.keys(PERSISTED_NAMES));
    const missing = Object.keys(DEFAULT_CONFIG).filter((key) => !covered.has(key));
    expect(missing.sort()).toEqual(["midi_file", "pedal_threshold_off", "pedal_threshold_on"]);
  });
});

describe("configFromSaved", () => {
  it("restores saved values on top of the defaults, reading the original file names", () => {
    const restored = configFromSaved({
      tempo: SAVED_TEMPO,
      enable_vary_timing: true,
      value_timing_variance: SAVED_VARIANCE,
      enable_hand_drift: true,
      pedal_style: "legato",
    });
    expect(restored).toEqual({
      ...DEFAULT_CONFIG,
      tempo: SAVED_TEMPO,
      vary_timing: true,
      timing_variance: SAVED_VARIANCE,
      enable_drift_correction: true,
      pedal_style: "legato",
    });
  });

  it("round-trips a full config", () => {
    const config = { ...DEFAULT_CONFIG, transpose: 3, countdown: false, vary_articulation: true, articulation: 80 };
    expect(configFromSaved(configToPatch(config))).toEqual(config);
  });

  it("ignores values of the wrong type, non-finite numbers, unknown pedal styles and unknown keys", () => {
    const restored = configFromSaved({
      tempo: "fast",
      countdown: 1,
      transpose: Number.NaN,
      pedal_style: "Auto (Default)",
      unknown: 1,
    });
    expect(restored).toBeUndefined();
  });

  it("never restores the per-song fields even if they are in the file", () => {
    const restored = configFromSaved({
      tempo: SAVED_TEMPO,
      midi_file: "old.mid",
      pedal_threshold_on: SAVED_THRESHOLD,
      pedal_threshold_off: SAVED_THRESHOLD,
    });
    expect(restored?.midi_file).toBe("");
    expect(restored?.pedal_threshold_on).toBe(DEFAULT_CONFIG.pedal_threshold_on);
    expect(restored?.pedal_threshold_off).toBe(DEFAULT_CONFIG.pedal_threshold_off);
  });

  it("returns undefined when nothing usable is stored", () => {
    expect(configFromSaved({})).toBeUndefined();
  });
});
