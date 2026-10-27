use super::*;

mod test_parse_chunk {
    use super::*;

    #[test]
    fn test_single_char_no_dash() {
        assert_eq!(VirtualPianoFormat::parse_chunk("y"), vec![(vec!['y'], 0)]);
    }

    #[test]
    fn test_single_char_with_dashes() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("y---"),
            vec![(vec!['y'], 3)]
        );
    }

    #[test]
    fn test_chord_brackets() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("[abc]"),
            vec![(vec!['a', 'b', 'c'], 0)]
        );
    }

    #[test]
    fn test_chord_with_dashes() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("[abc]--"),
            vec![(vec!['a', 'b', 'c'], 2)]
        );
    }

    #[test]
    fn test_adjacent_note_after_dashes_no_space_needed() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("y-t"),
            vec![(vec!['y'], 1), (vec!['t'], 0)]
        );
    }

    #[test]
    fn test_unterminated_bracket_falls_back_to_literal_chars() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("[abc"),
            vec![(vec!['a'], 0), (vec!['b'], 0), (vec!['c'], 0)]
        );
    }

    #[test]
    fn test_empty_chunk() {
        assert_eq!(VirtualPianoFormat::parse_chunk(""), vec![]);
    }

    #[test]
    fn test_only_dashes() {
        assert_eq!(VirtualPianoFormat::parse_chunk("---"), vec![]);
    }

    #[test]
    fn test_empty_brackets_produce_no_token() {
        assert_eq!(VirtualPianoFormat::parse_chunk("[]"), vec![]);
    }

    #[test]
    fn test_multiple_tokens_in_one_chunk() {
        assert_eq!(
            VirtualPianoFormat::parse_chunk("a-[bc]--d"),
            vec![(vec!['a'], 1), (vec!['b', 'c'], 2), (vec!['d'], 0)]
        );
    }
}

mod test_parse {
    use super::*;

    fn mapper() -> KeyMapper {
        KeyMapper::new(false)
    }

    #[test]
    fn test_single_note_pitch_and_timing() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&ch.to_string(), 60.0, &km);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[0].start_time, 0.0);
        assert_eq!(notes[0].duration, 0.25);
        assert_eq!(notes[0].velocity, 64);
        assert_eq!(notes[0].hand, "unknown");
    }

    #[test]
    fn test_dash_extends_duration_by_power_of_two() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("{ch}--"), 60.0, &km);
        assert_eq!(notes[0].duration, 1.0);
    }

    #[test]
    fn test_chord_notes_share_start_time() {
        let km = mapper();
        let p1 = km.get_key_for_pitch(60).unwrap();
        let p2 = km.get_key_for_pitch(62).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("[{p1}{p2}]"), 60.0, &km);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].start_time, notes[1].start_time);
    }

    #[test]
    fn test_sequential_tokens_advance_time() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("{ch} {ch}"), 60.0, &km);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].start_time, 0.0);
        assert_eq!(notes[1].start_time, 0.25);
    }

    #[test]
    fn test_unmapped_char_is_skipped_but_still_advances_time() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("~ {ch}"), 60.0, &km);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].start_time, 0.25);
    }

    #[test]
    fn test_multi_line_input() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("{ch}\n{ch}"), 60.0, &km);
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn test_blank_lines_ignored() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("{ch}\n\n\n{ch}"), 60.0, &km);
        assert_eq!(notes.len(), 2);
    }

    #[test]
    fn test_empty_text_produces_no_notes() {
        let km = mapper();
        assert!(VirtualPianoFormat::parse("", 60.0, &km).is_empty());
    }

    #[test]
    fn test_whitespace_only_text_produces_no_notes() {
        let km = mapper();
        assert!(VirtualPianoFormat::parse("   \n  \n", 60.0, &km).is_empty());
    }

    #[test]
    fn test_note_ids_are_sequential() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let notes = VirtualPianoFormat::parse(&format!("{ch} {ch} {ch}"), 60.0, &km);
        assert_eq!(
            notes.iter().map(|n| n.id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn test_higher_bpm_shortens_duration() {
        let km = mapper();
        let ch = km.get_key_for_pitch(60).unwrap();
        let slow = VirtualPianoFormat::parse(&ch.to_string(), 60.0, &km);
        let fast = VirtualPianoFormat::parse(&ch.to_string(), 120.0, &km);
        assert!(fast[0].duration < slow[0].duration);
    }
}

mod test_serialize {
    use super::*;

    fn mapper() -> KeyMapper {
        KeyMapper::new(false)
    }

    fn tempo_map_120() -> TempoMap {
        TempoMap::new(vec![(0.0, 500_000)], vec![])
    }

    #[test]
    fn test_empty_notes_returns_empty_string() {
        let km = mapper();
        let tm = tempo_map_120();
        assert_eq!(VirtualPianoFormat::serialize(&[], &km, &tm), "");
    }

    #[test]
    fn test_single_note_round_trips_to_its_char() {
        let km = mapper();
        let tm = tempo_map_120();
        let ch = km.get_key_for_pitch(60).unwrap();
        let note = Note::new(0, 60, 64, 0.0, 0.25);
        let sheet = VirtualPianoFormat::serialize(&[note], &km, &tm);
        assert!(sheet.starts_with(ch));
    }

    #[test]
    fn test_all_unmappable_notes_produce_fallback_message() {
        let km = mapper();
        let tm = tempo_map_120();
        let note = Note::new(0, 200, 64, 0.0, 0.25);
        let sheet = VirtualPianoFormat::serialize(&[note], &km, &tm);
        assert!(sheet.starts_with("# No mappable notes"));
    }

    #[test]
    fn test_same_quantized_time_groups_into_chord() {
        let km = mapper();
        let tm = tempo_map_120();
        let n1 = Note::new(0, 60, 64, 0.0, 0.25);
        let n2 = Note::new(1, 62, 64, 0.0, 0.25);
        let sheet = VirtualPianoFormat::serialize(&[n1, n2], &km, &tm);
        assert!(sheet.starts_with('['));
        assert!(sheet.contains(']'));
    }

    #[test]
    fn test_long_duration_clamps_dash_count_to_three() {
        let km = mapper();
        let tm = tempo_map_120();
        let note = Note::new(0, 60, 64, 0.0, 100.0);
        let sheet = VirtualPianoFormat::serialize(&[note], &km, &tm);
        let dash_count = sheet.chars().filter(|&c| c == '-').count();
        assert_eq!(dash_count, 3);
    }

    #[test]
    fn test_bpm_estimated_from_tempo_map_first_event() {
        let km = mapper();
        let tm_fast = TempoMap::new(vec![(0.0, 250_000)], vec![]);
        let ch = km.get_key_for_pitch(60).unwrap();
        // 250_000 us/beat = 240 BPM -> base_16th = 60/(240*4) = 0.0625
        let n1 = Note::new(0, 60, 64, 0.0, 0.0625);
        let n2 = Note::new(1, 60, 64, 0.0625, 0.0625);
        let sheet = VirtualPianoFormat::serialize(&[n1, n2], &km, &tm_fast);
        assert_eq!(sheet, format!("{ch} {ch}"));
    }

    #[test]
    fn test_defaults_to_120_bpm_when_tempo_map_has_no_events() {
        let km = mapper();
        let tm_empty = TempoMap::new(vec![], vec![]);
        let ch = km.get_key_for_pitch(60).unwrap();
        // default 120 BPM -> base_16th = 60/(120*4) = 0.125
        let note = Note::new(0, 60, 64, 0.0, 0.125);
        let sheet = VirtualPianoFormat::serialize(&[note], &km, &tm_empty);
        assert_eq!(sheet, ch.to_string());
    }

    #[test]
    fn test_long_line_wraps_at_eighty_chars() {
        let km = mapper();
        let tm = tempo_map_120();
        let mut notes = Vec::new();
        let ch = km.get_key_for_pitch(60).unwrap();
        for i in 0..60 {
            notes.push(Note::new(i, 60, 64, i as f64 * 0.25, 0.25));
        }
        let sheet = VirtualPianoFormat::serialize(&notes, &km, &tm);
        assert!(sheet.contains('\n'));
        for line in sheet.lines() {
            assert!(line.len() <= 80);
        }
        let _ = ch;
    }
}

mod test_format_registry {
    use super::*;

    #[test]
    fn test_names_contains_virtual_piano() {
        assert_eq!(FormatRegistry::names(), vec!["Virtual Piano"]);
    }

    #[test]
    fn test_get_known_format() {
        assert_eq!(FormatRegistry::get("Virtual Piano"), Some("Virtual Piano"));
    }

    #[test]
    fn test_get_unknown_format_returns_none() {
        assert_eq!(FormatRegistry::get("Nonexistent"), None);
    }
}

mod test_round_trip {
    use super::*;

    #[test]
    fn test_parse_then_serialize_recovers_the_same_chars() {
        let km = KeyMapper::new(false);
        let tm = tempo_map_120();
        let ch = km.get_key_for_pitch(60).unwrap();
        let text = format!("{ch} {ch}--");
        let notes = VirtualPianoFormat::parse(&text, 120.0, &km);
        let sheet = VirtualPianoFormat::serialize(&notes, &km, &tm);
        assert!(sheet.contains(ch));
    }

    fn tempo_map_120() -> TempoMap {
        TempoMap::new(vec![(0.0, 500_000)], vec![])
    }
}
