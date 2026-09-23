use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

use humidi_tauri_lib::commands::{
    compile_notes, compile_notes_from_sheet, compile_pedal, compile_pedal_and_play,
    parse_midi_structure, resume_from_save, start_playback, PedalData, TimelineData,
};
use humidi_tauri_lib::core::config::PlaybackConfig;
use humidi_tauri_lib::core::midi::KeyMapper;
use humidi_tauri_lib::core::pedal::model::PedalModel;
use humidi_tauri_lib::managers::config_manager::ConfigManager;
use humidi_tauri_lib::managers::hotkey_manager::HotkeyManager;
use humidi_tauri_lib::managers::theme_manager::ThemeManager;
use humidi_tauri_lib::state::{AppState, PlaybackSession};
use tauri::test::{mock_app, MockRuntime};
use tauri::Manager;

type MockHandle = tauri::AppHandle<MockRuntime>;
type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn poll_once<T>(fut: &mut BoxedFuture<T>) -> Poll<T> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    fut.as_mut().poll(&mut cx)
}

fn hermetic_state(dir: &Path) -> AppState {
    AppState {
        pedal_model: Mutex::new(None),
        parsed_tracks: Mutex::new(None),
        parsed_tempo_map: Mutex::new(None),
        loaded_pedal_count: Mutex::new(0),
        midi_pedal_events: Mutex::new(Vec::new()),
        playback_session: Mutex::new(PlaybackSession::default()),
        playback_handle: Mutex::new(None),
        hotkey_manager: Mutex::new(HotkeyManager::new()),
        config_manager: Mutex::new(ConfigManager {
            save_dir: dir.join("saves"),
            midi_dir: std::path::PathBuf::new(),
            config_dir: dir.to_path_buf(),
            config_path: dir.join("config.json"),
        }),
        app_config: Mutex::new(serde_json::json!({})),
        theme_manager: Mutex::new(ThemeManager::new()),
    }
}

struct MockHarness {
    app: tauri::App<MockRuntime>,
    _dir: tempfile::TempDir,
}

impl MockHarness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        app.manage(hermetic_state(dir.path()));
        MockHarness { app, _dir: dir }
    }

    fn handle(&self) -> MockHandle {
        self.app.handle().clone()
    }

    fn with_state<T>(&self, f: impl FnOnce(&AppState) -> T) -> T {
        let state = self.app.state::<AppState>();
        f(&state)
    }
}

struct LockHolder {
    release: mpsc::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}

impl LockHolder {
    fn release(self) {
        let _ = self.release.send(());
        self.thread.join().unwrap();
    }
}

fn hold<X: 'static>(handle: &MockHandle, select: fn(&AppState) -> &Mutex<X>) -> LockHolder {
    let (locked_tx, locked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let handle = handle.clone();
    let thread = std::thread::spawn(move || {
        let state = handle.state::<AppState>();
        let _guard = select(&state).lock().unwrap();
        locked_tx.send(()).unwrap();
        let _ = release_rx.recv_timeout(Duration::from_secs(5));
    });
    locked_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    LockHolder {
        release: release_tx,
        thread,
    }
}

fn finish_while_holding<T>(holder: LockHolder, mut fut: BoxedFuture<T>) -> T {
    assert!(
        poll_once(&mut fut).is_pending(),
        "the command ran on the invoking thread instead of a worker thread"
    );
    holder.release();
    tauri::async_runtime::block_on(fut)
}

fn sheet_and_config() -> (PlaybackConfig, String) {
    let ch = KeyMapper::new(false).get_key_for_pitch(60).unwrap();
    let mut config = PlaybackConfig::default();
    config.pedal_style = "harmonic".to_string();
    (config, ch.to_string())
}

fn multi_note_sheet() -> String {
    let mapper = KeyMapper::new(false);
    [60, 62, 64, 65, 67, 69]
        .iter()
        .map(|p| mapper.get_key_for_pitch(*p).unwrap().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_temp_midi_single_note() -> tempfile::TempPath {
    use midly::num::{u15, u28, u4, u7};
    use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

    let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
    let track = vec![
        TrackEvent {
            delta: u28::from(0),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOn {
                    key: u7::from(60),
                    vel: u7::from(64),
                },
            },
        },
        TrackEvent {
            delta: u28::from(480),
            kind: TrackEventKind::Midi {
                channel: u4::from(0),
                message: MidiMessage::NoteOff {
                    key: u7::from(60),
                    vel: u7::from(0),
                },
            },
        },
    ];
    let smf = Smf {
        header,
        tracks: vec![track],
    };
    let mut buf = Vec::new();
    smf.write(&mut buf).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), &buf).unwrap();
    file.into_temp_path()
}

fn temp_midi_config(path: &tempfile::TempPath) -> PlaybackConfig {
    let mut config = PlaybackConfig::default();
    config.midi_file = path.to_string_lossy().to_string();
    config.pedal_style = "harmonic".to_string();
    config
}

fn write_valid_save(dir: &Path) -> String {
    let path = dir.join("save.json");
    let data = serde_json::json!({
        "metadata": {"track_details": [], "compiled_pedal_count": 0},
        "compiled_events": [
            {"time": 0.0, "priority": 2, "action": "press", "key_char": "a", "pitch": 60, "velocity": 64},
            {"time": 0.5, "priority": 4, "action": "release", "key_char": "a", "pitch": 60, "velocity": null},
        ],
    });
    std::fs::write(&path, data.to_string()).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn parse_compile_notes_and_compile_pedal_round_trip_through_the_async_commands() {
    let harness = MockHarness::new();
    let midi = write_temp_midi_single_note();
    let config = temp_midi_config(&midi);

    let parsed = tauri::async_runtime::block_on(parse_midi_structure(
        harness.handle(),
        config.midi_file.clone(),
    ))
    .unwrap();
    assert_eq!(parsed.tracks.len(), 1);
    assert!(harness.with_state(|s| s.parsed_tracks.lock().unwrap().is_some()));

    let timeline = tauri::async_runtime::block_on(compile_notes(
        harness.handle(),
        config.clone(),
        vec![(0, "Right Hand".to_string())],
    ))
    .unwrap();
    assert_eq!(timeline.final_notes.len(), 1);

    let pedal = tauri::async_runtime::block_on(compile_pedal(harness.handle(), config)).unwrap();
    assert!(pedal.ai_thresholds.is_none());
    assert!(harness.with_state(|s| s.playback_session.lock().unwrap().merged_events.is_some()));
}

#[test]
fn sheet_compile_then_pedal_compile_populates_the_session() {
    let harness = MockHarness::new();
    let (config, sheet) = sheet_and_config();

    let timeline = tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        config.clone(),
        sheet,
        120.0,
    ))
    .unwrap();
    assert_eq!(timeline.final_notes.len(), 1);

    tauri::async_runtime::block_on(compile_pedal(harness.handle(), config)).unwrap();
    harness.with_state(|s| {
        let session = s.playback_session.lock().unwrap();
        assert!(session.merged_events.is_some());
        assert!(session.pedal_config_snapshot.is_some());
    });
}

#[test]
fn errors_from_the_logic_layer_survive_the_worker_hop_unchanged() {
    let harness = MockHarness::new();
    let (config, _) = sheet_and_config();

    let pedal = tauri::async_runtime::block_on(compile_pedal(harness.handle(), config));
    assert_eq!(pedal.unwrap_err(), "Notes must be compiled before pedal.");

    let parse = tauri::async_runtime::block_on(parse_midi_structure(
        harness.handle(),
        "definitely-not-a-real-file.mid".to_string(),
    ));
    assert!(parse.is_err());

    let sheet = tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        PlaybackConfig::default(),
        "a".to_string(),
        0.0,
    ));
    assert_eq!(sheet.unwrap_err(), "BPM must be a positive, finite number.");
}

#[test]
fn compile_notes_does_not_block_the_invoker_while_the_session_is_locked() {
    let harness = MockHarness::new();
    let midi = write_temp_midi_single_note();
    let config = temp_midi_config(&midi);
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result = finish_while_holding(
        holder,
        Box::pin(compile_notes(
            harness.handle(),
            config,
            vec![(0, "Right Hand".to_string())],
        )),
    );
    assert_eq!(result.unwrap().final_notes.len(), 1);
}

#[test]
fn compile_notes_from_sheet_does_not_block_the_invoker_while_the_session_is_locked() {
    let harness = MockHarness::new();
    let (config, sheet) = sheet_and_config();
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result = finish_while_holding(
        holder,
        Box::pin(compile_notes_from_sheet(harness.handle(), config, sheet, 120.0)),
    );
    assert_eq!(result.unwrap().final_notes.len(), 1);
}

#[test]
fn compile_pedal_does_not_block_the_invoker_while_the_session_is_locked() {
    let harness = MockHarness::new();
    let (config, sheet) = sheet_and_config();
    tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        config.clone(),
        sheet,
        120.0,
    ))
    .unwrap();
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result = finish_while_holding(holder, Box::pin(compile_pedal(harness.handle(), config)));
    assert!(result.is_ok());
}

#[test]
fn compile_pedal_and_play_does_not_block_the_invoker_and_never_reaches_playback_without_notes() {
    let harness = MockHarness::new();
    let (config, _) = sheet_and_config();
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result = finish_while_holding(
        holder,
        Box::pin(compile_pedal_and_play(harness.handle(), config)),
    );
    assert_eq!(result.unwrap_err(), "Notes must be compiled before pedal.");
    assert!(harness.with_state(|s| s.playback_handle.lock().unwrap().is_none()));
}

#[test]
fn start_playback_does_not_block_the_invoker_while_the_session_is_locked() {
    let harness = MockHarness::new();
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result = finish_while_holding(
        holder,
        Box::pin(start_playback(harness.handle(), PlaybackConfig::default())),
    );
    assert_eq!(
        result.unwrap_err(),
        "Pedal must be compiled before playback can start."
    );
    assert!(harness.with_state(|s| s.playback_handle.lock().unwrap().is_none()));
}

#[test]
fn resume_from_save_does_not_block_the_invoker_while_the_session_is_locked() {
    let harness = MockHarness::new();
    let save_dir = tempfile::tempdir().unwrap();
    let save_path = write_valid_save(save_dir.path());
    let holder = hold(&harness.handle(), |s| &s.playback_session);

    let result =
        finish_while_holding(holder, Box::pin(resume_from_save(harness.handle(), save_path)));
    assert_eq!(result.unwrap().final_notes.len(), 1);
    assert!(harness.with_state(|s| s.playback_session.lock().unwrap().merged_events.is_some()));
}

#[test]
fn parse_midi_structure_does_not_block_the_invoker_while_its_state_is_locked() {
    let harness = MockHarness::new();
    let midi = write_temp_midi_single_note();
    let holder = hold(&harness.handle(), |s| &s.parsed_tracks);

    let result = finish_while_holding(
        holder,
        Box::pin(parse_midi_structure(
            harness.handle(),
            midi.to_string_lossy().to_string(),
        )),
    );
    assert_eq!(result.unwrap().tracks.len(), 1);
}

#[test]
fn queued_commands_all_wait_for_the_lock_holder_and_then_complete() {
    let harness = MockHarness::new();
    let (config, sheet) = sheet_and_config();
    tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        config.clone(),
        sheet.clone(),
        120.0,
    ))
    .unwrap();

    let holder = hold(&harness.handle(), |s| &s.playback_session);
    let mut pedal: BoxedFuture<Result<PedalData, String>> =
        Box::pin(compile_pedal(harness.handle(), config.clone()));
    assert!(poll_once(&mut pedal).is_pending());
    let mut notes: BoxedFuture<Result<TimelineData, String>> = Box::pin(compile_notes_from_sheet(
        harness.handle(),
        config,
        sheet,
        120.0,
    ));
    assert!(poll_once(&mut notes).is_pending());
    holder.release();

    let (pedal, notes) = tauri::async_runtime::block_on(async {
        let pedal = tauri::async_runtime::spawn(pedal);
        let notes = tauri::async_runtime::spawn(notes);
        (pedal.await.unwrap(), notes.await.unwrap())
    });
    assert!(pedal.is_ok());
    assert!(notes.is_ok());
}

#[test]
fn overlapping_compiles_all_finish_and_leave_a_consistent_session() {
    let harness = MockHarness::new();
    let (config, sheet) = sheet_and_config();
    tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        config.clone(),
        sheet.clone(),
        120.0,
    ))
    .unwrap();

    let (done_tx, done_rx) = mpsc::channel();
    let workers: Vec<_> = (0..16)
        .map(|i| {
            let handle = harness.handle();
            let config = config.clone();
            let sheet = sheet.clone();
            let done_tx = done_tx.clone();
            std::thread::spawn(move || {
                let outcome = if i % 2 == 0 {
                    tauri::async_runtime::block_on(compile_pedal(handle, config)).map(|_| ())
                } else {
                    tauri::async_runtime::block_on(compile_notes_from_sheet(
                        handle, config, sheet, 120.0,
                    ))
                    .map(|_| ())
                };
                done_tx.send(outcome).unwrap();
            })
        })
        .collect();

    for _ in 0..16 {
        let outcome = done_rx
            .recv_timeout(Duration::from_secs(30))
            .expect("a command never finished, which points at a deadlock");
        assert_eq!(outcome, Ok(()));
    }
    for w in workers {
        w.join().unwrap();
    }

    harness.with_state(|s| {
        let session = s.playback_session.lock().unwrap();
        assert!(session.notes_config_snapshot.is_some());
        assert_eq!(session.pedal_events.is_some(), session.merged_events.is_some());
        assert_eq!(
            session.pedal_events.is_some(),
            session.pedal_config_snapshot.is_some()
        );
    });
}

#[test]
fn an_ai_compile_never_holds_the_model_mutex_while_it_runs() {
    let harness = MockHarness::new();
    let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("pedal_bilstm.safetensors");
    harness.with_state(|s| {
        *s.pedal_model.lock().unwrap() = Some(Arc::new(PedalModel::load(model_path).unwrap()));
    });

    let mut config = PlaybackConfig::default();
    config.pedal_style = "ai".to_string();
    tauri::async_runtime::block_on(compile_notes_from_sheet(
        harness.handle(),
        config.clone(),
        multi_note_sheet(),
        120.0,
    ))
    .unwrap();

    let (done_tx, done_rx) = mpsc::channel();
    let handle = harness.handle();
    let worker = std::thread::spawn(move || {
        let result = tauri::async_runtime::block_on(compile_pedal(handle, config));
        done_tx.send(result.map(|_| ())).unwrap();
    });

    let started = Instant::now();
    let mut probes = 0u32;
    let mut blocked_since: Option<Instant> = None;
    let outcome = loop {
        if let Ok(outcome) = done_rx.try_recv() {
            break outcome;
        }
        assert!(
            started.elapsed() < Duration::from_secs(120),
            "the compile never finished"
        );
        let free = harness.with_state(|s| s.pedal_model.try_lock().is_ok());
        probes += 1;
        if free {
            blocked_since = None;
        } else {
            let since = *blocked_since.get_or_insert_with(Instant::now);
            assert!(
                since.elapsed() < Duration::from_millis(250),
                "the model mutex stayed locked while the compile ran"
            );
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    worker.join().unwrap();
    assert_eq!(outcome, Ok(()));
    assert!(
        probes >= 10,
        "the compile finished after only {probes} probes, too quickly to prove anything"
    );
}
