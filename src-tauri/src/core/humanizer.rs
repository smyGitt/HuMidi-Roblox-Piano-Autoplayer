use std::collections::HashSet;
use std::f64::consts::PI;

use rand::Rng;

use crate::core::config::PlaybackConfig;
use crate::core::midi::get_time_group_indices;
use crate::core::models::{MusicalSection, Note};

const DRIFT_NOISE_SIGMA: f64 = 0.004;

pub fn round_resync_key(t: f64) -> i64 {
    (t * 100.0).round() as i64
}

fn sample_gaussian(rng: &mut impl Rng, mean: f64, std_dev: f64) -> f64 {
    if std_dev <= 0.0 {
        return mean;
    }
    let u1: f64 = rng.random_range(f64::EPSILON..1.0);
    let u2: f64 = rng.random::<f64>();
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
    mean + z0 * std_dev
}

pub struct Humanizer {
    config: PlaybackConfig,
    pub left_hand_drift: f64,
    pub right_hand_drift: f64,
}

impl Humanizer {
    pub fn new(config: PlaybackConfig) -> Self {
        Humanizer {
            config,
            left_hand_drift: 0.0,
            right_hand_drift: 0.0,
        }
    }

    pub fn apply_to_hand(&mut self, notes: &mut [Note], hand: &str, resync_points: &HashSet<i64>) {
        let vary_timing = self.config.vary_timing;
        let vary_articulation = self.config.vary_articulation;
        let enable_drift = self.config.enable_drift_correction;
        let enable_chord_roll = self.config.enable_chord_roll;

        if !(vary_timing || vary_articulation || enable_drift || enable_chord_roll) {
            return;
        }

        let groups = get_time_group_indices(notes);
        let timing_sigma = if vary_timing {
            self.config.timing_variance
        } else {
            0.0
        };

        let mut rng = rand::rng();

        for group in groups {
            let first_start = notes[group[0]].start_time;
            let is_resync = resync_points.contains(&round_resync_key(first_start));

            if enable_drift && is_resync {
                let decay = self.config.drift_decay_factor;
                if hand == "left" {
                    self.left_hand_drift *= decay;
                } else {
                    self.right_hand_drift *= decay;
                }
            }

            let mut timing_offset = 0.0;
            if vary_timing {
                timing_offset = sample_gaussian(&mut rng, 0.0, timing_sigma);
                timing_offset = timing_offset.clamp(-3.0 * timing_sigma, 3.0 * timing_sigma);
            }

            if enable_chord_roll && group.len() > 1 {
                let ascending = rng.random::<f64>() > 0.2;
                let stagger = rng.random_range(0.004..0.010);
                let mut sorted_group = group.clone();
                sorted_group.sort_by(|&a, &b| {
                    let pitch_a = notes[a].pitch;
                    let pitch_b = notes[b].pitch;
                    if ascending {
                        pitch_a.cmp(&pitch_b)
                    } else {
                        pitch_b.cmp(&pitch_a)
                    }
                });
                for (i, &idx) in sorted_group.iter().enumerate() {
                    notes[idx].start_time += i as f64 * stagger;
                }
            }

            let articulation_scale = if vary_articulation {
                let base = self.config.articulation;
                Some(base - rng.random::<f64>() * 0.1)
            } else {
                None
            };

            let current_drift = if hand == "left" {
                self.left_hand_drift
            } else {
                self.right_hand_drift
            };

            for &idx in &group {
                notes[idx].start_time += timing_offset;
                if enable_drift {
                    notes[idx].start_time += current_drift;
                }
                if let Some(scale) = articulation_scale {
                    notes[idx].duration = (notes[idx].duration * scale).max(0.03);
                }
            }

            if enable_drift {
                let drift_noise = sample_gaussian(&mut rng, 0.0, DRIFT_NOISE_SIGMA);
                if hand == "left" {
                    self.left_hand_drift += drift_noise;
                } else {
                    self.right_hand_drift += drift_noise;
                }
            }
        }
    }

    pub fn apply_tempo_rubato(&mut self, all_notes: &mut [Note], sections: &[MusicalSection]) {
        if !self.config.enable_tempo_sway {
            return;
        }

        let base_intensity = self.config.tempo_sway_intensity;
        let invert_sway = self.config.invert_tempo_sway;

        let mut note_index: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for (i, note) in all_notes.iter().enumerate() {
            note_index.insert(note.id, i);
        }

        for section in sections {
            let section_duration = section.end_time - section.start_time;
            if section_duration < 1.0 {
                continue;
            }

            let pace_multiplier = match section.pace_label.as_str() {
                "fast" => {
                    if invert_sway {
                        1.5
                    } else {
                        0.25
                    }
                }
                "slow" => {
                    if invert_sway {
                        0.25
                    } else {
                        1.5
                    }
                }
                _ => 1.0,
            };

            let intensity = base_intensity * pace_multiplier;

            for note in &section.notes {
                if let Some(&idx) = note_index.get(&note.id) {
                    let rel_pos = (note.start_time - section.start_time) / section_duration;
                    let shift = (rel_pos * PI).sin() * intensity;
                    all_notes[idx].start_time -= shift;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
