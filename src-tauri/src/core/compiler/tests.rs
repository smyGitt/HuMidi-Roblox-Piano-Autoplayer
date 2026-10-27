use super::*;

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

fn base_config() -> PlaybackConfig {
    PlaybackConfig::default()
}

mod test_compile_note_events {
    use super::*;

    #[test]
    fn test_empty_notes_produces_empty_events() {
        let (events, all_notes) = compile_note_events(&base_config(), &[], &[]);
        assert!(events.is_empty());
        assert!(all_notes.is_empty());
    }

    #[test]
    fn test_unknown_hand_notes_are_dropped() {
        let notes = vec![
            note(0, 60, 0.0, 0.5, "unknown"),
            note(1, 62, 1.0, 0.5, "left"),
        ];
        let (_, all_notes) = compile_note_events(&base_config(), &notes, &[]);
        assert_eq!(all_notes.len(), 1);
        assert_eq!(all_notes[0].id, 1);
    }

    #[test]
    fn test_each_mapped_note_produces_press_and_release() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].action, "press");
        assert_eq!(events[1].action, "release");
    }

    #[test]
    fn test_events_are_sorted_by_time() {
        let notes = vec![
            note(0, 60, 1.0, 0.1, "left"),
            note(1, 62, 0.0, 0.1, "right"),
        ];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        for w in events.windows(2) {
            assert!(w[0].time <= w[1].time);
        }
    }

    #[test]
    fn test_press_event_carries_pitch_and_velocity() {
        let mut n = note(0, 60, 0.0, 0.5, "left");
        n.velocity = 100;
        let (events, _) = compile_note_events(&base_config(), &[n], &[]);
        let press = &events[0];
        assert_eq!(press.pitch, Some(60));
        assert_eq!(press.velocity, Some(100));
    }

    #[test]
    fn test_release_event_has_no_velocity() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        let release = &events[1];
        assert_eq!(release.action, "release");
        assert_eq!(release.velocity, None);
    }

    #[test]
    fn test_release_time_is_note_end_time() {
        let notes = vec![note(0, 60, 1.0, 0.5, "left")];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        let release = &events[1];
        assert!((release.time - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_extreme_pitch_still_maps_via_octave_wrap() {
        let notes = vec![note(0, 200, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_press_priority_is_2_release_priority_is_4() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        assert_eq!(events[0].priority, 2);
        assert_eq!(events[1].priority, 4);
    }

    #[test]
    fn test_disabled_mistakes_never_injects_a_mistake_pitch() {
        let mut cfg = base_config();
        cfg.enable_mistakes = false;
        cfg.mistake_chance = 100.0;
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&cfg, &notes, &[]);
        assert_eq!(events[0].pitch, Some(60));
    }

    #[test]
    fn test_guaranteed_mistake_still_produces_a_press_release_pair() {
        let mut cfg = base_config();
        cfg.enable_mistakes = true;
        cfg.mistake_chance = 100.0;
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_note_events(&cfg, &notes, &[]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].action, "press");
        assert_eq!(events[1].action, "release");
    }

    #[test]
    fn test_multiple_notes_each_produce_a_pair() {
        let notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 62, 0.5, 0.5, "left"),
            note(2, 64, 1.0, 0.5, "right"),
        ];
        let (events, _) = compile_note_events(&base_config(), &notes, &[]);
        assert_eq!(events.len(), 6);
    }

    #[test]
    fn test_resync_points_computed_from_left_right_intersection() {
        let mut cfg = base_config();
        cfg.enable_drift_correction = true;
        cfg.drift_decay_factor = 0.0;
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 80, 0.0, 0.5, "right"),
        ];
        let (_, all_notes) = compile_note_events(&cfg, &notes, &[]);
        assert_eq!(all_notes.len(), 2);
    }
}

mod test_compile_note_events_with_played {
    use super::*;

    fn press_events(events: &[KeyEvent]) -> Vec<&KeyEvent> {
        events.iter().filter(|e| e.action == "press").collect()
    }

    #[test]
    fn test_one_played_note_per_press_event() {
        let notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 62, 0.5, 0.5, "left"),
            note(2, 64, 1.0, 0.5, "right"),
        ];
        let (events, _, played) = compile_note_events_with_played(&base_config(), &notes, &[]);
        assert_eq!(played.len(), press_events(&events).len());
    }

    #[test]
    fn test_played_notes_carry_the_humanized_timing() {
        let mut cfg = base_config();
        cfg.vary_timing = true;
        cfg.timing_variance = 0.05;
        let notes: Vec<Note> = (0..20)
            .map(|i| note(i, 60 + (i % 12), f64::from(i) * 0.5, 0.25, "right"))
            .collect();
        let (_, humanized, played) = compile_note_events_with_played(&cfg, &notes, &[]);
        let mut played_starts: Vec<f64> = played.iter().map(|n| n.start_time).collect();
        let mut humanized_starts: Vec<f64> = humanized.iter().map(|n| n.start_time).collect();
        played_starts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        humanized_starts.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(played_starts, humanized_starts);
    }

    #[test]
    fn test_played_note_start_and_end_match_its_press_and_release_events() {
        let notes = vec![note(0, 60, 1.0, 0.5, "left")];
        let (events, _, played) = compile_note_events_with_played(&base_config(), &notes, &[]);
        assert_eq!(played.len(), 1);
        assert!((played[0].start_time - events[0].time).abs() < 1e-9);
        assert!((played[0].end_time() - events[1].time).abs() < 1e-9);
    }

    #[test]
    fn test_played_note_keeps_its_hand() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 72, 0.0, 0.5, "right")];
        let (_, _, played) = compile_note_events_with_played(&base_config(), &notes, &[]);
        let hands: Vec<&str> = played.iter().map(|n| n.hand.as_str()).collect();
        assert!(hands.contains(&"left"));
        assert!(hands.contains(&"right"));
    }

    #[test]
    fn test_unknown_hand_notes_are_not_played() {
        let notes = vec![note(0, 60, 0.0, 0.5, "unknown"), note(1, 62, 1.0, 0.5, "left")];
        let (_, _, played) = compile_note_events_with_played(&base_config(), &notes, &[]);
        assert_eq!(played.len(), 1);
        assert_eq!(played[0].id, 1);
    }

    #[test]
    fn test_mistake_note_is_played_at_the_mistake_pitch() {
        let mut cfg = base_config();
        cfg.enable_mistakes = true;
        cfg.mistake_chance = 100.0;
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _, played) = compile_note_events_with_played(&cfg, &notes, &[]);
        assert_eq!(played.len(), 1);
        assert_eq!(Some(played[0].pitch), events[0].pitch);
        assert_ne!(played[0].pitch, 60);
    }

    #[test]
    fn test_compile_note_events_returns_the_same_events_and_humanized_notes() {
        let notes = vec![note(0, 60, 0.0, 0.5, "left"), note(1, 64, 0.5, 0.5, "right")];
        let summarize = |events: &[KeyEvent]| -> Vec<(f64, String, Option<i32>)> {
            events.iter().map(|e| (e.time, e.action.clone(), e.pitch)).collect()
        };
        let (events, humanized) = compile_note_events(&base_config(), &notes, &[]);
        let (events_with, humanized_with, _) = compile_note_events_with_played(&base_config(), &notes, &[]);
        assert_eq!(summarize(&events), summarize(&events_with));
        assert_eq!(humanized, humanized_with);
    }
}

mod test_compile_midi_pedal_events {
    use super::*;

    #[test]
    fn test_empty_input_produces_empty_output() {
        assert!(compile_midi_pedal_events(&[]).is_empty());
    }

    #[test]
    fn test_on_event_maps_to_down_priority_1() {
        let events = compile_midi_pedal_events(&[(1.0, true)]);
        assert_eq!(events[0].action, "pedal");
        assert_eq!(events[0].key_char, "down");
        assert_eq!(events[0].priority, 1);
    }

    #[test]
    fn test_off_event_maps_to_up_priority_0() {
        let events = compile_midi_pedal_events(&[(1.0, false)]);
        assert_eq!(events[0].key_char, "up");
        assert_eq!(events[0].priority, 0);
    }

    #[test]
    fn test_preserves_input_order() {
        let events = compile_midi_pedal_events(&[(1.0, true), (2.0, false), (3.0, true)]);
        let times: Vec<f64> = events.iter().map(|e| e.time).collect();
        assert_eq!(times, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_time_values_preserved_exactly() {
        let events = compile_midi_pedal_events(&[(0.123, true)]);
        assert!((events[0].time - 0.123).abs() < 1e-12);
    }
}

mod test_merge_compiled {
    use super::*;

    fn press(t: f64) -> KeyEvent {
        KeyEvent::new(t, 2, "press", "a")
    }

    fn pedal(t: f64, down: bool) -> KeyEvent {
        KeyEvent::new(t, if down { 1 } else { 0 }, "pedal", if down { "down" } else { "up" })
    }

    #[test]
    fn test_empty_inputs_produce_empty_output() {
        assert!(merge_compiled(&[], &[]).is_empty());
    }

    #[test]
    fn test_note_events_only_pass_through_sorted() {
        let notes = vec![press(1.0), press(0.0), press(2.0)];
        let merged = merge_compiled(&notes, &[]);
        let times: Vec<f64> = merged.iter().map(|e| e.time).collect();
        assert_eq!(times, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_pedal_events_only_pass_through_sorted() {
        let pedals = vec![pedal(1.0, true), pedal(0.0, false)];
        let merged = merge_compiled(&[], &pedals);
        let times: Vec<f64> = merged.iter().map(|e| e.time).collect();
        assert_eq!(times, vec![0.0, 1.0]);
    }

    #[test]
    fn test_interleaves_note_and_pedal_events_by_time() {
        let notes = vec![press(0.0), press(2.0)];
        let pedals = vec![pedal(1.0, true)];
        let merged = merge_compiled(&notes, &pedals);
        let times: Vec<f64> = merged.iter().map(|e| e.time).collect();
        assert_eq!(times, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn test_does_not_mutate_inputs() {
        let notes = vec![press(1.0), press(0.0)];
        let pedals = vec![pedal(0.5, true)];
        let _ = merge_compiled(&notes, &pedals);
        assert_eq!(notes[0].time, 1.0);
        assert_eq!(notes[1].time, 0.0);
        assert_eq!(pedals[0].time, 0.5);
    }

    #[test]
    fn test_result_length_is_sum_of_inputs() {
        let notes = vec![press(0.0), press(1.0), press(2.0)];
        let pedals = vec![pedal(0.5, true), pedal(1.5, false)];
        let merged = merge_compiled(&notes, &pedals);
        assert_eq!(merged.len(), 5);
    }
}

mod test_compile_pedal_events {
    use super::*;

    #[test]
    fn test_none_style_produces_no_pedal_events() {
        let cfg = base_config();
        let (events, meta) = compile_pedal_events(None, &cfg, &[], &[], None);
        assert!(events.is_empty());
        assert!(meta.is_none());
    }

    #[test]
    fn test_midi_pedal_bypass_used_when_enabled_and_present() {
        let mut cfg = base_config();
        cfg.use_midi_pedal = true;
        cfg.pedal_style = "rhythmic".to_string();
        let midi_events = vec![(1.0, true), (2.0, false)];
        let (events, meta) =
            compile_pedal_events(None, &cfg, &[], &[], Some(&midi_events));
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].key_char, "down");
        assert!(meta.is_none());
    }

    #[test]
    fn test_midi_pedal_ignored_when_flag_off() {
        let mut cfg = base_config();
        cfg.use_midi_pedal = false;
        cfg.pedal_style = "none".to_string();
        let midi_events = vec![(1.0, true), (2.0, false)];
        let (events, _) = compile_pedal_events(None, &cfg, &[], &[], Some(&midi_events));
        assert!(events.is_empty());
    }

    #[test]
    fn test_midi_pedal_ignored_when_events_empty() {
        let mut cfg = base_config();
        cfg.use_midi_pedal = true;
        cfg.pedal_style = "none".to_string();
        let midi_events: Vec<(f64, bool)> = vec![];
        let (events, _) = compile_pedal_events(None, &cfg, &[], &[], Some(&midi_events));
        assert!(events.is_empty());
    }

    #[test]
    fn test_falls_through_to_generate_events_when_no_midi_events() {
        let mut cfg = base_config();
        cfg.pedal_style = "ai".to_string();
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 42, 0.5, 0.5, "left"),
        ];
        let (events, meta) = compile_pedal_events(None, &cfg, &notes, &[], None);
        assert!(!events.is_empty());
        assert!(meta.is_none());
    }
}

mod test_compile_events {
    use super::*;
    use crate::core::models::MusicalSection;

    #[test]
    fn test_full_pipeline_produces_merged_sorted_events() {
        let mut cfg = base_config();
        cfg.pedal_style = "rhythmic".to_string();
        let notes = vec![
            note(0, 60, 0.0, 0.5, "left"),
            note(1, 64, 0.5, 0.5, "right"),
        ];
        let section = MusicalSection::new(0.0, 1.0, notes.clone());
        let (events, meta) = compile_events(None, &cfg, &notes, &[section], None);
        assert!(!events.is_empty());
        for w in events.windows(2) {
            assert!(w[0].time <= w[1].time);
        }
        assert!(meta.is_none());
    }

    #[test]
    fn test_midi_pedal_events_flow_through_full_pipeline() {
        let mut cfg = base_config();
        cfg.use_midi_pedal = true;
        cfg.pedal_style = "none".to_string();
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let midi_events = vec![(0.1, true), (0.9, false)];
        let (events, _) =
            compile_events(None, &cfg, &notes, &[], Some(&midi_events));
        let pedal_count = events.iter().filter(|e| e.action == "pedal").count();
        assert_eq!(pedal_count, 2);
    }

    fn second_pedal_down_time(before_humanize: bool) -> f64 {
        let mut cfg = base_config();
        cfg.pedal_style = "hybrid".to_string();
        cfg.use_ai_pedal = false;
        cfg.enable_tempo_sway = true;
        cfg.tempo_sway_intensity = 0.1;
        cfg.pedal_before_humanize = before_humanize;
        let notes = vec![
            note(0, 40, 0.0, 0.5, "left"),
            note(1, 43, 1.0, 0.5, "left"),
        ];
        let section = MusicalSection::new(0.0, 2.0, notes.clone());
        let (events, _) = compile_events(None, &cfg, &notes, &[section], None);
        events
            .iter()
            .filter(|e| e.action == "pedal" && e.key_char == "down")
            .map(|e| e.time)
            .nth(1)
            .unwrap()
    }

    #[test]
    fn test_pedal_follows_the_original_notes_when_generated_before_humanizing() {
        assert!((second_pedal_down_time(true) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_pedal_follows_the_humanized_notes_when_the_toggle_is_off() {
        assert!((second_pedal_down_time(false) - 0.9).abs() < 1e-9);
    }

    #[test]
    fn test_none_pedal_style_produces_only_note_events() {
        let mut cfg = base_config();
        cfg.pedal_style = "none".to_string();
        let notes = vec![note(0, 60, 0.0, 0.5, "left")];
        let (events, _) = compile_events(None, &cfg, &notes, &[], None);
        assert!(events.iter().all(|e| e.action != "pedal"));
    }
}

mod test_get_mistake_pitch {
    use super::*;

    #[test]
    fn test_white_key_origin_never_returns_black_key() {
        let mut rng = rand::rng();
        for _ in 0..500 {
            let result = get_mistake_pitch(&mut rng, 60);
            if let Some(p) = result {
                assert!(!KeyMapper::is_black_key(p), "white-origin mistake {p} was black");
            }
        }
    }

    #[test]
    fn test_black_key_origin_only_returns_black_or_white_from_candidate_set() {
        let mut rng = rand::rng();
        for _ in 0..500 {
            if let Some(p) = get_mistake_pitch(&mut rng, 61) {
                let candidates = [59, 60, 62, 63];
                assert!(candidates.contains(&p));
            }
        }
    }

    #[test]
    fn test_result_is_never_the_original_pitch() {
        let mut rng = rand::rng();
        for _ in 0..500 {
            if let Some(p) = get_mistake_pitch(&mut rng, 60) {
                assert_ne!(p, 60);
            }
        }
    }

    #[test]
    fn test_result_within_two_semitones() {
        let mut rng = rand::rng();
        for _ in 0..500 {
            if let Some(p) = get_mistake_pitch(&mut rng, 60) {
                assert!((p - 60).abs() <= 2);
            }
        }
    }

    #[test]
    fn test_white_key_can_produce_multiple_distinct_outcomes() {
        let mut rng = rand::rng();
        let mut seen = HashSet::new();
        for _ in 0..500 {
            if let Some(p) = get_mistake_pitch(&mut rng, 60) {
                seen.insert(p);
            }
        }
        assert!(seen.len() > 1, "expected more than one distinct mistake pitch over 500 draws");
    }
}
