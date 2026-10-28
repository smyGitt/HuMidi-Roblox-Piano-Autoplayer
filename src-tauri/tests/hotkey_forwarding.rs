use std::sync::{Arc, Mutex};

use humidi_tauri_lib::commands::{
    deliver_key, emit_hotkey_events, forward_key, handle_device_key, handle_hook_event, handle_source_key,
    patch_app_config, KeySource,
};
use humidi_tauri_lib::core::keyboard_driver::Platform;
use humidi_tauri_lib::managers::keyboard_hook::{HookSource, KEYBOARD_HOTKEYS_CONFIG_KEY};
use humidi_tauri_lib::managers::config_manager::ConfigManager;
use humidi_tauri_lib::managers::hotkey_manager::{dom_code_to_raw, HotkeyEvent, RawKey};
use humidi_tauri_lib::state::AppState;
use tauri::test::{mock_app, MockRuntime};
use tauri::{App, Listener, Manager};

const ALL_CHANNELS: [&str; 4] = [
    "hotkey_bound_updated",
    "hotkey_bound_save_updated",
    "hotkey_toggle_requested",
    "hotkey_save_requested",
];

struct Harness {
    app: App<MockRuntime>,
    emitted: Arc<Mutex<Vec<(String, String)>>>,
    _dir: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        app.manage(AppState::from_config_manager(ConfigManager {
            save_dir: dir.path().join("saves"),
            midi_dir: std::path::PathBuf::new(),
            config_dir: dir.path().to_path_buf(),
            config_path: dir.path().join("config.json"),
        }));
        let emitted = Arc::new(Mutex::new(Vec::new()));
        for channel in ALL_CHANNELS {
            let sink = Arc::clone(&emitted);
            app.listen(channel, move |event| {
                sink.lock()
                    .unwrap()
                    .push((channel.to_string(), event.payload().to_string()));
            });
        }
        Harness {
            app,
            emitted,
            _dir: dir,
        }
    }

    fn forward(&self, code: &str, pressed: bool) -> Result<(), String> {
        forward_key(
            self.app.handle().clone(),
            self.app.state::<AppState>(),
            code.to_string(),
            pressed,
        )
    }

    fn emitted(&self) -> Vec<(String, String)> {
        self.emitted.lock().unwrap().clone()
    }

    fn start_binding(&self) {
        self.app
            .state::<AppState>()
            .hotkey_manager
            .lock()
            .unwrap()
            .start_binding();
    }

    fn start_save_binding(&self) {
        self.app
            .state::<AppState>()
            .hotkey_manager
            .lock()
            .unwrap()
            .start_save_binding();
    }

    fn playback_hotkey(&self) -> String {
        self.app
            .state::<AppState>()
            .hotkey_manager
            .lock()
            .unwrap()
            .format_hotkey_string()
    }
}

fn pair(channel: &str, payload: &str) -> (String, String) {
    (channel.to_string(), payload.to_string())
}

#[test]
fn pressing_g_in_the_focused_window_while_listening_binds_g_and_tells_the_frontend() {
    let harness = Harness::new();
    harness.start_binding();
    assert_eq!(harness.forward("KeyG", true), Ok(()));
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_updated", "\"G\"")]);
    assert_eq!(harness.playback_hotkey(), "G");
}

#[test]
fn pressing_d_in_the_focused_window_while_listening_for_save_binds_d_for_save() {
    let harness = Harness::new();
    harness.start_save_binding();
    assert_eq!(harness.forward("KeyD", true), Ok(()));
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_save_updated", "\"D\"")]);
}

#[test]
fn the_playback_hotkey_pressed_in_the_focused_window_requests_a_toggle() {
    let harness = Harness::new();
    assert_eq!(harness.forward("F6", true), Ok(()));
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn the_save_combo_pressed_in_the_focused_window_requests_a_save() {
    let harness = Harness::new();
    harness.forward("ControlLeft", true).unwrap();
    harness.forward("KeyS", true).unwrap();
    assert_eq!(harness.emitted(), vec![pair("hotkey_save_requested", "null")]);
}

#[test]
fn releasing_the_modifier_in_the_focused_window_stops_the_combo_from_matching() {
    let harness = Harness::new();
    harness.forward("ControlLeft", true).unwrap();
    harness.forward("ControlLeft", false).unwrap();
    harness.forward("KeyS", true).unwrap();
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn a_key_with_no_hotkey_meaning_is_accepted_and_ignored() {
    let harness = Harness::new();
    assert_eq!(harness.forward("NumpadEnter", true), Ok(()));
    assert_eq!(harness.forward("", true), Ok(()));
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn a_key_with_no_hotkey_meaning_does_not_end_listening_or_bind_anything() {
    let harness = Harness::new();
    harness.start_binding();
    assert_eq!(harness.forward("NumpadEnter", true), Ok(()));
    assert_eq!(harness.forward("Unidentified", true), Ok(()));
    assert_eq!(harness.emitted(), vec![]);
    assert_eq!(harness.playback_hotkey(), "F6");
    harness.forward("KeyG", true).unwrap();
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_updated", "\"G\"")]);
}

#[test]
fn a_release_alone_emits_nothing() {
    let harness = Harness::new();
    assert_eq!(harness.forward("F6", false), Ok(()));
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn a_key_the_global_hook_already_delivered_is_not_delivered_a_second_time_by_the_page() {
    let harness = Harness::new();
    let state = harness.app.state::<AppState>();
    let events = deliver_key(
        &state,
        dom_code_to_raw("F6").unwrap(),
        true,
        KeySource::GlobalHook,
    );
    assert_eq!(events, vec![HotkeyEvent::ToggleRequested]);
    assert_eq!(harness.forward("F6", true), Ok(()));
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn the_same_key_pressed_twice_in_the_focused_window_requests_two_toggles() {
    let harness = Harness::new();
    harness.forward("F6", true).unwrap();
    harness.forward("F6", false).unwrap();
    harness.forward("F6", true).unwrap();
    assert_eq!(
        harness.emitted(),
        vec![
            pair("hotkey_toggle_requested", "null"),
            pair("hotkey_toggle_requested", "null"),
        ]
    );
}

#[test]
fn a_global_hook_press_of_g_while_listening_binds_g_and_tells_the_frontend() {
    let harness = Harness::new();
    harness.start_binding();
    handle_hook_event(harness.app.handle(), &rdev::EventType::KeyPress(rdev::Key::KeyG));
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_updated", "\"G\"")]);
    assert_eq!(harness.playback_hotkey(), "G");
}

#[test]
fn a_global_hook_press_of_the_playback_hotkey_requests_a_toggle() {
    let harness = Harness::new();
    handle_hook_event(harness.app.handle(), &rdev::EventType::KeyPress(rdev::Key::F6));
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn global_hook_events_that_are_not_mapped_keys_do_nothing() {
    let harness = Harness::new();
    harness.start_binding();
    let app = harness.app.handle();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::Unknown(0)));
    handle_hook_event(app, &rdev::EventType::ButtonPress(rdev::Button::Left));
    handle_hook_event(app, &rdev::EventType::MouseMove { x: 0.0, y: 0.0 });
    assert_eq!(harness.emitted(), vec![]);
    assert_eq!(harness.playback_hotkey(), "F6");
}

#[test]
fn a_global_hook_release_of_a_modifier_stops_the_combo_from_matching() {
    let harness = Harness::new();
    let app = harness.app.handle();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::ControlLeft));
    handle_hook_event(app, &rdev::EventType::KeyRelease(rdev::Key::ControlLeft));
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::KeyS));
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn a_key_heard_by_the_global_hook_and_then_forwarded_by_the_page_triggers_once() {
    let harness = Harness::new();
    handle_hook_event(harness.app.handle(), &rdev::EventType::KeyPress(rdev::Key::F6));
    harness.forward("F6", true).unwrap();
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn a_combo_started_in_another_window_and_finished_in_the_focused_one_still_matches() {
    let harness = Harness::new();
    handle_hook_event(harness.app.handle(), &rdev::EventType::KeyPress(rdev::Key::ControlLeft));
    harness.forward("KeyS", true).unwrap();
    assert_eq!(harness.emitted(), vec![pair("hotkey_save_requested", "null")]);
}

#[test]
fn the_save_combo_pressed_outside_the_window_emits_nothing() {
    let harness = Harness::new();
    let app = harness.app.handle();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::ControlLeft));
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::KeyS));
    handle_hook_event(app, &rdev::EventType::KeyRelease(rdev::Key::KeyS));
    handle_hook_event(app, &rdev::EventType::KeyRelease(rdev::Key::ControlLeft));
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn the_save_combo_the_window_also_reports_requests_exactly_one_save() {
    let harness = Harness::new();
    let app = harness.app.handle();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::ControlLeft));
    harness.forward("ControlLeft", true).unwrap();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::KeyS));
    harness.forward("KeyS", true).unwrap();
    assert_eq!(harness.emitted(), vec![pair("hotkey_save_requested", "null")]);
}

#[test]
fn the_playback_hotkey_pressed_outside_the_window_toggles_even_though_save_does_not() {
    let harness = Harness::new();
    let app = harness.app.handle();
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::F6));
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::ControlLeft));
    handle_hook_event(app, &rdev::EventType::KeyPress(rdev::Key::KeyS));
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn one_key_bound_to_both_hotkeys_toggles_outside_the_window_and_also_saves_inside_it() {
    let harness = Harness::new();
    harness
        .app
        .state::<AppState>()
        .hotkey_manager
        .lock()
        .unwrap()
        .set_save_hotkey_from_string("F6");
    handle_hook_event(harness.app.handle(), &rdev::EventType::KeyPress(rdev::Key::F6));
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
    harness.forward("F6", true).unwrap();
    assert_eq!(
        harness.emitted(),
        vec![
            pair("hotkey_toggle_requested", "null"),
            pair("hotkey_save_requested", "null"),
        ]
    );
}

#[test]
fn emitting_every_kind_of_event_uses_its_own_channel_and_payload() {
    let harness = Harness::new();
    emit_hotkey_events(
        harness.app.handle(),
        &[
            HotkeyEvent::BoundUpdated("Alt+P".to_string()),
            HotkeyEvent::BoundSaveUpdated("Ctrl+K".to_string()),
            HotkeyEvent::ToggleRequested,
            HotkeyEvent::SaveRequested,
        ],
    );
    assert_eq!(
        harness.emitted(),
        vec![
            pair("hotkey_bound_updated", "\"Alt+P\""),
            pair("hotkey_bound_save_updated", "\"Ctrl+K\""),
            pair("hotkey_toggle_requested", "null"),
            pair("hotkey_save_requested", "null"),
        ]
    );
}

#[test]
fn emitting_no_events_emits_nothing() {
    let harness = Harness::new();
    emit_hotkey_events(harness.app.handle(), &[]);
    assert_eq!(harness.emitted(), vec![]);
}

const ONE_KEYBOARD_OPEN: usize = 1;
const NO_KEYBOARD_OPEN: usize = 0;

fn f6() -> RawKey {
    RawKey::Named("F6")
}

fn from_source(harness: &Harness, source: HookSource, raw: RawKey, platform: Platform, devices: usize) {
    handle_source_key(harness.app.handle(), source, raw, true, platform, devices);
}

fn turn_keyboard_hotkeys(harness: &Harness, on: bool) {
    let state = harness.app.state::<AppState>();
    patch_app_config(&state, &serde_json::json!({ KEYBOARD_HOTKEYS_CONFIG_KEY: on })).unwrap();
}

#[test]
fn on_linux_with_a_keyboard_open_a_key_from_the_keyboard_devices_toggles_playback() {
    let harness = Harness::new();
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn on_linux_with_a_keyboard_open_the_old_hook_is_ignored() {
    let harness = Harness::new();
    from_source(&harness, HookSource::Rdev, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![]);
}

#[test]
fn on_linux_with_no_keyboard_open_the_old_hook_is_used_and_the_devices_are_ignored() {
    let harness = Harness::new();
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, NO_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![]);
    from_source(&harness, HookSource::Rdev, f6(), Platform::Linux, NO_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn with_the_setting_off_the_old_hook_is_used_even_with_a_keyboard_open() {
    let harness = Harness::new();
    turn_keyboard_hotkeys(&harness, false);
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![]);
    from_source(&harness, HookSource::Rdev, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn turning_the_setting_on_and_off_takes_effect_at_once() {
    let harness = Harness::new();
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    turn_keyboard_hotkeys(&harness, false);
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    turn_keyboard_hotkeys(&harness, true);
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted().len(), 2);
}

#[test]
fn off_linux_the_devices_are_never_used() {
    let harness = Harness::new();
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Windows, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![]);
    from_source(&harness, HookSource::Rdev, f6(), Platform::Windows, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn a_key_heard_by_both_hooks_toggles_once() {
    let harness = Harness::new();
    from_source(&harness, HookSource::Rdev, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    from_source(&harness, HookSource::KeyboardDevices, f6(), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_toggle_requested", "null")]);
}

#[test]
fn a_key_from_the_keyboard_devices_binds_a_new_hotkey_while_listening() {
    let harness = Harness::new();
    harness.start_binding();
    from_source(&harness, HookSource::KeyboardDevices, RawKey::Char('g'), Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_updated", "\"G\"")]);
}

#[test]
fn a_release_from_the_keyboard_devices_reaches_the_hotkey_state() {
    let harness = Harness::new();
    let app = harness.app.handle();
    let ctrl = RawKey::ModLeft(humidi_tauri_lib::managers::hotkey_manager::Modifier::Ctrl);
    handle_source_key(app, HookSource::KeyboardDevices, ctrl, true, Platform::Linux, ONE_KEYBOARD_OPEN);
    handle_source_key(app, HookSource::KeyboardDevices, ctrl, false, Platform::Linux, ONE_KEYBOARD_OPEN);
    harness.start_binding();
    handle_source_key(app, HookSource::KeyboardDevices, RawKey::Char('g'), true, Platform::Linux, ONE_KEYBOARD_OPEN);
    assert_eq!(harness.emitted(), vec![pair("hotkey_bound_updated", "\"G\"")]);
}

#[test]
fn a_key_reported_by_the_keyboard_devices_while_none_is_open_is_ignored_on_every_system() {
    let harness = Harness::new();
    handle_device_key(harness.app.handle(), f6(), true);
    assert_eq!(harness.emitted(), vec![]);
}
