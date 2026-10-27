use super::*;

mod test_hotkey_persistence {
    use super::*;
    use crate::managers::hotkey_manager::HotkeyManager;

    #[test]
    fn test_a_rebound_hotkey_event_becomes_a_config_patch_under_its_own_key() {
        assert_eq!(
            hotkey_config_patch(&HotkeyEvent::BoundUpdated("F9".to_string())),
            Some(serde_json::json!({ "hotkey_playback": "F9" }))
        );
        assert_eq!(
            hotkey_config_patch(&HotkeyEvent::BoundSaveUpdated("Alt+W".to_string())),
            Some(serde_json::json!({ "hotkey_save": "Alt+W" }))
        );
    }

    #[test]
    fn test_trigger_events_are_not_persisted() {
        assert_eq!(hotkey_config_patch(&HotkeyEvent::ToggleRequested), None);
        assert_eq!(hotkey_config_patch(&HotkeyEvent::SaveRequested), None);
    }

    #[test]
    fn test_a_patched_config_restores_the_same_bindings_in_a_fresh_manager() {
        let mut config = serde_json::json!({});
        for event in [
            HotkeyEvent::BoundUpdated("Ctrl+K".to_string()),
            HotkeyEvent::BoundSaveUpdated("Shift+F2".to_string()),
        ] {
            config = merge_app_config(&config, &hotkey_config_patch(&event).unwrap());
        }
        let mut mgr = HotkeyManager::new();
        mgr.restore_from_config(&config);
        assert_eq!(mgr.format_hotkey_string(), "Ctrl+K");
        assert_eq!(mgr.format_save_hotkey_string(), "Shift+F2");
    }
}

fn pedal_ev(t: f64, direction: &str) -> KeyEvent {
    KeyEvent::new(t, 1, "pedal", direction)
}

fn press_ev(t: f64) -> KeyEvent {
    KeyEvent::new(t, 0, "press", "a")
}

fn write_temp_midi_single_note() -> tempfile::TempPath {
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
    let track = vec![
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOn {
                    key: u7::from(60),
                    vel: u7::from(64),
                },
            },
        },
        TrackEvent {
            delta: u28::from(480),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOff {
                    key: u7::from(60),
                    vel: u7::from(0),
                },
            },
        },
    ];
    let smf = Smf {
        header,
        tracks: vec![track],
    };
    let mut buf = Vec::new();
    smf.write(&mut buf).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &buf).unwrap();
    file.into_temp_path()
}

fn write_temp_midi_with_tempo(tempo_us: u32) -> tempfile::TempPath {
    use midly::num::{u15, u24, u28, u4, u7};
    use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
    let track = vec![
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(tempo_us))),
        },
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOn {
                    key: u7::from(60),
                    vel: u7::from(64),
                },
            },
        },
        TrackEvent {
            delta: u28::from(480),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOff {
                    key: u7::from(60),
                    vel: u7::from(0),
                },
            },
        },
    ];
    let smf = Smf {
        header,
        tracks: vec![track],
    };
    let mut buf = Vec::new();
    smf.write(&mut buf).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &buf).unwrap();
    file.into_temp_path()
}

mod test_merge_app_config {
    use super::*;

    #[test]
    fn test_merges_new_keys_into_existing_object() {
        let current = serde_json::json!({"always_on_top": false, "opacity": 100});
        let patch = serde_json::json!({"opacity": 80});
        let merged = merge_app_config(&current, &patch);
        assert_eq!(merged, serde_json::json!({"always_on_top": false, "opacity": 80}));
    }

    #[test]
    fn test_patch_keys_not_present_in_current_are_added() {
        let current = serde_json::json!({"always_on_top": false});
        let patch = serde_json::json!({"pedal_prompt_threshold": 8});
        let merged = merge_app_config(&current, &patch);
        assert_eq!(
            merged,
            serde_json::json!({"always_on_top": false, "pedal_prompt_threshold": 8})
        );
    }

    #[test]
    fn test_empty_current_object_takes_every_patch_key() {
        let current = serde_json::json!({});
        let patch = serde_json::json!({"opacity": 50, "auto_check_updates": true});
        let merged = merge_app_config(&current, &patch);
        assert_eq!(merged, serde_json::json!({"opacity": 50, "auto_check_updates": true}));
    }

    #[test]
    fn test_non_object_current_is_replaced_wholesale_by_patch() {
        let current = Value::Null;
        let patch = serde_json::json!({"opacity": 50});
        let merged = merge_app_config(&current, &patch);
        assert_eq!(merged, serde_json::json!({"opacity": 50}));
    }

    #[test]
    fn test_independent_callers_patching_different_keys_do_not_clobber_each_other() {
        let mut current = serde_json::json!({});
        current = merge_app_config(&current, &serde_json::json!({"always_on_top": true}));
        current = merge_app_config(&current, &serde_json::json!({"redact_debug_paths": false}));
        assert_eq!(
            current,
            serde_json::json!({"always_on_top": true, "redact_debug_paths": false})
        );
    }
}

mod test_parse_midi_structure_logic {
    use super::*;

    #[test]
    fn test_default_tempo_when_no_tempo_event() {
        let path = write_temp_midi_single_note();
        let (_, _, _, _, result) =
            parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
        assert!((result.initial_bpm - 120.0).abs() < 1e-6);
    }

    #[test]
    fn test_explicit_tempo_event_reflected() {
        let path = write_temp_midi_with_tempo(400_000);
        let (_, _, _, _, result) =
            parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
        assert!((result.initial_bpm - 150.0).abs() < 1e-6);
    }

    #[test]
    fn test_tracks_field_matches_summaries() {
        let path = write_temp_midi_single_note();
        let (_, _, _, _, result) =
            parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
        assert_eq!(result.tracks.len(), 1);
        assert_eq!(result.tracks[0].note_count, 1);
    }

    #[test]
    fn test_missing_file_errors() {
        assert!(parse_midi_structure_logic("does_not_exist.mid").is_err());
    }
}

mod test_compute_total_dur {
    use super::*;

    fn note(start_time: f64, duration: f64) -> Note {
        Note::new(0, 60, 64, start_time, duration)
    }

    #[test]
    fn test_empty_defaults_to_one() {
        assert_eq!(compute_total_dur(&[]), 1.0);
    }

    #[test]
    fn test_uses_max_end_time() {
        let notes = vec![note(0.0, 1.0), note(2.0, 5.0), note(1.0, 0.5)];
        assert_eq!(compute_total_dur(&notes), 7.0);
    }
}

mod test_compile_notes_logic {
    use super::*;
    use crate::state::PlaybackSession;

    #[test]
    fn test_populates_session_and_returns_timeline() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let mut session = PlaybackSession::default();

        let result =
            compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session)
                .unwrap();

        assert_eq!(result.final_notes.len(), 1);
        assert!(session.notes_config_snapshot.is_some());
        assert!(session.final_notes.is_some());
        assert!(session.humanized_notes.is_some());
        assert!(session.note_events.is_some());
        assert!(session.tempo_map.is_some());
    }

    #[test]
    fn test_played_notes_match_the_compiled_press_events() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let mut session = PlaybackSession::default();

        let result =
            compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session)
                .unwrap();

        let presses: Vec<&KeyEvent> = session
            .note_events
            .as_ref()
            .unwrap()
            .iter()
            .filter(|e| e.action == "press")
            .collect();
        assert_eq!(result.played_notes.len(), presses.len());
        assert!((result.played_notes[0].start_time - presses[0].time).abs() < 1e-9);
        assert_eq!(Some(result.played_notes[0].pitch), presses[0].pitch);
    }

    #[test]
    fn test_returns_tempo_events_and_measure_boundaries_matching_session_tempo_map() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let mut session = PlaybackSession::default();

        let result =
            compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session)
                .unwrap();

        let session_tempo_map = session.tempo_map.as_ref().unwrap();
        assert_eq!(result.tempo_events, session_tempo_map.events());
        assert_eq!(result.time_signatures, session_tempo_map.time_signatures());
        assert_eq!(
            result.measure_boundaries,
            session_tempo_map.get_measure_boundaries(result.total_dur)
        );
        assert!(!result.measure_boundaries.is_empty());
    }

    #[test]
    fn test_invalidates_previously_compiled_pedal_state() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let mut session = PlaybackSession::default();
        session.pedal_events = Some(vec![]);
        session.merged_events = Some(vec![]);
        session.pedal_config_snapshot = Some(serde_json::json!({}));

        compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).unwrap();

        assert!(session.pedal_events.is_none());
        assert!(session.merged_events.is_none());
        assert!(session.pedal_config_snapshot.is_none());
    }

    #[test]
    fn test_empty_selection_defaults_total_dur_to_one() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let mut session = PlaybackSession::default();

        let result = compile_notes_logic(&config, &[], &mut session).unwrap();

        assert!(result.final_notes.is_empty());
        assert_eq!(result.total_dur, 1.0);
    }

    #[test]
    fn test_missing_file_returns_error() {
        let mut config = PlaybackConfig::default();
        config.midi_file = "does_not_exist.mid".to_string();
        let mut session = PlaybackSession::default();

        assert!(compile_notes_logic(&config, &[], &mut session).is_err());
    }

    #[test]
    fn test_zero_tempo_errors_instead_of_panicking() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.tempo = 0.0;
        let mut session = PlaybackSession::default();

        assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
    }

    #[test]
    fn test_negative_tempo_errors() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.tempo = -1.0;
        let mut session = PlaybackSession::default();

        assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
    }

    #[test]
    fn test_nan_tempo_errors() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.tempo = f64::NAN;
        let mut session = PlaybackSession::default();

        assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
    }

    #[test]
    fn test_infinite_tempo_errors() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.tempo = f64::INFINITY;
        let mut session = PlaybackSession::default();

        assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
    }
}

mod test_compile_notes_from_sheet {
    use super::*;
    use crate::state::PlaybackSession;

    fn km() -> KeyMapper {
        KeyMapper::new(false)
    }

    #[test]
    fn test_empty_sheet_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result = compile_notes_from_sheet_logic(&config, "", 120.0, &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_only_sheet_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result = compile_notes_from_sheet_logic(&config, "   \n  ", 120.0, &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_unmapped_chars_only_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result = compile_notes_from_sheet_logic(&config, "~~~", 120.0, &mut session);
        assert!(result.is_err());
    }

    #[test]
    fn test_single_note_matches_translate_sheet_timing() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let raw = translate_sheet_to_notes(ch.to_string(), 120.0, false).unwrap();

        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result =
            compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
                .unwrap();

        assert_eq!(result.final_notes.len(), 1);
        assert_eq!(result.final_notes[0].pitch, raw[0].pitch);
        assert!((result.final_notes[0].start_time - raw[0].start_time).abs() < 1e-9);
        assert!((result.final_notes[0].duration - raw[0].duration).abs() < 1e-9);
    }

    #[test]
    fn test_returns_played_notes_for_the_sheet() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result =
            compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
                .unwrap();

        assert_eq!(result.played_notes.len(), 1);
        assert_eq!(result.played_notes[0].pitch, 60);
    }

    #[test]
    fn test_returns_single_synthetic_tempo_event_matching_bpm_and_no_time_signatures() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result =
            compile_notes_from_sheet_logic(&config, &ch.to_string(), 100.0, &mut session)
                .unwrap();

        assert_eq!(result.tempo_events.len(), 1);
        assert_eq!(result.tempo_events[0].0, 0.0);
        let expected_tempo_us = (60_000_000.0f64 / 100.0).round() as u32;
        assert_eq!(result.tempo_events[0].1, expected_tempo_us);
        assert!(result.time_signatures.is_empty());
        assert!(!result.measure_boundaries.is_empty());
    }

    #[test]
    fn test_chord_notes_share_start_time() {
        let mapper = km();
        let p1 = mapper.get_key_for_pitch(60).unwrap();
        let p2 = mapper.get_key_for_pitch(62).unwrap();
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result = compile_notes_from_sheet_logic(
            &config,
            &format!("[{p1}{p2}]"),
            120.0,
            &mut session,
        )
        .unwrap();
        assert_eq!(result.final_notes.len(), 2);
        assert_eq!(result.final_notes[0].start_time, result.final_notes[1].start_time);
    }

    #[test]
    fn test_hand_assignment_by_pitch_threshold_when_simulate_off() {
        let mapper = km();
        let low = mapper.get_key_for_pitch(40).unwrap();
        let high = mapper.get_key_for_pitch(81).unwrap();
        let mut session = PlaybackSession::default();
        let mut config = PlaybackConfig::default();
        config.simulate_hands = false;
        let result = compile_notes_from_sheet_logic(
            &config,
            &format!("{low} {high}"),
            120.0,
            &mut session,
        )
        .unwrap();
        let low_note = result.final_notes.iter().find(|n| n.pitch == 40).unwrap();
        let high_note = result.final_notes.iter().find(|n| n.pitch == 81).unwrap();
        assert_eq!(low_note.hand, "left");
        assert_eq!(high_note.hand, "right");
    }

    #[test]
    fn test_hand_assignment_uses_group_average_when_simulate_on() {
        let mapper = km();
        let p1 = mapper.get_key_for_pitch(40).unwrap();
        let p2 = mapper.get_key_for_pitch(50).unwrap();
        let mut session = PlaybackSession::default();
        let mut config = PlaybackConfig::default();
        config.simulate_hands = true;
        let result = compile_notes_from_sheet_logic(
            &config,
            &format!("[{p1}{p2}]"),
            120.0,
            &mut session,
        )
        .unwrap();
        assert!(result.final_notes.iter().all(|n| n.hand == "left"));
    }

    #[test]
    fn test_zero_bpm_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        assert!(compile_notes_from_sheet_logic(&config, "a", 0.0, &mut session).is_err());
    }

    #[test]
    fn test_negative_bpm_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        assert!(compile_notes_from_sheet_logic(&config, "a", -10.0, &mut session).is_err());
    }

    #[test]
    fn test_nan_bpm_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        assert!(
            compile_notes_from_sheet_logic(&config, "a", f64::NAN, &mut session).is_err()
        );
    }

    #[test]
    fn test_infinite_bpm_errors() {
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        assert!(compile_notes_from_sheet_logic(&config, "a", f64::INFINITY, &mut session)
            .is_err());
    }

    #[test]
    fn test_extremely_high_bpm_does_not_panic() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result = compile_notes_from_sheet_logic(
            &config,
            &ch.to_string(),
            100_000_000.0,
            &mut session,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_sheet_note_count_matches_token_count() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let sheet = std::iter::repeat(ch.to_string())
            .take(500)
            .collect::<Vec<_>>()
            .join(" ");
        let mut session = PlaybackSession::default();
        let config = PlaybackConfig::default();
        let result =
            compile_notes_from_sheet_logic(&config, &sheet, 120.0, &mut session).unwrap();
        assert_eq!(result.final_notes.len(), 500);
    }

    #[test]
    fn test_invalidates_previously_compiled_pedal_state() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let mut session = PlaybackSession::default();
        session.pedal_events = Some(vec![]);
        session.merged_events = Some(vec![]);
        session.pedal_config_snapshot = Some(serde_json::json!({}));
        let config = PlaybackConfig::default();
        compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
            .unwrap();
        assert!(session.pedal_events.is_none());
        assert!(session.merged_events.is_none());
        assert!(session.pedal_config_snapshot.is_none());
    }

    #[test]
    fn test_full_pipeline_reuse_pedal_compile_succeeds_after_sheet_compile() {
        let mapper = km();
        let ch = mapper.get_key_for_pitch(60).unwrap();
        let mut session = PlaybackSession::default();
        let mut config = PlaybackConfig::default();
        config.pedal_style = "none".to_string();
        compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
            .unwrap();

        let pedal_result = compile_pedal_logic(&config, None, &mut session).unwrap();
        assert!(pedal_result.pedal_intervals.is_empty());
        assert!(session.merged_events.is_some());
    }
}

mod test_compile_pedal_logic {
    use super::*;
    use crate::state::PlaybackSession;

    fn compiled_session() -> (PlaybackConfig, PlaybackSession) {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.pedal_style = "none".to_string();
        let mut session = PlaybackSession::default();
        compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).unwrap();
        (config, session)
    }

    #[test]
    fn test_errors_when_notes_not_compiled_first() {
        let config = PlaybackConfig::default();
        let mut session = PlaybackSession::default();
        assert!(compile_pedal_logic(&config, None, &mut session).is_err());
    }

    #[test]
    fn test_none_style_produces_no_pedal_events() {
        let (config, mut session) = compiled_session();
        let result = compile_pedal_logic(&config, None, &mut session).unwrap();
        assert!(result.pedal_intervals.is_empty());
        assert_eq!(session.pedal_events, Some(vec![]));
    }

    #[test]
    fn test_without_a_model_there_is_no_confidence_curve_but_the_rate_is_reported() {
        let (config, mut session) = compiled_session();
        let result = compile_pedal_logic(&config, None, &mut session).unwrap();
        assert!(result.curve_on.is_empty() && result.curve_off.is_empty());
        assert!(result.confidence.is_empty());
        assert_eq!(result.confidence_fps, crate::core::pedal::FPS);
    }

    #[test]
    fn test_populates_merged_and_snapshot() {
        let (config, mut session) = compiled_session();
        compile_pedal_logic(&config, None, &mut session).unwrap();
        assert!(session.merged_events.is_some());
        assert_eq!(
            session.pedal_config_snapshot,
            Some(session_cache::extract_pedal_config(&config))
        );
    }

    #[test]
    fn test_merged_events_include_all_note_events() {
        let (config, mut session) = compiled_session();
        compile_pedal_logic(&config, None, &mut session).unwrap();
        let note_events = session.note_events.clone().unwrap();
        let merged = session.merged_events.clone().unwrap();
        assert_eq!(merged.len(), note_events.len());
    }

    #[test]
    fn test_harmonic_style_does_not_panic_without_model() {
        let (mut config, mut session) = compiled_session();
        config.pedal_style = "harmonic".to_string();
        compile_pedal_logic(&config, None, &mut session).unwrap();
        assert_eq!(
            session.pedal_config_snapshot,
            Some(session_cache::extract_pedal_config(&config))
        );
    }

    #[test]
    fn test_ai_style_without_model_returns_no_pedal_events_but_does_not_error() {
        let (mut config, mut session) = compiled_session();
        config.pedal_style = "ai".to_string();
        let result = compile_pedal_logic(&config, None, &mut session);
        assert!(result.is_ok());
    }
}

mod test_event_to_channel_payload {
    use super::*;

    #[test]
    fn test_status() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Status("hi".to_string()));
        assert_eq!(ch, "status_updated");
        assert_eq!(payload, serde_json::json!("hi"));
    }

    #[test]
    fn test_progress() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Progress(1.5));
        assert_eq!(ch, "progress_updated");
        assert_eq!(payload, serde_json::json!(1.5));
    }

    #[test]
    fn test_finished() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Finished);
        assert_eq!(ch, "playback_finished");
        assert_eq!(payload, serde_json::Value::Null);
    }

    #[test]
    fn test_visualizer() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Visualizer(vec![60, 64]));
        assert_eq!(ch, "visualizer_updated");
        assert_eq!(payload, serde_json::json!([60, 64]));
    }

    #[test]
    fn test_pedal() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Pedal(true));
        assert_eq!(ch, "pedal_updated");
        assert_eq!(payload, serde_json::json!(true));
    }

    #[test]
    fn test_auto_paused() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::AutoPaused);
        assert_eq!(ch, "auto_paused");
        assert_eq!(payload, serde_json::Value::Null);
    }

    #[test]
    fn test_error() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Error("oops".to_string()));
        assert_eq!(ch, "error_occurred");
        assert_eq!(payload, serde_json::json!("oops"));
    }

    #[test]
    fn test_section() {
        let (ch, payload) = event_to_channel_payload(&PlayerEvent::Section(3));
        assert_eq!(ch, "section_changed");
        assert_eq!(payload, serde_json::json!(3));
    }
}

mod test_platform_uinput_sink {
    use super::*;

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn test_off_linux_no_virtual_keyboard_is_opened() {
        let dir = tempfile::tempdir().unwrap();
        let state = hermetic_state(dir.path());
        assert!(platform_uinput_sink(&state).is_none());
        assert!(state.uinput_sink.lock().unwrap().is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "needs write access to /dev/uinput; CI grants it"]
    fn test_on_linux_the_virtual_keyboard_is_opened_once_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let state = hermetic_state(dir.path());
        assert!(platform_uinput_sink(&state).is_some());
        assert!(state.uinput_sink.lock().unwrap().is_some());
        let started = std::time::Instant::now();
        assert!(platform_uinput_sink(&state).is_some());
        assert!(started.elapsed() < UINPUT_SETTLE);
    }
}

mod test_engine_starting_at {
    use super::*;
    use crate::core::keyboard_driver::{Key, KeyActionResult};

    struct NoopDriver;
    impl KeyboardDriver for NoopDriver {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
    }

    fn events() -> Vec<KeyEvent> {
        let mut first = KeyEvent::new(0.0, 2, "press", "a");
        first.pitch = Some(60);
        let mut second = KeyEvent::new(5.0, 2, "press", "s");
        second.pitch = Some(62);
        vec![first, second]
    }

    #[test]
    fn test_a_start_time_skips_to_that_point_on_the_first_tick() {
        let mut engine = engine_starting_at(PlaybackConfig::default(), events(), 6.0, 4.0);
        engine.start(0.0);
        let outcome = engine.tick(&mut NoopDriver, 0.0);
        assert_eq!(engine.event_index(), 1);
        assert!(outcome.events.iter().any(|e| matches!(e, PlayerEvent::Progress(t) if (t - 4.0).abs() < 1e-9)));
        assert!((engine.current_time(0.5) - 4.5).abs() < 1e-9);
    }

    #[test]
    fn test_a_zero_start_time_plays_from_the_beginning_without_a_seek() {
        let mut engine = engine_starting_at(PlaybackConfig::default(), events(), 6.0, 0.0);
        engine.start(0.0);
        let outcome = engine.tick(&mut NoopDriver, 0.0);
        assert_eq!(engine.event_index(), 1);
        assert!(!outcome.events.iter().any(|e| matches!(e, PlayerEvent::Progress(_))));
        assert_eq!(engine.compiled_event_count(), 2);
    }

    #[test]
    fn test_the_events_and_length_are_loaded() {
        let mut engine = engine_starting_at(PlaybackConfig::default(), events(), 6.0, 0.0);
        assert_eq!(engine.compiled_event_count(), 2);
        engine.start(0.0);
        engine.tick(&mut NoopDriver, 6.0);
        assert!(!engine.is_paused());
        engine.tick(&mut NoopDriver, 6.2);
        assert!(engine.is_paused());
    }
}

mod test_run_playback_loop {
    use super::*;
    use crate::core::keyboard_driver::{Key, KeyActionResult};
    use std::cell::RefCell;

    struct NoopDriver;
    impl KeyboardDriver for NoopDriver {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            Ok(())
        }
    }

    struct LoggingDriver {
        log: Vec<(Key, bool)>,
    }
    impl KeyboardDriver for LoggingDriver {
        fn press(&mut self, key: Key) -> KeyActionResult {
            self.log.push((key, true));
            Ok(())
        }
        fn release(&mut self, key: Key) -> KeyActionResult {
            self.log.push((key, false));
            Ok(())
        }
    }

    #[test]
    fn test_the_pedal_key_is_not_released_at_the_start_by_default() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let mut engine = PlaybackEngine::new(config, Vec::new());
        engine.load_compiled_events(vec![KeyEvent::new(0.0, 1, "pedal", "down")], 1.0);
        let mut driver = LoggingDriver { log: Vec::new() };
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Shutdown).unwrap();

        run_playback_loop(engine, &mut driver, &rx, |_| {}, || 0.0, |_| {});

        assert_eq!(driver.log.first(), Some(&(Key::Space, true)));
    }

    #[test]
    fn test_the_pedal_key_is_released_before_any_event_plays() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        config.release_pedal_on_start = true;
        let mut engine = PlaybackEngine::new(config, Vec::new());
        engine.load_compiled_events(vec![KeyEvent::new(0.0, 1, "pedal", "down")], 1.0);
        let mut driver = LoggingDriver { log: Vec::new() };
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Shutdown).unwrap();

        run_playback_loop(engine, &mut driver, &rx, |_| {}, || 0.0, |_| {});

        assert_eq!(driver.log.first(), Some(&(Key::Space, false)));
    }

    #[test]
    fn test_stop_command_breaks_loop_and_emits_final_finished() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Stop).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {},
        );

        let events = events.into_inner();
        assert!(events.iter().any(|e| matches!(e, PlayerEvent::Finished)));
    }

    #[test]
    fn test_stop_clears_visualizer_pitches_and_pedal_indicator() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Stop).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {},
        );

        let events = events.into_inner();
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::Visualizer(pitches) if pitches.is_empty())));
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::Pedal(false))));
        let visualizer_clear_idx = events
            .iter()
            .position(|e| matches!(e, PlayerEvent::Visualizer(p) if p.is_empty()))
            .unwrap();
        let finished_idx = events
            .iter()
            .position(|e| matches!(e, PlayerEvent::Finished))
            .unwrap();
        assert!(
            visualizer_clear_idx <= finished_idx,
            "visualizer/pedal clear must be emitted no later than Finished so the \
                 frontend never renders a stale highlighted key/pedal after playback ends"
        );
    }

    #[test]
    fn test_shutdown_command_also_clears_visualizer_pitches_and_pedal_indicator() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Shutdown).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {},
        );

        let events = events.into_inner();
        assert!(events
            .iter()
            .any(|e| matches!(e, PlayerEvent::Visualizer(pitches) if pitches.is_empty())));
        assert!(events.iter().any(|e| matches!(e, PlayerEvent::Pedal(false))));
    }

    #[test]
    fn test_shutdown_command_also_breaks_loop_and_emits_final_finished() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Shutdown).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {},
        );

        assert!(events
            .into_inner()
            .iter()
            .any(|e| matches!(e, PlayerEvent::Finished)));
    }

    #[test]
    fn test_countdown_emits_four_status_messages_with_second_long_sleeps() {
        let mut config = PlaybackConfig::default();
        config.countdown = true;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Stop).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        let sleeps: RefCell<Vec<f64>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |s| sleeps.borrow_mut().push(s),
        );

        let events = events.into_inner();
        let status_msgs: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                PlayerEvent::Status(s) => Some(s.clone()),
                _ => None,
            })
            .take(4)
            .collect();
        assert_eq!(status_msgs, PlaybackEngine::countdown_messages());
        assert_eq!(sleeps.into_inner().iter().filter(|&&s| s == 1.0).count(), 4);
    }

    #[test]
    fn test_no_countdown_skips_status_messages() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();
        tx.send(PlaybackCommand::Stop).unwrap();

        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {},
        );

        let has_countdown_status = events
            .into_inner()
            .iter()
            .any(|e| matches!(e, PlayerEvent::Status(s) if s == "Get ready..."));
        assert!(!has_countdown_status);
    }

    struct SlowDriver<'a> {
        clock: &'a std::cell::Cell<f64>,
    }
    impl KeyboardDriver for SlowDriver<'_> {
        fn press(&mut self, _key: Key) -> KeyActionResult {
            self.clock.set(self.clock.get() + SLOW_KEY_SECONDS);
            Ok(())
        }
        fn release(&mut self, _key: Key) -> KeyActionResult {
            self.clock.set(self.clock.get() + SLOW_KEY_SECONDS);
            Ok(())
        }
    }

    const SLOW_KEY_SECONDS: f64 = 0.1;
    const LOOP_STEP_SECONDS: f64 = 0.02;
    const RUN_UNTIL_SECONDS: f64 = 1.5;
    const SAME_TIME: f64 = 1e-9;

    fn slow_chord_events() -> Vec<KeyEvent> {
        let mut press = KeyEvent::new(0.5, 2, "press", "a");
        press.pitch = Some(60);
        press.velocity = Some(64);
        let mut release = KeyEvent::new(1.0, 4, "release", "a");
        release.pitch = Some(60);
        vec![press, release]
    }

    fn progress_and_clock_pairs(
        engine: PlaybackEngine,
        commands: Vec<(f64, PlaybackCommand)>,
    ) -> Vec<(f64, f64)> {
        let clock = std::cell::Cell::new(0.0);
        let mut driver = SlowDriver { clock: &clock };
        let (tx, rx) = mpsc::channel();
        let pending = RefCell::new(commands);
        let pairs: RefCell<Vec<(f64, f64)>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| {
                if let PlayerEvent::Progress(t) = ev {
                    pairs.borrow_mut().push((*t, clock.get()));
                }
            },
            || clock.get(),
            |_| {
                clock.set(clock.get() + LOOP_STEP_SECONDS);
                let mut pending = pending.borrow_mut();
                while pending.first().is_some_and(|(at, _)| clock.get() >= *at) {
                    let _ = tx.send(pending.remove(0).1);
                }
                if clock.get() >= RUN_UNTIL_SECONDS {
                    let _ = tx.send(PlaybackCommand::Stop);
                }
            },
        );
        pairs.into_inner()
    }

    fn engine_for_slow_chords() -> PlaybackEngine {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let mut engine = PlaybackEngine::new(config, Vec::new());
        engine.load_compiled_events(slow_chord_events(), 1.0);
        engine
    }

    #[test]
    fn test_progress_carries_the_time_it_is_sent_even_after_slow_key_sends() {
        let pairs = progress_and_clock_pairs(engine_for_slow_chords(), Vec::new());
        assert!(pairs.len() > 1);
        for (reported, sent_at) in pairs {
            assert!((reported - sent_at).abs() < SAME_TIME, "reported {reported} but sent at {sent_at}");
        }
    }

    #[test]
    fn test_progress_while_paused_stays_at_the_pause_point() {
        let pairs = progress_and_clock_pairs(
            engine_for_slow_chords(),
            vec![(0.3, PlaybackCommand::TogglePause), (0.6, PlaybackCommand::Seek(0.2))],
        );
        let after_seek: Vec<f64> = pairs
            .iter()
            .filter(|(_, sent_at)| *sent_at >= 0.6)
            .map(|(reported, _)| *reported)
            .collect();
        assert!(!after_seek.is_empty());
        for reported in after_seek {
            assert!((reported - 0.2).abs() < SAME_TIME, "reported {reported} while paused at 0.2");
        }
    }

    #[test]
    fn test_no_pending_command_does_not_break_loop_immediately() {
        let mut config = PlaybackConfig::default();
        config.countdown = false;
        let engine = PlaybackEngine::new(config, Vec::new());
        let mut driver = NoopDriver;
        let (tx, rx) = mpsc::channel();

        let tick_count = RefCell::new(0);
        let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
        run_playback_loop(
            engine,
            &mut driver,
            &rx,
            |ev| events.borrow_mut().push(ev.clone()),
            || 0.0,
            |_| {
                *tick_count.borrow_mut() += 1;
                if *tick_count.borrow() >= 3 {
                    let _ = tx.send(PlaybackCommand::Stop);
                }
            },
        );

        assert!(*tick_count.borrow() >= 3);
        assert!(events
            .into_inner()
            .iter()
            .any(|e| matches!(e, PlayerEvent::Finished)));
    }
}

fn track_fixture() -> MidiTrack {
    MidiTrack {
        index: 0,
        name: "Melody".to_string(),
        program_change: 0,
        is_drum: false,
        notes: vec![Note::new(0, 60, 64, 0.0, 1.0)],
    }
}

mod test_save_playback_logic {
    use super::*;

    fn compiled_session(config: &PlaybackConfig, selected: &[(i32, String)]) -> PlaybackSession {
        let mut session = PlaybackSession::default();
        compile_notes_logic(config, selected, &mut session).unwrap();
        compile_pedal_logic(config, None, &mut session).unwrap();
        session
    }

    #[test]
    fn test_writes_file_and_returns_path() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];

        let session = compiled_session(&config, &[(0, "Right Hand".to_string())]);
        let result = save_playback_logic(
            &config,
            &session,
            &tracks,
            &[(0, "Right Hand".to_string())],
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        )
        .unwrap();

        assert!(std::path::Path::new(&result).exists());
        assert!(result.ends_with("song_20260101_000000.json"));
    }

    #[test]
    fn test_uses_the_exact_name_with_no_timestamp_suffix() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];

        let session = compiled_session(&config, &[(0, "Right Hand".to_string())]);
        let result = save_playback_logic(
            &config,
            &session,
            &tracks,
            &[(0, "Right Hand".to_string())],
            save_dir.path(),
            "song.mid",
            "  My Song  ",
        )
        .unwrap();

        assert_eq!(
            std::path::Path::new(&result).file_name().unwrap().to_string_lossy(),
            "My Song.json"
        );
    }

    #[test]
    fn test_refuses_to_overwrite_an_existing_save() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        std::fs::write(save_dir.path().join("taken.json"), "keep me").unwrap();
        let tracks = vec![track_fixture()];

        let session = compiled_session(&config, &[(0, "Right Hand".to_string())]);
        let result = save_playback_logic(
            &config,
            &session,
            &tracks,
            &[(0, "Right Hand".to_string())],
            save_dir.path(),
            "song.mid",
            "taken",
        );

        assert_eq!(result.unwrap_err(), "A save named \"taken\" already exists.");
        assert_eq!(
            std::fs::read_to_string(save_dir.path().join("taken.json")).unwrap(),
            "keep me"
        );
    }

    #[test]
    fn test_save_output_path_returns_the_cleaned_name() {
        let save_dir = tempfile::tempdir().unwrap();
        let (name, path) = save_output_path(save_dir.path(), "  My Song.json ").unwrap();
        assert_eq!(name, "My Song");
        assert_eq!(path, save_dir.path().join("My Song.json"));
    }

    #[test]
    fn test_rejects_invalid_names_before_compiling() {
        let config = PlaybackConfig::default();
        let save_dir = tempfile::tempdir().unwrap();

        for bad in [
            "", "   ", "a/b", "a\\b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b", "a\nb", "CON", "...",
        ] {
            let result = save_playback_logic(
                &config,
                &PlaybackSession::default(),
                &[],
                &[],
                save_dir.path(),
                "song.mid",
                bad,
            );
            assert!(result.is_err(), "{bad:?} should be rejected");
            assert!(!result.unwrap_err().contains("compiled"), "{bad:?} reached the compile check");
        }
    }

    #[test]
    fn test_saved_file_contains_expected_schema() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];

        let session = compiled_session(&config, &[(0, "Right Hand".to_string())]);
        let result_path = save_playback_logic(
            &config,
            &session,
            &tracks,
            &[(0, "Right Hand".to_string())],
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        )
        .unwrap();

        let data: Value =
            serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
        assert!(data["metadata"]["track_details"].is_array());
        assert_eq!(data["metadata"]["source_midi_filename"], "song.mid");
        assert!(!data["compiled_events"].as_array().unwrap().is_empty());
        let first_event = &data["compiled_events"][0];
        assert!(first_event.get("velocity").is_some());
    }

    #[test]
    fn test_track_details_include_pitch_range_and_role() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];

        let session = compiled_session(&config, &[(0, "Left Hand".to_string())]);
        let result_path = save_playback_logic(
            &config,
            &session,
            &tracks,
            &[(0, "Left Hand".to_string())],
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        )
        .unwrap();
        let data: Value =
            serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
        let detail = &data["metadata"]["track_details"][0];
        assert_eq!(detail["role"], "Left Hand");
        assert_eq!(detail["pitch_min"], 60);
        assert_eq!(detail["pitch_max"], 60);
        assert_eq!(detail["note_count"], 1);
    }

    #[test]
    fn test_track_details_record_the_midi_track_index_so_the_save_can_be_reopened_for_editing() {
        const MISSING_TRACK_INDEX: i32 = 7;
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];
        let selected = [(0, "Right Hand".to_string()), (MISSING_TRACK_INDEX, "Left Hand".to_string())];

        let session = compiled_session(&config, &selected[..1]);
        let result_path = save_playback_logic(
            &config,
            &session,
            &tracks,
            &selected,
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        )
        .unwrap();
        let data: Value =
            serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
        assert_eq!(data["metadata"]["track_details"][0]["index"], 0);
        assert_eq!(data["metadata"]["track_details"][1]["index"], MISSING_TRACK_INDEX);
    }

    #[test]
    fn test_empty_compiled_playback_errors_with_zero_events_message() {
        let config = PlaybackConfig::default();
        let save_dir = tempfile::tempdir().unwrap();
        let mut session = PlaybackSession::default();
        session.merged_events = Some(Vec::new());

        let result = save_playback_logic(
            &config,
            &session,
            &[],
            &[],
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        );
        assert!(result.unwrap_err().contains("zero events"));
    }

    #[test]
    fn test_errors_when_nothing_is_compiled_yet() {
        let config = PlaybackConfig::default();
        let save_dir = tempfile::tempdir().unwrap();

        let result = save_playback_logic(
            &config,
            &PlaybackSession::default(),
            &[],
            &[],
            save_dir.path(),
            "song.mid",
            "song_20260101_000000",
        );
        assert_eq!(result.unwrap_err(), "Pedal must be compiled before saving.");
        assert!(std::fs::read_dir(save_dir.path()).unwrap().next().is_none());
    }

    #[test]
    fn test_saves_exactly_the_compiled_session_events() {
        let path = write_temp_midi_single_note();
        let mut config = PlaybackConfig::default();
        config.midi_file = path.to_string_lossy().to_string();
        config.pedal_style = "harmonic".to_string();
        config.vary_timing = true;
        config.timing_variance = 0.05;
        let save_dir = tempfile::tempdir().unwrap();
        let tracks = vec![track_fixture()];
        let selected = [(0, "Right Hand".to_string())];
        let session = compiled_session(&config, &selected);

        let result = save_playback_logic(
            &config,
            &session,
            &tracks,
            &selected,
            save_dir.path(),
            "song.mid",
            "exact",
        )
        .unwrap();

        let data: Value = serde_json::from_str(&std::fs::read_to_string(result).unwrap()).unwrap();
        let saved: Vec<(f64, String)> = data["compiled_events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e["time"].as_f64().unwrap(), e["action"].as_str().unwrap().to_string()))
            .collect();
        let compiled: Vec<(f64, String)> = session
            .merged_events
            .as_ref()
            .unwrap()
            .iter()
            .map(|e| (e.time, e.action.clone()))
            .collect();
        assert_eq!(saved, compiled);
    }
}

mod test_validate_save_data {
    use super::*;

    fn valid_data() -> Value {
        serde_json::json!({
            "metadata": {"track_details": [], "compiled_pedal_count": 0},
            "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
        })
    }

    #[test]
    fn test_valid_data_passes() {
        assert!(validate_save_data(&valid_data()).is_ok());
    }

    #[test]
    fn test_missing_metadata_fails() {
        let mut data = valid_data();
        data.as_object_mut().unwrap().remove("metadata");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_empty_events_fails() {
        let mut data = valid_data();
        data["compiled_events"] = serde_json::json!([]);
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_missing_events_field_fails() {
        let mut data = valid_data();
        data.as_object_mut().unwrap().remove("compiled_events");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_first_event_missing_time_fails() {
        let mut data = valid_data();
        data["compiled_events"][0].as_object_mut().unwrap().remove("time");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_first_event_missing_priority_fails() {
        let mut data = valid_data();
        data["compiled_events"][0]
            .as_object_mut()
            .unwrap()
            .remove("priority");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_first_event_missing_action_fails() {
        let mut data = valid_data();
        data["compiled_events"][0].as_object_mut().unwrap().remove("action");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_first_event_missing_key_char_fails() {
        let mut data = valid_data();
        data["compiled_events"][0]
            .as_object_mut()
            .unwrap()
            .remove("key_char");
        assert!(validate_save_data(&data).is_err());
    }

    #[test]
    fn test_older_version_missing_track_details_fails() {
        let mut data = valid_data();
        data["metadata"].as_object_mut().unwrap().remove("track_details");
        let err = validate_save_data(&data).unwrap_err();
        assert!(err.contains("older"));
    }

    #[test]
    fn test_older_version_missing_compiled_pedal_count_fails() {
        let mut data = valid_data();
        data["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("compiled_pedal_count");
        let err = validate_save_data(&data).unwrap_err();
        assert!(err.contains("older"));
    }

    #[test]
    fn test_only_first_event_is_validated() {
        let mut data = valid_data();
        data["compiled_events"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"garbage": true}));
        assert!(validate_save_data(&data).is_ok());
    }

    #[test]
    fn test_float_valued_priority_on_first_event_still_passes() {
        let mut data = valid_data();
        data["compiled_events"][0]["priority"] = serde_json::json!(0.0);
        assert!(validate_save_data(&data).is_ok());
    }
}

mod test_save_edit_plan_logic {
    use super::*;
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    const RIGHT_HAND: &str = "Right Hand";
    const LEFT_HAND: &str = "Left Hand";
    const SAME_NAME: &str = "Same";
    const TICKS_PER_BEAT: u16 = 480;
    const NOTE_VELOCITY: u8 = 64;
    const MISSING_TRACK_INDEX: i64 = 9;

    fn config_for(midi: &tempfile::TempPath) -> PlaybackConfig {
        let mut config = PlaybackConfig::default();
        config.midi_file = midi.to_string_lossy().to_string();
        config.pedal_style = "harmonic".to_string();
        config
    }

    fn real_save(config: &PlaybackConfig, role: &str) -> Value {
        let selected = [(0, role.to_string())];
        let mut session = PlaybackSession::default();
        compile_notes_logic(config, &selected, &mut session).unwrap();
        compile_pedal_logic(config, None, &mut session).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = save_playback_logic(
            config,
            &session,
            &[track_fixture()],
            &selected,
            dir.path(),
            "song.mid",
            "song",
        )
        .unwrap();
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn without_track_index(mut data: Value) -> Value {
        for detail in data["metadata"]["track_details"].as_array_mut().unwrap() {
            detail.as_object_mut().unwrap().remove("index");
        }
        data
    }

    fn note_track(pitch: u8) -> Vec<TrackEvent<'static>> {
        let channel = u4::from(0);
        vec![
            TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Meta(MetaMessage::TrackName(SAME_NAME.as_bytes())),
            },
            TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOn {
                        key: u7::from(pitch),
                        vel: u7::from(NOTE_VELOCITY),
                    },
                },
            },
            TrackEvent {
                delta: u28::from(u32::from(TICKS_PER_BEAT)),
                kind: TrackEventKind::Midi {
                    channel,
                    message: MidiMessage::NoteOff {
                        key: u7::from(pitch),
                        vel: u7::from(0),
                    },
                },
            },
        ]
    }

    fn two_identical_tracks_midi() -> tempfile::TempPath {
        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(u15::from(TICKS_PER_BEAT))),
            tracks: vec![note_track(60), note_track(62)],
        };
        let mut bytes = Vec::new();
        smf.write(&mut bytes).unwrap();
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), &bytes).unwrap();
        file.into_temp_path()
    }

    fn two_track_save(midi: &tempfile::TempPath) -> Value {
        let config = config_for(midi);
        let tracks: Vec<MidiTrack> = (0..2)
            .map(|index| MidiTrack {
                index,
                name: SAME_NAME.to_string(),
                program_change: 0,
                is_drum: false,
                notes: vec![Note::new(0, 60, 64, 0.0, 1.0)],
            })
            .collect();
        let selected = [(0, RIGHT_HAND.to_string()), (1, LEFT_HAND.to_string())];
        let mut session = PlaybackSession::default();
        compile_notes_logic(&config, &selected, &mut session).unwrap();
        compile_pedal_logic(&config, None, &mut session).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = save_playback_logic(&config, &session, &tracks, &selected, dir.path(), "song.mid", "song")
            .unwrap();
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn test_a_real_save_gives_back_its_midi_path_settings_and_tracks() {
        let midi = write_temp_midi_single_note();
        let mut config = config_for(&midi);
        config.tempo = 0.8;
        config.transpose = 2;
        config.use_88_key_layout = true;
        config.vary_timing = true;
        config.timing_variance = 0.02;
        let plan = save_edit_plan_logic(&real_save(&config, LEFT_HAND)).unwrap();

        assert_eq!(plan.midi_file, config.midi_file);
        assert_eq!(plan.settings.midi_file, config.midi_file);
        assert_eq!(plan.settings.tempo, 0.8);
        assert_eq!(plan.settings.transpose, 2);
        assert_eq!(plan.settings.pedal_style, "harmonic");
        assert!(plan.settings.use_88_key_layout);
        assert!(plan.settings.vary_timing);
        assert_eq!(plan.settings.timing_variance, 0.02);
        assert_eq!(
            plan.tracks,
            vec![EditTrack {
                index: 0,
                role: LEFT_HAND.to_string(),
                name: "Track 0".to_string(),
                note_count: 1,
            }]
        );
        assert!(!plan.legacy_pedal_thresholds);
    }

    #[test]
    fn test_the_saved_pedal_curves_come_back_in_the_settings() {
        let midi = write_temp_midi_single_note();
        let mut config = config_for(&midi);
        config.pedal_curve_on = vec![
            ThresholdNode { time: 0.0, value: 0.6 },
            ThresholdNode { time: 5.0, value: 0.4 },
        ];
        config.pedal_curve_off = vec![
            ThresholdNode { time: 0.0, value: 0.5 },
            ThresholdNode { time: 5.0, value: 0.3 },
        ];
        let plan = save_edit_plan_logic(&real_save(&config, RIGHT_HAND)).unwrap();

        assert_eq!(plan.settings.pedal_curve_on.len(), 2);
        assert_eq!(plan.settings.pedal_curve_on[1].value, 0.4);
        assert_eq!(plan.settings.pedal_curve_off[0].value, 0.5);
        assert!(!plan.legacy_pedal_thresholds);
    }

    #[test]
    fn test_an_old_threshold_pair_becomes_flat_curves_and_is_flagged() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        let settings = data["metadata"]["playback_settings"].as_object_mut().unwrap();
        settings.remove("pedal_curve_on");
        settings.remove("pedal_curve_off");
        settings.insert("pedal_threshold_on".to_string(), serde_json::json!(0.5));
        settings.insert("pedal_threshold_off".to_string(), serde_json::json!(0.3));
        let plan = save_edit_plan_logic(&data).unwrap();

        assert!(plan.legacy_pedal_thresholds);
        assert!(!plan.settings.pedal_curve_on.is_empty());
        assert!(plan.settings.pedal_curve_on.iter().all(|n| n.value == 0.5));
        assert!(plan.settings.pedal_curve_off.iter().all(|n| n.value == 0.3));
    }

    #[test]
    fn test_curves_that_cannot_be_read_mean_automatic_curves_not_a_failure() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        let settings = data["metadata"]["playback_settings"].as_object_mut().unwrap();
        settings.insert("pedal_curve_on".to_string(), serde_json::json!("garbage"));
        let plan = save_edit_plan_logic(&data).unwrap();

        assert!(plan.settings.pedal_curve_on.is_empty());
        assert!(plan.settings.pedal_curve_off.is_empty());
    }

    #[test]
    fn test_a_save_from_before_track_indices_matches_tracks_by_name_and_note_count() {
        let midi = write_temp_midi_single_note();
        let mut data = without_track_index(real_save(&config_for(&midi), RIGHT_HAND));
        let detail = data["metadata"]["track_details"][0].as_object_mut().unwrap();
        detail.insert("name".to_string(), serde_json::json!("Track 0"));
        let plan = save_edit_plan_logic(&data).unwrap();

        assert_eq!(plan.tracks.len(), 1);
        assert_eq!(plan.tracks[0].index, 0);
        assert_eq!(plan.tracks[0].role, RIGHT_HAND);
    }

    #[test]
    fn test_two_unindexed_tracks_with_the_same_name_and_count_get_different_midi_tracks() {
        let midi = two_identical_tracks_midi();
        let plan = save_edit_plan_logic(&without_track_index(two_track_save(&midi))).unwrap();

        let picked: Vec<(i32, &str)> = plan.tracks.iter().map(|t| (t.index, t.role.as_str())).collect();
        assert_eq!(picked, vec![(0, RIGHT_HAND), (1, LEFT_HAND)]);
    }

    #[test]
    fn test_an_explicit_index_is_not_taken_by_an_unindexed_track_with_the_same_name() {
        let midi = two_identical_tracks_midi();
        let mut data = two_track_save(&midi);
        data["metadata"]["track_details"][0].as_object_mut().unwrap().remove("index");
        data["metadata"]["track_details"][1]["index"] = serde_json::json!(0);
        let plan = save_edit_plan_logic(&data).unwrap();

        let picked: Vec<(i32, &str)> = plan.tracks.iter().map(|t| (t.index, t.role.as_str())).collect();
        assert_eq!(picked, vec![(1, RIGHT_HAND), (0, LEFT_HAND)]);
    }

    #[test]
    fn test_an_unindexed_track_missing_from_the_midi_is_reported_by_name() {
        let midi = write_temp_midi_single_note();
        let mut data = without_track_index(real_save(&config_for(&midi), RIGHT_HAND));
        data["metadata"]["track_details"][0]["name"] = serde_json::json!("Strings");
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("\"Strings\""), "{error}");
        assert!(error.contains("not found in the MIDI file"), "{error}");
    }

    #[test]
    fn test_an_unindexed_track_with_a_different_note_count_is_not_a_match() {
        let midi = write_temp_midi_single_note();
        let mut data = without_track_index(real_save(&config_for(&midi), RIGHT_HAND));
        data["metadata"]["track_details"][0]["name"] = serde_json::json!("Track 0");
        data["metadata"]["track_details"][0]["note_count"] = serde_json::json!(5);

        assert!(save_edit_plan_logic(&data).unwrap_err().contains("not found in the MIDI file"));
    }

    #[test]
    fn test_a_track_index_the_midi_no_longer_has_is_reported() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"]["track_details"][0]["index"] = serde_json::json!(MISSING_TRACK_INDEX);
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("not found in the MIDI file"), "{error}");
    }

    #[test]
    fn test_a_track_without_a_hand_role_is_reported() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"]["track_details"][0].as_object_mut().unwrap().remove("role");
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("no hand role"), "{error}");
    }

    #[test]
    fn test_a_save_with_no_track_selection_is_reported() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"]["track_details"] = serde_json::json!([]);

        assert!(save_edit_plan_logic(&data).unwrap_err().contains("track selection"));
    }

    #[test]
    fn test_a_save_with_no_midi_path_says_to_use_load() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"]["playback_settings"]["midi_file"] = serde_json::json!("  ");
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("does not record which MIDI file"), "{error}");
        assert!(error.contains("Use Load"), "{error}");
    }

    #[test]
    fn test_a_midi_file_that_has_moved_is_reported_with_its_path() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        let missing = std::env::temp_dir().join("humidi-no-such-song.mid");
        data["metadata"]["playback_settings"]["midi_file"] = serde_json::json!(missing.to_string_lossy());
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("was not found"), "{error}");
        assert!(error.contains("humidi-no-such-song.mid"), "{error}");
        assert!(error.contains("Load to play the save as it was"), "{error}");
    }

    #[test]
    fn test_a_midi_path_that_is_not_a_midi_file_is_reported_not_a_panic() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        let not_midi = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(not_midi.path(), b"this is not a midi file").unwrap();
        data["metadata"]["playback_settings"]["midi_file"] =
            serde_json::json!(not_midi.path().to_string_lossy());
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("could not be read"), "{error}");
    }

    #[test]
    fn test_a_save_with_no_settings_is_reported() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"].as_object_mut().unwrap().remove("playback_settings");

        assert!(save_edit_plan_logic(&data).unwrap_err().contains("no recorded settings"));
    }

    #[test]
    fn test_a_save_without_metadata_is_reported() {
        assert_eq!(
            save_edit_plan_logic(&serde_json::json!({})).unwrap_err(),
            "Save file is missing 'metadata'."
        );
    }

    #[test]
    fn test_settings_of_the_wrong_type_are_reported() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        data["metadata"]["playback_settings"]["tempo"] = serde_json::json!("fast");
        let error = save_edit_plan_logic(&data).unwrap_err();

        assert!(error.contains("settings could not be read"), "{error}");
    }

    #[test]
    fn test_settings_the_save_does_not_have_fall_back_to_the_defaults() {
        let midi = write_temp_midi_single_note();
        let mut data = real_save(&config_for(&midi), RIGHT_HAND);
        let settings = data["metadata"]["playback_settings"].as_object_mut().unwrap();
        settings.remove("transpose");
        settings.remove("pedal_style");
        let plan = save_edit_plan_logic(&data).unwrap();

        assert_eq!(plan.settings.transpose, PlaybackConfig::default().transpose);
        assert_eq!(plan.settings.pedal_style, PlaybackConfig::default().pedal_style);
    }
}

mod test_load_save_file_logic {
    use super::*;

    fn valid_save_json() -> Value {
        serde_json::json!({
            "metadata": {
                "track_details": [],
                "compiled_pedal_count": 0,
                "last_accessed": "old-value",
            },
            "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
        })
    }

    #[test]
    fn test_loads_and_returns_parsed_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.json");
        std::fs::write(&path, valid_save_json().to_string()).unwrap();

        let data = load_save_file_logic(&path).unwrap();
        assert_eq!(data["metadata"]["compiled_pedal_count"], 0);
    }

    #[test]
    fn test_stamps_last_accessed_in_returned_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.json");
        std::fs::write(&path, valid_save_json().to_string()).unwrap();

        let data = load_save_file_logic(&path).unwrap();
        assert_ne!(data["metadata"]["last_accessed"], serde_json::json!("old-value"));
    }

    #[test]
    fn test_invalid_save_data_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.json");
        std::fs::write(&path, serde_json::json!({"nope": true}).to_string()).unwrap();

        assert!(load_save_file_logic(&path).is_err());
    }

    #[test]
    fn test_missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(load_save_file_logic(&path).is_err());
    }

    #[test]
    fn test_eventually_persists_stamped_last_accessed_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.json");
        std::fs::write(&path, valid_save_json().to_string()).unwrap();

        let data = load_save_file_logic(&path).unwrap();
        let expected = data["metadata"]["last_accessed"].clone();

        for _ in 0..50 {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(on_disk) = serde_json::from_str::<Value>(&content) {
                    if on_disk["metadata"]["last_accessed"] == expected {
                        return;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("stamped last_accessed was not persisted to disk in time");
    }
}

mod test_list_saves_logic {
    use super::*;

    fn save_json(
        song_name: &str,
        created: &str,
        last_accessed: &str,
        note_counts: &[i64],
        pedal_count: i64,
    ) -> Value {
        save_json_with_settings(
            song_name,
            created,
            last_accessed,
            note_counts,
            pedal_count,
            serde_json::json!({}),
        )
    }

    fn save_json_with_settings(
        song_name: &str,
        created: &str,
        last_accessed: &str,
        note_counts: &[i64],
        pedal_count: i64,
        playback_settings: Value,
    ) -> Value {
        let track_details: Vec<Value> = note_counts
            .iter()
            .map(|&n| {
                serde_json::json!({
                    "name": "t", "note_count": n, "pitch_min": 0, "pitch_max": 0, "role": "Right Hand"
                })
            })
            .collect();
        serde_json::json!({
            "metadata": {
                "creation_timestamp": created,
                "last_accessed": last_accessed,
                "source_midi_filename": song_name,
                "track_details": track_details,
                "compiled_pedal_count": pedal_count,
                "playback_settings": playback_settings,
            },
            "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
        })
    }

    #[test]
    fn test_missing_directory_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does_not_exist");
        assert!(list_saves_logic(&missing).is_empty());
    }

    #[test]
    fn test_empty_directory_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_saves_logic(dir.path()).is_empty());
    }

    #[test]
    fn test_single_valid_save_returns_one_summary() {
        let dir = tempfile::tempdir().unwrap();
        let data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[5],
            2,
        );
        std::fs::write(dir.path().join("song_20260101_000000.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].song_name, "song.mid");
        assert_eq!(saves[0].note_count, 5);
        assert_eq!(saves[0].pedal_count, 2);
        assert_eq!(saves[0].track_count, 1);
        assert_eq!(saves[0].filename, "song_20260101_000000.json");
        assert_eq!(saves[0].tempo, 100.0);
        assert_eq!(saves[0].pedal_style, "none");
        assert!(!saves[0].use_88_key_layout);
        assert!(saves[0].humanization.is_empty());
    }

    #[test]
    fn test_playback_settings_reflected_in_summary() {
        let dir = tempfile::tempdir().unwrap();
        let settings = serde_json::json!({
            "tempo": 0.8,
            "pedal_style": "ai",
            "use_88_key_layout": true,
            "vary_timing": true,
            "enable_tempo_sway": true,
        });
        let data = save_json_with_settings(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
            settings,
        );
        std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].tempo, 80.0);
        assert_eq!(saves[0].pedal_style, "ai");
        assert!(saves[0].use_88_key_layout);
        assert_eq!(
            saves[0].humanization,
            vec!["Vary Timing".to_string(), "Tempo Sway".to_string()]
        );
    }

    #[test]
    fn test_missing_playback_settings_uses_safe_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        data["metadata"].as_object_mut().unwrap().remove("playback_settings");
        std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].tempo, 100.0);
        assert_eq!(saves[0].pedal_style, "none");
        assert!(!saves[0].use_88_key_layout);
        assert!(saves[0].humanization.is_empty());
    }

    #[test]
    fn test_non_json_files_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), "hello").unwrap();
        assert!(list_saves_logic(dir.path()).is_empty());
    }

    #[test]
    fn test_malformed_json_skipped_others_still_returned() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.json"), "{not valid json").unwrap();
        let data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        std::fs::write(dir.path().join("good.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].song_name, "song.mid");
    }

    #[test]
    fn test_directory_named_dot_json_skipped_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("weird.json")).unwrap();
        assert!(list_saves_logic(dir.path()).is_empty());
    }

    #[test]
    fn test_valid_json_failing_schema_validation_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("old.json"),
            serde_json::json!({"metadata": {}, "compiled_events": []}).to_string(),
        )
        .unwrap();
        assert!(list_saves_logic(dir.path()).is_empty());
    }

    #[test]
    fn test_older_version_missing_track_details_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        data["metadata"].as_object_mut().unwrap().remove("track_details");
        std::fs::write(dir.path().join("old.json"), data.to_string()).unwrap();
        assert!(list_saves_logic(dir.path()).is_empty());
    }

    #[test]
    fn test_sorted_newest_last_accessed_first() {
        let dir = tempfile::tempdir().unwrap();
        let older = save_json(
            "a.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        let newer = save_json(
            "b.mid",
            "2026-01-02T00:00:00+00:00",
            "2026-01-02T00:00:00+00:00",
            &[1],
            0,
        );
        std::fs::write(dir.path().join("a.json"), older.to_string()).unwrap();
        std::fs::write(dir.path().join("b.json"), newer.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 2);
        assert_eq!(saves[0].song_name, "b.mid");
        assert_eq!(saves[1].song_name, "a.mid");
    }

    #[test]
    fn test_sorts_by_actual_instant_not_lexicographic_string() {
        let dir = tempfile::tempdir().unwrap();
        let actually_later = save_json("later.mid", "x", "2026-01-01T23:00:00-05:00", &[1], 0);
        let actually_earlier =
            save_json("earlier.mid", "x", "2026-01-02T00:30:00+00:00", &[1], 0);
        std::fs::write(dir.path().join("later.json"), actually_later.to_string()).unwrap();
        std::fs::write(dir.path().join("earlier.json"), actually_earlier.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].song_name, "later.mid");
        assert_eq!(saves[1].song_name, "earlier.mid");
    }

    #[test]
    fn test_missing_last_accessed_falls_back_to_creation_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        data["metadata"].as_object_mut().unwrap().remove("last_accessed");
        std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].last_accessed, "2026-01-01T00:00:00+00:00");
    }

    #[test]
    fn test_note_count_sums_across_multiple_tracks() {
        let dir = tempfile::tempdir().unwrap();
        let data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[3, 7],
            0,
        );
        std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].note_count, 10);
        assert_eq!(saves[0].track_count, 2);
    }

    #[test]
    fn test_multiple_valid_saves_all_returned() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..5 {
            let data = save_json(
                &format!("song{i}.mid"),
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            std::fs::write(dir.path().join(format!("song{i}.json")), data.to_string()).unwrap();
        }
        assert_eq!(list_saves_logic(dir.path()).len(), 5);
    }

    #[test]
    fn test_python_naive_isoformat_timestamps_sort_correctly_not_as_min_utc() {
        let dir = tempfile::tempdir().unwrap();
        let older = save_json("a.mid", "2026-01-01T00:00:00.123456", "2026-01-01T00:00:00.123456", &[1], 0);
        let newer = save_json("b.mid", "2026-01-02T00:00:00", "2026-01-02T00:00:00", &[1], 0);
        std::fs::write(dir.path().join("a.json"), older.to_string()).unwrap();
        std::fs::write(dir.path().join("b.json"), newer.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 2);
        assert_eq!(saves[0].song_name, "b.mid");
        assert_eq!(saves[1].song_name, "a.mid");
    }

    #[test]
    fn test_python_naive_timestamp_sorts_above_unparseable_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let naive = save_json("a.mid", "2026-01-01T00:00:00", "2026-01-01T00:00:00", &[1], 0);
        let garbage = save_json("b.mid", "not-a-timestamp", "not-a-timestamp", &[1], 0);
        std::fs::write(dir.path().join("a.json"), naive.to_string()).unwrap();
        std::fs::write(dir.path().join("b.json"), garbage.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves[0].song_name, "a.mid");
        assert_eq!(saves[1].song_name, "b.mid");
    }

    #[test]
    fn test_missing_source_midi_filename_defaults_instead_of_dropping_save() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        data["metadata"].as_object_mut().unwrap().remove("source_midi_filename");
        std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 1);
        assert_eq!(saves[0].song_name, "Unknown MIDI");
    }

    #[test]
    fn test_uppercase_json_extension_included_case_insensitively() {
        let dir = tempfile::tempdir().unwrap();
        let data = save_json(
            "song.mid",
            "2026-01-01T00:00:00+00:00",
            "2026-01-01T00:00:00+00:00",
            &[1],
            0,
        );
        std::fs::write(dir.path().join("song.JSON"), data.to_string()).unwrap();

        let saves = list_saves_logic(dir.path());
        assert_eq!(saves.len(), 1);
    }
}

mod test_delete_save_logic {
    use super::*;

    #[test]
    fn test_deletes_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.json");
        std::fs::write(&path, "{}").unwrap();

        assert!(delete_save_logic(&path).is_ok());
        assert!(!path.exists());
    }

    #[test]
    fn test_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(delete_save_logic(&path).is_err());
    }

    #[test]
    fn test_refuses_non_json_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mid");
        std::fs::write(&path, "not a save").unwrap();

        assert!(delete_save_logic(&path).is_err());
        assert!(path.exists());
    }

    #[test]
    fn test_uppercase_json_extension_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.JSON");
        std::fs::write(&path, "{}").unwrap();

        assert!(delete_save_logic(&path).is_ok());
        assert!(!path.exists());
    }

    #[test]
    fn test_refuses_file_with_no_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noext");
        std::fs::write(&path, "x").unwrap();

        assert!(delete_save_logic(&path).is_err());
        assert!(path.exists());
    }

    #[test]
    fn test_directory_named_dot_json_does_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("weird.json");
        std::fs::create_dir(&path).unwrap();

        assert!(delete_save_logic(&path).is_err());
    }
}

mod test_sanitize_save_name {
    use super::*;

    #[test]
    fn test_simple_name_passes_through() {
        assert_eq!(sanitize_save_name("My Practice Run").unwrap(), "My Practice Run");
    }

    #[test]
    fn test_empty_name_errors() {
        assert!(sanitize_save_name("").is_err());
    }

    #[test]
    fn test_whitespace_only_errors() {
        assert!(sanitize_save_name("   ").is_err());
    }

    #[test]
    fn test_trailing_json_extension_stripped_case_insensitively() {
        assert_eq!(sanitize_save_name("song.json").unwrap(), "song");
        assert_eq!(sanitize_save_name("song.JSON").unwrap(), "song");
        assert_eq!(sanitize_save_name("song.Json").unwrap(), "song");
    }

    #[test]
    fn test_only_extension_after_stripping_errors() {
        assert!(sanitize_save_name(".json").is_err());
    }

    #[test]
    fn test_forward_slash_rejected() {
        assert!(sanitize_save_name("a/b").is_err());
    }

    #[test]
    fn test_backslash_rejected() {
        assert!(sanitize_save_name("a\\b").is_err());
    }

    #[test]
    fn test_parent_traversal_rejected() {
        assert!(sanitize_save_name("../evil").is_err());
        assert!(sanitize_save_name("..\\evil").is_err());
    }

    #[test]
    fn test_bare_dot_and_dotdot_rejected() {
        assert!(sanitize_save_name(".").is_err());
        assert!(sanitize_save_name("..").is_err());
    }

    #[test]
    fn test_windows_reserved_characters_rejected() {
        for ch in ['<', '>', ':', '"', '|', '?', '*'] {
            let name = format!("bad{ch}name");
            assert!(sanitize_save_name(&name).is_err(), "expected '{ch}' to be rejected");
        }
    }

    #[test]
    fn test_control_characters_rejected() {
        assert!(sanitize_save_name("bad\u{0000}name").is_err());
        assert!(sanitize_save_name("bad\tname").is_err());
    }

    #[test]
    fn test_trailing_dot_trimmed_away() {
        assert_eq!(sanitize_save_name("name.").unwrap(), "name");
    }

    #[test]
    fn test_trailing_space_trimmed_away() {
        assert_eq!(sanitize_save_name("name ").unwrap(), "name");
    }

    #[test]
    fn test_all_trailing_dots_and_spaces_trimmed() {
        assert_eq!(sanitize_save_name("name. . .").unwrap(), "name");
    }

    #[test]
    fn test_name_that_is_only_dots_and_spaces_errors() {
        assert!(sanitize_save_name(". . .").is_err());
    }

    #[test]
    fn test_unicode_name_accepted() {
        assert_eq!(sanitize_save_name("Clair de Lune \u{1f3b9}").unwrap(), "Clair de Lune \u{1f3b9}");
    }

    #[test]
    fn test_leading_and_trailing_whitespace_trimmed() {
        assert_eq!(sanitize_save_name("  song  ").unwrap(), "song");
    }

    #[test]
    fn test_multibyte_name_near_extension_check_does_not_panic() {
        assert_eq!(sanitize_save_name("\u{c9}tude").unwrap(), "\u{c9}tude");
        assert_eq!(sanitize_save_name("\u{65e5}\u{672c}\u{8a9e}").unwrap(), "\u{65e5}\u{672c}\u{8a9e}");
        assert_eq!(
            sanitize_save_name("\u{41f}\u{440}\u{435}\u{43b}\u{44e}\u{434}\u{438}\u{44f}").unwrap(),
            "\u{41f}\u{440}\u{435}\u{43b}\u{44e}\u{434}\u{438}\u{44f}"
        );
        assert_eq!(sanitize_save_name("\u{1f3b9}\u{1f3b9}").unwrap(), "\u{1f3b9}\u{1f3b9}");
    }

    #[test]
    fn test_multibyte_name_with_json_suffix_still_strips_extension() {
        assert_eq!(sanitize_save_name("\u{65e5}\u{672c}.json").unwrap(), "\u{65e5}\u{672c}");
    }

    #[test]
    fn test_windows_reserved_device_names_rejected() {
        for name in ["CON", "con", "NUL", "PRN", "AUX", "COM1", "LPT9"] {
            assert!(sanitize_save_name(name).is_err(), "expected '{name}' to be rejected");
        }
    }

    #[test]
    fn test_reserved_device_name_as_substring_is_allowed() {
        assert_eq!(sanitize_save_name("CONcerto").unwrap(), "CONcerto");
    }
}

mod test_rename_save_logic {
    use super::*;

    #[test]
    fn test_renames_and_returns_new_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, "{\"a\":1}").unwrap();

        let new_path = rename_save_logic(&path, "new name").unwrap();
        assert!(new_path.ends_with("new name.json"));
        assert!(!path.exists());
        assert!(std::path::Path::new(&new_path).exists());
    }

    #[test]
    fn test_preserves_file_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, "{\"marker\":\"hello\"}").unwrap();

        let new_path = rename_save_logic(&path, "renamed").unwrap();
        let content = std::fs::read_to_string(new_path).unwrap();
        assert!(content.contains("hello"));
    }

    #[test]
    fn test_missing_source_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");
        assert!(rename_save_logic(&path, "whatever").is_err());
    }

    #[test]
    fn test_invalid_new_name_errors_and_leaves_original_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, "{}").unwrap();

        assert!(rename_save_logic(&path, "bad/name").is_err());
        assert!(path.exists());
    }

    #[test]
    fn test_target_name_already_taken_errors_and_leaves_both_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        let taken_path = dir.path().join("taken.json");
        std::fs::write(&path, "{\"marker\":\"old\"}").unwrap();
        std::fs::write(&taken_path, "{\"marker\":\"taken\"}").unwrap();

        assert!(rename_save_logic(&path, "taken").is_err());
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&taken_path).unwrap(), "{\"marker\":\"taken\"}");
    }

    #[test]
    fn test_renaming_to_its_own_current_name_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("same.json");
        std::fs::write(&path, "{}").unwrap();

        assert!(rename_save_logic(&path, "same").is_ok());
    }

    #[test]
    fn test_new_name_with_json_extension_does_not_double_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, "{}").unwrap();

        let new_path = rename_save_logic(&path, "renamed.json").unwrap();
        assert!(new_path.ends_with("renamed.json"));
        assert!(!new_path.ends_with("renamed.json.json"));
    }

    #[test]
    fn test_refuses_to_rename_a_non_json_source_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.mid");
        std::fs::write(&path, "not a save").unwrap();

        assert!(rename_save_logic(&path, "renamed").is_err());
        assert!(path.exists());
    }

    #[test]
    fn test_reserved_device_name_rejected_and_leaves_original_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.json");
        std::fs::write(&path, "{}").unwrap();

        assert!(rename_save_logic(&path, "CON").is_err());
        assert!(path.exists());
    }

    #[test]
    fn test_case_only_rename_succeeds_instead_of_reporting_name_taken() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.json");
        std::fs::write(&path, "{\"marker\":\"hi\"}").unwrap();

        let new_path = rename_save_logic(&path, "Song").unwrap();
        assert!(new_path.ends_with("Song.json"));
        assert!(std::path::Path::new(&new_path).exists());
        assert_eq!(std::fs::read_to_string(&new_path).unwrap(), "{\"marker\":\"hi\"}");
    }
}

mod test_deliver_key {
    use super::*;
    use crate::core::keyboard_driver::Key;
    use crate::managers::hotkey_manager::{dom_code_to_raw, RawKey};

    fn raw(code: &str) -> RawKey {
        dom_code_to_raw(code).unwrap()
    }

    fn press(state: &AppState, code: &str, source: KeySource) -> Vec<HotkeyEvent> {
        deliver_key(state, raw(code), true, source)
    }

    fn release(state: &AppState, code: &str, source: KeySource) -> Vec<HotkeyEvent> {
        deliver_key(state, raw(code), false, source)
    }

    fn state_in(dir: &tempfile::TempDir) -> AppState {
        hermetic_state(dir.path())
    }

    fn held(state: &AppState) -> usize {
        state.hotkey_manager.lock().unwrap().held_modifiers().len()
    }

    #[test]
    fn test_a_global_hook_press_while_listening_binds_and_saves_the_hotkey() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.hotkey_manager.lock().unwrap().start_binding();
        let events = press(&state, "KeyG", KeySource::GlobalHook);
        assert_eq!(events, vec![HotkeyEvent::BoundUpdated("G".to_string())]);
        assert_eq!(state.app_config.lock().unwrap()[PLAYBACK_HOTKEY_CONFIG_KEY], "G");
    }

    #[test]
    fn test_a_page_press_while_listening_binds_and_saves_the_hotkey_when_the_hook_heard_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.hotkey_manager.lock().unwrap().start_binding();
        let events = press(&state, "KeyG", KeySource::Page);
        assert_eq!(events, vec![HotkeyEvent::BoundUpdated("G".to_string())]);
        assert_eq!(state.app_config.lock().unwrap()[PLAYBACK_HOTKEY_CONFIG_KEY], "G");
    }

    #[test]
    fn test_a_page_press_while_listening_for_save_saves_it_under_the_save_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.hotkey_manager.lock().unwrap().start_save_binding();
        let events = press(&state, "KeyD", KeySource::Page);
        assert_eq!(events, vec![HotkeyEvent::BoundSaveUpdated("D".to_string())]);
        assert_eq!(state.app_config.lock().unwrap()[SAVE_HOTKEY_CONFIG_KEY], "D");
    }

    #[test]
    fn test_a_trigger_is_not_saved_to_the_config() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
        assert_eq!(*state.app_config.lock().unwrap(), serde_json::json!({}));
    }

    #[test]
    fn test_a_page_press_with_no_hook_press_before_it_triggers() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::Page), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_a_page_press_right_after_the_hook_delivered_the_same_key_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
        assert!(press(&state, "F6", KeySource::Page).is_empty());
    }

    #[test]
    fn test_a_duplicate_does_not_swallow_the_next_real_press() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
        assert!(press(&state, "F6", KeySource::Page).is_empty());
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
        assert!(press(&state, "F6", KeySource::Page).is_empty());
    }

    #[test]
    fn test_page_only_presses_each_count_when_the_hook_is_deaf() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::Page), vec![HotkeyEvent::ToggleRequested]);
        assert_eq!(press(&state, "F6", KeySource::Page), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_a_different_key_from_the_page_is_not_mistaken_for_the_hooks_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "F7", KeySource::GlobalHook);
        assert_eq!(press(&state, "F6", KeySource::Page), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_a_modifier_pressed_and_released_through_both_sources_ends_up_released() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        press(&state, "ControlLeft", KeySource::Page);
        assert_eq!(held(&state), 1);
        release(&state, "ControlLeft", KeySource::GlobalHook);
        release(&state, "ControlLeft", KeySource::Page);
        assert_eq!(held(&state), 0);
    }

    #[test]
    fn test_a_page_release_clears_a_modifier_the_page_pressed() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ShiftRight", KeySource::Page);
        assert_eq!(held(&state), 1);
        assert!(release(&state, "ShiftRight", KeySource::Page).is_empty());
        assert_eq!(held(&state), 0);
    }

    #[test]
    fn test_a_page_release_the_hook_already_delivered_is_dropped_and_harmless() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::Page);
        release(&state, "ControlLeft", KeySource::GlobalHook);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        release(&state, "ControlLeft", KeySource::Page);
        assert_eq!(held(&state), 0);
    }

    #[test]
    fn test_a_combo_started_through_the_hook_finishes_through_the_page() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        assert_eq!(press(&state, "KeyS", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_a_save_combo_pressed_outside_the_window_does_not_request_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
    }

    #[test]
    fn test_a_save_combo_started_in_the_window_and_finished_outside_does_not_request_a_save() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::Page);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
    }

    #[test]
    fn test_the_playback_hotkey_pressed_outside_the_window_still_toggles() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_a_plain_key_save_hotkey_does_nothing_outside_the_window_and_saves_inside() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert!(state.hotkey_manager.lock().unwrap().set_save_hotkey_from_string("D"));
        assert!(press(&state, "KeyD", KeySource::GlobalHook).is_empty());
        assert_eq!(press(&state, "KeyD", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_a_save_the_hook_heard_is_requested_once_when_the_window_reports_the_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
        assert!(press(&state, "ControlLeft", KeySource::Page).is_empty());
        assert_eq!(press(&state, "KeyS", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_the_window_confirming_a_save_does_not_confirm_the_next_one() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
        assert_eq!(press(&state, "KeyS", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
        release(&state, "KeyS", KeySource::GlobalHook);
        release(&state, "KeyS", KeySource::Page);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
    }

    #[test]
    fn test_a_save_heard_only_by_the_hook_is_never_requested_later() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
        release(&state, "KeyS", KeySource::GlobalHook);
        assert!(press(&state, "KeyF", KeySource::Page).is_empty());
    }

    #[test]
    fn test_the_same_key_for_both_hotkeys_toggles_once_and_saves_only_inside_the_window() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert!(state.hotkey_manager.lock().unwrap().set_save_hotkey_from_string("F6"));
        assert_eq!(press(&state, "F6", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
        assert_eq!(press(&state, "F6", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_binding_the_save_hotkey_still_works_from_outside_the_window() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.hotkey_manager.lock().unwrap().start_save_binding();
        assert_eq!(
            press(&state, "KeyD", KeySource::GlobalHook),
            vec![HotkeyEvent::BoundSaveUpdated("D".to_string())]
        );
        assert!(press(&state, "KeyD", KeySource::Page).is_empty());
    }

    #[test]
    fn test_a_playback_combo_started_through_the_page_finishes_through_the_hook() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        assert!(state.hotkey_manager.lock().unwrap().set_playback_hotkey_from_string("Ctrl+K"));
        press(&state, "ControlLeft", KeySource::Page);
        assert_eq!(press(&state, "KeyK", KeySource::GlobalHook), vec![HotkeyEvent::ToggleRequested]);
    }

    #[test]
    fn test_the_apps_own_key_is_dropped_when_it_arrives_from_the_page() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::Page);
        state.injected_echoes.expect(Key::Char('s'), true);
        assert!(press(&state, "KeyS", KeySource::Page).is_empty());
        assert_eq!(press(&state, "KeyS", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_the_apps_own_key_is_dropped_from_the_hook_and_from_its_page_copy() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        press(&state, "ControlLeft", KeySource::GlobalHook);
        state.injected_echoes.expect(Key::Char('s'), true);
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
        assert!(press(&state, "KeyS", KeySource::Page).is_empty());
        assert!(press(&state, "KeyS", KeySource::GlobalHook).is_empty());
        assert_eq!(press(&state, "KeyS", KeySource::Page), vec![HotkeyEvent::SaveRequested]);
    }

    #[test]
    fn test_the_apps_own_modifier_never_reaches_the_held_set_from_either_source() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        state.injected_echoes.expect(Key::Ctrl, true);
        assert!(press(&state, "ControlLeft", KeySource::Page).is_empty());
        assert_eq!(held(&state), 0);
    }

    #[test]
    fn test_a_poisoned_hotkey_manager_yields_no_events() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        std::thread::scope(|scope| {
            let poisoner = scope.spawn(|| {
                let _guard = state.hotkey_manager.lock().unwrap();
                panic!("poison the hotkey manager");
            });
            let _ = poisoner.join();
        });
        assert!(press(&state, "F6", KeySource::GlobalHook).is_empty());
        assert!(release(&state, "F6", KeySource::Page).is_empty());
    }
}

mod test_key_from_rdev_event {
    use super::*;
    use crate::managers::hotkey_manager::{Modifier, RawKey};

    #[test]
    fn test_a_mapped_key_press_is_a_press() {
        assert_eq!(
            key_from_rdev_event(&rdev::EventType::KeyPress(rdev::Key::KeyG)),
            Some((RawKey::Char('g'), true))
        );
    }

    #[test]
    fn test_a_mapped_key_release_is_a_release() {
        assert_eq!(
            key_from_rdev_event(&rdev::EventType::KeyRelease(rdev::Key::ControlLeft)),
            Some((RawKey::ModLeft(Modifier::Ctrl), false))
        );
    }

    #[test]
    fn test_an_unmapped_key_is_ignored_in_both_directions() {
        assert_eq!(key_from_rdev_event(&rdev::EventType::KeyPress(rdev::Key::Unknown(0))), None);
        assert_eq!(key_from_rdev_event(&rdev::EventType::KeyRelease(rdev::Key::Unknown(0))), None);
    }

    #[test]
    fn test_mouse_events_are_ignored() {
        assert_eq!(key_from_rdev_event(&rdev::EventType::ButtonPress(rdev::Button::Left)), None);
        assert_eq!(key_from_rdev_event(&rdev::EventType::ButtonRelease(rdev::Button::Left)), None);
        assert_eq!(key_from_rdev_event(&rdev::EventType::MouseMove { x: 0.0, y: 0.0 }), None);
        assert_eq!(key_from_rdev_event(&rdev::EventType::Wheel { delta_x: 0, delta_y: 0 }), None);
    }
}

mod test_hotkey_event_to_channel_payload {
    use super::*;

    #[test]
    fn test_bound_updated() {
        let (ch, payload) =
            hotkey_event_to_channel_payload(&HotkeyEvent::BoundUpdated("Alt+P".to_string()));
        assert_eq!(ch, "hotkey_bound_updated");
        assert_eq!(payload, serde_json::json!("Alt+P"));
    }

    #[test]
    fn test_bound_save_updated() {
        let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::BoundSaveUpdated(
            "Shift+K".to_string(),
        ));
        assert_eq!(ch, "hotkey_bound_save_updated");
        assert_eq!(payload, serde_json::json!("Shift+K"));
    }

    #[test]
    fn test_toggle_requested() {
        let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::ToggleRequested);
        assert_eq!(ch, "hotkey_toggle_requested");
        assert_eq!(payload, Value::Null);
    }

    #[test]
    fn test_save_requested() {
        let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::SaveRequested);
        assert_eq!(ch, "hotkey_save_requested");
        assert_eq!(payload, Value::Null);
    }
}

mod test_reconstruct_notes_for_visualizer {
    use super::*;

    fn press(t: f64, pitch: i32, vel: i32) -> KeyEvent {
        let mut e = KeyEvent::new(t, 2, "press", "a");
        e.pitch = Some(pitch);
        e.velocity = Some(vel);
        e
    }

    fn release(t: f64, pitch: i32) -> KeyEvent {
        let mut e = KeyEvent::new(t, 4, "release", "a");
        e.pitch = Some(pitch);
        e
    }

    #[test]
    fn test_empty_events_produce_no_notes() {
        assert!(reconstruct_notes_for_visualizer(&[]).is_empty());
    }

    #[test]
    fn test_simple_press_release_pair() {
        let events = [press(1.0, 60, 80), release(1.5, 60)];
        let notes = reconstruct_notes_for_visualizer(&events);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[0].velocity, 80);
        assert!((notes[0].start_time - 1.0).abs() < 1e-9);
        assert!((notes[0].duration - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_dangling_press_without_release_dropped() {
        let events = [press(1.0, 60, 80)];
        assert!(reconstruct_notes_for_visualizer(&events).is_empty());
    }

    #[test]
    fn test_release_without_prior_press_ignored() {
        let events = [release(1.0, 60)];
        assert!(reconstruct_notes_for_visualizer(&events).is_empty());
    }

    #[test]
    fn test_fifo_pairing_for_repeated_same_pitch() {
        let events = [
            press(0.0, 60, 64),
            press(1.0, 60, 90),
            release(0.5, 60),
            release(2.0, 60),
        ];
        let notes = reconstruct_notes_for_visualizer(&events);
        assert_eq!(notes.len(), 2);
        assert!((notes[0].start_time - 0.0).abs() < 1e-9);
        assert!((notes[0].duration - 0.5).abs() < 1e-9);
        assert!((notes[1].start_time - 1.0).abs() < 1e-9);
        assert!((notes[1].duration - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_events_without_pitch_skipped() {
        let down = KeyEvent::new(0.0, 1, "pedal", "down");
        let up = KeyEvent::new(1.0, 0, "pedal", "up");
        assert!(reconstruct_notes_for_visualizer(&[down, up]).is_empty());
    }

    #[test]
    fn test_missing_velocity_defaults_to_64() {
        let mut p = press(0.0, 60, 0);
        p.velocity = None;
        let notes = reconstruct_notes_for_visualizer(&[p, release(0.5, 60)]);
        assert_eq!(notes[0].velocity, 64);
    }

    #[test]
    fn test_zero_duration_note_dropped() {
        let events = [press(1.0, 60, 64), release(1.0, 60)];
        assert!(reconstruct_notes_for_visualizer(&events).is_empty());
    }

    #[test]
    fn test_pedal_events_ignored_for_note_reconstruction() {
        let pedal_down = KeyEvent::new(0.0, 1, "pedal", "down");
        let events = [pedal_down, press(0.0, 60, 64), release(0.5, 60)];
        let notes = reconstruct_notes_for_visualizer(&events);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_note_ids_sequential() {
        let events = [
            press(0.0, 60, 64),
            release(0.5, 60),
            press(1.0, 62, 64),
            release(1.5, 62),
        ];
        let notes = reconstruct_notes_for_visualizer(&events);
        assert_eq!(notes[0].id, 0);
        assert_eq!(notes[1].id, 1);
    }
}

mod test_resume_from_save_logic {
    use super::*;
    use crate::state::PlaybackSession;

    fn events_value(events: Vec<Value>) -> Value {
        serde_json::json!({
            "metadata": {"track_details": [], "compiled_pedal_count": 0},
            "compiled_events": events,
        })
    }

    fn key_for(pitch: i32) -> String {
        KeyMapper::new(false)
            .get_key_data(pitch)
            .map(|d| d.key.to_string())
            .unwrap_or_default()
    }

    fn press_v(t: f64, pitch: i32) -> Value {
        serde_json::json!({"time": t, "priority": 2, "action": "press", "key_char": key_for(pitch), "pitch": pitch, "velocity": 64})
    }

    fn release_v(t: f64, pitch: i32) -> Value {
        serde_json::json!({"time": t, "priority": 4, "action": "release", "key_char": key_for(pitch), "pitch": pitch, "velocity": null})
    }

    #[test]
    fn test_valid_save_populates_session_for_playback() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert_eq!(result.final_notes.len(), 1);
        assert!((result.total_dur - 0.5).abs() < 1e-9);
        assert_eq!(session.merged_events.as_ref().unwrap().len(), 2);
        assert!(session.tempo_map.is_some());
        assert!((session.total_dur - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_played_notes_are_the_notes_rebuilt_from_the_saved_events() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert_eq!(result.played_notes, result.final_notes);
    }

    #[test]
    fn test_returns_track_count_and_pedal_count_from_save_metadata() {
        let mut data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        data["metadata"]["track_details"] = serde_json::json!([{"name": "Melody"}, {"name": "Bass"}]);
        data["metadata"]["compiled_pedal_count"] = serde_json::json!(7);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert_eq!(result.track_count, 2);
        assert_eq!(result.pedal_count, 7);
    }

    #[test]
    fn test_a_save_with_the_old_single_thresholds_resumes_as_flat_lines_and_is_flagged() {
        let mut data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        data["metadata"]["playback_settings"] =
            serde_json::json!({"pedal_threshold_on": 0.57, "pedal_threshold_off": 0.54});
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert!(result.legacy_pedal_thresholds);
        assert_eq!(result.pedal_curve_on.len(), 1);
        assert_eq!(result.pedal_curve_on[0].value, 0.57);
        assert_eq!(result.pedal_curve_off[0].value, 0.54);
    }

    #[test]
    fn test_a_save_with_curves_resumes_them_without_the_legacy_flag() {
        let mut data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        data["metadata"]["playback_settings"] = serde_json::json!({
            "pedal_curve_on": [{"time": 0.0, "value": 0.6}, {"time": 5.0, "value": 0.4}],
            "pedal_curve_off": [{"time": 0.0, "value": 0.5}, {"time": 5.0, "value": 0.3}],
        });
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert!(!result.legacy_pedal_thresholds);
        assert_eq!(result.pedal_curve_on.len(), 2);
        assert_eq!(result.pedal_curve_off[1].value, 0.3);
    }

    #[test]
    fn test_a_save_without_pedal_settings_resumes_with_empty_curves() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert!(!result.legacy_pedal_thresholds);
        assert!(result.pedal_curve_on.is_empty() && result.pedal_curve_off.is_empty());
    }

    #[test]
    fn test_missing_metadata_counts_default_to_zero() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        assert_eq!(result.track_count, 0);
        assert_eq!(result.pedal_count, 0);
    }

    #[test]
    fn test_returns_default_tempo_fields_matching_session_tempo_map() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();

        let session_tempo_map = session.tempo_map.as_ref().unwrap();
        assert_eq!(result.tempo_events, session_tempo_map.events());
        assert_eq!(result.time_signatures, session_tempo_map.time_signatures());
        assert_eq!(
            result.measure_boundaries,
            session_tempo_map.get_measure_boundaries(result.total_dur)
        );
        assert!(!result.measure_boundaries.is_empty());
    }

    #[test]
    fn test_notes_config_snapshot_left_none_so_pedal_recompile_still_gated() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
        let mut session = PlaybackSession::default();
        resume_from_save_logic(&data, &mut session).unwrap();
        assert!(session.notes_config_snapshot.is_none());

        let config = PlaybackConfig::default();
        assert!(compile_pedal_logic(&config, None, &mut session).is_err());
    }

    #[test]
    fn test_empty_compiled_events_errors() {
        let data = events_value(vec![]);
        let mut session = PlaybackSession::default();
        assert!(resume_from_save_logic(&data, &mut session).is_err());
    }

    #[test]
    fn test_malformed_event_deep_in_array_errors_with_index() {
        let mut events = vec![press_v(0.0, 60), release_v(0.5, 60)];
        let mut bad = press_v(1.0, 62);
        bad.as_object_mut().unwrap().remove("time");
        events.push(bad);
        let data = events_value(events);
        let mut session = PlaybackSession::default();
        let err = resume_from_save_logic(&data, &mut session).unwrap_err();
        assert!(err.contains("#2"));
    }

    #[test]
    fn test_out_of_order_events_are_sorted_before_playback() {
        let data = events_value(vec![release_v(0.5, 60), press_v(0.0, 60)]);
        let mut session = PlaybackSession::default();
        resume_from_save_logic(&data, &mut session).unwrap();
        let merged = session.merged_events.unwrap();
        assert_eq!(merged[0].action, "press");
        assert_eq!(merged[1].action, "release");
    }

    #[test]
    fn test_total_dur_uses_max_time_not_last_array_element() {
        let data = events_value(vec![
            press_v(5.0, 60),
            release_v(5.5, 60),
            press_v(0.0, 62),
            release_v(0.2, 62),
        ]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();
        assert!((result.total_dur - 5.5).abs() < 1e-9);
    }

    #[test]
    fn test_dangling_press_still_included_in_merged_events_but_not_visualized() {
        let data = events_value(vec![press_v(0.0, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();
        assert!(result.final_notes.is_empty());
        assert_eq!(session.merged_events.unwrap().len(), 1);
    }

    #[test]
    fn test_pedal_events_pass_through_to_merged_events() {
        let pedal_down =
            serde_json::json!({"time": 0.1, "priority": 1, "action": "pedal", "key_char": "down"});
        let pedal_up =
            serde_json::json!({"time": 0.9, "priority": 0, "action": "pedal", "key_char": "up"});
        let data = events_value(vec![press_v(0.0, 60), pedal_down, release_v(0.5, 60), pedal_up]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();
        assert_eq!(result.final_notes.len(), 1);
        assert_eq!(session.merged_events.unwrap().len(), 4);
    }

    #[test]
    fn test_invalid_save_data_rejected_before_touching_session() {
        let data = serde_json::json!({"nope": true});
        let mut session = PlaybackSession::default();
        assert!(resume_from_save_logic(&data, &mut session).is_err());
        assert!(session.merged_events.is_none());
    }

    #[test]
    fn test_all_events_at_time_zero_total_dur_floors_to_one() {
        let data = events_value(vec![press_v(0.0, 60), release_v(0.0, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();
        assert_eq!(result.total_dur, 1.0);
        assert_eq!(session.total_dur, 1.0);
    }

    #[test]
    fn test_negative_time_events_total_dur_floors_to_one_not_negative() {
        let data = events_value(vec![press_v(-5.0, 60), release_v(-3.0, 60)]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session).unwrap();
        assert_eq!(result.total_dur, 1.0);
    }

    #[test]
    fn test_float_valued_priority_and_pitch_are_coerced_not_rejected() {
        let event = serde_json::json!({
            "time": 0.0, "priority": 2.0, "action": "press", "key_char": key_for(60), "pitch": 60.0, "velocity": 64.0
        });
        let data = events_value(vec![event]);
        let mut session = PlaybackSession::default();
        let result = resume_from_save_logic(&data, &mut session);
        assert!(result.is_ok(), "float-valued numeric fields should be coerced, got {result:?}");
        assert_eq!(session.merged_events.unwrap()[0].pitch, Some(60));
    }

    #[test]
    fn test_out_of_i32_range_pitch_is_blocked_not_silently_truncated() {
        let event = serde_json::json!({
            "time": 0.0, "priority": 2, "action": "press", "key_char": key_for(60),
            "pitch": 5_000_000_000i64, "velocity": 64
        });
        let data = events_value(vec![event]);
        let mut session = PlaybackSession::default();
        let err = resume_from_save_logic(&data, &mut session).unwrap_err();
        assert!(err.starts_with(crate::core::input_scan::BLOCKED_INPUT_PREFIX));
        assert!(session.merged_events.is_none());
    }

    fn assert_blocked_and_session_untouched(events: Vec<Value>) -> String {
        let data = events_value(events);
        let mut session = PlaybackSession::default();
        let err = resume_from_save_logic(&data, &mut session).unwrap_err();
        assert!(err.starts_with(crate::core::input_scan::BLOCKED_INPUT_PREFIX));
        assert!(session.merged_events.is_none());
        assert!(session.final_notes.is_none());
        err
    }

    #[test]
    fn test_unknown_action_in_a_save_is_blocked_before_anything_is_loaded() {
        let shortcut = serde_json::json!({"time": 0.0, "priority": 2, "action": "shortcut", "key_char": "ctrl+alt+delete"});
        let err = assert_blocked_and_session_untouched(vec![press_v(0.0, 60), shortcut]);
        assert!(err.contains("event #1"));
        assert!(err.contains("unknown action 'shortcut'"));
    }

    #[test]
    fn test_press_of_a_non_piano_key_in_a_save_is_blocked() {
        let enter = serde_json::json!({"time": 0.0, "priority": 2, "action": "press", "key_char": "
", "pitch": 60, "velocity": 64});
        assert_blocked_and_session_untouched(vec![enter]);
    }

    #[test]
    fn test_press_without_a_note_in_a_save_is_blocked() {
        let bare = serde_json::json!({"time": 0.0, "priority": 2, "action": "press", "key_char": "r"});
        assert_blocked_and_session_untouched(vec![bare]);
    }

    #[test]
    fn test_unknown_pedal_command_in_a_save_is_blocked() {
        let pedal = serde_json::json!({"time": 0.0, "priority": 1, "action": "pedal", "key_char": "enter"});
        assert_blocked_and_session_untouched(vec![pedal]);
    }
}

mod test_translate_sheet_to_notes {
    use super::*;

    #[test]
    fn test_maps_char_to_pitch_via_key_mapper() {
        let km = KeyMapper::new(false);
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = translate_sheet_to_notes(ch.to_string(), 60.0, false).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);
    }

    #[test]
    fn test_empty_sheet_produces_no_notes() {
        let notes = translate_sheet_to_notes(String::new(), 120.0, false).unwrap();
        assert!(notes.is_empty());
    }
}

mod test_notes_to_sheet {
    use super::*;

    #[test]
    fn test_round_trips_through_key_mapper_and_tempo_map() {
        let km = KeyMapper::new(false);
        let ch = km.get_key_for_pitch(60).unwrap();
        let note = Note::new(0, 60, 64, 0.0, 0.25);
        let sheet = notes_to_sheet(vec![note], false, vec![(0.0, 500_000)], vec![]).unwrap();
        assert!(sheet.starts_with(ch));
    }

    #[test]
    fn test_empty_notes_produces_empty_string() {
        let sheet = notes_to_sheet(vec![], false, vec![], vec![]).unwrap();
        assert_eq!(sheet, "");
    }
}

mod test_extract_pedal_intervals {
    use super::*;

    #[test]
    fn test_empty_returns_empty() {
        assert_eq!(extract_pedal_intervals(&[]), vec![]);
    }

    #[test]
    fn test_single_down_up_pair() {
        let events = [pedal_ev(1.0, "down"), pedal_ev(2.0, "up")];
        assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
    }

    #[test]
    fn test_multiple_pairs() {
        let events = [
            pedal_ev(0.0, "down"),
            pedal_ev(0.5, "up"),
            pedal_ev(1.0, "down"),
            pedal_ev(1.8, "up"),
        ];
        assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 0.5), (1.0, 1.8)]);
    }

    #[test]
    fn test_unpaired_down_dropped() {
        assert_eq!(extract_pedal_intervals(&[pedal_ev(1.0, "down")]), vec![]);
    }

    #[test]
    fn test_up_without_prior_down_ignored() {
        let events = [pedal_ev(1.0, "up"), pedal_ev(2.0, "down"), pedal_ev(3.0, "up")];
        assert_eq!(extract_pedal_intervals(&events), vec![(2.0, 3.0)]);
    }

    #[test]
    fn test_non_pedal_events_ignored() {
        let events = [
            press_ev(0.0),
            pedal_ev(1.0, "down"),
            press_ev(1.5),
            pedal_ev(2.0, "up"),
        ];
        assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
    }

    #[test]
    fn test_interval_values_preserved() {
        let events = [pedal_ev(0.123, "down"), pedal_ev(4.567, "up")];
        let intervals = extract_pedal_intervals(&events);
        assert!((intervals[0].0 - 0.123).abs() < 1e-9);
        assert!((intervals[0].1 - 4.567).abs() < 1e-9);
    }

    #[test]
    fn test_consecutive_downs_first_wins() {
        let events = [pedal_ev(1.0, "down"), pedal_ev(1.5, "down"), pedal_ev(2.0, "up")];
        assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
    }

    #[test]
    fn test_events_out_of_time_order_are_sorted_first() {
        let events = [
            pedal_ev(0.0, "down"),
            pedal_ev(2.0, "up"),
            pedal_ev(1.0, "down"),
            pedal_ev(3.0, "up"),
            pedal_ev(2.0, "down"),
            pedal_ev(4.0, "up"),
        ];
        assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 2.0), (2.0, 3.0)]);
    }

    #[test]
    fn test_overlapping_downs_never_produce_overlapping_intervals() {
        let events = [
            pedal_ev(0.0, "down"),
            pedal_ev(10.0, "up"),
            pedal_ev(1.0, "down"),
            pedal_ev(2.0, "up"),
        ];
        assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 2.0)]);
    }

    #[test]
    fn test_same_time_up_before_down_gives_adjacent_intervals_without_a_zero_length_one() {
        let down = |t: f64| KeyEvent::new(t, 1, "pedal", "down");
        let up = |t: f64| KeyEvent::new(t, 0, "pedal", "up");
        let events = [down(0.0), down(0.0), up(0.0), down(1.0), up(1.0), down(1.0), up(2.0)];
        assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 1.0), (1.0, 2.0)]);
    }

    #[test]
    fn test_consecutive_ups_only_first_consumed() {
        let events = [pedal_ev(1.0, "down"), pedal_ev(2.0, "up"), pedal_ev(3.0, "up")];
        assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
    }

    #[test]
    fn test_only_pedal_actions_inspected() {
        let weird = KeyEvent::new(1.0, 1, "pedal", "wiggle");
        let events = [pedal_ev(0.0, "down"), weird, pedal_ev(2.0, "up")];
        assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 2.0)]);
    }
}

mod test_apply_hand_assignment {
    use super::*;

    fn note(id: i32, pitch: i32, start_time: f64, hand: &str) -> Note {
        Note {
            id,
            pitch,
            velocity: 64,
            start_time,
            duration: 0.5,
            hand: hand.to_string(),
            original_track_index: -1,
            channel: -1,
        }
    }

    #[test]
    fn test_simulate_hands_off_defaults_by_pitch_threshold() {
        let mut notes = vec![note(0, 40, 0.0, "unknown"), note(1, 70, 1.0, "unknown")];
        let mut cfg = PlaybackConfig::default();
        cfg.simulate_hands = false;
        apply_hand_assignment(&mut notes, &cfg);
        assert_eq!(notes[0].hand, "left");
        assert_eq!(notes[1].hand, "right");
    }

    #[test]
    fn test_simulate_hands_off_never_touches_already_assigned() {
        let mut notes = vec![note(0, 90, 0.0, "left")];
        let mut cfg = PlaybackConfig::default();
        cfg.simulate_hands = false;
        apply_hand_assignment(&mut notes, &cfg);
        assert_eq!(notes[0].hand, "left");
    }

    #[test]
    fn test_simulate_hands_on_uses_group_average() {
        let mut notes = vec![
            note(0, 40, 0.0, "unknown"),
            note(1, 50, 0.0, "unknown"),
        ];
        let mut cfg = PlaybackConfig::default();
        cfg.simulate_hands = true;
        apply_hand_assignment(&mut notes, &cfg);
        assert_eq!(notes[0].hand, "left");
        assert_eq!(notes[1].hand, "left");
    }

    #[test]
    fn test_pitch_60_defaults_right_when_simulate_off() {
        let mut notes = vec![note(0, 60, 0.0, "unknown")];
        let mut cfg = PlaybackConfig::default();
        cfg.simulate_hands = false;
        apply_hand_assignment(&mut notes, &cfg);
        assert_eq!(notes[0].hand, "right");
    }
}

mod test_prepare_notes_from_bytes {
    use super::*;
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    fn simple_midi_two_tracks() -> Vec<u8> {
        let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
        let track_a = vec![
            TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel: u4::from(0),
                    message: MidiMessage::NoteOn {
                        key: u7::from(40),
                        vel: u7::from(64),
                    },
                },
            },
            TrackEvent {
                delta: u28::from(480),
                kind: TrackEventKind::Midi {
                    channel: u4::from(0),
                    message: MidiMessage::NoteOff {
                        key: u7::from(40),
                        vel: u7::from(0),
                    },
                },
            },
        ];
        let track_b = vec![
            TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Midi {
                    channel: u4::from(0),
                    message: MidiMessage::NoteOn {
                        key: u7::from(72),
                        vel: u7::from(64),
                    },
                },
            },
            TrackEvent {
                delta: u28::from(480),
                kind: TrackEventKind::Midi {
                    channel: u4::from(0),
                    message: MidiMessage::NoteOff {
                        key: u7::from(72),
                        vel: u7::from(0),
                    },
                },
            },
        ];
        let smf = Smf {
            header,
            tracks: vec![track_a, track_b],
        };
        let mut buf = Vec::new();
        smf.write(&mut buf).unwrap();
        buf
    }

    #[test]
    fn test_only_selected_tracks_included() {
        let bytes = simple_midi_two_tracks();
        let cfg = PlaybackConfig::default();
        let (notes, _, _) =
            prepare_notes_from_bytes(&bytes, &cfg, &[(0, "Right Hand".to_string())]).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 40);
    }

    #[test]
    fn test_role_assignment_sets_hand() {
        let bytes = simple_midi_two_tracks();
        let cfg = PlaybackConfig::default();
        let (notes, _, _) = prepare_notes_from_bytes(
            &bytes,
            &cfg,
            &[(0, "Left Hand".to_string()), (1, "Right Hand".to_string())],
        )
        .unwrap();
        let left: Vec<&Note> = notes.iter().filter(|n| n.pitch == 40).collect();
        let right: Vec<&Note> = notes.iter().filter(|n| n.pitch == 72).collect();
        assert_eq!(left[0].hand, "left");
        assert_eq!(right[0].hand, "right");
    }

    #[test]
    fn test_notes_sorted_by_start_time_across_tracks() {
        let bytes = simple_midi_two_tracks();
        let cfg = PlaybackConfig::default();
        let (notes, _, _) = prepare_notes_from_bytes(
            &bytes,
            &cfg,
            &[(0, "Left Hand".to_string()), (1, "Right Hand".to_string())],
        )
        .unwrap();
        for w in notes.windows(2) {
            assert!(w[0].start_time <= w[1].start_time);
        }
    }

    #[test]
    fn test_no_selected_tracks_produces_no_notes() {
        let bytes = simple_midi_two_tracks();
        let cfg = PlaybackConfig::default();
        let (notes, _, _) = prepare_notes_from_bytes(&bytes, &cfg, &[]).unwrap();
        assert!(notes.is_empty());
    }

    #[test]
    fn test_unrecognized_role_leaves_hand_for_later_assignment() {
        let bytes = simple_midi_two_tracks();
        let mut cfg = PlaybackConfig::default();
        cfg.simulate_hands = false;
        let (notes, _, _) =
            prepare_notes_from_bytes(&bytes, &cfg, &[(0, "Unassigned".to_string())]).unwrap();
        assert_eq!(notes[0].hand, "left");
    }
}

struct NoopWake;

impl std::task::Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

type BoxedFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

fn poll_once<T>(fut: &mut BoxedFuture<T>) -> std::task::Poll<T> {
    let waker = std::task::Waker::from(Arc::new(NoopWake));
    let mut cx = std::task::Context::from_waker(&waker);
    std::future::Future::poll(fut.as_mut(), &mut cx)
}

fn hermetic_state(dir: &Path) -> AppState {
    use crate::managers::config_manager::ConfigManager;
    use crate::managers::hotkey_manager::HotkeyManager;
    use crate::managers::theme_manager::ThemeManager;
    use std::sync::Mutex;

    AppState {
        pedal_model: Mutex::new(None),
        parsed_tracks: Mutex::new(None),
        parsed_tempo_map: Mutex::new(None),
        loaded_pedal_count: Mutex::new(0),
        midi_pedal_events: Mutex::new(Vec::new()),
        playback_session: Mutex::new(PlaybackSession::default()),
        playback_handle: Mutex::new(None),
        hotkey_manager: Mutex::new(HotkeyManager::new()),
        injected_echoes: std::sync::Arc::new(crate::managers::injected_echo::InjectedEchoes::new()),
        hook_seen: crate::managers::hook_seen::HookSeen::new(),
        config_manager: Mutex::new(ConfigManager {
            save_dir: dir.join("saves"),
            midi_dir: std::path::PathBuf::new(),
            config_dir: dir.to_path_buf(),
            config_path: dir.join("config.json"),
        }),
        app_config: Mutex::new(serde_json::json!({})),
        theme_manager: Mutex::new(ThemeManager::new()),
        uinput_sink: Mutex::new(None),
    }
}

mod test_clear_loaded_song {
    use super::*;

    fn populated_state(dir: &Path) -> AppState {
        let state = hermetic_state(dir);
        {
            let mut session = state.playback_session.lock().unwrap();
            session.final_notes = Some(Vec::new());
            session.note_events = Some(Vec::new());
            session.pedal_events = Some(Vec::new());
            session.merged_events = Some(Vec::new());
            session.total_dur = 12.5;
            session.midi_pedal_events.push((1.0, true));
        }
        *state.parsed_tracks.lock().unwrap() = Some(Vec::new());
        *state.loaded_pedal_count.lock().unwrap() = 3;
        state.midi_pedal_events.lock().unwrap().push((1.0, true));
        state
    }

    #[test]
    fn test_resets_the_session_to_its_default() {
        let dir = tempfile::tempdir().unwrap();
        let state = populated_state(dir.path());
        clear_loaded_song_logic(&state).unwrap();
        let session = state.playback_session.lock().unwrap();
        assert!(session.final_notes.is_none());
        assert!(session.note_events.is_none());
        assert!(session.pedal_events.is_none());
        assert!(session.merged_events.is_none());
        assert_eq!(session.total_dur, 0.0);
        assert!(session.midi_pedal_events.is_empty());
    }

    #[test]
    fn test_clears_the_parsed_file_and_midi_pedal_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = populated_state(dir.path());
        clear_loaded_song_logic(&state).unwrap();
        assert!(state.parsed_tracks.lock().unwrap().is_none());
        assert!(state.parsed_tempo_map.lock().unwrap().is_none());
        assert_eq!(*state.loaded_pedal_count.lock().unwrap(), 0);
        assert!(state.midi_pedal_events.lock().unwrap().is_empty());
    }

    #[test]
    fn test_clearing_an_already_empty_state_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let state = hermetic_state(dir.path());
        clear_loaded_song_logic(&state).unwrap();
        clear_loaded_song_logic(&state).unwrap();
    }
}

mod test_run_blocking {
    use super::*;

    #[test]
    fn test_returns_the_ok_value() {
        let result = tauri::async_runtime::block_on(run_blocking(|| Ok(41 + 1)));
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_returns_the_err_string_unchanged() {
        let result: Result<(), String> =
            tauri::async_runtime::block_on(run_blocking(|| Err("boom".to_string())));
        assert_eq!(result, Err("boom".to_string()));
    }

    #[test]
    fn test_closure_runs_on_a_different_thread_than_the_caller() {
        let caller = std::thread::current().id();
        let worker = tauri::async_runtime::block_on(run_blocking(|| {
            Ok(std::thread::current().id())
        }))
        .unwrap();
        assert_ne!(caller, worker);
    }

    #[test]
    fn test_polling_does_not_run_the_closure_inline() {
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let (started_tx, started_rx) = mpsc::channel::<()>();
        let mut fut: BoxedFuture<Result<u8, String>> = Box::pin(run_blocking(move || {
            started_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(5));
            Ok(7)
        }));

        assert!(poll_once(&mut fut).is_pending());
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(poll_once(&mut fut).is_pending());

        release_tx.send(()).unwrap();
        assert_eq!(tauri::async_runtime::block_on(fut), Ok(7));
    }

    #[test]
    fn test_a_panicking_closure_becomes_an_err_instead_of_unwinding_into_the_caller() {
        let result: Result<(), String> = tauri::async_runtime::block_on(run_blocking(
            || -> Result<(), String> { panic!("worker exploded") },
        ));
        let message = result.unwrap_err();
        assert!(message.starts_with("Background task failed"), "{message}");
    }

    #[test]
    fn test_the_pool_keeps_working_after_a_panicking_closure() {
        let _: Result<(), String> = tauri::async_runtime::block_on(run_blocking(
            || -> Result<(), String> { panic!("worker exploded") },
        ));
        let result = tauri::async_runtime::block_on(run_blocking(|| Ok("still alive")));
        assert_eq!(result, Ok("still alive"));
    }

    #[test]
    fn test_many_closures_run_concurrently_rather_than_one_after_another() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let futures: Vec<_> = (0..4)
            .map(|i| {
                let running = Arc::clone(&running);
                let peak = Arc::clone(&peak);
                run_blocking(move || {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(300));
                    running.fetch_sub(1, Ordering::SeqCst);
                    Ok(i)
                })
            })
            .collect();
        let results = tauri::async_runtime::block_on(async move {
            let handles: Vec<_> = futures
                .into_iter()
                .map(tauri::async_runtime::spawn)
                .collect();
            let mut out = Vec::new();
            for h in handles {
                out.push(h.await.unwrap().unwrap());
            }
            out
        });
        assert_eq!(results, vec![0, 1, 2, 3]);
        assert!(peak.load(Ordering::SeqCst) >= 2);
    }
}

mod test_pedal_model_sharing {
    use super::*;

    fn model_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("pedal_bilstm.safetensors")
    }

    fn real_model() -> Arc<PedalModel> {
        Arc::new(PedalModel::load(model_path()).unwrap())
    }

    fn config_with_style(style: &str) -> PlaybackConfig {
        let mut config = PlaybackConfig::default();
        config.pedal_style = style.to_string();
        config
    }

    fn forbidden_resolver() -> Result<std::path::PathBuf, String> {
        panic!("the resource path must not be resolved here")
    }

    fn state_in(dir: &tempfile::TempDir) -> AppState {
        hermetic_state(dir.path())
    }

    #[test]
    fn test_styles_that_do_not_use_the_model_never_resolve_a_path_or_load_it() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        for style in ["none", "harmonic", "legato"] {
            let model =
                pedal_model_for(&state, &config_with_style(style), forbidden_resolver).unwrap();
            assert!(model.is_none(), "{style}");
        }
        assert!(state.pedal_model.lock().unwrap().is_none());
    }

    #[test]
    fn test_ai_and_hybrid_styles_return_the_cached_model_without_reloading() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        let stored = real_model();
        *state.pedal_model.lock().unwrap() = Some(Arc::clone(&stored));
        for style in ["ai", "hybrid"] {
            let got = pedal_model_for(&state, &config_with_style(style), forbidden_resolver)
                .unwrap()
                .unwrap();
            assert!(Arc::ptr_eq(&got, &stored), "{style}");
        }
    }

    #[test]
    fn test_the_first_call_loads_the_model_once_and_every_later_call_shares_it() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        let first = pedal_model_for(&state, &config_with_style("ai"), || Ok(model_path()))
            .unwrap()
            .unwrap();
        let second = pedal_model_for(&state, &config_with_style("hybrid"), forbidden_resolver)
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        let stored = state.pedal_model.lock().unwrap().clone().unwrap();
        assert!(Arc::ptr_eq(&first, &stored));
    }

    #[test]
    fn test_the_mutex_is_free_while_a_caller_keeps_using_the_returned_model() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        *state.pedal_model.lock().unwrap() = Some(real_model());
        let in_use = pedal_model_for(&state, &config_with_style("ai"), forbidden_resolver)
            .unwrap()
            .unwrap();
        assert!(
            state.pedal_model.try_lock().is_ok(),
            "a caller using the model must not keep the AppState mutex locked"
        );
        drop(in_use);
    }

    #[test]
    fn test_the_model_stays_usable_from_another_thread_while_the_mutex_is_held_elsewhere() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        *state.pedal_model.lock().unwrap() = Some(real_model());
        let in_use = pedal_model_for(&state, &config_with_style("ai"), forbidden_resolver)
            .unwrap()
            .unwrap();

        std::thread::scope(|scope| {
            let (locked_tx, locked_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel::<()>();
            let state_ref = &state;
            let holder = scope.spawn(move || {
                let _guard = state_ref.pedal_model.lock().unwrap();
                locked_tx.send(()).unwrap();
                let _ = release_rx.recv_timeout(Duration::from_secs(5));
            });
            locked_rx.recv_timeout(Duration::from_secs(5)).unwrap();

            let preds = in_use.forward(&vec![0.0f32; 10 * FEATURES], 10).unwrap();
            assert_eq!(preds.len(), 10);

            release_tx.send(()).unwrap();
            holder.join().unwrap();
        });
    }

    #[test]
    fn test_a_failing_path_resolver_is_an_error_and_leaves_the_cache_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        let result = pedal_model_for(&state, &config_with_style("ai"), || {
            Err("no such resource".to_string())
        });
        assert_eq!(result.err(), Some("no such resource".to_string()));
        assert!(state.pedal_model.lock().unwrap().is_none());
    }

    #[test]
    fn test_an_unreadable_model_file_is_an_error_and_leaves_the_cache_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_in(&dir);
        let junk = dir.path().join("junk.safetensors");
        std::fs::write(&junk, b"not a safetensors file").unwrap();
        let result = pedal_model_for(&state, &config_with_style("hybrid"), || Ok(junk));
        assert!(result.is_err());
        assert!(state.pedal_model.lock().unwrap().is_none());
    }
}
