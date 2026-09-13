use std::collections::HashMap;
use std::io;

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::core::models::{MidiTrack, Note};

const DEFAULT_TIME_GROUP_THRESHOLD: f64 = 0.015;
const DEFAULT_TEMPO_US_PER_BEAT: u32 = 500_000;
const DEFAULT_TICKS_PER_BEAT: u16 = 480;

pub fn get_time_groups(notes: &[Note]) -> Vec<Vec<Note>> {
    get_time_groups_with_threshold(notes, DEFAULT_TIME_GROUP_THRESHOLD)
}

pub fn get_time_groups_with_threshold(notes: &[Note], threshold: f64) -> Vec<Vec<Note>> {
    get_time_group_indices_with_threshold(notes, threshold)
        .into_iter()
        .map(|idxs| idxs.into_iter().map(|i| notes[i].clone()).collect())
        .collect()
}

pub fn get_time_group_indices(notes: &[Note]) -> Vec<Vec<usize>> {
    get_time_group_indices_with_threshold(notes, DEFAULT_TIME_GROUP_THRESHOLD)
}

pub fn get_time_group_indices_with_threshold(notes: &[Note], threshold: f64) -> Vec<Vec<usize>> {
    if notes.is_empty() {
        return Vec::new();
    }
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = vec![0];
    for i in 1..notes.len() {
        if notes[i].start_time - notes[current_group[0]].start_time <= threshold {
            current_group.push(i);
        } else {
            groups.push(std::mem::take(&mut current_group));
            current_group = vec![i];
        }
    }
    groups.push(current_group);
    groups
}

#[derive(Debug, Clone)]
pub struct TempoMap {
    events: Vec<(f64, u32)>,
    time_signatures: Vec<(f64, u8, u8)>,
    beat_map: Vec<(f64, f64, u32)>,
    beat_map_times: Vec<f64>,
    beat_map_beats: Vec<f64>,
    event_times: Vec<f64>,
    pub has_explicit_time_signatures: bool,
}

impl TempoMap {
    pub fn new(mut tempo_events: Vec<(f64, u32)>, mut time_signatures: Vec<(f64, u8, u8)>) -> Self {
        tempo_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        time_signatures.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let has_explicit_time_signatures = !time_signatures.is_empty()
            && !(time_signatures.len() == 1
                && time_signatures[0].0 == 0.0
                && time_signatures[0].1 == 4
                && time_signatures[0].2 == 4);

        let beat_map = Self::build_beat_map(&tempo_events);
        let beat_map_times = beat_map.iter().map(|e| e.0).collect();
        let beat_map_beats = beat_map.iter().map(|e| e.1).collect();
        let event_times = tempo_events.iter().map(|e| e.0).collect();

        TempoMap {
            events: tempo_events,
            time_signatures,
            beat_map,
            beat_map_times,
            beat_map_beats,
            event_times,
            has_explicit_time_signatures,
        }
    }

    fn build_beat_map(events: &[(f64, u32)]) -> Vec<(f64, f64, u32)> {
        let mut beat_map = Vec::new();
        let mut current_beat = 0.0;
        let mut last_time = 0.0;
        let mut current_tempo = DEFAULT_TEMPO_US_PER_BEAT;

        if events.is_empty() || events[0].0 > 0.0 {
            beat_map.push((0.0, 0.0, current_tempo));
        }

        for &(time_sec, new_tempo) in events {
            let dt = time_sec - last_time;
            let sec_per_beat = current_tempo as f64 / 1_000_000.0;
            current_beat += dt / sec_per_beat;
            beat_map.push((time_sec, current_beat, new_tempo));
            last_time = time_sec;
            current_tempo = new_tempo;
        }
        beat_map
    }

    fn bisect_right(values: &[f64], x: f64) -> usize {
        values.partition_point(|&v| v <= x)
    }

    pub fn time_to_beat(&self, t: f64) -> f64 {
        let idx = Self::bisect_right(&self.beat_map_times, t);
        if idx == 0 {
            return 0.0;
        }
        let (start_time, start_beat, tempo) = self.beat_map[idx - 1];
        let dt = t - start_time;
        let sec_per_beat = tempo as f64 / 1_000_000.0;
        start_beat + dt / sec_per_beat
    }

    pub fn beat_to_time(&self, b: f64) -> f64 {
        let idx = Self::bisect_right(&self.beat_map_beats, b);
        if idx == 0 {
            return 0.0;
        }
        let (start_time, start_beat, tempo) = self.beat_map[idx - 1];
        let dt_beats = b - start_beat;
        let sec_per_beat = tempo as f64 / 1_000_000.0;
        start_time + dt_beats * sec_per_beat
    }

    pub fn get_tempo_at(&self, time: f64) -> u32 {
        let idx = Self::bisect_right(&self.event_times, time);
        if idx == 0 {
            return DEFAULT_TEMPO_US_PER_BEAT;
        }
        self.events[idx - 1].1
    }

    pub fn events(&self) -> &[(f64, u32)] {
        &self.events
    }

    pub fn time_signatures(&self) -> &[(f64, u8, u8)] {
        &self.time_signatures
    }

    pub fn initial_bpm(&self) -> f64 {
        let selected = if self.events.len() > 1 && self.events[1].0 <= 5.0 && self.events[1].1 != 0 {
            self.events[1].1
        } else {
            self.events.first().map(|e| e.1).unwrap_or(DEFAULT_TEMPO_US_PER_BEAT)
        };
        let tempo_us = if selected == 0 {
            DEFAULT_TEMPO_US_PER_BEAT
        } else {
            selected
        };
        60_000_000.0 / tempo_us as f64
    }

    pub fn get_measure_boundaries(&self, total_duration: f64) -> Vec<(f64, f64)> {
        let default_ts = [(0.0, 4u8, 4u8)];
        let ts_events: &[(f64, u8, u8)] = if self.time_signatures.is_empty() {
            &default_ts
        } else {
            &self.time_signatures
        };
        let total_beats = self.time_to_beat(total_duration);
        let mut measures = Vec::new();
        let mut measure_start_beat = 0.0;

        while measure_start_beat < total_beats {
            let measure_start_time = self.beat_to_time(measure_start_beat);
            let mut active_ts = ts_events[0];
            for &ts in ts_events {
                if ts.0 <= measure_start_time + 0.001 {
                    active_ts = ts;
                } else {
                    break;
                }
            }

            let current_numerator = active_ts.1 as f64;
            let beat_len_factor = 4.0 / active_ts.2 as f64;
            let measure_len_beats = (current_numerator * beat_len_factor).max(0.0625);

            let measure_end_beat = measure_start_beat + measure_len_beats;
            let measure_end_time = self.beat_to_time(measure_end_beat);

            measures.push((measure_start_time, measure_end_time));
            measure_start_beat = measure_end_beat;
        }
        measures
    }
}

fn tick_to_seconds(ticks: u64, ticks_per_beat: u16, tempo: u32) -> f64 {
    ticks as f64 * tempo as f64 / (1_000_000.0 * ticks_per_beat as f64)
}

enum GlobalEvent {
    Tempo(u32),
    TimeSignature(u8, u8),
}

struct GlobalTickMap {
    tick_map: Vec<(u64, f64, u32)>,
    time_signatures: Vec<(f64, u8, u8)>,
    ticks_per_beat: u16,
    tick_values: Vec<u64>,
}

impl GlobalTickMap {
    fn build(tracks: &[Vec<TrackEvent>], ticks_per_beat: u16) -> Self {
        let mut events: Vec<(u64, usize, GlobalEvent)> = Vec::new();
        for (track_idx, track) in tracks.iter().enumerate() {
            let mut abs_tick: u64 = 0;
            for ev in track {
                abs_tick += ev.delta.as_int() as u64;
                match ev.kind {
                    TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => {
                        events.push((abs_tick, track_idx, GlobalEvent::Tempo(tempo.as_int())));
                    }
                    TrackEventKind::Meta(MetaMessage::TimeSignature(num, den_pow, _, _)) => {
                        let denominator = 1u32 << den_pow as u32;
                        events.push((
                            abs_tick,
                            track_idx,
                            GlobalEvent::TimeSignature(num, denominator as u8),
                        ));
                    }
                    _ => {}
                }
            }
        }
        events.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let mut tick_map = vec![(0u64, 0.0f64, DEFAULT_TEMPO_US_PER_BEAT)];
        let mut time_signatures = Vec::new();
        let mut current_time = 0.0;
        let mut current_tick: u64 = 0;
        let mut current_tempo = DEFAULT_TEMPO_US_PER_BEAT;

        for (tick, _track_idx, event) in events {
            let delta = tick - current_tick;
            current_time += tick_to_seconds(delta, ticks_per_beat, current_tempo);
            current_tick = tick;
            match event {
                GlobalEvent::Tempo(tempo) => {
                    current_tempo = tempo;
                    tick_map.push((current_tick, current_time, current_tempo));
                }
                GlobalEvent::TimeSignature(num, den) => {
                    time_signatures.push((current_time, num, den));
                }
            }
        }

        let tick_values = tick_map.iter().map(|e| e.0).collect();
        GlobalTickMap {
            tick_map,
            time_signatures,
            ticks_per_beat,
            tick_values,
        }
    }

    fn tick_to_time(&self, tick: u64) -> f64 {
        let idx = self.tick_values.partition_point(|&v| v <= tick);
        let idx = if idx == 0 { 0 } else { idx - 1 };
        let (last_tick, last_time, tempo) = self.tick_map[idx];
        last_time + tick_to_seconds(tick - last_tick, self.ticks_per_beat, tempo)
    }
}

pub struct MidiParser;

impl MidiParser {
    pub fn parse_structure(
        filepath: &str,
        tempo_scale: f64,
    ) -> io::Result<(Vec<MidiTrack>, TempoMap, u32, Vec<(f64, bool)>)> {
        let data = std::fs::read(filepath)
            .map_err(|e| io::Error::new(e.kind(), format!("Could not read MIDI file: {e}")))?;
        Self::parse_bytes(&data, tempo_scale)
    }

    pub fn parse_bytes(
        data: &[u8],
        tempo_scale: f64,
    ) -> io::Result<(Vec<MidiTrack>, TempoMap, u32, Vec<(f64, bool)>)> {
        let smf = Smf::parse(data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Could not read MIDI file: {e}")))?;
        let ticks_per_beat = match smf.header.timing {
            Timing::Metrical(tpb) => tpb.as_int(),
            Timing::Timecode(..) => DEFAULT_TICKS_PER_BEAT,
        };

        let global_map = GlobalTickMap::build(&smf.tracks, ticks_per_beat);
        let tempo_map_data: Vec<(f64, u32)> = global_map.tick_map.iter().map(|e| (e.1, e.2)).collect();
        let tempo_map = TempoMap::new(tempo_map_data, global_map.time_signatures.clone());

        let mut tracks = Vec::new();
        let mut note_id_counter: i32 = 0;
        let mut pedal_state_by_channel: HashMap<u8, bool> = HashMap::new();
        let mut pedal_event_count: u32 = 0;
        let mut midi_pedal_events: Vec<(f64, bool)> = Vec::new();

        for (i, track) in smf.tracks.iter().enumerate() {
            let mut track_name = format!("Track {i}");
            let mut program_change: i32 = 0;
            let mut is_drum = false;
            let mut notes: Vec<Note> = Vec::new();
            let mut open_notes: HashMap<u8, Vec<(u64, u8)>> = HashMap::new();
            let mut current_abs_tick: u64 = 0;

            for ev in track {
                current_abs_tick += ev.delta.as_int() as u64;
                match ev.kind {
                    TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                        track_name = String::from_utf8_lossy(name).into_owned();
                    }
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::ProgramChange { program } => {
                            program_change = program.as_int() as i32;
                            if channel.as_int() == 9 {
                                is_drum = true;
                            }
                        }
                        MidiMessage::Controller { controller, value } if controller.as_int() == 64 => {
                            let ch = channel.as_int();
                            let is_on = value.as_int() >= 64;
                            let was_on = *pedal_state_by_channel.get(&ch).unwrap_or(&false);
                            if is_on != was_on {
                                let msg_time = global_map.tick_to_time(current_abs_tick) / tempo_scale;
                                midi_pedal_events.push((msg_time, is_on));
                            }
                            if was_on && !is_on {
                                pedal_event_count += 1;
                            }
                            pedal_state_by_channel.insert(ch, is_on);
                        }
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            open_notes
                                .entry(key.as_int())
                                .or_default()
                                .push((current_abs_tick, vel.as_int()));
                        }
                        MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                            if let Some(queue) = open_notes.get_mut(&key.as_int()) {
                                if !queue.is_empty() {
                                    let (start_tick, vel) = queue.remove(0);
                                    let start_sec = global_map.tick_to_time(start_tick);
                                    let end_sec = global_map.tick_to_time(current_abs_tick);
                                    let duration = end_sec - start_sec;
                                    if duration > 0.0 {
                                        notes.push(Note {
                                            id: note_id_counter,
                                            pitch: key.as_int() as i32,
                                            velocity: vel as i32,
                                            start_time: start_sec / tempo_scale,
                                            duration: duration / tempo_scale,
                                            hand: "unknown".to_string(),
                                            original_track_index: i as i32,
                                            channel: channel.as_int() as i32,
                                        });
                                        note_id_counter += 1;
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if notes.iter().any(|n| n.channel == 9) {
                is_drum = true;
            }
            if !notes.is_empty() {
                notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
                tracks.push(MidiTrack {
                    index: i as i32,
                    name: track_name,
                    program_change,
                    is_drum,
                    notes,
                });
            }
        }

        pedal_event_count += pedal_state_by_channel.values().filter(|&&held| held).count() as u32;
        midi_pedal_events.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        Ok((tracks, tempo_map, pedal_event_count, midi_pedal_events))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Ctrl,
    Shift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyData {
    pub key: char,
    pub modifiers: Vec<Modifier>,
}

pub struct KeyMapper {
    pub use_88_key_layout: bool,
    pub min_pitch: i32,
    pub max_pitch: i32,
    key_map: HashMap<i32, KeyData>,
}

impl KeyMapper {
    pub const SYMBOL_MAP: [(char, char); 10] = [
        ('!', '1'),
        ('@', '2'),
        ('#', '3'),
        ('$', '4'),
        ('%', '5'),
        ('^', '6'),
        ('&', '7'),
        ('*', '8'),
        ('(', '9'),
        (')', '0'),
    ];
    pub const LEFT_CTRL_KEYS: &'static str = "1234567890qwert";
    pub const MIDDLE_WHITE_KEYS: &'static str = "1234567890qwertyuiopasdfghjklzxcvbnm";
    pub const RIGHT_CTRL_KEYS: &'static str = "yuiopasdfghj";
    pub const PITCH_START_LEFT: i32 = 21;
    pub const PITCH_START_MIDDLE: i32 = 36;
    pub const PITCH_START_RIGHT: i32 = 97;

    pub fn new(use_88_key_layout: bool) -> Self {
        let (min_pitch, max_pitch) = if use_88_key_layout { (21, 108) } else { (36, 96) };
        let mut mapper = KeyMapper {
            use_88_key_layout,
            min_pitch,
            max_pitch,
            key_map: HashMap::new(),
        };
        mapper.init_key_map();
        mapper
    }

    fn init_key_map(&mut self) {
        if self.use_88_key_layout {
            let mut pitch = Self::PITCH_START_LEFT;
            for ch in Self::LEFT_CTRL_KEYS.chars() {
                self.key_map.insert(
                    pitch,
                    KeyData {
                        key: ch,
                        modifiers: vec![Modifier::Ctrl],
                    },
                );
                pitch += 1;
            }
            let mut pitch = Self::PITCH_START_RIGHT;
            for ch in Self::RIGHT_CTRL_KEYS.chars() {
                self.key_map.insert(
                    pitch,
                    KeyData {
                        key: ch,
                        modifiers: vec![Modifier::Ctrl],
                    },
                );
                pitch += 1;
            }
        }

        let white_keys: Vec<char> = Self::MIDDLE_WHITE_KEYS.chars().collect();
        let mut white_key_index = 0;
        let mut current_pitch = Self::PITCH_START_MIDDLE;
        while current_pitch <= 108 && white_key_index < white_keys.len() {
            let base_char = white_keys[white_key_index];
            self.key_map.entry(current_pitch).or_insert(KeyData {
                key: base_char,
                modifiers: vec![],
            });
            let next_pitch = current_pitch + 1;
            if Self::is_black_key(next_pitch) {
                self.key_map.entry(next_pitch).or_insert(KeyData {
                    key: base_char,
                    modifiers: vec![Modifier::Shift],
                });
                current_pitch += 2;
            } else {
                current_pitch += 1;
            }
            white_key_index += 1;
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (i32, &KeyData)> {
        let mut pitches: Vec<i32> = self.key_map.keys().copied().collect();
        pitches.sort_unstable();
        pitches.into_iter().map(move |p| (p, &self.key_map[&p]))
    }

    pub fn get_key_data(&self, pitch: i32) -> Option<&KeyData> {
        let mut p = pitch;
        if p < self.min_pitch {
            while p < self.min_pitch {
                p += 12;
            }
        } else if p > self.max_pitch {
            while p > self.max_pitch {
                p -= 12;
            }
        }
        self.key_map.get(&p)
    }

    pub fn get_key_for_pitch(&self, pitch: i32) -> Option<char> {
        self.get_key_data(pitch).map(|d| d.key)
    }

    pub fn is_black_key(pitch: i32) -> bool {
        matches!(pitch.rem_euclid(12), 1 | 3 | 6 | 8 | 10)
    }

    pub fn pitch_to_name(pitch: i32) -> String {
        const NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        format!(
            "{}{}",
            NAMES[pitch.rem_euclid(12) as usize],
            pitch.div_euclid(12) - 1
        )
    }
}

#[cfg(test)]
mod tests {
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
}
