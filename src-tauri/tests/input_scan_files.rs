use std::path::PathBuf;

use humidi_tauri_lib::commands::{load_save_file_logic, resume_from_save_logic};
use humidi_tauri_lib::core::input_scan::BLOCKED_INPUT_PREFIX;
use humidi_tauri_lib::state::PlaybackSession;

fn sample(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test-files")
        .join(name)
}

fn load(name: &str) -> (Result<(), String>, PlaybackSession) {
    let mut session = PlaybackSession::default();
    let data = load_save_file_logic(&sample(name)).unwrap();
    let result = resume_from_save_logic(&data, &mut session).map(|_| ());
    (result, session)
}

#[test]
fn the_valid_sample_save_loads() {
    let (result, session) = load("input_scan_valid.save.json");
    result.unwrap();
    assert!(session.merged_events.is_some());
}

#[test]
fn every_blocked_sample_save_is_stopped_and_leaves_the_session_empty() {
    for name in [
        "input_scan_blocked_key.save.json",
        "input_scan_blocked_wrong_key.save.json",
        "input_scan_blocked_action.save.json",
        "input_scan_blocked_pedal.save.json",
    ] {
        let (result, session) = load(name);
        let err = result.unwrap_err();
        assert!(err.starts_with(BLOCKED_INPUT_PREFIX), "{name}: {err}");
        assert!(session.merged_events.is_none(), "{name}");
    }
}

#[test]
fn the_sample_midi_exists_and_is_a_midi_header() {
    let bytes = std::fs::read(sample("input_scan_valid.mid")).unwrap();
    assert_eq!(&bytes[..4], b"MThd");
}
