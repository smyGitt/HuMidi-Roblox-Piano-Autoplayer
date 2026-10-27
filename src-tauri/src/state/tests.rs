use super::*;
use crate::managers::hotkey_manager::RawKey;
use std::path::{Path, PathBuf};

fn config_manager_in(dir: &Path) -> ConfigManager {
    ConfigManager {
        save_dir: dir.join("saves"),
        midi_dir: PathBuf::new(),
        config_dir: dir.to_path_buf(),
        config_path: dir.join("config.json"),
    }
}

mod test_from_config_manager {
    use super::*;

    #[test]
    fn test_the_saved_hotkeys_are_active_from_the_start() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"hotkey_playback":"F9","hotkey_save":"Alt+W"}"#,
        )
        .unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        let hotkeys = state.hotkey_manager.lock().unwrap();
        assert_eq!(hotkeys.format_hotkey_string(), "F9");
        assert_eq!(hotkeys.format_save_hotkey_string(), "Alt+W");
    }

    #[test]
    fn test_no_virtual_keyboard_is_open_until_playback_needs_one() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        assert!(state.uinput_sink.lock().unwrap().is_none());
    }

    #[test]
    fn test_the_default_hotkeys_stay_without_a_saved_config() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        let hotkeys = state.hotkey_manager.lock().unwrap();
        assert_eq!(hotkeys.format_hotkey_string(), "F6");
        assert_eq!(hotkeys.format_save_hotkey_string(), "Ctrl+S");
    }

    #[test]
    fn test_the_app_config_is_what_was_loaded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.json"), r#"{"tempo":140}"#).unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        assert_eq!(state.app_config.lock().unwrap()["tempo"], 140);
    }

    #[test]
    fn test_nothing_is_loaded_and_nothing_is_playing_at_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        assert!(state.pedal_model.lock().unwrap().is_none());
        assert!(state.parsed_tracks.lock().unwrap().is_none());
        assert!(state.parsed_tempo_map.lock().unwrap().is_none());
        assert_eq!(*state.loaded_pedal_count.lock().unwrap(), 0);
        assert!(state.midi_pedal_events.lock().unwrap().is_empty());
        assert!(state.playback_session.lock().unwrap().final_notes.is_none());
        assert!(state.playback_handle.lock().unwrap().is_none());
    }

    #[test]
    fn test_no_keys_are_pending_at_the_start() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        assert!(!state.injected_echoes.consume(RawKey::Char('g'), true));
        assert!(!state.hook_seen.is_duplicate(RawKey::Char('g'), true));
    }

    #[test]
    fn test_the_hook_record_and_the_echo_list_are_separate() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::from_config_manager(config_manager_in(dir.path()));
        state.hook_seen.record(RawKey::Char('g'), true);
        assert!(!state.injected_echoes.consume(RawKey::Char('g'), true));
        assert!(state.hook_seen.is_duplicate(RawKey::Char('g'), true));
    }
}
