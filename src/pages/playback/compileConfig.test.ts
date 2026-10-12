import { describe, expect, it } from "vitest";
import { configDiffers, NOTES_CONFIG_KEYS, PEDAL_CONFIG_KEYS, pickConfig } from "./compileConfig";
import { DEFAULT_CONFIG } from "./types";

describe("compile config keys", () => {
  it("cover every setting that changes the compiled notes or pedal, and nothing else", () => {
    const covered = new Set<string>([...NOTES_CONFIG_KEYS, ...PEDAL_CONFIG_KEYS]);
    const uncovered = Object.keys(DEFAULT_CONFIG).filter((key) => !covered.has(key));
    expect(uncovered.sort()).toEqual(
      ["countdown", "debug_mode", "midi_file", "release_pedal_on_start", "transpose"].sort(),
    );
  });

  it("keep the notes and pedal keys separate", () => {
    const overlap = NOTES_CONFIG_KEYS.filter((key) => PEDAL_CONFIG_KEYS.includes(key));
    expect(overlap).toEqual([]);
  });
});

describe("pickConfig", () => {
  it("copies only the requested keys", () => {
    expect(pickConfig(DEFAULT_CONFIG, ["tempo", "pedal_style"])).toEqual({ tempo: 100, pedal_style: "ai" });
  });
});

describe("configDiffers", () => {
  const compiled = pickConfig(DEFAULT_CONFIG, NOTES_CONFIG_KEYS);

  it("is false when nothing was compiled yet", () => {
    expect(configDiffers({ ...DEFAULT_CONFIG, tempo: 140 }, null, NOTES_CONFIG_KEYS)).toBe(false);
  });

  it("is false for an identical config and for a change outside the keys", () => {
    expect(configDiffers(DEFAULT_CONFIG, compiled, NOTES_CONFIG_KEYS)).toBe(false);
    expect(configDiffers({ ...DEFAULT_CONFIG, pedal_style: "none" }, compiled, NOTES_CONFIG_KEYS)).toBe(false);
  });

  it("is false when a value is changed and changed back", () => {
    const changedBack = { ...{ ...DEFAULT_CONFIG, tempo: 140 }, tempo: DEFAULT_CONFIG.tempo };
    expect(configDiffers(changedBack, compiled, NOTES_CONFIG_KEYS)).toBe(false);
  });

  it("is true when a compiled setting differs", () => {
    expect(configDiffers({ ...DEFAULT_CONFIG, vary_timing: true }, compiled, NOTES_CONFIG_KEYS)).toBe(true);
  });
});
