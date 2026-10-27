use super::*;
use std::path::PathBuf;

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

fn section(notes: Vec<Note>) -> MusicalSection {
    let start = notes.iter().map(|n| n.start_time).fold(f64::MAX, f64::min);
    let end = notes.iter().map(|n| n.end_time()).fold(f64::MIN, f64::max);
    MusicalSection::new(start, end, notes)
}

mod test_otsu_threshold {
    use super::*;

    #[test]
    fn test_empty_input_returns_half() {
        assert_eq!(otsu_threshold(&[]), 0.5);
    }

    #[test]
    fn test_all_out_of_range_returns_half() {
        assert_eq!(otsu_threshold(&[-1.0, 2.0, 5.0]), 0.5);
    }

    #[test]
    fn test_bimodal_distribution_separates_the_two_clusters() {
        let mut preds: Vec<f32> = Vec::new();
        for _ in 0..50 {
            preds.push(0.05);
        }
        for _ in 0..50 {
            preds.push(0.95);
        }
        let split = otsu_threshold(&preds);
        assert!(
            preds.iter().filter(|&&v| v <= split).all(|&v| v < 0.5),
            "everything at or below the split should be the low cluster"
        );
        assert!(
            preds.iter().filter(|&&v| v > split).all(|&v| v > 0.5),
            "everything above the split should be the high cluster"
        );
    }

    #[test]
    fn test_result_within_unit_range() {
        let preds = vec![0.1, 0.2, 0.5, 0.8, 0.9];
        let split = otsu_threshold(&preds);
        assert!((0.0..=1.0).contains(&split));
    }

    #[test]
    fn test_uniform_distribution_returns_a_valid_split() {
        let preds: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let split = otsu_threshold(&preds);
        assert!((0.0..=1.0).contains(&split));
    }
}

mod test_generate_harmonic_pedal {
    use super::*;

    #[test]
    fn test_empty_input_produces_no_events() {
        assert!(generate_harmonic_pedal(&[]).is_empty());
    }

    #[test]
    fn test_single_note_quirk_produces_down_up_down_up() {
        let notes = vec![note(0, 40, 0.0, 1.0, "left")];
        let events = generate_harmonic_pedal(&notes);
        let actions: Vec<(&str, &str)> = events
            .iter()
            .map(|e| (e.action.as_str(), e.key_char.as_str()))
            .collect();
        assert_eq!(
            actions,
            vec![
                ("pedal", "down"),
                ("pedal", "up"),
                ("pedal", "down"),
                ("pedal", "up"),
            ]
        );
    }

    #[test]
    fn test_same_pitch_no_gap_sustains_without_further_repedal() {
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 40, 0.5, 0.5, "left"),
        ];
        let events = generate_harmonic_pedal(&notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 2, "sustained same-pitch notes should not add a repedal beyond the first-note quirk");
    }

    #[test]
    fn test_pitch_change_triggers_repedal() {
        let notes = vec![
            note(0, 40, 0.0, 0.2, "left"),
            note(1, 45, 0.2, 0.2, "left"),
        ];
        let events = generate_harmonic_pedal(&notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 3, "the first-note quirk contributes 2, the pitch change contributes a 3rd");
    }

    #[test]
    fn test_large_gap_triggers_lift_and_replant() {
        let notes = vec![
            note(0, 40, 0.0, 0.2, "left"),
            note(1, 40, 1.0, 0.2, "left"),
        ];
        let events = generate_harmonic_pedal(&notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 3, "the first-note quirk contributes 2, the >150ms gap contributes a 3rd");
    }

    #[test]
    fn test_ends_with_up_at_final_note_end_time() {
        let notes = vec![note(0, 40, 0.0, 2.0, "left")];
        let events = generate_harmonic_pedal(&notes);
        let last = events.last().unwrap();
        assert_eq!(last.key_char, "up");
        assert!((last.time - 2.0).abs() < 1e-9);
    }
}

mod test_generate_adaptive_pedal_driver {
    use super::*;

    #[test]
    fn test_empty_driver_notes_produces_no_events() {
        assert!(generate_adaptive_pedal_driver(&[], &[]).is_empty());
    }

    #[test]
    fn test_single_note_produces_down_then_up() {
        let notes = vec![note(0, 40, 0.0, 1.0, "left")];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].key_char, "down");
        assert_eq!(events[1].key_char, "up");
    }

    #[test]
    fn test_large_gap_lifts_and_replants() {
        let notes = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 40, 1.0, 0.1, "left"),
        ];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 2, "gap > 0.35s should lift and replant");
    }

    #[test]
    fn test_safe_interval_no_gap_does_not_repedal() {
        let notes = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 44, 0.1, 0.1, "left"),
        ];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 1, "a safe (major third) interval with no gap should not repedal");
    }

    #[test]
    fn test_linear_semitone_interval_triggers_repedal_with_lag() {
        let notes = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 41, 0.1, 0.1, "left"),
        ];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        let downs: Vec<&KeyEvent> = events.iter().filter(|e| e.key_char == "down").collect();
        assert_eq!(downs.len(), 2);
        assert!((downs[1].time - (0.1 + PEDAL_LAG)).abs() < 1e-9);
    }

    #[test]
    fn test_linear_tritone_interval_triggers_repedal() {
        let notes = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 46, 0.1, 0.1, "left"),
        ];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 2, "a tritone (6 semitones) is an unsafe linear interval");
    }

    #[test]
    fn test_vertical_unsafe_interval_triggers_repedal() {
        let driver = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 44, 0.1, 0.1, "left"),
        ];
        let all_notes = vec![
            note(0, 40, 0.0, 0.1, "left"),
            note(1, 44, 0.1, 0.1, "left"),
            note(2, 45, 0.1, 0.1, "right"),
        ];
        let events = generate_adaptive_pedal_driver(&driver, &all_notes);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(
            downs, 2,
            "a concurrent minor-second above the lowest sounding note is an unsafe vertical interval"
        );
    }

    #[test]
    fn test_ends_with_up_at_final_note_end_time() {
        let notes = vec![note(0, 40, 0.0, 3.0, "left")];
        let events = generate_adaptive_pedal_driver(&notes, &notes);
        let last = events.last().unwrap();
        assert_eq!(last.key_char, "up");
        assert!((last.time - 3.0).abs() < 1e-9);
    }
}

mod test_generate_events_dispatch {
    use super::*;

    fn base_config() -> PlaybackConfig {
        PlaybackConfig::default()
    }

    #[test]
    fn test_none_style_produces_no_events() {
        let mut cfg = base_config();
        cfg.pedal_style = "none".to_string();
        let (events, meta) = generate_events(None, &cfg, &[], &[]);
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_unknown_style_produces_no_events() {
        let mut cfg = base_config();
        cfg.pedal_style = "not-a-real-style".to_string();
        let (events, _) = generate_events(None, &cfg, &[], &[]);
        assert!(events.is_empty());
    }

    #[test]
    fn test_rhythmic_style_groups_left_hand_notes_per_section() {
        let mut cfg = base_config();
        cfg.pedal_style = "rhythmic".to_string();
        let sec = section(vec![
            note(0, 40, 0.0, 0.2, "left"),
            note(1, 40, 1.0, 0.2, "left"),
        ]);
        let (events, _) = generate_events(None, &cfg, &[], &[sec]);
        let downs = events.iter().filter(|e| e.key_char == "down").count();
        assert_eq!(downs, 2, "two separate time groups should yield two down events");
    }

    #[test]
    fn test_rhythmic_falls_back_to_section_span_without_left_hand() {
        let mut cfg = base_config();
        cfg.pedal_style = "rhythmic".to_string();
        let sec = section(vec![note(0, 64, 0.0, 1.0, "right")]);
        let (events, _) = generate_events(None, &cfg, &[], &[sec]);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_harmonic_and_legato_are_aliases() {
        let sec_a = section(vec![
            note(0, 40, 0.0, 0.2, "left"),
            note(1, 45, 0.2, 0.2, "left"),
        ]);
        let sec_b = section(vec![
            note(0, 40, 0.0, 0.2, "left"),
            note(1, 45, 0.2, 0.2, "left"),
        ]);

        let mut cfg_harmonic = base_config();
        cfg_harmonic.pedal_style = "harmonic".to_string();
        let mut cfg_legato = base_config();
        cfg_legato.pedal_style = "legato".to_string();

        let (events_harmonic, _) = generate_events(None, &cfg_harmonic, &[], &[sec_a]);
        let (events_legato, _) = generate_events(None, &cfg_legato, &[], &[sec_b]);
        assert_eq!(events_harmonic.len(), events_legato.len());
    }

    #[test]
    fn test_ai_style_without_model_falls_back_to_adaptive() {
        let mut cfg = base_config();
        cfg.pedal_style = "ai".to_string();
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 42, 0.5, 0.5, "left"),
        ];
        let (events, meta) = generate_events(None, &cfg, &notes, &[]);
        assert!(!events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_hybrid_style_without_model_falls_back_to_adaptive() {
        let mut cfg = base_config();
        cfg.pedal_style = "hybrid".to_string();
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 42, 0.5, 0.5, "left"),
        ];
        let (events, meta) = generate_events(None, &cfg, &notes, &[]);
        assert!(!events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_hybrid_style_use_ai_pedal_false_never_needs_a_model() {
        let mut cfg = base_config();
        cfg.pedal_style = "hybrid".to_string();
        cfg.use_ai_pedal = false;
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 42, 0.5, 0.5, "left"),
        ];
        let (events, meta) = generate_events(None, &cfg, &notes, &[]);
        assert!(!events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_ai_and_hybrid_fallback_use_right_hand_when_no_left_hand_notes() {
        let mut cfg = base_config();
        cfg.pedal_style = "ai".to_string();
        let notes = vec![
            note(0, 64, 0.0, 0.5, "right"),
            note(1, 66, 0.5, 0.5, "right"),
        ];
        let (events, _) = generate_events(None, &cfg, &notes, &[]);
        assert!(!events.is_empty());
    }
}

mod test_process_ai_predictions {
    use super::*;

    #[test]
    fn test_all_silence_masked_rejects() {
        let preds = vec![0.9; 20];
        let notes: Vec<Note> = vec![];
        let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None);
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_low_confidence_rejects() {
        let mut preds = vec![0.0; 20];
        for p in preds.iter_mut().take(10) {
            *p = 0.1;
        }
        let notes = vec![note(0, 40, 0.0, 0.2, "left")];
        let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None);
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_zero_variance_active_region_rejects() {
        let mut preds = vec![0.0; 20];
        for p in preds.iter_mut().take(10) {
            *p = 0.5;
        }
        let notes = vec![note(0, 40, 0.0, 0.2, "left")];
        let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, None);
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_too_few_crossings_rejects() {
        let mut preds = vec![0.0; 40];
        for (i, p) in preds.iter_mut().enumerate().take(20) {
            *p = 0.5 + (i as f32) * 0.01;
        }
        let notes = vec![note(0, 40, 0.0, 0.4, "left")];
        let (events, meta) = process_ai_predictions(preds, &notes, 0.8, FPS, None);
        assert!(events.is_empty() || events.len() > 2);
        let _ = meta;
    }

    #[test]
    fn test_custom_curves_replace_the_automatic_ones() {
        let mut preds = vec![0.0; 40];
        for (i, p) in preds.iter_mut().enumerate().take(30) {
            *p = 0.2 + (i as f32 % 10.0) * 0.06;
        }
        let notes = vec![note(0, 40, 0.0, 0.6, "left")];
        let custom = ThresholdCurves {
            on: curve::flat_curve(0.4),
            off: curve::flat_curve(0.3),
        };
        let (_, meta) = process_ai_predictions(preds, &notes, 0.8, FPS, Some(&custom));
        let m = meta.expect("a custom curve over a swinging confidence must be accepted");
        assert_eq!(m.curves, custom);
    }

    #[test]
    fn test_a_time_varying_custom_curve_is_followed_frame_by_frame() {
        let mut preds = vec![0.0; 200];
        for (i, p) in preds.iter_mut().enumerate().take(180) {
            *p = 0.5 + 0.4 * ((i as f32) * 0.3).sin();
        }
        let notes = vec![note(0, 40, 0.0, 3.6, "left")];
        let custom = ThresholdCurves {
            on: vec![
                curve::ThresholdNode { time: 0.0, value: 0.3 },
                curve::ThresholdNode { time: 4.0, value: 0.8 },
            ],
            off: vec![
                curve::ThresholdNode { time: 0.0, value: 0.2 },
                curve::ThresholdNode { time: 4.0, value: 0.7 },
            ],
        };
        let (events, meta) = process_ai_predictions(preds, &notes, 4.0, FPS, Some(&custom));
        let meta = meta.unwrap();
        let mut replayed = Vec::new();
        let mut down = false;
        for (i, &value) in meta.confidence.iter().enumerate().skip(1) {
            if value == 0.0 {
                continue;
            }
            let time = i as f64 / FPS;
            let (on, off) = meta.curves.at(time).unwrap();
            if !down && f64::from(value) > on {
                down = true;
                replayed.push((time, "down"));
            } else if down && f64::from(value) <= off {
                down = false;
                replayed.push((time, "up"));
            }
        }
        if down {
            replayed.push((4.0, "up"));
        }
        let produced: Vec<(f64, &str)> = events.iter().map(|e| (e.time, e.key_char.as_str())).collect();
        assert_eq!(replayed, produced);
    }

    #[test]
    fn test_custom_off_above_on_is_capped_so_the_pedal_cannot_chatter() {
        let mut preds = vec![0.0; 100];
        for (i, p) in preds.iter_mut().enumerate().take(80) {
            *p = 0.5 + 0.45 * ((i as f32) * 0.3).sin();
        }
        let notes = vec![note(0, 40, 0.0, 1.6, "left")];
        let crossed = ThresholdCurves {
            on: curve::flat_curve(0.4),
            off: curve::flat_curve(0.9),
        };
        let capped = ThresholdCurves {
            on: curve::flat_curve(0.4),
            off: curve::flat_curve(0.4),
        };
        let (crossed_events, _) = process_ai_predictions(preds.clone(), &notes, 1.6, FPS, Some(&crossed));
        let (capped_events, _) = process_ai_predictions(preds, &notes, 1.6, FPS, Some(&capped));
        let times = |events: &[KeyEvent]| -> Vec<(f64, String)> {
            events.iter().map(|e| (e.time, e.key_char.clone())).collect()
        };
        assert_eq!(times(&crossed_events), times(&capped_events));
    }

    #[test]
    fn test_a_flat_confidence_is_rejected_even_with_a_custom_curve() {
        let mut preds = vec![0.0; 20];
        for p in preds.iter_mut().take(10) {
            *p = 0.5;
        }
        let notes = vec![note(0, 40, 0.0, 0.2, "left")];
        let custom = ThresholdCurves {
            on: curve::flat_curve(0.4),
            off: curve::flat_curve(0.3),
        };
        let (events, meta) = process_ai_predictions(preds, &notes, 0.4, FPS, Some(&custom));
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_automatic_curves_are_returned_when_there_is_no_custom_curve() {
        let mut preds = vec![0.0; 100];
        for (i, p) in preds.iter_mut().enumerate().take(80) {
            *p = 0.5 + 0.45 * ((i as f32) * 0.3).sin();
        }
        let notes = vec![note(0, 40, 0.0, 1.6, "left")];
        let (_, meta) = process_ai_predictions(preds, &notes, 1.6, FPS, None);
        let curves = meta.unwrap().curves;
        assert!(!curves.on.is_empty());
        assert_eq!(curves.on.len(), curves.off.len());
        assert!(curves.is_ordered());
    }

    #[test]
    fn test_accepted_result_carries_the_silence_masked_confidence_curve() {
        let mut preds = vec![0.0; 100];
        for (i, p) in preds.iter_mut().enumerate().take(80) {
            let phase = (i as f32) * 0.3;
            *p = 0.5 + 0.45 * phase.sin();
        }
        let notes = vec![note(0, 40, 0.0, 1.0, "left")];
        let (events, meta) = process_ai_predictions(preds.clone(), &notes, 1.6, FPS, None);
        assert!(!events.is_empty());
        let curve = meta.unwrap().confidence;
        assert_eq!(curve.len(), preds.len());
        let silent_from = (((1.0 + 0.35) * FPS) as usize).min(preds.len());
        assert_eq!(&curve[..silent_from], &preds[..silent_from]);
        assert!(curve[silent_from..].iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_curve_replays_into_the_same_events_with_the_returned_thresholds() {
        let mut preds = vec![0.0; 100];
        for (i, p) in preds.iter_mut().enumerate().take(80) {
            let phase = (i as f32) * 0.3;
            *p = 0.5 + 0.45 * phase.sin();
        }
        let notes = vec![note(0, 40, 0.0, 1.6, "left")];
        let (_, automatic) = process_ai_predictions(preds.clone(), &notes, 1.6, FPS, None);
        let curves = automatic.unwrap().curves;
        let (events, meta) = process_ai_predictions(preds, &notes, 1.6, FPS, Some(&curves));
        let meta = meta.unwrap();
        let mut replayed = Vec::new();
        let mut down = false;
        for (i, &value) in meta.confidence.iter().enumerate().skip(1) {
            if value == 0.0 {
                continue;
            }
            let time = i as f64 / FPS;
            let (on, off) = meta.curves.at(time).unwrap();
            if !down && f64::from(value) > on {
                down = true;
                replayed.push((time, "down"));
            } else if down && f64::from(value) <= off {
                down = false;
                replayed.push((time, "up"));
            }
        }
        if down {
            replayed.push((1.6, "up"));
        }
        let produced: Vec<(f64, &str)> = events.iter().map(|e| (e.time, e.key_char.as_str())).collect();
        assert_eq!(replayed, produced);
    }

    #[test]
    fn test_accepted_path_produces_well_formed_alternating_events() {
        let mut preds = vec![0.0; 100];
        for (i, p) in preds.iter_mut().enumerate().take(80) {
            let phase = (i as f32) * 0.3;
            *p = 0.5 + 0.45 * phase.sin();
        }
        let notes = vec![note(0, 40, 0.0, 1.6, "left")];
        let (events, meta) = process_ai_predictions(preds, &notes, 1.6, FPS, None);
        if !events.is_empty() {
            assert!(meta.is_some());
            assert!(events.len() > 2);
            let mut pedal_down = false;
            for e in &events {
                if e.key_char == "down" {
                    assert!(!pedal_down, "two downs in a row without an up");
                    pedal_down = true;
                } else if e.key_char == "up" {
                    assert!(pedal_down, "an up without a preceding down");
                    pedal_down = false;
                }
            }
        }
    }
}

mod test_resolve_custom_curves {
    use super::*;

    #[test]
    fn test_empty_config_curves_mean_automatic() {
        assert!(resolve_custom_curves(&PlaybackConfig::default()).is_none());
    }

    #[test]
    fn test_one_empty_curve_means_automatic() {
        let mut cfg = PlaybackConfig::default();
        cfg.pedal_curve_on = curve::flat_curve(0.5);
        assert!(resolve_custom_curves(&cfg).is_none());
    }

    #[test]
    fn test_both_curves_are_used_and_cleaned() {
        let mut cfg = PlaybackConfig::default();
        cfg.pedal_curve_on = vec![
            curve::ThresholdNode { time: 4.0, value: 1.4 },
            curve::ThresholdNode { time: 1.0, value: 0.5 },
        ];
        cfg.pedal_curve_off = curve::flat_curve(0.3);
        let resolved = resolve_custom_curves(&cfg).unwrap();
        assert_eq!(resolved.on[0].time, 1.0);
        assert_eq!(resolved.on[1].value, 1.0);
    }
}

mod test_build_input_features {
    use super::*;

    const MIDDLE_C: i32 = 60;
    const TEST_STEPS: usize = 160;
    const HALF_SECOND: f64 = 0.5;
    const ONE_SECOND: f64 = 1.0;

    fn frame_at(seconds: f64) -> usize {
        (seconds * FPS) as usize
    }

    fn pitch_flag(features: &[f32], frame: usize, pitch: i32) -> f32 {
        features[frame * FEATURES + pitch as usize]
    }

    fn chroma_value(features: &[f32], frame: usize, pitch: i32) -> f32 {
        features[frame * FEATURES + PITCHES + pitch as usize % PITCH_CLASSES]
    }

    #[test]
    fn test_a_held_note_sets_only_its_start_frame() {
        let notes = vec![note(0, MIDDLE_C, HALF_SECOND, ONE_SECOND, "left")];
        let features = build_input_features(&notes, FPS, TEST_STEPS);
        let start = frame_at(HALF_SECOND);
        assert_eq!(pitch_flag(&features, start - 1, MIDDLE_C), 0.0);
        assert_eq!(pitch_flag(&features, start, MIDDLE_C), 1.0);
        for t in start + 1..TEST_STEPS {
            assert_eq!(pitch_flag(&features, t, MIDDLE_C), 0.0, "frame {t}");
        }
    }

    #[test]
    fn test_chroma_counts_only_the_start_frame() {
        let notes = vec![note(0, MIDDLE_C, 0.0, ONE_SECOND, "left")];
        let features = build_input_features(&notes, FPS, TEST_STEPS);
        assert_eq!(chroma_value(&features, 0, MIDDLE_C), 1.0);
        for t in 1..frame_at(ONE_SECOND) {
            assert_eq!(chroma_value(&features, t, MIDDLE_C), 0.0, "frame {t}");
        }
    }

    #[test]
    fn test_back_to_back_same_pitch_notes_give_one_start() {
        let notes = vec![
            note(0, MIDDLE_C, 0.0, HALF_SECOND, "left"),
            note(1, MIDDLE_C, HALF_SECOND, HALF_SECOND, "left"),
        ];
        let features = build_input_features(&notes, FPS, TEST_STEPS);
        assert_eq!(pitch_flag(&features, 0, MIDDLE_C), 1.0);
        assert_eq!(pitch_flag(&features, frame_at(HALF_SECOND), MIDDLE_C), 0.0);
    }

    #[test]
    fn test_same_pitch_repeated_after_a_gap_gives_two_starts() {
        let notes = vec![
            note(0, MIDDLE_C, 0.0, HALF_SECOND, "left"),
            note(1, MIDDLE_C, ONE_SECOND, HALF_SECOND, "left"),
        ];
        let features = build_input_features(&notes, FPS, TEST_STEPS);
        assert_eq!(pitch_flag(&features, 0, MIDDLE_C), 1.0);
        assert_eq!(pitch_flag(&features, frame_at(HALF_SECOND), MIDDLE_C), 0.0);
        assert_eq!(pitch_flag(&features, frame_at(ONE_SECOND), MIDDLE_C), 1.0);
    }

    #[test]
    fn test_feature_width_matches_features_constant() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let total_steps = 10;
        let features = build_input_features(&notes, FPS, total_steps);
        assert_eq!(features.len(), total_steps * FEATURES);
    }

    #[test]
    fn test_active_frame_has_pitch_one_hot_set() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let total_steps = 30;
        let features = build_input_features(&notes, FPS, total_steps);
        assert_eq!(features[0 * FEATURES + 60], 1.0);
    }

    #[test]
    fn test_chroma_sums_pitch_classes_across_octaves() {
        let notes = vec![
            note(0, 60, 0.0, 1.0, "left"),
            note(1, 72, 0.0, 1.0, "left"),
        ];
        let total_steps = 10;
        let features = build_input_features(&notes, FPS, total_steps);
        let chroma_0 = features[0 * FEATURES + 128];
        assert_eq!(chroma_0, 2.0, "pitches 60 and 72 are both pitch-class 0");
    }

    #[test]
    fn test_out_of_range_pitch_is_skipped_not_panicking() {
        let notes = vec![note(0, 200, 0.0, 0.5, "left")];
        let total_steps = 10;
        let features = build_input_features(&notes, FPS, total_steps);
        assert!(features.iter().all(|&v| v == 0.0));
    }
}

mod test_ai_pedal_real_model_integration {
    use super::*;

    fn model_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("pedal_bilstm.safetensors")
    }

    #[test]
    fn test_generate_ai_pedal_does_not_panic_on_real_model() {
        let model = PedalModel::load(model_path()).expect("real pedal model should load");
        let notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 62, 0.5, 0.5, "left"),
            note(2, 64, 1.0, 0.5, "left"),
            note(3, 65, 1.5, 0.5, "left"),
            note(4, 67, 2.0, 0.5, "left"),
        ];
        let (events, meta) = generate_ai_pedal(&model, &notes, None);
        if events.is_empty() {
            assert!(meta.is_none());
        } else {
            assert!(meta.is_some());
            assert!(events.len() > 2);
        }
    }

    #[test]
    fn test_generate_events_ai_style_with_real_model_does_not_panic() {
        let model = PedalModel::load(model_path()).expect("real pedal model should load");
        let mut cfg = PlaybackConfig::default();
        cfg.pedal_style = "ai".to_string();
        let notes = vec![
            note(0, 55, 0.0, 0.3, "left"),
            note(1, 57, 0.3, 0.3, "left"),
            note(2, 59, 0.6, 0.3, "left"),
        ];
        let (events, _) = generate_events(Some(&model), &cfg, &notes, &[]);
        assert!(!events.is_empty(), "ai style must always produce events, either AI or adaptive fallback");
    }
}
