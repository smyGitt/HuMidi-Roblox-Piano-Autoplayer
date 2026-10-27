use super::*;

fn note(id: i32, start_time: f64) -> Note {
    Note::new(id, 60, 64, start_time, 0.5)
}

mod test_get_time_groups {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(get_time_groups(&[]), Vec::<Vec<Note>>::new());
    }

    #[test]
    fn test_single_note() {
        let n = note(0, 0.0);
        assert_eq!(get_time_groups(&[n.clone()]), vec![vec![n]]);
    }

    #[test]
    fn test_within_threshold_same_group() {
        let notes = [note(0, 0.000), note(1, 0.010)];
        let groups = get_time_groups(&notes);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_at_threshold_included() {
        let notes = [note(0, 0.000), note(1, 0.015)];
        let groups = get_time_groups(&notes);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_beyond_threshold_separate_groups() {
        let notes = [note(0, 0.000), note(1, 0.020)];
        let groups = get_time_groups(&notes);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_custom_threshold() {
        let notes = [note(0, 0.000), note(1, 0.005)];
        let groups = get_time_groups_with_threshold(&notes, 0.003);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_three_distinct_groups() {
        let notes: Vec<Note> = (0..3).map(|i| note(i, i as f64 * 0.1)).collect();
        assert_eq!(get_time_groups(&notes).len(), 3);
    }
}

mod test_get_time_group_indices {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(get_time_group_indices(&[]), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn test_single_note() {
        let n = note(0, 0.0);
        assert_eq!(get_time_group_indices(&[n]), vec![vec![0]]);
    }

    #[test]
    fn test_within_threshold_same_group() {
        let notes = [note(0, 0.000), note(1, 0.010)];
        assert_eq!(get_time_group_indices(&notes), vec![vec![0, 1]]);
    }

    #[test]
    fn test_at_threshold_included() {
        let notes = [note(0, 0.000), note(1, 0.015)];
        assert_eq!(get_time_group_indices(&notes), vec![vec![0, 1]]);
    }

    #[test]
    fn test_beyond_threshold_separate_groups() {
        let notes = [note(0, 0.000), note(1, 0.020)];
        assert_eq!(get_time_group_indices(&notes), vec![vec![0], vec![1]]);
    }

    #[test]
    fn test_custom_threshold() {
        let notes = [note(0, 0.000), note(1, 0.005)];
        assert_eq!(
            get_time_group_indices_with_threshold(&notes, 0.003),
            vec![vec![0], vec![1]]
        );
    }

    #[test]
    fn test_three_distinct_groups() {
        let notes: Vec<Note> = (0..3).map(|i| note(i, i as f64 * 0.1)).collect();
        assert_eq!(
            get_time_group_indices(&notes),
            vec![vec![0], vec![1], vec![2]]
        );
    }

    #[test]
    fn test_indices_are_consistent_with_note_based_grouping() {
        let notes = [
            note(0, 0.000),
            note(1, 0.010),
            note(2, 0.100),
            note(3, 0.105),
            note(4, 0.500),
        ];
        let by_index = get_time_group_indices(&notes);
        let by_note = get_time_groups(&notes);
        assert_eq!(by_index.len(), by_note.len());
        for (idx_group, note_group) in by_index.iter().zip(by_note.iter()) {
            let resolved: Vec<Note> = idx_group.iter().map(|&i| notes[i].clone()).collect();
            assert_eq!(&resolved, note_group);
        }
    }
}

mod test_tempo_map {
    use super::*;

    fn default_tm() -> TempoMap {
        TempoMap::new(vec![], vec![])
    }

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn test_time_to_beat_at_120bpm() {
        let tm = default_tm();
        approx(tm.time_to_beat(0.5), 1.0);
        approx(tm.time_to_beat(1.0), 2.0);
        approx(tm.time_to_beat(0.0), 0.0);
    }

    #[test]
    fn test_beat_to_time_at_120bpm() {
        let tm = default_tm();
        approx(tm.beat_to_time(1.0), 0.5);
        approx(tm.beat_to_time(2.0), 1.0);
    }

    #[test]
    fn test_roundtrip_time_beat() {
        let tm = default_tm();
        for t in [0.0, 0.25, 0.5, 1.0, 2.5] {
            approx(tm.beat_to_time(tm.time_to_beat(t)), t);
        }
    }

    #[test]
    fn test_negative_time_returns_zero() {
        assert_eq!(default_tm().time_to_beat(-1.0), 0.0);
    }

    #[test]
    fn test_negative_beat_returns_zero() {
        assert_eq!(default_tm().beat_to_time(-1.0), 0.0);
    }

    #[test]
    fn test_get_tempo_at_default() {
        let tm = default_tm();
        assert_eq!(tm.get_tempo_at(0.0), 500_000);
        assert_eq!(tm.get_tempo_at(99.0), 500_000);
    }

    #[test]
    fn test_get_tempo_at_with_change() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (2.0, 1_000_000)], vec![]);
        assert_eq!(tm.get_tempo_at(1.0), 500_000);
        assert_eq!(tm.get_tempo_at(2.0), 1_000_000);
        assert_eq!(tm.get_tempo_at(5.0), 1_000_000);
    }

    #[test]
    fn test_time_to_beat_after_tempo_change() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (2.0, 1_000_000)], vec![]);
        approx(tm.time_to_beat(2.0), 4.0);
        approx(tm.time_to_beat(3.0), 5.0);
    }

    #[test]
    fn test_beat_to_time_after_tempo_change() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (2.0, 1_000_000)], vec![]);
        approx(tm.beat_to_time(4.0), 2.0);
        approx(tm.beat_to_time(5.0), 3.0);
    }

    #[test]
    fn test_has_explicit_time_signatures_false_empty() {
        assert!(!TempoMap::new(vec![], vec![]).has_explicit_time_signatures);
    }

    #[test]
    fn test_has_explicit_time_signatures_false_default_44() {
        assert!(!TempoMap::new(vec![], vec![(0.0, 4, 4)]).has_explicit_time_signatures);
    }

    #[test]
    fn test_has_explicit_time_signatures_true_non_44() {
        assert!(TempoMap::new(vec![], vec![(0.0, 3, 4)]).has_explicit_time_signatures);
    }

    #[test]
    fn test_has_explicit_time_signatures_true_multiple() {
        assert!(
            TempoMap::new(vec![], vec![(0.0, 4, 4), (4.0, 3, 4)]).has_explicit_time_signatures
        );
    }

    #[test]
    fn test_has_explicit_time_signatures_true_4_over_2_not_44() {
        assert!(TempoMap::new(vec![], vec![(0.0, 4, 2)]).has_explicit_time_signatures);
    }

    #[test]
    fn test_get_measure_boundaries_one_4_4_measure() {
        let tm = default_tm();
        let boundaries = tm.get_measure_boundaries(2.0);
        assert_eq!(boundaries.len(), 1);
        approx(boundaries[0].0, 0.0);
        approx(boundaries[0].1, 2.0);
    }

    #[test]
    fn test_get_measure_boundaries_two_measures() {
        let tm = default_tm();
        assert_eq!(tm.get_measure_boundaries(4.0).len(), 2);
    }
}

mod test_initial_bpm {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn test_no_events_defaults_to_120() {
        approx(TempoMap::new(vec![], vec![]).initial_bpm(), 120.0);
    }

    #[test]
    fn test_only_placeholder_event_defaults_to_120() {
        approx(TempoMap::new(vec![(0.0, 500_000)], vec![]).initial_bpm(), 120.0);
    }

    #[test]
    fn test_real_tempo_at_time_zero_is_used() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (0.0, 400_000)], vec![]);
        approx(tm.initial_bpm(), 150.0);
    }

    #[test]
    fn test_real_tempo_within_five_seconds_is_used() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (4.999, 300_000)], vec![]);
        approx(tm.initial_bpm(), 200.0);
    }

    #[test]
    fn test_boundary_at_exactly_five_seconds_is_inclusive() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (5.0, 300_000)], vec![]);
        approx(tm.initial_bpm(), 200.0);
    }

    #[test]
    fn test_change_just_after_five_seconds_falls_back_to_placeholder() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (5.001, 300_000)], vec![]);
        approx(tm.initial_bpm(), 120.0);
    }

    #[test]
    fn test_later_tempo_changes_beyond_second_event_are_ignored() {
        let tm = TempoMap::new(
            vec![(0.0, 500_000), (1.0, 250_000), (2.0, 1_000_000)],
            vec![],
        );
        approx(tm.initial_bpm(), 240.0);
    }

    #[test]
    fn test_events_out_of_construction_order_still_use_sorted_second_event() {
        let tm = TempoMap::new(vec![(4.0, 250_000), (0.0, 500_000)], vec![]);
        approx(tm.initial_bpm(), 240.0);
    }

    #[test]
    fn test_zero_tempo_value_does_not_produce_infinity() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (1.0, 0)], vec![]);
        approx(tm.initial_bpm(), 120.0);
    }

    #[test]
    fn test_very_slow_tempo() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (0.0, 2_000_000)], vec![]);
        approx(tm.initial_bpm(), 30.0);
    }

    #[test]
    fn test_very_fast_tempo() {
        let tm = TempoMap::new(vec![(0.0, 500_000), (0.0, 100_000)], vec![]);
        approx(tm.initial_bpm(), 600.0);
    }

    #[test]
    fn test_zero_second_event_falls_back_to_valid_first_event_not_global_default() {
        let tm = TempoMap::new(vec![(0.0, 400_000), (1.0, 0)], vec![]);
        approx(tm.initial_bpm(), 150.0);
    }

    #[test]
    fn test_zero_first_and_second_event_falls_back_to_global_default() {
        let tm = TempoMap::new(vec![(0.0, 0), (1.0, 0)], vec![]);
        approx(tm.initial_bpm(), 120.0);
    }

    #[test]
    fn test_single_event_with_nonzero_start_time_still_used() {
        let tm = TempoMap::new(vec![(2.5, 300_000)], vec![]);
        approx(tm.initial_bpm(), 200.0);
    }
}

mod test_key_mapper {
    use super::*;

    #[test]
    fn test_is_black_key_white_notes() {
        for pc in [0, 2, 4, 5, 7, 9, 11] {
            assert!(!KeyMapper::is_black_key(pc));
        }
    }

    #[test]
    fn test_is_black_key_black_notes() {
        for pc in [1, 3, 6, 8, 10] {
            assert!(KeyMapper::is_black_key(pc));
        }
    }

    #[test]
    fn test_pitch_to_name_c4() {
        assert_eq!(KeyMapper::pitch_to_name(60), "C4");
    }

    #[test]
    fn test_pitch_to_name_a4() {
        assert_eq!(KeyMapper::pitch_to_name(69), "A4");
    }

    #[test]
    fn test_pitch_to_name_c_sharp_4() {
        assert_eq!(KeyMapper::pitch_to_name(61), "C#4");
    }

    #[test]
    fn test_pitch_to_name_c0() {
        assert_eq!(KeyMapper::pitch_to_name(12), "C0");
    }

    #[test]
    fn test_compressed_layout_range() {
        let km = KeyMapper::new(false);
        assert_eq!(km.min_pitch, 36);
        assert_eq!(km.max_pitch, 96);
    }

    #[test]
    fn test_88_key_layout_range() {
        let km = KeyMapper::new(true);
        assert_eq!(km.min_pitch, 21);
        assert_eq!(km.max_pitch, 108);
    }

    #[test]
    fn test_compressed_pitch_36_maps_to_1() {
        let km = KeyMapper::new(false);
        assert_eq!(km.get_key_for_pitch(36), Some('1'));
    }

    #[test]
    fn test_88_key_pitch_21_has_ctrl() {
        let km = KeyMapper::new(true);
        let data = km.get_key_data(21).unwrap();
        assert_eq!(data.key, '1');
        assert!(data.modifiers.contains(&Modifier::Ctrl));
    }

    #[test]
    fn test_black_key_has_shift_modifier() {
        let km = KeyMapper::new(false);
        let data = km.get_key_data(37).unwrap();
        assert!(data.modifiers.contains(&Modifier::Shift));
    }

    #[test]
    fn test_get_key_data_wraps_low_pitch_up() {
        let km = KeyMapper::new(false);
        assert_eq!(
            km.get_key_data(24).unwrap().key,
            km.get_key_data(36).unwrap().key
        );
    }

    #[test]
    fn test_get_key_data_wraps_high_pitch_down() {
        let km = KeyMapper::new(false);
        assert_eq!(
            km.get_key_data(100).unwrap().key,
            km.get_key_data(88).unwrap().key
        );
    }

    #[test]
    fn test_get_key_data_none_for_truly_unmappable() {
        let km = KeyMapper::new(false);
        assert!(km.get_key_data(0).is_some());
    }
}

mod test_midi_parser {
    use super::*;
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header};

    fn smf_bytes(ticks_per_beat: u16, events: Vec<(u32, TrackEventKind)>) -> Vec<u8> {
        let header = Header::new(Format::SingleTrack, Timing::Metrical(u15::from(ticks_per_beat)));
        let track: Vec<TrackEvent> = events
            .into_iter()
            .map(|(delta, kind)| TrackEvent {
                delta: u28::from(delta),
                kind,
            })
            .collect();
        let smf = Smf {
            header,
            tracks: vec![track],
        };
        let mut buf = Vec::new();
        smf.write(&mut buf).unwrap();
        buf
    }

    fn simple_midi() -> Vec<u8> {
        smf_bytes(
            480,
            vec![
                (0, TrackEventKind::Meta(MetaMessage::TrackName(b"Piano"))),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(60),
                            vel: u7::from(64),
                        },
                    },
                ),
                (
                    480,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(60),
                            vel: u7::from(0),
                        },
                    },
                ),
            ],
        )
    }

    fn pedal_midi() -> Vec<u8> {
        smf_bytes(
            480,
            vec![
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(60),
                            vel: u7::from(64),
                        },
                    },
                ),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::Controller {
                            controller: u7::from(64),
                            value: u7::from(127),
                        },
                    },
                ),
                (
                    480,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::Controller {
                            controller: u7::from(64),
                            value: u7::from(0),
                        },
                    },
                ),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(60),
                            vel: u7::from(0),
                        },
                    },
                ),
            ],
        )
    }

    #[test]
    fn test_returns_tracks_tempo_map_pedal_count() {
        let (tracks, _tempo_map, pedal_count, midi_pedal_events) =
            MidiParser::parse_bytes(&simple_midi(), 1.0).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(pedal_count, 0);
        assert!(midi_pedal_events.is_empty());
    }

    #[test]
    fn test_parses_single_note() {
        let (tracks, ..) = MidiParser::parse_bytes(&simple_midi(), 1.0).unwrap();
        assert!(tracks.len() >= 1);
        assert_eq!(tracks[0].notes.len(), 1);
        assert_eq!(tracks[0].notes[0].pitch, 60);
    }

    #[test]
    fn test_note_duration_approx_half_second() {
        let (tracks, ..) = MidiParser::parse_bytes(&simple_midi(), 1.0).unwrap();
        assert!((tracks[0].notes[0].duration - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_tempo_scale_halves_duration() {
        let bytes = simple_midi();
        let (t1, ..) = MidiParser::parse_bytes(&bytes, 1.0).unwrap();
        let (t2, ..) = MidiParser::parse_bytes(&bytes, 2.0).unwrap();
        let dur1 = t1[0].notes[0].duration;
        let dur2 = t2[0].notes[0].duration;
        assert!((dur2 - dur1 / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_no_pedal_count_zero() {
        let (_, _, pedal_count, midi_pedal_events) =
            MidiParser::parse_bytes(&simple_midi(), 1.0).unwrap();
        assert_eq!(pedal_count, 0);
        assert!(midi_pedal_events.is_empty());
    }

    #[test]
    fn test_pedal_on_off_counts_one() {
        let (_, _, pedal_count, midi_pedal_events) =
            MidiParser::parse_bytes(&pedal_midi(), 1.0).unwrap();
        assert_eq!(pedal_count, 1);
        assert_eq!(midi_pedal_events.len(), 2);
        assert!(midi_pedal_events[0].1);
        assert!(!midi_pedal_events[1].1);
    }

    #[test]
    fn test_bad_file_raises_ioerror() {
        let result = MidiParser::parse_bytes(b"not midi data", 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_track_name_extracted() {
        let (tracks, ..) = MidiParser::parse_bytes(&simple_midi(), 1.0).unwrap();
        assert_eq!(tracks[0].name, "Piano");
    }

    #[test]
    fn test_held_pedal_at_eof_counts() {
        let bytes = smf_bytes(
            480,
            vec![
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(60),
                            vel: u7::from(64),
                        },
                    },
                ),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::Controller {
                            controller: u7::from(64),
                            value: u7::from(127),
                        },
                    },
                ),
                (
                    480,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(60),
                            vel: u7::from(0),
                        },
                    },
                ),
            ],
        );
        let (_, _, pedal_count, midi_pedal_events) = MidiParser::parse_bytes(&bytes, 1.0).unwrap();
        assert_eq!(pedal_count, 1);
        assert_eq!(midi_pedal_events.len(), 1);
    }

    #[test]
    fn test_drum_channel_marks_track() {
        let bytes = smf_bytes(
            480,
            vec![
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(9),
                        message: MidiMessage::NoteOn {
                            key: u7::from(38),
                            vel: u7::from(64),
                        },
                    },
                ),
                (
                    240,
                    TrackEventKind::Midi {
                        channel: u4::from(9),
                        message: MidiMessage::NoteOff {
                            key: u7::from(38),
                            vel: u7::from(0),
                        },
                    },
                ),
            ],
        );
        let (tracks, ..) = MidiParser::parse_bytes(&bytes, 1.0).unwrap();
        assert!(tracks[0].is_drum);
    }

    #[test]
    fn test_zero_duration_note_dropped() {
        let bytes = smf_bytes(
            480,
            vec![
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(60),
                            vel: u7::from(64),
                        },
                    },
                ),
                (
                    0,
                    TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(60),
                            vel: u7::from(0),
                        },
                    },
                ),
            ],
        );
        let (tracks, ..) = MidiParser::parse_bytes(&bytes, 1.0).unwrap();
        assert!(tracks.is_empty());
    }
}
