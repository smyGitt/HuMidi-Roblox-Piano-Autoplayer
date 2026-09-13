export interface PlaybackConfig {
  midi_file: string;
  tempo: number;
  transpose: number;
  countdown: boolean;
  use_88_key_layout: boolean;
  pedal_style: string;
  pedal_threshold_on: number;
  pedal_threshold_off: number;
  debug_mode: boolean;
  simulate_hands: boolean;
  enable_chord_roll: boolean;
  vary_timing: boolean;
  timing_variance: number;
  vary_articulation: boolean;
  articulation: number;
  enable_drift_correction: boolean;
  drift_decay_factor: number;
  enable_mistakes: boolean;
  mistake_chance: number;
  enable_tempo_sway: boolean;
  tempo_sway_intensity: number;
  invert_tempo_sway: boolean;
  use_midi_pedal: boolean;
  use_velocity_accent: boolean;
}

export const PEDAL_MAPPING: Record<string, string> = {
  "Auto (Default)": "hybrid",
  PedalAI: "ai",
  Harmonic: "legato",
  Rhythmic: "rhythmic",
  None: "none",
};

export const DEFAULT_CONFIG: PlaybackConfig = {
  midi_file: "",
  tempo: 100,
  transpose: 0,
  countdown: true,
  use_88_key_layout: false,
  pedal_style: "ai",
  pedal_threshold_on: -1,
  pedal_threshold_off: -1,
  debug_mode: false,
  simulate_hands: false,
  enable_chord_roll: false,
  vary_timing: false,
  timing_variance: 0.01,
  vary_articulation: false,
  articulation: 95,
  enable_drift_correction: false,
  drift_decay_factor: 25,
  enable_mistakes: false,
  mistake_chance: 0.5,
  enable_tempo_sway: false,
  tempo_sway_intensity: 0.015,
  invert_tempo_sway: false,
  use_midi_pedal: false,
  use_velocity_accent: false,
};
