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
mod tests;
