use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlaybackConfig {
    pub midi_file: String,
    pub tempo: f64,
    pub transpose: i32,
    pub countdown: bool,
    pub use_88_key_layout: bool,
    pub pedal_style: String,
    pub pedal_threshold_on: f64,
    pub pedal_threshold_off: f64,
    pub debug_mode: bool,
    pub simulate_hands: bool,
    pub vary_velocity: bool,
    pub enable_chord_roll: bool,
    pub vary_timing: bool,
    pub timing_variance: f64,
    pub vary_articulation: bool,
    pub articulation: f64,
    pub enable_drift_correction: bool,
    pub drift_decay_factor: f64,
    pub enable_mistakes: bool,
    pub mistake_chance: f64,
    pub enable_tempo_sway: bool,
    pub tempo_sway_intensity: f64,
    pub invert_tempo_sway: bool,
    pub use_midi_pedal: bool,
    pub use_velocity_accent: bool,
    pub use_ai_pedal: bool,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        PlaybackConfig {
            midi_file: String::new(),
            tempo: 1.0,
            transpose: 0,
            countdown: true,
            use_88_key_layout: false,
            pedal_style: "none".to_string(),
            pedal_threshold_on: -1.0,
            pedal_threshold_off: -1.0,
            debug_mode: false,
            simulate_hands: false,
            vary_velocity: false,
            enable_chord_roll: false,
            vary_timing: false,
            timing_variance: 0.0,
            vary_articulation: false,
            articulation: 0.0,
            enable_drift_correction: false,
            drift_decay_factor: 0.0,
            enable_mistakes: false,
            mistake_chance: 0.0,
            enable_tempo_sway: false,
            tempo_sway_intensity: 0.0,
            invert_tempo_sway: false,
            use_midi_pedal: false,
            use_velocity_accent: false,
            use_ai_pedal: true,
        }
    }
}
