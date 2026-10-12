import { DEFAULT_CONFIG, PEDAL_MAPPING, type PlaybackConfig } from "./types";

type ConfigKey = Exclude<keyof PlaybackConfig, "midi_file" | "pedal_curve_on" | "pedal_curve_off">;

export const PERSISTED_NAMES: Record<ConfigKey, string> = {
  tempo: "tempo",
  transpose: "transpose",
  pedal_style: "pedal_style",
  use_88_key_layout: "use_88_key_layout",
  countdown: "countdown",
  release_pedal_on_start: "release_pedal_on_start",
  debug_mode: "debug_mode",
  simulate_hands: "simulate_hands",
  enable_chord_roll: "enable_chord_roll",
  vary_timing: "enable_vary_timing",
  timing_variance: "value_timing_variance",
  vary_articulation: "enable_vary_articulation",
  articulation: "value_articulation",
  enable_drift_correction: "enable_hand_drift",
  drift_decay_factor: "value_hand_drift_decay",
  enable_mistakes: "enable_mistakes",
  mistake_chance: "value_mistake_chance",
  enable_tempo_sway: "enable_tempo_sway",
  tempo_sway_intensity: "value_tempo_sway_intensity",
  invert_tempo_sway: "invert_tempo_sway",
  use_midi_pedal: "use_midi_pedal",
  pedal_before_humanize: "pedal_before_humanize",
};

const CONFIG_KEYS = Object.keys(PERSISTED_NAMES) as ConfigKey[];
const PEDAL_STYLES: readonly unknown[] = Object.values(PEDAL_MAPPING);

function isValid(key: ConfigKey, value: unknown): boolean {
  if (key === "pedal_style") return PEDAL_STYLES.includes(value);
  const expected = typeof DEFAULT_CONFIG[key];
  return typeof value === expected && (expected !== "number" || Number.isFinite(value));
}

export function configToPatch(config: PlaybackConfig): Record<string, unknown> {
  const patch: Record<string, unknown> = {};
  for (const key of CONFIG_KEYS) patch[PERSISTED_NAMES[key]] = config[key];
  return patch;
}

export function configFromSaved(saved: Record<string, unknown>): PlaybackConfig | undefined {
  const restored: Record<string, unknown> = {};
  for (const key of CONFIG_KEYS) {
    const value = saved[PERSISTED_NAMES[key]];
    if (isValid(key, value)) restored[key] = value;
  }
  return Object.keys(restored).length > 0 ? { ...DEFAULT_CONFIG, ...restored } : undefined;
}
