import type { PlaybackConfig } from "./types";

export const NOTES_CONFIG_KEYS: readonly (keyof PlaybackConfig)[] = [
  "simulate_hands",
  "vary_timing",
  "timing_variance",
  "vary_articulation",
  "articulation",
  "enable_drift_correction",
  "drift_decay_factor",
  "enable_chord_roll",
  "enable_tempo_sway",
  "tempo_sway_intensity",
  "invert_tempo_sway",
  "enable_mistakes",
  "mistake_chance",
  "tempo",
  "use_88_key_layout",
];

export const PEDAL_CONFIG_KEYS: readonly (keyof PlaybackConfig)[] = [
  "pedal_style",
  "pedal_threshold_on",
  "pedal_threshold_off",
  "use_midi_pedal",
  "pedal_before_humanize",
];

export function pickConfig(config: PlaybackConfig, keys: readonly (keyof PlaybackConfig)[]): Partial<PlaybackConfig> {
  const picked: Record<string, unknown> = {};
  for (const key of keys) picked[key] = config[key];
  return picked as Partial<PlaybackConfig>;
}

export function configDiffers(
  config: PlaybackConfig,
  compiled: Partial<PlaybackConfig> | null,
  keys: readonly (keyof PlaybackConfig)[],
): boolean {
  if (compiled === null) return false;
  return keys.some((key) => config[key] !== compiled[key]);
}
