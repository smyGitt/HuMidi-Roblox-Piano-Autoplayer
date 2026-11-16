import { DEFAULT_CONFIG, type PlaybackConfig } from "./types";

function defaultsFor<K extends keyof PlaybackConfig>(keys: readonly K[]): Pick<PlaybackConfig, K> {
  return Object.fromEntries(keys.map((key) => [key, DEFAULT_CONFIG[key]])) as Pick<PlaybackConfig, K>;
}

export const TIMING_AND_FEEL_DEFAULTS = defaultsFor([
  "vary_timing",
  "timing_variance",
  "vary_articulation",
  "articulation",
  "enable_tempo_sway",
  "tempo_sway_intensity",
  "invert_tempo_sway",
]);

export const HANDS_AND_IMPERFECTION_DEFAULTS = defaultsFor([
  "enable_drift_correction",
  "drift_decay_factor",
  "enable_mistakes",
  "mistake_chance",
]);
