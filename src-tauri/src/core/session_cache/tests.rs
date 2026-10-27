use super::*;
use crate::core::models::Note;
use crate::core::pedal::curve::flat_curve;
use serde_json::json;

fn sample_config() -> PlaybackConfig {
    PlaybackConfig {
        tempo: 1.5,
        simulate_hands: true,
        pedal_style: "harmonic".to_string(),
        pedal_curve_on: flat_curve(0.3),
        pedal_curve_off: flat_curve(0.1),
        use_midi_pedal: true,
        ..PlaybackConfig::default()
    }
}

mod test_extract_notes_config {
    use super::*;

    #[test]
    fn test_includes_only_notes_keys() {
        let extracted = extract_notes_config(&sample_config());
        let obj = extracted.as_object().unwrap();
        for key in obj.keys() {
            assert!(NOTES_CONFIG_KEYS.contains(&key.as_str()));
        }
    }

    #[test]
    fn test_excludes_pedal_only_keys() {
        let extracted = extract_notes_config(&sample_config());
        assert!(extracted.get("pedal_style").is_none());
        assert!(extracted.get("use_midi_pedal").is_none());
    }

    #[test]
    fn test_tempo_value_matches() {
        let extracted = extract_notes_config(&sample_config());
        assert_eq!(extracted["tempo"], json!(1.5));
    }

    #[test]
    fn test_all_notes_keys_present_since_config_is_a_typed_struct() {
        let extracted = extract_notes_config(&sample_config());
        let obj = extracted.as_object().unwrap();
        assert_eq!(obj.len(), NOTES_CONFIG_KEYS.len());
    }
}

mod test_extract_pedal_config {
    use super::*;

    #[test]
    fn test_includes_only_pedal_keys() {
        let extracted = extract_pedal_config(&sample_config());
        let obj = extracted.as_object().unwrap();
        for key in obj.keys() {
            assert!(PEDAL_CONFIG_KEYS.contains(&key.as_str()));
        }
    }

    #[test]
    fn test_excludes_notes_only_keys() {
        let extracted = extract_pedal_config(&sample_config());
        assert!(extracted.get("tempo").is_none());
        assert!(extracted.get("simulate_hands").is_none());
    }

    #[test]
    fn test_pedal_style_value_matches() {
        let extracted = extract_pedal_config(&sample_config());
        assert_eq!(extracted["pedal_style"], json!("harmonic"));
    }
}

mod test_notes_config_matches {
    use super::*;

    #[test]
    fn test_none_snapshot_never_matches() {
        assert!(!notes_config_matches(None, &sample_config()));
    }

    #[test]
    fn test_matching_snapshot_matches() {
        let config = sample_config();
        let snapshot = extract_notes_config(&config);
        assert!(notes_config_matches(Some(&snapshot), &config));
    }

    #[test]
    fn test_snapshot_from_different_config_does_not_match() {
        let config = sample_config();
        let snapshot = extract_notes_config(&config);
        let mut other = config.clone();
        other.tempo = 2.0;
        assert!(!notes_config_matches(Some(&snapshot), &other));
    }

    #[test]
    fn test_pedal_only_change_does_not_affect_notes_match() {
        let config = sample_config();
        let snapshot = extract_notes_config(&config);
        let mut other = config.clone();
        other.pedal_style = "adaptive".to_string();
        assert!(notes_config_matches(Some(&snapshot), &other));
    }
}

mod test_pedal_config_matches {
    use super::*;

    #[test]
    fn test_none_snapshot_never_matches() {
        assert!(!pedal_config_matches(None, &sample_config()));
    }

    #[test]
    fn test_matching_snapshot_matches() {
        let config = sample_config();
        let snapshot = extract_pedal_config(&config);
        assert!(pedal_config_matches(Some(&snapshot), &config));
    }

    #[test]
    fn test_snapshot_from_different_config_does_not_match() {
        let config = sample_config();
        let snapshot = extract_pedal_config(&config);
        let mut other = config.clone();
        other.pedal_curve_on = flat_curve(0.9);
        assert!(!pedal_config_matches(Some(&snapshot), &other));
    }

    #[test]
    fn test_notes_only_change_does_not_affect_pedal_match() {
        let config = sample_config();
        let snapshot = extract_pedal_config(&config);
        let mut other = config.clone();
        other.tempo = 3.0;
        assert!(pedal_config_matches(Some(&snapshot), &other));
    }
}

mod test_tempo_map_to_dict {
    use super::*;

    #[test]
    fn test_events_and_time_signatures_serialized() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (2.0, 600_000)], vec![(0.0, 3, 4)]);
        let dict = tempo_map_to_dict(&tm);
        assert_eq!(dict["events"], json!([[0.0, 500_000], [2.0, 600_000]]));
        assert_eq!(dict["time_signatures"], json!([[0.0, 3, 4]]));
    }

    #[test]
    fn test_empty_tempo_map() {
        let tm = TempoMap::new(vec![], vec![]);
        let dict = tempo_map_to_dict(&tm);
        assert_eq!(dict["events"], json!([]));
        assert_eq!(dict["time_signatures"], json!([]));
    }
}

mod test_cache_path {
    use super::*;

    #[test]
    fn test_ends_with_expected_filename() {
        let p = cache_path();
        assert!(p.ends_with("humidi_session.json"));
    }

    #[test]
    fn test_lives_under_temp_dir() {
        assert_eq!(cache_path().parent().unwrap(), std::env::temp_dir());
    }
}

mod test_write_and_read_cache {
    use super::*;

    fn sample_event() -> KeyEvent {
        let mut ev = KeyEvent::new(1.0, 0, "press", "a");
        ev.pitch = Some(60);
        ev.velocity = Some(100);
        ev
    }

    fn sample_note() -> Note {
        Note::new(0, 60, 100, 0.0, 1.0)
    }

    #[test]
    fn test_round_trip_write_then_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let notes_config = json!({"tempo": 1.0});
        let pedal_config = json!({"pedal_style": "none"});
        let events = vec![sample_event()];
        let notes = vec![sample_note()];
        write_cache_at(
            path.clone(),
            notes_config.clone(),
            pedal_config.clone(),
            &events,
            &events,
            &notes,
            &notes,
            json!({"events": []}),
            12.5,
        );
        let cached = read_cache_at(&path).unwrap();
        assert_eq!(cached["notes_config"], notes_config);
        assert_eq!(cached["pedal_config"], pedal_config);
        assert_eq!(cached["total_dur"], json!(12.5));
    }

    #[test]
    fn test_serialized_event_omits_velocity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let events = vec![sample_event()];
        let notes = vec![sample_note()];
        write_cache_at(
            path.clone(),
            json!({}),
            json!({}),
            &events,
            &[],
            &notes,
            &notes,
            json!({}),
            0.0,
        );
        let cached = read_cache_at(&path).unwrap();
        let ev = &cached["note_events"][0];
        assert!(ev.get("velocity").is_none());
        assert_eq!(ev["pitch"], json!(60));
    }

    #[test]
    fn test_serialized_note_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let notes = vec![sample_note()];
        write_cache_at(
            path.clone(),
            json!({}),
            json!({}),
            &[],
            &[],
            &notes,
            &notes,
            json!({}),
            0.0,
        );
        let cached = read_cache_at(&path).unwrap();
        let n = &cached["final_notes"][0];
        assert_eq!(n["pitch"], json!(60));
        assert_eq!(n["velocity"], json!(100));
        assert_eq!(n["hand"], json!("unknown"));
    }

    #[test]
    fn test_read_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(read_cache_at(&path).is_none());
    }

    #[test]
    fn test_read_corrupted_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(&path, "not json {{{").unwrap();
        assert!(read_cache_at(&path).is_none());
    }

    #[test]
    fn test_write_overwrites_previous_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        write_cache_at(
            path.clone(),
            json!({"tempo": 1.0}),
            json!({}),
            &[],
            &[],
            &[],
            &[],
            json!({}),
            0.0,
        );
        write_cache_at(
            path.clone(),
            json!({"tempo": 2.0}),
            json!({}),
            &[],
            &[],
            &[],
            &[],
            json!({}),
            0.0,
        );
        let cached = read_cache_at(&path).unwrap();
        assert_eq!(cached["notes_config"], json!({"tempo": 2.0}));
    }
}

mod test_clear_cache {
    use super::*;

    #[test]
    fn test_removes_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(&path, "{}").unwrap();
        clear_cache_at(&path);
        assert!(!path.exists());
    }

    #[test]
    fn test_no_op_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        clear_cache_at(&path);
        assert!(!path.exists());
    }
}

mod test_write_cache_async {
    use super::*;

    #[test]
    fn test_write_cache_eventually_persists_to_default_path_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("async_session.json");
        let data = build_cache_data(
            json!({"tempo": 1.0}),
            json!({}),
            &[],
            &[],
            &[],
            &[],
            json!({}),
            0.0,
        );
        let handle = std::thread::spawn({
            let path = path.clone();
            move || {
                let _ = write_json_atomic(&path, &data);
            }
        });
        handle.join().unwrap();
        assert!(read_cache_at(&path).is_some());
    }
}
