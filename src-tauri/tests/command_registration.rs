use humidi_tauri_lib::managers::config_manager::ConfigManager;
use humidi_tauri_lib::state::AppState;
use humidi_tauri_lib::invoke_handler;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

const NO_SUCH_COMMAND: &str = "no_such_command";
const TICKS_PER_BEAT: u16 = 480;
const NOTE_PITCH: u8 = 60;
const NOTE_VELOCITY: u8 = 64;
const SAVED_TEMPO: f64 = 0.8;

struct Harness {
    _app: App<tauri::test::MockRuntime>,
    window: WebviewWindow<tauri::test::MockRuntime>,
    _dir: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_builder()
            .invoke_handler(invoke_handler())
            .manage(AppState::from_config_manager(ConfigManager {
                save_dir: dir.path().join("saves"),
                midi_dir: std::path::PathBuf::new(),
                config_dir: dir.path().to_path_buf(),
                config_path: dir.path().join("config.json"),
            }))
            .build(mock_context(noop_assets()))
            .unwrap();
        let window = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        Harness {
            _app: app,
            window,
            _dir: dir,
        }
    }

    fn invoke(&self, cmd: &str, body: Value) -> Result<Value, Value> {
        let request = InvokeRequest {
            cmd: cmd.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: self.window.url().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        };
        get_ipc_response(&self.window, request).map(|response| match response {
            InvokeResponseBody::Json(text) => serde_json::from_str(&text).unwrap(),
            InvokeResponseBody::Raw(_) => Value::Null,
        })
    }
}

fn is_not_found(error: &Value) -> bool {
    error.to_string().contains("not found")
}

#[test]
fn a_command_that_does_not_exist_is_refused_as_not_found() {
    let harness = Harness::new();
    let error = harness.invoke(NO_SUCH_COMMAND, json!({})).unwrap_err();
    assert!(is_not_found(&error), "{error}");
}

#[test]
fn forward_key_is_registered_and_takes_code_and_pressed_the_way_the_frontend_sends_them() {
    let harness = Harness::new();
    harness.invoke("start_binding", json!({})).unwrap();
    harness
        .invoke("forward_key", json!({ "code": "KeyG", "pressed": true }))
        .unwrap();
    assert_eq!(harness.invoke("get_hotkeys", json!({})).unwrap(), json!(["G", "Ctrl+S"]));
}

#[test]
fn forward_key_ignores_a_code_with_no_hotkey_meaning() {
    let harness = Harness::new();
    assert_eq!(
        harness.invoke("forward_key", json!({ "code": "NumpadEnter", "pressed": true })),
        Ok(Value::Null)
    );
    assert_eq!(harness.invoke("get_hotkeys", json!({})).unwrap(), json!(["F6", "Ctrl+S"]));
}

#[test]
fn forward_key_refuses_a_request_missing_its_arguments_without_calling_it_unknown() {
    let harness = Harness::new();
    let error = harness.invoke("forward_key", json!({})).unwrap_err();
    assert!(!is_not_found(&error), "{error}");
}

#[test]
fn the_other_hotkey_commands_are_registered() {
    let harness = Harness::new();
    assert_eq!(harness.invoke("start_save_binding", json!({})), Ok(Value::Null));
    harness
        .invoke("forward_key", json!({ "code": "KeyD", "pressed": true }))
        .unwrap();
    assert_eq!(harness.invoke("get_hotkeys", json!({})).unwrap(), json!(["F6", "D"]));
}

#[test]
fn start_playback_takes_a_start_time_and_still_requires_compiled_notes() {
    let harness = Harness::new();
    let error = harness
        .invoke("start_playback", json!({ "config": {}, "startTime": 4.0 }))
        .unwrap_err();
    assert!(error.to_string().contains("must be compiled"), "{error}");
}

#[test]
fn start_playback_refuses_a_request_without_a_start_time() {
    let harness = Harness::new();
    let error = harness.invoke("start_playback", json!({ "config": {} })).unwrap_err();
    assert!(error.to_string().contains("startTime"), "{error}");
}

fn write_midi_with_one_note(path: &std::path::Path) {
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let channel = u4::from(0);
    let track = vec![
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOn {
                    key: u7::from(NOTE_PITCH),
                    vel: u7::from(NOTE_VELOCITY),
                },
            },
        },
        TrackEvent {
            delta: u28::from(u32::from(TICKS_PER_BEAT)),
            kind: TrackEventKind::Midi {
                channel,
                message: MidiMessage::NoteOff {
                    key: u7::from(NOTE_PITCH),
                    vel: u7::from(0),
                },
            },
        },
    ];
    let smf = Smf {
        header: Header::new(Format::Parallel, Timing::Metrical(u15::from(TICKS_PER_BEAT))),
        tracks: vec![track],
    };
    let mut bytes = Vec::new();
    smf.write(&mut bytes).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn write_save_pointing_at(dir: &std::path::Path, midi: &std::path::Path) -> std::path::PathBuf {
    let save = json!({
        "metadata": {
            "playback_settings": {
                "midi_file": midi.to_string_lossy(),
                "tempo": SAVED_TEMPO,
                "pedal_style": "harmonic",
            },
            "track_details": [{ "index": 0, "name": "Track 0", "note_count": 1, "role": "Left Hand" }],
            "compiled_pedal_count": 0,
        },
        "compiled_events": [
            { "time": 0.0, "priority": 2, "action": "press", "key_char": "a", "pitch": NOTE_PITCH, "velocity": NOTE_VELOCITY },
        ],
    });
    let path = dir.join("song.json");
    std::fs::write(&path, save.to_string()).unwrap();
    path
}

#[test]
fn load_save_for_edit_returns_the_plan_the_frontend_reads() {
    let harness = Harness::new();
    let midi = harness._dir.path().join("song.mid");
    write_midi_with_one_note(&midi);
    let save = write_save_pointing_at(harness._dir.path(), &midi);

    let plan = harness
        .invoke("load_save_for_edit", json!({ "filepath": save.to_string_lossy() }))
        .unwrap();

    assert_eq!(plan["midi_file"], json!(midi.to_string_lossy()));
    assert_eq!(plan["settings"]["tempo"], json!(SAVED_TEMPO));
    assert_eq!(plan["settings"]["pedal_style"], json!("harmonic"));
    assert_eq!(
        plan["tracks"],
        json!([{ "index": 0, "role": "Left Hand", "name": "Track 0", "note_count": 1 }])
    );
    assert_eq!(plan["legacy_pedal_thresholds"], json!(false));
}

#[test]
fn load_save_for_edit_explains_a_save_whose_midi_file_has_moved() {
    let harness = Harness::new();
    let missing = harness._dir.path().join("gone.mid");
    let save = write_save_pointing_at(harness._dir.path(), &missing);

    let error = harness
        .invoke("load_save_for_edit", json!({ "filepath": save.to_string_lossy() }))
        .unwrap_err();

    assert!(error.to_string().contains("gone.mid"), "{error}");
    assert!(error.to_string().contains("was not found"), "{error}");
}

#[test]
fn load_save_for_edit_refuses_a_request_without_its_filepath_without_calling_it_unknown() {
    let harness = Harness::new();
    let error = harness.invoke("load_save_for_edit", json!({})).unwrap_err();
    assert!(error.to_string().contains("filepath"), "{error}");
}
