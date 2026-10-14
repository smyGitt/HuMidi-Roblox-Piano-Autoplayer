use humidi_tauri_lib::managers::config_manager::ConfigManager;
use humidi_tauri_lib::state::AppState;
use humidi_tauri_lib::invoke_handler;
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

const NO_SUCH_COMMAND: &str = "no_such_command";

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
