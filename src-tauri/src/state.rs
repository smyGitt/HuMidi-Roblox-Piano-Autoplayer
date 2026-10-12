use crate::core::midi::TempoMap;
use crate::core::models::{KeyEvent, MidiTrack, Note};
use crate::core::pedal::model::PedalModel;
use crate::managers::config_manager::ConfigManager;
use crate::managers::hook_seen::HookSeen;
use crate::managers::hotkey_manager::HotkeyManager;
use crate::managers::injected_echo::InjectedEchoes;
use crate::managers::theme_manager::ThemeManager;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[derive(Default)]
pub struct PlaybackSession {
    pub final_notes: Option<Vec<Note>>,
    pub humanized_notes: Option<Vec<Note>>,
    pub note_events: Option<Vec<KeyEvent>>,
    pub pedal_events: Option<Vec<KeyEvent>>,
    pub merged_events: Option<Vec<KeyEvent>>,
    pub tempo_map: Option<TempoMap>,
    pub total_dur: f64,
    pub midi_pedal_events: Vec<(f64, bool)>,
    pub notes_config_snapshot: Option<serde_json::Value>,
    pub pedal_config_snapshot: Option<serde_json::Value>,
}

pub enum PlaybackCommand {
    TogglePause,
    Stop,
    Seek(f64),
    Shutdown,
}

pub struct PlaybackHandle {
    pub cmd_tx: Sender<PlaybackCommand>,
    pub thread: JoinHandle<()>,
}

pub struct AppState {
    pub pedal_model: Mutex<Option<Arc<PedalModel>>>,
    pub parsed_tracks: Mutex<Option<Vec<MidiTrack>>>,
    pub parsed_tempo_map: Mutex<Option<TempoMap>>,
    pub loaded_pedal_count: Mutex<u32>,
    pub midi_pedal_events: Mutex<Vec<(f64, bool)>>,
    pub playback_session: Mutex<PlaybackSession>,
    pub playback_handle: Mutex<Option<PlaybackHandle>>,
    pub hotkey_manager: Mutex<HotkeyManager>,
    pub injected_echoes: Arc<InjectedEchoes>,
    pub hook_seen: HookSeen,
    pub config_manager: Mutex<ConfigManager>,
    pub app_config: Mutex<serde_json::Value>,
    pub theme_manager: Mutex<ThemeManager>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_config_manager(ConfigManager::new())
    }
}

impl AppState {
    pub fn from_config_manager(mut config_manager: ConfigManager) -> Self {
        let app_config = config_manager.load();
        let mut hotkey_manager = HotkeyManager::new();
        hotkey_manager.restore_from_config(&app_config);
        AppState {
            pedal_model: Mutex::new(None),
            parsed_tracks: Mutex::new(None),
            parsed_tempo_map: Mutex::new(None),
            loaded_pedal_count: Mutex::new(0),
            midi_pedal_events: Mutex::new(Vec::new()),
            playback_session: Mutex::new(PlaybackSession::default()),
            playback_handle: Mutex::new(None),
            hotkey_manager: Mutex::new(hotkey_manager),
            injected_echoes: Arc::new(InjectedEchoes::new()),
            hook_seen: HookSeen::new(),
            config_manager: Mutex::new(config_manager),
            app_config: Mutex::new(app_config),
            theme_manager: Mutex::new(ThemeManager::new()),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
