use serde_json::{Map, Value};
use std::path::PathBuf;

use crate::core::atomic_write::write_json_atomic;
use crate::core::config::PlaybackConfig;
use crate::core::midi::TempoMap;
use crate::core::models::{KeyEvent, Note};

pub const NOTES_CONFIG_KEYS: &[&str] = &[
    "simulate_hands",
    "vary_timing",
    "timing_variance",
    "vary_articulation",
    "articulation",
    "enable_drift_correction",
    "drift_decay_factor",
    "enable_chord_roll",
    "enable_tempo_sway",
    "tempo_sway_intensity",
    "invert_tempo_sway",
    "enable_mistakes",
    "mistake_chance",
    "tempo",
    "use_88_key_layout",
];

pub const PEDAL_CONFIG_KEYS: &[&str] = &[
    "pedal_style",
    "use_ai_pedal",
    "pedal_curve_on",
    "pedal_curve_off",
    "use_midi_pedal",
    "pedal_before_humanize",
];

fn extract_keys(config: &PlaybackConfig, keys: &[&str]) -> Value {
    let full = serde_json::to_value(config).unwrap_or(Value::Object(Map::new()));
    let full_map = full.as_object().cloned().unwrap_or_default();
    let mut out = Map::new();
    for &k in keys {
        if let Some(v) = full_map.get(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    Value::Object(out)
}

pub fn extract_notes_config(config: &PlaybackConfig) -> Value {
    extract_keys(config, NOTES_CONFIG_KEYS)
}

pub fn extract_pedal_config(config: &PlaybackConfig) -> Value {
    extract_keys(config, PEDAL_CONFIG_KEYS)
}

pub fn notes_config_matches(snapshot: Option<&Value>, config: &PlaybackConfig) -> bool {
    match snapshot {
        None => false,
        Some(s) => *s == extract_notes_config(config),
    }
}

pub fn pedal_config_matches(snapshot: Option<&Value>, config: &PlaybackConfig) -> bool {
    match snapshot {
        None => false,
        Some(s) => *s == extract_pedal_config(config),
    }
}

pub fn cache_path() -> PathBuf {
    std::env::temp_dir().join("humidi_session.json")
}

pub fn tempo_map_to_dict(tempo_map: &TempoMap) -> Value {
    serde_json::json!({
        "events": tempo_map
            .events()
            .iter()
            .map(|(t, us)| serde_json::json!([t, us]))
            .collect::<Vec<_>>(),
        "time_signatures": tempo_map
            .time_signatures()
            .iter()
            .map(|(t, n, d)| serde_json::json!([t, n, d]))
            .collect::<Vec<_>>(),
    })
}

fn ser_event(ev: &KeyEvent) -> Value {
    serde_json::json!({
        "time": ev.time,
        "priority": ev.priority,
        "action": ev.action,
        "key_char": ev.key_char,
        "pitch": ev.pitch,
    })
}

fn ser_note(n: &Note) -> Value {
    serde_json::json!({
        "id": n.id,
        "pitch": n.pitch,
        "velocity": n.velocity,
        "start_time": n.start_time,
        "duration": n.duration,
        "hand": n.hand,
    })
}

fn build_cache_data(
    notes_config: Value,
    pedal_config: Value,
    note_events: &[KeyEvent],
    pedal_events: &[KeyEvent],
    humanized_notes: &[Note],
    final_notes: &[Note],
    tempo_map_data: Value,
    total_dur: f64,
) -> Value {
    serde_json::json!({
        "notes_config": notes_config,
        "pedal_config": pedal_config,
        "note_events": note_events.iter().map(ser_event).collect::<Vec<_>>(),
        "pedal_events": pedal_events.iter().map(ser_event).collect::<Vec<_>>(),
        "humanized_notes": humanized_notes.iter().map(ser_note).collect::<Vec<_>>(),
        "final_notes": final_notes.iter().map(ser_note).collect::<Vec<_>>(),
        "tempo_map": tempo_map_data,
        "total_dur": total_dur,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn write_cache_at(
    path: PathBuf,
    notes_config: Value,
    pedal_config: Value,
    note_events: &[KeyEvent],
    pedal_events: &[KeyEvent],
    humanized_notes: &[Note],
    final_notes: &[Note],
    tempo_map_data: Value,
    total_dur: f64,
) {
    let data = build_cache_data(
        notes_config,
        pedal_config,
        note_events,
        pedal_events,
        humanized_notes,
        final_notes,
        tempo_map_data,
        total_dur,
    );
    let _ = write_json_atomic(&path, &data);
}

#[allow(clippy::too_many_arguments)]
pub fn write_cache(
    notes_config: Value,
    pedal_config: Value,
    note_events: &[KeyEvent],
    pedal_events: &[KeyEvent],
    humanized_notes: &[Note],
    final_notes: &[Note],
    tempo_map_data: Value,
    total_dur: f64,
) {
    let data = build_cache_data(
        notes_config,
        pedal_config,
        note_events,
        pedal_events,
        humanized_notes,
        final_notes,
        tempo_map_data,
        total_dur,
    );
    let path = cache_path();
    std::thread::spawn(move || {
        let _ = write_json_atomic(&path, &data);
    });
}

pub fn read_cache_at(path: &std::path::Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn read_cache() -> Option<Value> {
    read_cache_at(&cache_path())
}

pub fn clear_cache_at(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
}

pub fn clear_cache() {
    clear_cache_at(&cache_path());
}

#[cfg(test)]
mod tests;
