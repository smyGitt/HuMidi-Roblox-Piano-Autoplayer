use crate::core::midi::TempoMap;
use crate::core::models::{KeyEvent, MidiTrack, Note};
use crate::core::pedal::model::PedalModel;
use crate::managers::config_manager::ConfigManager;
use crate::managers::hotkey_manager::HotkeyManager;
use crate::managers::theme_manager::ThemeManager;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
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
    pub pedal_model: Mutex<Option<PedalModel>>,
    pub parsed_tracks: Mutex<Option<Vec<MidiTrack>>>,
    pub parsed_tempo_map: Mutex<Option<TempoMap>>,
    pub loaded_pedal_count: Mutex<u32>,
    pub midi_pedal_events: Mutex<Vec<(f64, bool)>>,
    pub playback_session: Mutex<PlaybackSession>,
    pub playback_handle: Mutex<Option<PlaybackHandle>>,
    pub hotkey_manager: Mutex<HotkeyManager>,
    pub config_manager: Mutex<ConfigManager>,
    pub app_config: Mutex<serde_json::Value>,
    pub theme_manager: Mutex<ThemeManager>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut config_manager = ConfigManager::new();
        let app_config = config_manager.load();
        AppState {
            pedal_model: Mutex::new(None),
            parsed_tracks: Mutex::new(None),
            parsed_tempo_map: Mutex::new(None),
            loaded_pedal_count: Mutex::new(0),
            midi_pedal_events: Mutex::new(Vec::new()),
            playback_session: Mutex::new(PlaybackSession::default()),
            playback_handle: Mutex::new(None),
            hotkey_manager: Mutex::new(HotkeyManager::new()),
            config_manager: Mutex::new(config_manager),
            app_config: Mutex::new(app_config),
            theme_manager: Mutex::new(ThemeManager::new()),
        }
    }
}
