use super::*;

fn note(overrides: impl FnOnce(&mut Note)) -> Note {
    let mut n = Note::new(0, 60, 64, 0.0, 0.5);
    overrides(&mut n);
    n
}

mod test_note {
    use super::*;

    #[test]
    fn test_end_time() {
        let n = note(|n| {
            n.start_time = 1.0;
            n.duration = 0.5;
        });
        assert!((n.end_time() - 1.5).abs() < 1e-9);
    }

    #[test]
    fn test_end_time_zero_duration() {
        let n = note(|n| {
            n.start_time = 2.0;
            n.duration = 0.0;
        });
        assert!((n.end_time() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_defaults() {
        let n = note(|_| {});
        assert_eq!(n.hand, "unknown");
        assert_eq!(n.original_track_index, -1);
        assert_eq!(n.channel, -1);
    }
}

mod test_midi_track {
    use super::*;

    fn track(notes: Vec<Note>, program_change: i32, is_drum: bool) -> MidiTrack {
        MidiTrack {
            index: 0,
            name: String::new(),
            program_change,
            is_drum,
            notes,
        }
    }

    #[test]
    fn test_note_count_empty() {
        assert_eq!(track(vec![], 0, false).note_count(), 0);
    }

    #[test]
    fn test_note_count() {
        let notes: Vec<Note> = (0..5).map(|i| note(|n| n.id = i)).collect();
        assert_eq!(track(notes, 0, false).note_count(), 5);
    }

    #[test]
    fn test_instrument_name_piano() {
        assert_eq!(track(vec![], 0, false).instrument_name(), "Piano");
        assert_eq!(track(vec![], 7, false).instrument_name(), "Piano");
    }

    #[test]
    fn test_instrument_name_drums() {
        assert_eq!(track(vec![], 0, true).instrument_name(), "Drums/Percussion");
    }

    #[test]
    fn test_instrument_name_drums_overrides_program_change() {
        assert_eq!(track(vec![], 0, true).instrument_name(), "Drums/Percussion");
    }

    #[test]
    fn test_instrument_name_chromatic_perc() {
        assert_eq!(track(vec![], 8, false).instrument_name(), "Chromatic Perc");
        assert_eq!(track(vec![], 15, false).instrument_name(), "Chromatic Perc");
    }

    #[test]
    fn test_instrument_name_organ() {
        assert_eq!(track(vec![], 16, false).instrument_name(), "Organ");
        assert_eq!(track(vec![], 23, false).instrument_name(), "Organ");
    }

    #[test]
    fn test_instrument_name_guitar() {
        assert_eq!(track(vec![], 24, false).instrument_name(), "Guitar");
        assert_eq!(track(vec![], 31, false).instrument_name(), "Guitar");
    }

    #[test]
    fn test_instrument_name_bass() {
        assert_eq!(track(vec![], 32, false).instrument_name(), "Bass");
        assert_eq!(track(vec![], 39, false).instrument_name(), "Bass");
    }

    #[test]
    fn test_instrument_name_strings() {
        assert_eq!(track(vec![], 40, false).instrument_name(), "Strings");
        assert_eq!(track(vec![], 47, false).instrument_name(), "Strings");
    }

    #[test]
    fn test_instrument_name_ensemble() {
        assert_eq!(track(vec![], 48, false).instrument_name(), "Ensemble");
        assert_eq!(track(vec![], 55, false).instrument_name(), "Ensemble");
    }

    #[test]
    fn test_instrument_name_fallthrough() {
        assert_eq!(track(vec![], 56, false).instrument_name(), "Instrument 56");
        assert_eq!(track(vec![], 200, false).instrument_name(), "Instrument 200");
    }
}

mod test_key_event {
    use super::*;

    #[test]
    fn test_ordering_by_time() {
        let e1 = KeyEvent::new(1.0, 0, "press", "a");
        let e2 = KeyEvent::new(2.0, 0, "press", "a");
        assert!(e1 < e2);
    }

    #[test]
    fn test_ordering_by_priority_at_same_time() {
        let e1 = KeyEvent::new(1.0, 1, "press", "a");
        let e2 = KeyEvent::new(1.0, 2, "press", "a");
        assert!(e1 < e2);
    }

    #[test]
    fn test_non_compare_fields_ignored_in_equality() {
        let mut e1 = KeyEvent::new(1.0, 1, "press", "a");
        e1.pitch = Some(60);
        let mut e2 = KeyEvent::new(1.0, 1, "pedal", "down");
        e2.pitch = None;
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_pitch_defaults_to_none() {
        let e = KeyEvent::new(0.0, 0, "pedal", "down");
        assert_eq!(e.pitch, None);
    }

    #[test]
    fn test_sort_stability_across_types() {
        let mut events = vec![
            KeyEvent::new(2.0, 0, "release", "a"),
            KeyEvent::new(1.0, 1, "press", "b"),
            KeyEvent::new(1.0, 0, "pedal", "down"),
        ];
        events.sort();
        assert!((events[0].time - 1.0).abs() < 1e-9 && events[0].priority == 0);
        assert!((events[1].time - 1.0).abs() < 1e-9 && events[1].priority == 1);
        assert!((events[2].time - 2.0).abs() < 1e-9);
    }
}

mod test_key_state {
    use super::*;

    #[test]
    fn test_initial_state() {
        let ks = KeyState::new("a");
        assert!(!ks.is_active);
        assert!(!ks.is_sustained);
        assert!(!ks.is_physically_down());
    }

    #[test]
    fn test_press_activates() {
        let mut ks = KeyState::new("a");
        ks.press();
        assert!(ks.is_active);
        assert!(ks.is_physically_down());
    }

    #[test]
    fn test_release_deactivates() {
        let mut ks = KeyState::new("a");
        ks.press();
        ks.release();
        assert!(!ks.is_active);
        assert!(!ks.is_sustained);
        assert!(!ks.is_physically_down());
    }
}

mod test_musical_section {
    use super::*;

    #[test]
    fn test_fields_and_defaults() {
        let notes = vec![note(|_| {})];
        let mut sec = MusicalSection::new(0.0, 1.0, notes.clone());
        sec.articulation_label = "legato".to_string();
        sec.pace_label = "fast".to_string();
        assert_eq!(sec.start_beat, 0.0);
        assert_eq!(sec.end_beat, 0.0);
        assert_eq!(sec.articulation_label, "legato");
        assert_eq!(sec.pace_label, "fast");
        assert_eq!(sec.notes, notes);
    }

    #[test]
    fn test_default_labels() {
        let sec = MusicalSection::new(0.0, 1.0, vec![]);
        assert_eq!(sec.articulation_label, "unknown");
        assert_eq!(sec.pace_label, "normal");
    }
}
