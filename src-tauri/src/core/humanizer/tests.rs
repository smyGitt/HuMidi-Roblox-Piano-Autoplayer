use super::*;

fn base_config() -> PlaybackConfig {
    PlaybackConfig {
        vary_timing: false,
        vary_articulation: false,
        enable_drift_correction: false,
        enable_chord_roll: false,
        enable_tempo_sway: false,
        ..PlaybackConfig::default()
    }
}

fn note(id: i32, pitch: i32, start_time: f64, duration: f64, hand: &str) -> Note {
    Note {
        id,
        pitch,
        velocity: 64,
        start_time,
        duration,
        hand: hand.to_string(),
        original_track_index: -1,
        channel: -1,
    }
}

fn notes_n(n: i32, hand: &str, step: f64, duration: f64) -> Vec<Note> {
    (0..n)
        .map(|i| note(i, 60, i as f64 * step, duration, hand))
        .collect()
}

mod test_no_features {
    use super::*;

    #[test]
    fn test_skips_mutation_entirely() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let orig_times: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let orig_durs: Vec<f64> = notes.iter().map(|n| n.duration).collect();
        Humanizer::new(base_config()).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig_times
        );
        assert_eq!(
            notes.iter().map(|n| n.duration).collect::<Vec<_>>(),
            orig_durs
        );
    }
}

mod test_articulation {
    use super::*;

    #[test]
    fn test_duration_floor_never_below_30ms() {
        let mut notes = notes_n(20, "left", 0.5, 0.001);
        let cfg = PlaybackConfig {
            vary_articulation: true,
            articulation: 0.1,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!(notes.iter().all(|n| n.duration >= 0.030 - 1e-9));
    }

    #[test]
    fn test_duration_scaled_down() {
        let mut notes = notes_n(4, "left", 0.5, 1.0);
        let cfg = PlaybackConfig {
            vary_articulation: true,
            articulation: 0.80,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!(notes.iter().all(|n| n.duration < 1.0));
    }

    #[test]
    fn test_articulation_does_not_shift_start_times() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            vary_articulation: true,
            articulation: 0.80,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_articulation_scale_is_same_formula_as_python() {
        let mut notes = vec![note(0, 60, 0.0, 1.0, "left")];
        let cfg = PlaybackConfig {
            vary_articulation: true,
            articulation: 0.95,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!(notes[0].duration >= 0.85 - 1e-9 && notes[0].duration <= 0.95 + 1e-9);
    }
}

mod test_timing {
    use super::*;

    #[test]
    fn test_vary_timing_shifts_start_times() {
        let mut notes = notes_n(6, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            vary_timing: true,
            timing_variance: 0.02,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_ne!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_timing_clamped_within_3sigma() {
        let sigma = 0.02;
        let mut notes = notes_n(50, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            vary_timing: true,
            timing_variance: sigma,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        for (original, n) in orig.iter().zip(notes.iter()) {
            assert!((n.start_time - original).abs() <= 3.0 * sigma + 1e-9);
        }
    }

    #[test]
    fn test_group_atomicity_same_offset_per_group() {
        let mut notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 0.0, 0.5, "left")];
        let cfg = PlaybackConfig {
            vary_timing: true,
            timing_variance: 0.02,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!((notes[0].start_time - notes[1].start_time).abs() < 1e-9);
    }

    #[test]
    fn test_zero_sigma_never_shifts() {
        let mut notes = notes_n(10, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            vary_timing: true,
            timing_variance: 0.0,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_distinct_groups_can_receive_different_offsets() {
        let mut notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 60, 5.0, 0.5, "left"),
            note(2, 60, 10.0, 0.5, "left"),
            note(3, 60, 15.0, 0.5, "left"),
            note(4, 60, 20.0, 0.5, "left"),
        ];
        let cfg = PlaybackConfig {
            vary_timing: true,
            timing_variance: 0.02,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        let offsets: Vec<f64> = notes
            .iter()
            .zip([0.0, 5.0, 10.0, 15.0, 20.0])
            .map(|(n, orig)| n.start_time - orig)
            .collect();
        assert!(offsets.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-9));
    }
}

mod test_drift {
    use super::*;

    #[test]
    fn test_drift_decays_to_zero_at_resync_with_zero_factor() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let cfg = PlaybackConfig {
            enable_drift_correction: true,
            drift_decay_factor: 0.0,
            ..base_config()
        };
        let mut h = Humanizer::new(cfg);
        h.left_hand_drift = 0.5;
        let resync: HashSet<i64> = [round_resync_key(notes[0].start_time)].into_iter().collect();
        h.apply_to_hand(&mut notes, "left", &resync);
        assert!((h.left_hand_drift - 0.5).abs() > 1e-9);
    }

    #[test]
    fn test_drift_affects_start_times() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            enable_drift_correction: true,
            drift_decay_factor: 1.0,
            ..base_config()
        };
        let mut h = Humanizer::new(cfg);
        h.left_hand_drift = 0.1;
        h.apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_ne!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_left_and_right_drift_independent() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let cfg = PlaybackConfig {
            enable_drift_correction: true,
            drift_decay_factor: 1.0,
            ..base_config()
        };
        let mut h = Humanizer::new(cfg);
        h.left_hand_drift = 0.1;
        h.right_hand_drift = 0.0;
        h.apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(h.right_hand_drift, 0.0);
        assert!((h.left_hand_drift - 0.1).abs() > 1e-9);
    }

    #[test]
    fn test_decay_not_applied_without_resync() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let cfg = PlaybackConfig {
            enable_drift_correction: true,
            drift_decay_factor: 0.0,
            ..base_config()
        };
        let mut h = Humanizer::new(cfg);
        h.left_hand_drift = 0.5;
        h.apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!(h.left_hand_drift.abs() > 0.4);
    }

    #[test]
    fn test_every_note_in_group_gets_current_drift_before_noise_added() {
        let mut notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 0.0, 0.5, "left")];
        let cfg = PlaybackConfig {
            enable_drift_correction: true,
            drift_decay_factor: 1.0,
            ..base_config()
        };
        let mut h = Humanizer::new(cfg);
        h.left_hand_drift = 0.2;
        h.apply_to_hand(&mut notes, "left", &HashSet::new());
        assert!((notes[0].start_time - 0.2).abs() < 1e-9);
        assert!((notes[1].start_time - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_resync_key_rounds_to_two_decimals() {
        assert_eq!(round_resync_key(1.004), 100);
        assert_eq!(round_resync_key(1.006), 101);
        assert_eq!(round_resync_key(0.0), 0);
    }
}

mod test_chord_roll {
    use super::*;

    #[test]
    fn test_chord_roll_staggers_simultaneous_notes() {
        let mut notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 0.0, 0.5, "left")];
        let cfg = PlaybackConfig {
            enable_chord_roll: true,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_ne!(notes[0].start_time, notes[1].start_time);
    }

    #[test]
    fn test_chord_roll_does_not_stagger_single_note_groups() {
        let mut notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 1.0, 0.5, "left")];
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let cfg = PlaybackConfig {
            enable_chord_roll: true,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_chord_roll_stagger_is_within_documented_range() {
        let mut notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 0.0, 0.5, "left")];
        let cfg = PlaybackConfig {
            enable_chord_roll: true,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        let gap = (notes[0].start_time - notes[1].start_time).abs();
        assert!(gap >= 0.004 - 1e-9 && gap < 0.010 + 1e-9);
    }

    #[test]
    fn test_chord_roll_three_notes_produces_three_distinct_times() {
        let mut notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 64, 0.0, 0.5, "left"),
            note(2, 67, 0.0, 0.5, "left"),
        ];
        let cfg = PlaybackConfig {
            enable_chord_roll: true,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        let mut times: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!(times[1] > times[0] + 1e-9);
        assert!(times[2] > times[1] + 1e-9);
    }

    #[test]
    fn test_chord_roll_preserves_pitch_to_note_identity() {
        let mut notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 64, 0.0, 0.5, "left"),
            note(2, 67, 0.0, 0.5, "left"),
        ];
        let cfg = PlaybackConfig {
            enable_chord_roll: true,
            ..base_config()
        };
        Humanizer::new(cfg).apply_to_hand(&mut notes, "left", &HashSet::new());
        assert_eq!(notes[0].id, 0);
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[1].id, 1);
        assert_eq!(notes[1].pitch, 64);
        assert_eq!(notes[2].id, 2);
        assert_eq!(notes[2].pitch, 67);
    }
}

mod test_tempo_rubato {
    use super::*;

    #[test]
    fn test_disabled_no_mutation() {
        let mut notes = notes_n(4, "left", 0.5, 0.5);
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 2.0, notes.clone())
        };
        Humanizer::new(base_config()).apply_tempo_rubato(&mut notes, &[section]);
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_enabled_shifts_notes() {
        let mut notes: Vec<Note> = (0..4)
            .map(|i| note(i, 60, i as f64 / 3.0, 0.3, "left"))
            .collect();
        let orig_mid = notes[2].start_time;
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 3.0, notes.clone())
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.1,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[2].start_time - orig_mid).abs() > 1e-9);
    }

    #[test]
    fn test_short_section_skipped() {
        let mut notes: Vec<Note> = (0..3)
            .map(|i| note(i, 60, i as f64 * 0.1, 0.5, "left"))
            .collect();
        let orig: Vec<f64> = notes.iter().map(|n| n.start_time).collect();
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 0.2, notes.clone())
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.5,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert_eq!(
            notes.iter().map(|n| n.start_time).collect::<Vec<_>>(),
            orig
        );
    }

    #[test]
    fn test_invert_changes_direction() {
        let mut notes_a: Vec<Note> = (0..4)
            .map(|i| note(i, 60, i as f64 / 3.0, 0.3, "left"))
            .collect();
        let mut notes_b = notes_a.clone();
        let section_a = MusicalSection {
            pace_label: "fast".to_string(),
            ..MusicalSection::new(0.0, 3.0, notes_a.clone())
        };
        let section_b = MusicalSection {
            pace_label: "fast".to_string(),
            ..MusicalSection::new(0.0, 3.0, notes_b.clone())
        };
        let cfg_base = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.1,
            invert_tempo_sway: false,
            ..base_config()
        };
        let cfg_invert = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.1,
            invert_tempo_sway: true,
            ..base_config()
        };
        Humanizer::new(cfg_base).apply_tempo_rubato(&mut notes_a, &[section_a]);
        Humanizer::new(cfg_invert).apply_tempo_rubato(&mut notes_b, &[section_b]);
        assert!((notes_a[2].start_time - notes_b[2].start_time).abs() > 1e-9);
    }

    #[test]
    fn test_normal_pace_uses_multiplier_one() {
        let intensity = 0.2;
        let n_mid = note(0, 60, 1.0, 0.1, "left");
        let n_start = note(1, 60, 0.0, 0.1, "left");
        let mut notes = vec![n_start.clone(), n_mid.clone()];
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![n_start, n_mid])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: intensity,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[0].start_time - 0.0).abs() < 1e-9);
        assert!((notes[1].start_time - (1.0 - intensity)).abs() < 1e-9);
    }

    #[test]
    fn test_fast_pace_uses_quarter_intensity_when_not_inverted() {
        let n_mid = note(0, 60, 1.0, 0.1, "left");
        let n_start = note(1, 60, 0.0, 0.1, "left");
        let mut notes = vec![n_start.clone(), n_mid.clone()];
        let intensity = 0.2;
        let section = MusicalSection {
            pace_label: "fast".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![n_start, n_mid])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: intensity,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[1].start_time - (1.0 - intensity * 0.25)).abs() < 1e-9);
    }

    #[test]
    fn test_slow_pace_uses_one_point_five_intensity_when_not_inverted() {
        let n_mid = note(0, 60, 1.0, 0.1, "left");
        let n_start = note(1, 60, 0.0, 0.1, "left");
        let mut notes = vec![n_start.clone(), n_mid.clone()];
        let intensity = 0.2;
        let section = MusicalSection {
            pace_label: "slow".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![n_start, n_mid])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: intensity,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[1].start_time - (1.0 - intensity * 1.5)).abs() < 1e-9);
    }

    #[test]
    fn test_note_not_in_note_map_is_skipped() {
        let mut notes = vec![note(0, 60, 1.0, 0.1, "left")];
        let foreign_note = note(999, 60, 1.0, 0.1, "left");
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![foreign_note])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.2,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[0].start_time - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_boundary_note_at_end_of_section_shifts_by_zero() {
        let n_end = note(0, 60, 2.0, 0.1, "left");
        let mut notes = vec![n_end.clone()];
        let section = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![n_end])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.5,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section]);
        assert!((notes[0].start_time - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_multiple_sections_each_apply_independently() {
        let mut notes: Vec<Note> = vec![
            note(0, 60, 1.0, 0.1, "left"),
            note(1, 60, 4.0, 0.1, "left"),
        ];
        let section_a = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(0.0, 2.0, vec![notes[0].clone()])
        };
        let section_b = MusicalSection {
            pace_label: "normal".to_string(),
            ..MusicalSection::new(3.0, 5.0, vec![notes[1].clone()])
        };
        let cfg = PlaybackConfig {
            enable_tempo_sway: true,
            tempo_sway_intensity: 0.2,
            invert_tempo_sway: false,
            ..base_config()
        };
        Humanizer::new(cfg).apply_tempo_rubato(&mut notes, &[section_a, section_b]);
        assert!((notes[0].start_time - 0.8).abs() < 1e-9);
        assert!((notes[1].start_time - 3.8).abs() < 1e-9);
    }
}

mod test_sample_gaussian {
    use super::*;

    #[test]
    fn test_zero_std_dev_returns_mean() {
        let mut rng = rand::rng();
        assert_eq!(sample_gaussian(&mut rng, 5.0, 0.0), 5.0);
    }

    #[test]
    fn test_negative_std_dev_returns_mean() {
        let mut rng = rand::rng();
        assert_eq!(sample_gaussian(&mut rng, 5.0, -1.0), 5.0);
    }

    #[test]
    fn test_distribution_mean_and_spread_are_plausible() {
        let mut rng = rand::rng();
        let samples: Vec<f64> = (0..20_000)
            .map(|_| sample_gaussian(&mut rng, 0.0, 1.0))
            .collect();
        let mean: f64 = samples.iter().sum::<f64>() / samples.len() as f64;
        let variance: f64 =
            samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;
        assert!(mean.abs() < 0.05, "mean was {mean}");
        assert!((variance - 1.0).abs() < 0.1, "variance was {variance}");
    }
}
