use std::collections::HashMap;

use crate::core::midi::{KeyMapper, Modifier, TempoMap};
use crate::core::models::Note;

fn shift_num_to_symbol() -> HashMap<char, char> {
    KeyMapper::SYMBOL_MAP
        .iter()
        .map(|&(symbol, digit)| (digit, symbol))
        .collect()
}

fn build_pitch_to_char(key_mapper: &KeyMapper) -> HashMap<i32, char> {
    let shift_map = shift_num_to_symbol();
    let mut result = HashMap::new();
    for (pitch, data) in key_mapper.entries() {
        if data.modifiers.contains(&Modifier::Ctrl) {
            continue;
        }
        if data.modifiers.is_empty() {
            result.insert(pitch, data.key);
        } else if data.modifiers.contains(&Modifier::Shift) {
            if data.key.is_alphabetic() {
                result.insert(pitch, data.key.to_ascii_uppercase());
            } else if let Some(&symbol) = shift_map.get(&data.key) {
                result.insert(pitch, symbol);
            }
        }
    }
    result
}

fn build_char_to_pitch(key_mapper: &KeyMapper) -> HashMap<char, i32> {
    let pitch_to_char = build_pitch_to_char(key_mapper);
    let mut pitches: Vec<i32> = pitch_to_char.keys().copied().collect();
    pitches.sort_unstable();

    let mut result = HashMap::new();
    for pitch in pitches {
        let ch = pitch_to_char[&pitch];
        result.entry(ch).or_insert(pitch);
    }
    result
}

fn estimate_bpm(tempo_map: &TempoMap) -> f64 {
    if tempo_map.events().is_empty() {
        return 120.0;
    }
    60_000_000.0 / tempo_map.events()[0].1 as f64
}

pub struct VirtualPianoFormat;

impl VirtualPianoFormat {
    pub const NAME: &'static str = "Virtual Piano";

    fn parse_chunk(chunk: &str) -> Vec<(Vec<char>, u32)> {
        let chars: Vec<char> = chunk.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0usize;

        while i < chars.len() {
            let token_chars: Vec<char>;
            if chars[i] == '[' {
                match chars[i + 1..].iter().position(|&c| c == ']') {
                    Some(offset) => {
                        let j = i + 1 + offset;
                        token_chars = chars[i + 1..j].to_vec();
                        i = j + 1;
                    }
                    None => {
                        i += 1;
                        continue;
                    }
                }
            } else if chars[i] == '-' {
                i += 1;
                continue;
            } else {
                token_chars = vec![chars[i]];
                i += 1;
            }

            let mut dashes = 0u32;
            while i < chars.len() && chars[i] == '-' {
                dashes += 1;
                i += 1;
            }

            if !token_chars.is_empty() {
                tokens.push((token_chars, dashes));
            }
        }

        tokens
    }

    pub fn parse(text: &str, bpm: f64, key_mapper: &KeyMapper) -> Vec<Note> {
        let char_to_pitch = build_char_to_pitch(key_mapper);
        let base_16th = 60.0 / (bpm * 4.0);

        let mut notes = Vec::new();
        let mut note_id = 0i32;
        let mut current_time = 0.0f64;

        for line in text.trim().split('\n') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            for chunk in line.split_whitespace() {
                for (chars, dashes) in Self::parse_chunk(chunk) {
                    let duration = base_16th * 2f64.powi(dashes as i32);
                    for ch in chars {
                        if let Some(&pitch) = char_to_pitch.get(&ch) {
                            notes.push(Note {
                                id: note_id,
                                pitch,
                                velocity: 64,
                                start_time: current_time,
                                duration,
                                hand: "unknown".to_string(),
                                original_track_index: 0,
                                channel: 0,
                            });
                            note_id += 1;
                        }
                    }
                    current_time += duration;
                }
            }
        }

        notes
    }

    pub fn serialize(notes: &[Note], key_mapper: &KeyMapper, tempo_map: &TempoMap) -> String {
        if notes.is_empty() {
            return String::new();
        }

        let pitch_to_char = build_pitch_to_char(key_mapper);
        let avg_bpm = estimate_bpm(tempo_map);
        let base_16th = 60.0 / (avg_bpm * 4.0);

        let duration_to_dashes = |dur_secs: f64| -> i32 {
            let n16 = ((dur_secs / base_16th).round() as i64).max(1);
            let exp = (n16 as f64).log2().round() as i32;
            exp.clamp(0, 3)
        };

        struct Group {
            chars: Vec<char>,
            duration: f64,
        }

        let mut sorted_notes: Vec<&Note> = notes.iter().collect();
        sorted_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        let mut groups: HashMap<i64, Group> = HashMap::new();
        for note in sorted_notes {
            let Some(&ch) = pitch_to_char.get(&note.pitch) else {
                continue;
            };
            let q_units = (note.start_time / base_16th).round() as i64;
            let group = groups.entry(q_units).or_insert_with(|| Group {
                chars: Vec::new(),
                duration: note.duration,
            });
            group.duration = group.duration.max(note.duration);
            if !group.chars.contains(&ch) {
                group.chars.push(ch);
            }
        }

        if groups.is_empty() {
            return "# No mappable notes found (all notes may be out of range or on Ctrl keys)."
                .to_string();
        }

        let mut q_units: Vec<i64> = groups.keys().copied().collect();
        q_units.sort_unstable();

        let mut tokens = Vec::new();
        for q in q_units {
            let group = &groups[&q];
            let dashes = "-".repeat(duration_to_dashes(group.duration) as usize);
            if group.chars.len() == 1 {
                tokens.push(format!("{}{}", group.chars[0], dashes));
            } else {
                let chord: String = group.chars.iter().collect();
                tokens.push(format!("[{chord}]{dashes}"));
            }
        }

        let mut lines: Vec<String> = Vec::new();
        let mut current_line: Vec<String> = Vec::new();
        let mut line_len = 0usize;
        for token in tokens {
            if line_len + token.len() + 1 > 80 && !current_line.is_empty() {
                lines.push(current_line.join(" "));
                current_line.clear();
                line_len = 0;
            }
            current_line.push(token.clone());
            line_len += token.len() + 1;
        }
        if !current_line.is_empty() {
            lines.push(current_line.join(" "));
        }

        lines.join("\n")
    }
}

pub struct FormatRegistry;

impl FormatRegistry {
    pub fn names() -> Vec<&'static str> {
        vec![VirtualPianoFormat::NAME]
    }

    pub fn get(name: &str) -> Option<&'static str> {
        if name == VirtualPianoFormat::NAME {
            Some(VirtualPianoFormat::NAME)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
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
}
