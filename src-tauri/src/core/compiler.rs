use std::collections::HashSet;

use rand::Rng;

use crate::core::config::PlaybackConfig;
use crate::core::humanizer::{round_resync_key, Humanizer};
use crate::core::midi::KeyMapper;
use crate::core::models::{KeyEvent, MusicalSection, Note};
use crate::core::pedal::model::PedalModel;
use crate::core::pedal::{self, AiThresholds};

pub fn compile_note_events(
    config: &PlaybackConfig,
    notes: &[Note],
    sections: &[MusicalSection],
) -> (Vec<KeyEvent>, Vec<Note>) {
    let (events, humanized, _) = compile_note_events_with_played(config, notes, sections);
    (events, humanized)
}

pub fn compile_note_events_with_played(
    config: &PlaybackConfig,
    notes: &[Note],
    sections: &[MusicalSection],
) -> (Vec<KeyEvent>, Vec<Note>, Vec<Note>) {
    let mut humanizer = Humanizer::new(config.clone());

    let mut left_hand_notes: Vec<Note> = notes.iter().filter(|n| n.hand == "left").cloned().collect();
    let mut right_hand_notes: Vec<Note> =
        notes.iter().filter(|n| n.hand == "right").cloned().collect();

    let left_times: HashSet<i64> = left_hand_notes
        .iter()
        .map(|n| round_resync_key(n.start_time))
        .collect();
    let right_times: HashSet<i64> = right_hand_notes
        .iter()
        .map(|n| round_resync_key(n.start_time))
        .collect();
    let resync_points: HashSet<i64> = left_times.intersection(&right_times).copied().collect();

    humanizer.apply_to_hand(&mut left_hand_notes, "left", &resync_points);
    humanizer.apply_to_hand(&mut right_hand_notes, "right", &resync_points);

    let mut all_notes: Vec<Note> = left_hand_notes.into_iter().chain(right_hand_notes).collect();
    all_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

    humanizer.apply_tempo_rubato(&mut all_notes, sections);

    let mapper = KeyMapper::new(config.use_88_key_layout);
    let use_mistakes = config.enable_mistakes;
    let mistake_chance = config.mistake_chance / 100.0;
    let mut rng = rand::rng();
    let mut events: Vec<KeyEvent> = Vec::new();
    let mut played: Vec<Note> = Vec::new();

    for note in &all_notes {
        let mut scheduled = false;
        if use_mistakes && rng.random::<f64>() < mistake_chance {
            if let Some(mistake_pitch) = get_mistake_pitch(&mut rng, note.pitch) {
                if let Some(key_data) = mapper.get_key_data(mistake_pitch) {
                    let mk_char = key_data.key.to_string();
                    let mut press = KeyEvent::new(note.start_time, 2, "press", &mk_char);
                    press.pitch = Some(mistake_pitch);
                    press.velocity = Some(note.velocity);
                    events.push(press);
                    let mut release =
                        KeyEvent::new(note.start_time + note.duration, 4, "release", &mk_char);
                    release.pitch = Some(mistake_pitch);
                    events.push(release);
                    played.push(Note {
                        pitch: mistake_pitch,
                        ..note.clone()
                    });
                    scheduled = true;
                }
            }
        }
        if !scheduled {
            if let Some(key_data) = mapper.get_key_data(note.pitch) {
                let key_char = key_data.key.to_string();
                let mut press = KeyEvent::new(note.start_time, 2, "press", &key_char);
                press.pitch = Some(note.pitch);
                press.velocity = Some(note.velocity);
                events.push(press);
                let mut release = KeyEvent::new(note.end_time(), 4, "release", &key_char);
                release.pitch = Some(note.pitch);
                events.push(release);
                played.push(note.clone());
            }
        }
    }

    events.sort();
    (events, all_notes, played)
}

pub fn compile_midi_pedal_events(midi_pedal_events: &[(f64, bool)]) -> Vec<KeyEvent> {
    midi_pedal_events
        .iter()
        .map(|&(time_sec, is_on)| {
            let key_char = if is_on { "down" } else { "up" };
            let priority = if is_on { 1 } else { 0 };
            KeyEvent::new(time_sec, priority, "pedal", key_char)
        })
        .collect()
}

pub fn compile_pedal_events(
    model: Option<&PedalModel>,
    config: &PlaybackConfig,
    humanized_notes: &[Note],
    sections: &[MusicalSection],
    midi_pedal_events: Option<&[(f64, bool)]>,
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    if let Some(midi_events) = midi_pedal_events {
        if !midi_events.is_empty() && config.use_midi_pedal {
            return (compile_midi_pedal_events(midi_events), None);
        }
    }
    pedal::generate_events(model, config, humanized_notes, sections)
}

pub fn merge_compiled(note_events: &[KeyEvent], pedal_events: &[KeyEvent]) -> Vec<KeyEvent> {
    let mut merged: Vec<KeyEvent> = note_events
        .iter()
        .cloned()
        .chain(pedal_events.iter().cloned())
        .collect();
    merged.sort();
    merged
}

pub fn compile_events(
    model: Option<&PedalModel>,
    config: &PlaybackConfig,
    notes: &[Note],
    sections: &[MusicalSection],
    midi_pedal_events: Option<&[(f64, bool)]>,
) -> (Vec<KeyEvent>, Option<AiThresholds>) {
    let (note_events, humanized_notes) = compile_note_events(config, notes, sections);
    let pedal_notes: &[Note] = if config.pedal_before_humanize {
        notes
    } else {
        &humanized_notes
    };
    let (pedal_events, ai_meta) =
        compile_pedal_events(model, config, pedal_notes, sections, midi_pedal_events);
    (merge_compiled(&note_events, &pedal_events), ai_meta)
}

fn get_mistake_pitch(rng: &mut impl Rng, original_pitch: i32) -> Option<i32> {
    let candidates = [
        original_pitch - 2,
        original_pitch - 1,
        original_pitch + 1,
        original_pitch + 2,
    ];
    if KeyMapper::is_black_key(original_pitch) {
        let black_pool: Vec<i32> = candidates
            .iter()
            .copied()
            .filter(|&p| KeyMapper::is_black_key(p))
            .collect();
        let white_pool: Vec<i32> = candidates
            .iter()
            .copied()
            .filter(|&p| !KeyMapper::is_black_key(p))
            .collect();
        let prefer_black = rng.random::<f64>() < 0.5;
        let pool = if prefer_black && !black_pool.is_empty() {
            black_pool
        } else if !prefer_black && !white_pool.is_empty() {
            white_pool
        } else if !black_pool.is_empty() {
            black_pool
        } else {
            white_pool
        };
        if pool.is_empty() {
            None
        } else {
            Some(pool[rng.random_range(0..pool.len())])
        }
    } else {
        let valid: Vec<i32> = candidates
            .iter()
            .copied()
            .filter(|&p| !KeyMapper::is_black_key(p))
            .collect();
        if valid.is_empty() {
            None
        } else {
            Some(valid[rng.random_range(0..valid.len())])
        }
    }
}

#[cfg(test)]
mod tests;
