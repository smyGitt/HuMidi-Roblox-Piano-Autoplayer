use std::path::{Path, PathBuf};

use humidi_tauri_lib::commands::{load_save_file_logic, resume_from_save_logic};
use humidi_tauri_lib::core::input_scan::BLOCKED_INPUT_PREFIX;
use humidi_tauri_lib::core::models::KeyEvent;
use humidi_tauri_lib::state::PlaybackSession;
use serde_json::json;

const NOTE_KEY: &str = "t";
const NOTE_PITCH: i32 = 60;
const NOTE_VELOCITY: i32 = 64;
const PRESS_ACTION: &str = "press";
const RELEASE_ACTION: &str = "release";
const PEDAL_ACTION: &str = "pedal";
const UNKNOWN_ACTION: &str = "run";
const PRESS_PRIORITY: i32 = 1;
const RELEASE_PRIORITY: i32 = 0;
const PRESS_TIME: f64 = 0.0;
const RELEASE_TIME: f64 = 0.5;
const EXTRA_EVENT_TIME: f64 = 1.0;
const SHELL_COMMAND_KEY: &str = "cmd /c calc";
const WRONG_NOTE_KEY: &str = "x";
const PEDAL_SHORTCUT: &str = "win+r";
const LAST_ACCESSED: &str = "2026-10-10T03:18:03.839315600-05:00";

fn note_press(key_char: &str, time: f64) -> KeyEvent {
    KeyEvent {
        time,
        priority: PRESS_PRIORITY,
        action: PRESS_ACTION.to_string(),
        key_char: key_char.to_string(),
        pitch: Some(NOTE_PITCH),
        velocity: Some(NOTE_VELOCITY),
    }
}

fn note_release(time: f64) -> KeyEvent {
    KeyEvent {
        time,
        priority: RELEASE_PRIORITY,
        action: RELEASE_ACTION.to_string(),
        key_char: NOTE_KEY.to_string(),
        pitch: Some(NOTE_PITCH),
        velocity: None,
    }
}

fn valid_note() -> Vec<KeyEvent> {
    vec![note_press(NOTE_KEY, PRESS_TIME), note_release(RELEASE_TIME)]
}

fn valid_note_then(extra: KeyEvent) -> Vec<KeyEvent> {
    let mut events = valid_note();
    events.push(extra);
    events
}

fn blocked_cases() -> Vec<(&'static str, Vec<KeyEvent>)> {
    vec![
        (
            "blocked_key",
            valid_note_then(KeyEvent {
                pitch: None,
                ..note_press(SHELL_COMMAND_KEY, EXTRA_EVENT_TIME)
            }),
        ),
        (
            "blocked_wrong_key",
            vec![note_press(WRONG_NOTE_KEY, PRESS_TIME)],
        ),
        (
            "blocked_action",
            valid_note_then(KeyEvent {
                action: UNKNOWN_ACTION.to_string(),
                ..note_press(NOTE_KEY, EXTRA_EVENT_TIME)
            }),
        ),
        (
            "blocked_pedal",
            valid_note_then(KeyEvent {
                action: PEDAL_ACTION.to_string(),
                pitch: None,
                velocity: None,
                ..note_press(PEDAL_SHORTCUT, EXTRA_EVENT_TIME)
            }),
        ),
    ]
}

fn write_save(dir: &Path, name: &str, events: &[KeyEvent]) -> PathBuf {
    let save = json!({
        "compiled_events": events,
        "metadata": {
            "compiled_pedal_count": 0,
            "last_accessed": LAST_ACCESSED,
            "track_details": [],
        },
    });
    let path = dir.join(format!("{name}.save.json"));
    std::fs::write(&path, save.to_string()).unwrap();
    path
}

fn load(path: &Path) -> (Result<(), String>, PlaybackSession) {
    let mut session = PlaybackSession::default();
    let data = load_save_file_logic(path).unwrap();
    let result = resume_from_save_logic(&data, &mut session).map(|_| ());
    (result, session)
}

#[test]
fn a_valid_save_loads() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_save(dir.path(), "valid", &valid_note());
    let (result, session) = load(&path);
    result.unwrap();
    assert!(session.merged_events.is_some());
}

#[test]
fn every_blocked_save_is_stopped_and_leaves_the_session_empty() {
    let dir = tempfile::tempdir().unwrap();
    for (name, events) in blocked_cases() {
        let path = write_save(dir.path(), name, &events);
        let (result, session) = load(&path);
        let err = result.unwrap_err();
        assert!(err.starts_with(BLOCKED_INPUT_PREFIX), "{name}: {err}");
        assert!(session.merged_events.is_none(), "{name}");
    }
}
