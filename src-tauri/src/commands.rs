use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::core::atomic_write::write_json_atomic;
use crate::core::compiler::{compile_note_events_with_played, compile_pedal_events, merge_compiled};
use crate::core::config::PlaybackConfig;
use crate::core::input_scan::scan_events;
use crate::core::keyboard_driver::{EnigoDriver, KeyboardDriver, Platform};
use crate::core::midi::{KeyMapper, MidiParser, TempoMap};
use crate::core::models::{KeyEvent, MidiTrack, Note};
use crate::core::pedal::model::{PedalModel, FEATURES};
use crate::core::pedal::curve::ThresholdNode;
use crate::core::pedal::legacy::saved_pedal_curves;
use crate::core::pedal::FPS as PEDAL_CONFIDENCE_FPS;
use crate::core::player::{PlaybackEngine, PlayerEvent};
use crate::core::section_analyzer::{assign_hands, SectionAnalyzer};
use crate::core::session_cache;
use crate::core::translator::VirtualPianoFormat;
use crate::core::uinput_driver::{driver_or_explained_fallback, SharedSink};
use crate::managers::hotkey_manager::{
    dom_code_to_raw, rdev_key_to_raw, HotkeyEvent, RawKey, PLAYBACK_HOTKEY_CONFIG_KEY,
    SAVE_HOTKEY_CONFIG_KEY,
};
use crate::managers::injected_echo::EchoTrackingDriver;
use crate::state::{AppState, PlaybackCommand, PlaybackHandle, PlaybackSession};
use tauri::{path::BaseDirectory, Emitter, Manager, State};

async fn run_blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(work).await {
        Ok(result) => result,
        Err(e) => Err(format!("Background task failed: {e}")),
    }
}

fn pedal_resource_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<std::path::PathBuf, String> {
    app.path()
        .resolve("resources/pedal_bilstm.safetensors", BaseDirectory::Resource)
        .map_err(|e| e.to_string())
}

fn load_pedal_model(
    state: &AppState,
    resolve_path: impl FnOnce() -> Result<std::path::PathBuf, String>,
) -> Result<Arc<PedalModel>, String> {
    let cached = state.pedal_model.lock().map_err(|e| e.to_string())?.clone();
    if let Some(model) = cached {
        return Ok(model);
    }
    let loaded = Arc::new(PedalModel::load(resolve_path()?).map_err(|e| e.to_string())?);
    let mut guard = state.pedal_model.lock().map_err(|e| e.to_string())?;
    Ok(Arc::clone(guard.get_or_insert(loaded)))
}

fn pedal_model_for(
    state: &AppState,
    config: &PlaybackConfig,
    resolve_path: impl FnOnce() -> Result<std::path::PathBuf, String>,
) -> Result<Option<Arc<PedalModel>>, String> {
    if matches!(config.pedal_style.as_str(), "ai" | "hybrid") {
        load_pedal_model(state, resolve_path).map(Some)
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub fn run_pedal_smoketest<R: tauri::Runtime>(
    state: State<AppState>,
    app: tauri::AppHandle<R>,
) -> Result<Vec<f32>, String> {
    let model = load_pedal_model(&state, || pedal_resource_path(&app))?;

    let t_len = 50usize;
    let seq = vec![0.0f32; t_len * FEATURES];
    let preds = model.forward(&seq, t_len).map_err(|e| e.to_string())?;
    Ok(preds.into_iter().take(10).collect())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrackSummary {
    pub index: i32,
    pub name: String,
    pub note_count: usize,
    pub instrument_name: String,
    pub is_drum: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParsedMidiStructure {
    pub tracks: Vec<TrackSummary>,
    pub initial_bpm: f64,
}

pub fn parse_midi_structure_logic(
    filepath: &str,
) -> io::Result<(Vec<MidiTrack>, TempoMap, u32, Vec<(f64, bool)>, ParsedMidiStructure)> {
    let (tracks, tempo_map, pedal_count, midi_pedal_events) =
        MidiParser::parse_structure(filepath, 1.0)?;

    let summaries: Vec<TrackSummary> = tracks
        .iter()
        .map(|t| TrackSummary {
            index: t.index,
            name: t.name.clone(),
            note_count: t.note_count(),
            instrument_name: t.instrument_name(),
            is_drum: t.is_drum,
        })
        .collect();
    let initial_bpm = tempo_map.initial_bpm();

    Ok((
        tracks,
        tempo_map,
        pedal_count,
        midi_pedal_events,
        ParsedMidiStructure {
            tracks: summaries,
            initial_bpm,
        },
    ))
}

#[tauri::command]
pub async fn parse_midi_structure<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    filepath: String,
) -> Result<ParsedMidiStructure, String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let (tracks, tempo_map, pedal_count, midi_pedal_events, result) =
            parse_midi_structure_logic(&filepath).map_err(|e| e.to_string())?;

        *state.parsed_tracks.lock().map_err(|e| e.to_string())? = Some(tracks);
        *state.parsed_tempo_map.lock().map_err(|e| e.to_string())? = Some(tempo_map);
        *state.loaded_pedal_count.lock().map_err(|e| e.to_string())? = pedal_count;
        *state.midi_pedal_events.lock().map_err(|e| e.to_string())? = midi_pedal_events;

        Ok(result)
    })
    .await
}

pub fn extract_pedal_intervals(events: &[KeyEvent]) -> Vec<(f64, f64)> {
    let mut intervals = Vec::new();
    let mut down_time: Option<f64> = None;
    let mut pedal_events: Vec<&KeyEvent> = events.iter().filter(|ev| ev.action == "pedal").collect();
    pedal_events.sort_by(|a, b| a.cmp(b));
    for ev in pedal_events {
        match ev.key_char.as_str() {
            "down" => {
                if down_time.is_none() {
                    down_time = Some(ev.time);
                }
            }
            "up" => {
                if let Some(start) = down_time {
                    intervals.push((start, ev.time));
                    down_time = None;
                }
            }
            _ => {}
        }
    }
    intervals
}

pub fn apply_hand_assignment(notes: &mut [Note], config: &PlaybackConfig) {
    if config.simulate_hands {
        assign_hands(notes);
    } else {
        for note in notes.iter_mut() {
            if note.hand == "unknown" {
                note.hand = if note.pitch < 60 { "left" } else { "right" }.to_string();
            }
        }
    }
}

fn assign_track_roles(
    tracks: &[MidiTrack],
    selected_tracks_info: &[(i32, String)],
) -> Vec<Note> {
    let selected_indices: HashSet<i32> = selected_tracks_info.iter().map(|(idx, _)| *idx).collect();
    let role_map: HashMap<i32, &str> = selected_tracks_info
        .iter()
        .map(|(idx, role)| (*idx, role.as_str()))
        .collect();

    let mut final_notes: Vec<Note> = Vec::new();
    for track in tracks {
        if !selected_indices.contains(&track.index) {
            continue;
        }
        let role = role_map.get(&track.index).copied();
        for note in &track.notes {
            let mut new_note = note.clone();
            match role {
                Some("Left Hand") => new_note.hand = "left".to_string(),
                Some("Right Hand") => new_note.hand = "right".to_string(),
                _ => {}
            }
            final_notes.push(new_note);
        }
    }
    final_notes
}

fn validate_tempo_scale(tempo: f64) -> io::Result<()> {
    if !tempo.is_finite() || tempo <= 0.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Tempo multiplier must be a positive, finite number (got {tempo})"),
        ));
    }
    Ok(())
}

pub fn prepare_notes_from_bytes(
    data: &[u8],
    config: &PlaybackConfig,
    selected_tracks_info: &[(i32, String)],
) -> io::Result<(Vec<Note>, crate::core::midi::TempoMap, Vec<(f64, bool)>)> {
    validate_tempo_scale(config.tempo)?;
    let (tracks, tempo_map, _pedal_cc_count, midi_pedal_events) =
        MidiParser::parse_bytes(data, config.tempo)?;

    let mut final_notes = assign_track_roles(&tracks, selected_tracks_info);
    final_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    apply_hand_assignment(&mut final_notes, config);

    Ok((final_notes, tempo_map, midi_pedal_events))
}

pub fn prepare_notes(
    config: &PlaybackConfig,
    selected_tracks_info: &[(i32, String)],
) -> io::Result<(Vec<Note>, crate::core::midi::TempoMap, Vec<(f64, bool)>)> {
    validate_tempo_scale(config.tempo)?;
    let (tracks, tempo_map, _pedal_cc_count, midi_pedal_events) =
        MidiParser::parse_structure(&config.midi_file, config.tempo)?;

    let mut final_notes = assign_track_roles(&tracks, selected_tracks_info);
    final_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    apply_hand_assignment(&mut final_notes, config);

    Ok((final_notes, tempo_map, midi_pedal_events))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineData {
    pub final_notes: Vec<Note>,
    pub played_notes: Vec<Note>,
    pub total_dur: f64,
    pub tempo_events: Vec<(f64, u32)>,
    pub time_signatures: Vec<(f64, u8, u8)>,
    pub measure_boundaries: Vec<(f64, f64)>,
}

type TempoFields = (Vec<(f64, u32)>, Vec<(f64, u8, u8)>, Vec<(f64, f64)>);

fn timeline_tempo_fields(tempo_map: &TempoMap, total_dur: f64) -> TempoFields {
    (
        tempo_map.events().to_vec(),
        tempo_map.time_signatures().to_vec(),
        tempo_map.get_measure_boundaries(total_dur),
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PedalData {
    pub pedal_intervals: Vec<(f64, f64)>,
    pub curve_on: Vec<ThresholdNode>,
    pub curve_off: Vec<ThresholdNode>,
    pub confidence: Vec<f32>,
    pub confidence_fps: f64,
}

fn compute_total_dur(final_notes: &[Note]) -> f64 {
    if final_notes.is_empty() {
        1.0
    } else {
        final_notes
            .iter()
            .map(|n| n.end_time())
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

pub fn compile_notes_logic(
    config: &PlaybackConfig,
    selected_tracks_info: &[(i32, String)],
    session: &mut PlaybackSession,
) -> io::Result<TimelineData> {
    let (final_notes, tempo_map, midi_pedal_events) = prepare_notes(config, selected_tracks_info)?;
    let sections = SectionAnalyzer::new(final_notes.clone(), &tempo_map).analyze();
    let (note_events, humanized_notes, played_notes) =
        compile_note_events_with_played(config, &final_notes, &sections);
    scan_events(&note_events).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let total_dur = compute_total_dur(&final_notes);
    let (tempo_events, time_signatures, measure_boundaries) =
        timeline_tempo_fields(&tempo_map, total_dur);

    session.notes_config_snapshot = Some(session_cache::extract_notes_config(config));
    session.final_notes = Some(final_notes.clone());
    session.humanized_notes = Some(humanized_notes);
    session.note_events = Some(note_events);
    session.tempo_map = Some(tempo_map);
    session.total_dur = total_dur;
    session.midi_pedal_events = midi_pedal_events;
    session.pedal_events = None;
    session.merged_events = None;
    session.pedal_config_snapshot = None;

    Ok(TimelineData {
        final_notes,
        played_notes,
        total_dur,
        tempo_events,
        time_signatures,
        measure_boundaries,
    })
}

#[tauri::command]
pub async fn compile_notes<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
    selected_tracks_info: Vec<(i32, String)>,
) -> Result<TimelineData, String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let mut session = state.playback_session.lock().map_err(|e| e.to_string())?;
        compile_notes_logic(&config, &selected_tracks_info, &mut session).map_err(|e| e.to_string())
    })
    .await
}

pub fn compile_notes_from_notes_logic(
    config: &PlaybackConfig,
    mut final_notes: Vec<Note>,
    tempo_map: TempoMap,
    session: &mut PlaybackSession,
) -> TimelineData {
    final_notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
    apply_hand_assignment(&mut final_notes, config);

    let sections = SectionAnalyzer::new(final_notes.clone(), &tempo_map).analyze();
    let (note_events, humanized_notes, played_notes) =
        compile_note_events_with_played(config, &final_notes, &sections);
    let total_dur = compute_total_dur(&final_notes);
    let (tempo_events, time_signatures, measure_boundaries) =
        timeline_tempo_fields(&tempo_map, total_dur);

    session.notes_config_snapshot = Some(session_cache::extract_notes_config(config));
    session.final_notes = Some(final_notes.clone());
    session.humanized_notes = Some(humanized_notes);
    session.note_events = Some(note_events);
    session.tempo_map = Some(tempo_map);
    session.total_dur = total_dur;
    session.midi_pedal_events = Vec::new();
    session.pedal_events = None;
    session.merged_events = None;
    session.pedal_config_snapshot = None;

    TimelineData {
        final_notes,
        played_notes,
        total_dur,
        tempo_events,
        time_signatures,
        measure_boundaries,
    }
}

pub fn compile_notes_from_sheet_logic(
    config: &PlaybackConfig,
    sheet_text: &str,
    bpm: f64,
    session: &mut PlaybackSession,
) -> Result<TimelineData, String> {
    if !bpm.is_finite() || bpm <= 0.0 {
        return Err("BPM must be a positive, finite number.".to_string());
    }

    let key_mapper = KeyMapper::new(config.use_88_key_layout);
    let notes = VirtualPianoFormat::parse(sheet_text, bpm, &key_mapper);
    if notes.is_empty() {
        return Err(
            "Sheet produced zero playable notes -- verify the sheet contains mappable \
             characters within the selected keyboard layout."
                .to_string(),
        );
    }

    let tempo_us = (60_000_000.0 / bpm).round().clamp(1.0, u32::MAX as f64) as u32;
    let tempo_map = TempoMap::new(vec![(0.0, tempo_us)], vec![]);

    Ok(compile_notes_from_notes_logic(config, notes, tempo_map, session))
}

#[tauri::command]
pub async fn compile_notes_from_sheet<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
    sheet_text: String,
    bpm: f64,
) -> Result<TimelineData, String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let mut session = state.playback_session.lock().map_err(|e| e.to_string())?;
        compile_notes_from_sheet_logic(&config, &sheet_text, bpm, &mut session)
    })
    .await
}

pub fn compile_pedal_logic(
    config: &PlaybackConfig,
    model: Option<&PedalModel>,
    session: &mut PlaybackSession,
) -> Result<PedalData, String> {
    if session.notes_config_snapshot.is_none() {
        return Err("Notes must be compiled before pedal.".to_string());
    }
    let final_notes = session
        .final_notes
        .clone()
        .ok_or("Missing compiled notes.")?;
    let humanized_notes = session
        .humanized_notes
        .clone()
        .ok_or("Missing compiled notes.")?;
    let note_events = session.note_events.clone().ok_or("Missing compiled notes.")?;
    let tempo_map = session.tempo_map.clone().ok_or("Missing compiled notes.")?;
    let midi_pedal_events = session.midi_pedal_events.clone();
    let total_dur = session.total_dur;

    let pedal_notes = if config.pedal_before_humanize {
        final_notes.clone()
    } else {
        let mut notes = humanized_notes.clone();
        notes.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
        notes
    };
    let sections = SectionAnalyzer::new(pedal_notes.clone(), &tempo_map).analyze();
    let midi_pedal_opt = if midi_pedal_events.is_empty() {
        None
    } else {
        Some(midi_pedal_events.as_slice())
    };
    let (pedal_events, ai_meta) = compile_pedal_events(
        model,
        config,
        &pedal_notes,
        &sections,
        midi_pedal_opt,
    );
    let merged_events = merge_compiled(&note_events, &pedal_events);
    let pedal_intervals = extract_pedal_intervals(&pedal_events);

    session.pedal_events = Some(pedal_events.clone());
    session.merged_events = Some(merged_events);
    session.pedal_config_snapshot = Some(session_cache::extract_pedal_config(config));

    session_cache::write_cache(
        session_cache::extract_notes_config(config),
        session_cache::extract_pedal_config(config),
        &note_events,
        &pedal_events,
        &humanized_notes,
        &final_notes,
        session_cache::tempo_map_to_dict(&tempo_map),
        total_dur,
    );

    let (curve_on, curve_off, confidence) = match ai_meta {
        Some(t) => (t.curves.on, t.curves.off, t.confidence),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    Ok(PedalData {
        pedal_intervals,
        curve_on,
        curve_off,
        confidence,
        confidence_fps: PEDAL_CONFIDENCE_FPS,
    })
}

fn compile_pedal_blocking<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    config: &PlaybackConfig,
) -> Result<PedalData, String> {
    let model = pedal_model_for(state, config, || pedal_resource_path(app))?;
    let mut session = state.playback_session.lock().map_err(|e| e.to_string())?;
    compile_pedal_logic(config, model.as_deref(), &mut session)
}

#[tauri::command]
pub async fn compile_pedal<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
) -> Result<PedalData, String> {
    run_blocking(move || compile_pedal_blocking(&app.state::<AppState>(), &app, &config)).await
}

#[tauri::command]
pub async fn compile_pedal_and_play<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
) -> Result<PedalData, String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let data = compile_pedal_blocking(&state, &app, &config)?;
        start_playback_blocking(&state, &app, config, 0.0)?;
        Ok(data)
    })
    .await
}

fn event_to_channel_payload(event: &PlayerEvent) -> (&'static str, serde_json::Value) {
    match event {
        PlayerEvent::Status(s) => ("status_updated", serde_json::json!(s)),
        PlayerEvent::Progress(p) => ("progress_updated", serde_json::json!(p)),
        PlayerEvent::Finished => ("playback_finished", serde_json::Value::Null),
        PlayerEvent::Visualizer(v) => ("visualizer_updated", serde_json::json!(v)),
        PlayerEvent::Pedal(b) => ("pedal_updated", serde_json::json!(b)),
        PlayerEvent::AutoPaused => ("auto_paused", serde_json::Value::Null),
        PlayerEvent::Error(e) => ("error_occurred", serde_json::json!(e)),
        PlayerEvent::Section(idx) => ("section_changed", serde_json::json!(idx)),
    }
}

pub fn run_playback_loop(
    mut engine: PlaybackEngine,
    driver: &mut dyn KeyboardDriver,
    cmd_rx: &mpsc::Receiver<PlaybackCommand>,
    mut emit: impl FnMut(&PlayerEvent),
    mut now_fn: impl FnMut() -> f64,
    mut sleep_fn: impl FnMut(f64),
) {
    if engine.should_run_countdown() {
        for msg in PlaybackEngine::countdown_messages() {
            emit(&PlayerEvent::Status(msg));
            sleep_fn(1.0);
        }
    }

    if engine.should_release_pedal_on_start() {
        engine.release_pedal(driver);
    }
    engine.start(now_fn());
    let mut finished_emitted = false;

    loop {
        let outcome = engine.tick(driver, now_fn());
        for ev in &outcome.events {
            if matches!(ev, PlayerEvent::Finished) {
                finished_emitted = true;
            }
            match ev {
                PlayerEvent::Progress(_) => emit(&PlayerEvent::Progress(engine.current_time(now_fn()))),
                _ => emit(ev),
            }
        }
        if finished_emitted {
            break;
        }

        match cmd_rx.try_recv() {
            Ok(PlaybackCommand::TogglePause) => {
                for ev in engine.toggle_pause(now_fn()) {
                    emit(&ev);
                }
            }
            Ok(PlaybackCommand::Seek(target)) => engine.seek(target),
            Ok(PlaybackCommand::Stop) => {
                for ev in engine.stop() {
                    emit(&ev);
                }
                break;
            }
            Ok(PlaybackCommand::Shutdown) => break,
            Err(_) => {}
        }

        sleep_fn(outcome.suggested_sleep);
    }

    for ev in engine.shutdown(driver) {
        emit(&ev);
    }
    emit(&PlayerEvent::Visualizer(Vec::new()));
    emit(&PlayerEvent::Pedal(false));
    if !finished_emitted {
        emit(&PlayerEvent::Finished);
    }
}

#[cfg(target_os = "linux")]
const UINPUT_SETTLE: Duration = Duration::from_secs(1);

#[cfg(target_os = "linux")]
fn platform_uinput_sink(state: &AppState) -> Option<SharedSink> {
    crate::core::uinput_driver::shared_sink_or_open(
        &state.uinput_sink,
        crate::core::uinput_device::open,
        || std::thread::sleep(UINPUT_SETTLE),
    )
}

#[cfg(not(target_os = "linux"))]
fn platform_uinput_sink(_state: &AppState) -> Option<SharedSink> {
    None
}

pub fn engine_starting_at(
    config: PlaybackConfig,
    events: Vec<KeyEvent>,
    total_duration: f64,
    start_time: f64,
) -> PlaybackEngine {
    let mut engine = PlaybackEngine::new(config, Vec::new());
    engine.load_compiled_events(events, total_duration);
    if start_time > 0.0 {
        engine.seek(start_time);
    }
    engine
}

fn start_playback_blocking<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    config: PlaybackConfig,
    start_time: f64,
) -> Result<(), String> {
    let (merged_events, total_dur) = {
        let session = state.playback_session.lock().map_err(|e| e.to_string())?;
        let merged_events = session
            .merged_events
            .clone()
            .ok_or("Pedal must be compiled before playback can start.")?;
        scan_events(&merged_events)?;
        session
            .tempo_map
            .as_ref()
            .ok_or("Notes must be compiled before playback can start.")?;
        let total_dur = merged_events
            .last()
            .map(|e| e.time)
            .unwrap_or(session.total_dur);
        (merged_events, total_dur)
    };

    let mut driver = EchoTrackingDriver::new(
        driver_or_explained_fallback(Platform::current(), platform_uinput_sink(state), EnigoDriver::new)?,
        Arc::clone(&state.injected_echoes),
    );
    let engine = engine_starting_at(config, merged_events, total_dur, start_time);

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let app_for_thread = app.clone();
    let app_for_cleanup = app.clone();

    let thread = std::thread::spawn(move || {
        let start_instant = Instant::now();
        run_playback_loop(
            engine,
            &mut driver,
            &cmd_rx,
            |ev| {
                let (channel, payload) = event_to_channel_payload(ev);
                let _ = app_for_thread.emit(channel, payload);
            },
            || start_instant.elapsed().as_secs_f64(),
            |secs| {
                if secs > 0.0 {
                    std::thread::sleep(Duration::from_secs_f64(secs));
                }
            },
        );
    });

    *state.playback_handle.lock().map_err(|e| e.to_string())? = Some(PlaybackHandle {
        cmd_tx,
        thread,
    });

    let _ = app_for_cleanup.emit("playback_started", ());
    Ok(())
}

#[tauri::command]
pub async fn start_playback<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
    start_time: f64,
) -> Result<(), String> {
    run_blocking(move || start_playback_blocking(&app.state::<AppState>(), &app, config, start_time)).await
}

#[tauri::command]
pub fn toggle_pause(state: State<AppState>) -> Result<(), String> {
    let guard = state.playback_handle.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.as_ref() {
        let _ = handle.cmd_tx.send(PlaybackCommand::TogglePause);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_playback(state: State<AppState>) -> Result<(), String> {
    let guard = state.playback_handle.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.as_ref() {
        let _ = handle.cmd_tx.send(PlaybackCommand::Stop);
    }
    Ok(())
}

#[tauri::command]
pub fn seek_playback(state: State<AppState>, target_time: f64) -> Result<(), String> {
    let guard = state.playback_handle.lock().map_err(|e| e.to_string())?;
    if let Some(handle) = guard.as_ref() {
        let _ = handle.cmd_tx.send(PlaybackCommand::Seek(target_time));
    }
    Ok(())
}

#[tauri::command]
pub fn set_save_dir(state: State<AppState>, path: String) -> Result<(), String> {
    let mut config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    config_manager.set_save_dir(std::path::PathBuf::from(path));
    let app_config = state.app_config.lock().map_err(|e| e.to_string())?;
    config_manager.save(&app_config);
    Ok(())
}

#[tauri::command]
pub fn get_save_dir(state: State<AppState>) -> Result<String, String> {
    let config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    Ok(config_manager.save_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_midi_dir(state: State<AppState>, path: String) -> Result<(), String> {
    let mut config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    config_manager.set_midi_dir(std::path::PathBuf::from(path));
    let app_config = state.app_config.lock().map_err(|e| e.to_string())?;
    config_manager.save(&app_config);
    Ok(())
}

#[tauri::command]
pub fn get_midi_dir(state: State<AppState>) -> Result<String, String> {
    let config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    Ok(config_manager.midi_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_app_config(state: State<AppState>) -> Result<Value, String> {
    let app_config = state.app_config.lock().map_err(|e| e.to_string())?;
    Ok(app_config.clone())
}

pub fn merge_app_config(current: &Value, patch: &Value) -> Value {
    match (current, patch) {
        (Value::Object(current_map), Value::Object(patch_map)) => {
            let mut merged = current_map.clone();
            for (k, v) in patch_map {
                merged.insert(k.clone(), v.clone());
            }
            Value::Object(merged)
        }
        _ => patch.clone(),
    }
}

pub fn patch_app_config(state: &AppState, patch: &Value) -> Result<(), String> {
    let mut app_config = state.app_config.lock().map_err(|e| e.to_string())?;
    *app_config = merge_app_config(&app_config, patch);
    let config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    config_manager.save(&app_config);
    Ok(())
}

#[tauri::command]
pub fn save_app_config(state: State<AppState>, patch: Value) -> Result<(), String> {
    patch_app_config(&state, &patch)
}

#[tauri::command]
pub fn get_themes_file(state: State<AppState>) -> Result<String, String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    Ok(theme_manager.themes_file().to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_themes_dir(state: State<AppState>, path: String) -> Result<String, String> {
    let mut theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    theme_manager.set_themes_dir(std::path::PathBuf::from(path));
    Ok(theme_manager.themes_file().to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_active_theme_name(state: State<AppState>) -> Result<Option<String>, String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    Ok(theme_manager.get_active_name())
}

#[tauri::command]
pub fn set_active_theme_name(state: State<AppState>, name: String) -> Result<(), String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    theme_manager.set_active_name(&name);
    Ok(())
}

#[tauri::command]
pub fn get_custom_themes(
    state: State<AppState>,
) -> Result<Vec<crate::managers::theme_manager::ThemeColors>, String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    Ok(theme_manager.custom_themes())
}

#[tauri::command]
pub fn save_custom_theme(
    state: State<AppState>,
    theme: crate::managers::theme_manager::ThemeColors,
) -> Result<(), String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    theme_manager.save_custom(theme);
    Ok(())
}

#[tauri::command]
pub fn delete_custom_theme(state: State<AppState>, name: String) -> Result<(), String> {
    let theme_manager = state.theme_manager.lock().map_err(|e| e.to_string())?;
    theme_manager.delete_custom(&name);
    Ok(())
}

#[tauri::command]
pub fn export_theme_file(
    path: String,
    theme: crate::managers::theme_manager::ThemeColors,
) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(&theme).map_err(|e| e.to_string())?;
    std::fs::write(&path, serialized).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_theme_file(
    path: String,
) -> Result<crate::managers::theme_manager::ThemeColors, String> {
    let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_for_updates_now<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<crate::managers::update_manager::UpdateCheckOutcome, String> {
    let current_version = app.package_info().version.to_string();
    Ok(crate::managers::update_manager::check_for_updates(&current_version).await)
}

pub fn shutdown_playback_now(state: &AppState) {
    let handle = state.playback_handle.lock().ok().and_then(|mut g| g.take());
    if let Some(handle) = handle {
        let _ = handle.cmd_tx.send(PlaybackCommand::Shutdown);
        let _ = handle.thread.join();
    }
}

fn ser_save_event(ev: &KeyEvent) -> Value {
    serde_json::json!({
        "time": ev.time,
        "priority": ev.priority,
        "action": ev.action,
        "key_char": ev.key_char,
        "pitch": ev.pitch,
        "velocity": ev.velocity,
    })
}

fn save_output_path(save_dir: &Path, save_name: &str) -> Result<(String, std::path::PathBuf), String> {
    let name = sanitize_save_name(save_name)?;
    let output_path = save_dir.join(format!("{name}.json"));
    if output_path.exists() {
        return Err(format!("A save named \"{name}\" already exists."));
    }
    Ok((name, output_path))
}

#[tauri::command]
pub fn check_save_name(state: State<AppState>, name: String) -> Result<String, String> {
    let save_dir = state
        .config_manager
        .lock()
        .map_err(|e| e.to_string())?
        .save_dir
        .clone();
    save_output_path(&save_dir, &name).map(|(cleaned, _)| cleaned)
}

pub fn save_playback_logic(
    config: &PlaybackConfig,
    session: &PlaybackSession,
    tracks: &[MidiTrack],
    selected_tracks_info: &[(i32, String)],
    save_dir: &Path,
    original_filename: &str,
    save_name: &str,
) -> Result<String, String> {
    save_output_path(save_dir, save_name)?;
    let events = session
        .merged_events
        .as_deref()
        .ok_or("Pedal must be compiled before saving.")?;

    if events.is_empty() {
        return Err(
            "The compiled playback has zero events -- nothing to save. Verify that the selected \
             tracks contain notes within the keyboard's playable range."
                .to_string(),
        );
    }

    let serialized_events: Vec<Value> = events.iter().map(ser_save_event).collect();

    let track_map: HashMap<i32, &MidiTrack> = tracks.iter().map(|t| (t.index, t)).collect();
    let track_details: Vec<Value> = selected_tracks_info
        .iter()
        .map(|(idx, role)| {
            let track = track_map.get(idx).copied();
            let pitches: Vec<i32> = track
                .map(|t| t.notes.iter().map(|n| n.pitch).collect())
                .unwrap_or_default();
            serde_json::json!({
                "index": idx,
                "name": track.map(|t| t.name.clone()).unwrap_or_default(),
                "note_count": track.map(|t| t.notes.len()).unwrap_or(0),
                "pitch_min": pitches.iter().min(),
                "pitch_max": pitches.iter().max(),
                "role": role,
            })
        })
        .collect();

    let mut action_counts: HashMap<&str, i64> =
        [("press", 0i64), ("release", 0), ("pedal", 0)].into_iter().collect();
    let mut compiled_pedal_count = 0i64;
    for ev in events {
        if let Some(c) = action_counts.get_mut(ev.action.as_str()) {
            *c += 1;
        }
        if ev.action == "pedal" && ev.key_char == "down" {
            compiled_pedal_count += 1;
        }
    }

    let now_iso = chrono::Local::now().to_rfc3339();
    let config_value = serde_json::to_value(config).map_err(|e| e.to_string())?;
    let metadata = serde_json::json!({
        "creation_timestamp": now_iso,
        "last_accessed": now_iso,
        "source_midi_filename": original_filename,
        "playback_settings": config_value,
        "track_details": track_details,
        "compiled_pedal_count": compiled_pedal_count,
    });
    let save_data = serde_json::json!({
        "metadata": metadata,
        "compiled_events": serialized_events,
    });

    let (_, output_path) = save_output_path(save_dir, save_name)?;

    write_json_atomic(&output_path, &save_data).map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn save_playback<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    config: PlaybackConfig,
    selected_tracks_info: Vec<(i32, String)>,
    original_filename: String,
    save_name: String,
) -> Result<String, String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let tracks = state
            .parsed_tracks
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or("No MIDI file has been parsed.")?;
        let save_dir = state
            .config_manager
            .lock()
            .map_err(|e| e.to_string())?
            .save_dir
            .clone();

        let session = state.playback_session.lock().map_err(|e| e.to_string())?;
        save_playback_logic(
            &config,
            &session,
            &tracks,
            &selected_tracks_info,
            &save_dir,
            &original_filename,
            &save_name,
        )
    })
    .await
}

pub fn validate_save_data(data: &Value) -> Result<(), String> {
    let metadata = data
        .get("metadata")
        .and_then(|v| v.as_object())
        .ok_or("Save file is missing 'metadata'.")?;
    let events = data
        .get("compiled_events")
        .and_then(|v| v.as_array())
        .filter(|a| !a.is_empty())
        .ok_or("Save file has no compiled events.")?;

    let first = &events[0];
    if first.get("time").and_then(|v| v.as_f64()).is_none() {
        return Err("Save file's first event is missing a valid 'time'.".to_string());
    }
    if first.get("priority").and_then(json_number_as_i32).is_none() {
        return Err("Save file's first event is missing a valid 'priority'.".to_string());
    }
    if first.get("action").and_then(|v| v.as_str()).is_none() {
        return Err("Save file's first event is missing a valid 'action'.".to_string());
    }
    if first.get("key_char").and_then(|v| v.as_str()).is_none() {
        return Err("Save file's first event is missing a valid 'key_char'.".to_string());
    }
    if !metadata.contains_key("track_details") || !metadata.contains_key("compiled_pedal_count") {
        return Err(
            "Save file is from an older, incompatible version and cannot be loaded.".to_string(),
        );
    }
    Ok(())
}

pub fn load_save_file_logic(path: &Path) -> Result<Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut data: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    validate_save_data(&data)?;

    let now_iso = chrono::Local::now().to_rfc3339();
    if let Some(metadata) = data.get_mut("metadata").and_then(|v| v.as_object_mut()) {
        metadata.insert("last_accessed".to_string(), serde_json::json!(now_iso));
    }

    let owned_path = path.to_path_buf();
    let stamped = data.clone();
    std::thread::spawn(move || {
        let _ = write_json_atomic(&owned_path, &stamped);
    });

    Ok(data)
}

#[tauri::command]
pub fn load_save_file(filepath: String) -> Result<Value, String> {
    load_save_file_logic(Path::new(&filepath))
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EditTrack {
    pub index: i32,
    pub role: String,
    pub name: String,
    pub note_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveEditPlan {
    pub midi_file: String,
    pub settings: PlaybackConfig,
    pub tracks: Vec<EditTrack>,
    pub legacy_pedal_thresholds: bool,
}

const EDIT_NO_SETTINGS: &str =
    "This save has no recorded settings, so it cannot be reopened for editing. Use Load to play it as saved.";
const EDIT_NO_MIDI_PATH: &str = "This save does not record which MIDI file it came from, so it cannot be reopened for \
     editing. Use Load to play it as saved.";
const EDIT_NO_TRACKS: &str =
    "This save does not record its track selection, so it cannot be reopened for editing. Use Load to play it as saved.";
const CURVE_KEYS: [&str; 2] = ["pedal_curve_on", "pedal_curve_off"];

fn edit_track_not_found(name: &str) -> String {
    format!(
        "The track \"{name}\" that this save used was not found in the MIDI file. The file may have changed since \
         the save was made."
    )
}

fn resolve_edit_tracks(details: &[Value], parsed: &[TrackSummary]) -> Result<Vec<EditTrack>, String> {
    let explicit: Vec<Option<i32>> = details
        .iter()
        .map(|detail| detail.get("index").and_then(json_number_as_i32))
        .collect();
    let mut claimed: HashSet<i32> = explicit.iter().flatten().copied().collect();
    let mut resolved = Vec::with_capacity(details.len());
    for (detail, explicit_index) in details.iter().zip(explicit) {
        let name = detail.get("name").and_then(|v| v.as_str()).unwrap_or_default();
        let note_count = detail.get("note_count").and_then(|v| v.as_u64());
        let role = detail
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("The track \"{name}\" in this save has no hand role, so it cannot be reopened for editing."))?;
        let track = match explicit_index {
            Some(index) => parsed.iter().find(|t| t.index == index),
            None => {
                let found = parsed.iter().find(|t| {
                    !claimed.contains(&t.index) && t.name == name && Some(t.note_count as u64) == note_count
                });
                if let Some(t) = found {
                    claimed.insert(t.index);
                }
                found
            }
        }
        .ok_or_else(|| edit_track_not_found(name))?;
        resolved.push(EditTrack {
            index: track.index,
            role: role.to_string(),
            name: track.name.clone(),
            note_count: track.note_count,
        });
    }
    Ok(resolved)
}

pub fn save_edit_plan_logic(save_data: &Value) -> Result<SaveEditPlan, String> {
    let metadata = save_data
        .get("metadata")
        .and_then(|v| v.as_object())
        .ok_or("Save file is missing 'metadata'.")?;
    let settings_value = metadata
        .get("playback_settings")
        .filter(|v| v.is_object())
        .ok_or(EDIT_NO_SETTINGS)?;

    let mut without_curves = settings_value.clone();
    if let Some(object) = without_curves.as_object_mut() {
        for key in CURVE_KEYS {
            object.remove(key);
        }
    }
    let mut settings: PlaybackConfig = serde_json::from_value(without_curves)
        .map_err(|e| format!("This save's settings could not be read: {e}"))?;

    if settings.midi_file.trim().is_empty() {
        return Err(EDIT_NO_MIDI_PATH.to_string());
    }
    let midi_file = settings.midi_file.clone();
    if !Path::new(&midi_file).is_file() {
        return Err(format!(
            "The MIDI file for this save was not found: {midi_file}. Move it back to that location, or use Load to \
             play the save as it was."
        ));
    }
    let (_, _, _, _, parsed) = parse_midi_structure_logic(&midi_file)
        .map_err(|e| format!("The MIDI file for this save could not be read: {e}"))?;

    let details = metadata
        .get("track_details")
        .and_then(|v| v.as_array())
        .filter(|a| !a.is_empty())
        .ok_or(EDIT_NO_TRACKS)?;
    let tracks = resolve_edit_tracks(details, &parsed.tracks)?;

    let curves = saved_pedal_curves(Some(settings_value));
    settings.pedal_curve_on = curves.on;
    settings.pedal_curve_off = curves.off;

    Ok(SaveEditPlan {
        midi_file,
        settings,
        tracks,
        legacy_pedal_thresholds: curves.legacy,
    })
}

#[tauri::command]
pub async fn load_save_for_edit(filepath: String) -> Result<SaveEditPlan, String> {
    run_blocking(move || {
        let data = load_save_file_logic(Path::new(&filepath))?;
        save_edit_plan_logic(&data)
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResumedSession {
    pub final_notes: Vec<Note>,
    pub played_notes: Vec<Note>,
    pub total_dur: f64,
    pub tempo_events: Vec<(f64, u32)>,
    pub time_signatures: Vec<(f64, u8, u8)>,
    pub measure_boundaries: Vec<(f64, f64)>,
    pub track_count: usize,
    pub pedal_count: i64,
    pub pedal_curve_on: Vec<ThresholdNode>,
    pub pedal_curve_off: Vec<ThresholdNode>,
    pub legacy_pedal_thresholds: bool,
}

fn json_number_as_i32(v: &Value) -> Option<i32> {
    if let Some(i) = v.as_i64() {
        return i32::try_from(i).ok();
    }
    let f = v.as_f64()?;
    if f.is_finite() && f.fract() == 0.0 {
        i32::try_from(f as i64).ok()
    } else {
        None
    }
}

fn deser_save_event(v: &Value) -> Result<KeyEvent, String> {
    let time = v
        .get("time")
        .and_then(|x| x.as_f64())
        .filter(|t| t.is_finite())
        .ok_or("event is missing a valid 'time'")?;
    let priority = v
        .get("priority")
        .and_then(json_number_as_i32)
        .ok_or("event is missing a valid 'priority'")?;
    let action = v
        .get("action")
        .and_then(|x| x.as_str())
        .ok_or("event is missing a valid 'action'")?
        .to_string();
    let key_char = v
        .get("key_char")
        .and_then(|x| x.as_str())
        .ok_or("event is missing a valid 'key_char'")?
        .to_string();
    let pitch = v.get("pitch").and_then(json_number_as_i32);
    let velocity = v.get("velocity").and_then(json_number_as_i32);
    Ok(KeyEvent {
        time,
        priority,
        action,
        key_char,
        pitch,
        velocity,
    })
}

pub fn reconstruct_notes_for_visualizer(events: &[KeyEvent]) -> Vec<Note> {
    let mut open: HashMap<i32, VecDeque<(f64, i32)>> = HashMap::new();
    let mut notes = Vec::new();
    let mut id_counter = 0i32;

    for ev in events {
        let Some(pitch) = ev.pitch else { continue };
        match ev.action.as_str() {
            "press" => {
                open.entry(pitch)
                    .or_default()
                    .push_back((ev.time, ev.velocity.unwrap_or(64)));
            }
            "release" => {
                if let Some(queue) = open.get_mut(&pitch) {
                    if let Some((start, velocity)) = queue.pop_front() {
                        let duration = ev.time - start;
                        if duration > 0.0 {
                            notes.push(Note {
                                id: id_counter,
                                pitch,
                                velocity,
                                start_time: start,
                                duration,
                                hand: "unknown".to_string(),
                                original_track_index: -1,
                                channel: -1,
                            });
                            id_counter += 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    notes
}

pub fn resume_from_save_logic(
    save_data: &Value,
    session: &mut PlaybackSession,
) -> Result<ResumedSession, String> {
    validate_save_data(save_data)?;
    let events_json = save_data
        .get("compiled_events")
        .and_then(|v| v.as_array())
        .ok_or("Save file has no compiled events.")?;

    let mut events: Vec<KeyEvent> = Vec::with_capacity(events_json.len());
    for (i, ev) in events_json.iter().enumerate() {
        events.push(deser_save_event(ev).map_err(|e| format!("Save file event #{i}: {e}"))?);
    }
    scan_events(&events)?;
    events.sort();

    let final_notes = reconstruct_notes_for_visualizer(&events);
    let max_time = events
        .iter()
        .map(|e| e.time)
        .fold(f64::NEG_INFINITY, f64::max);
    let total_dur = if max_time.is_finite() && max_time > 0.0 { max_time } else { 1.0 };

    let resumed_tempo_map = TempoMap::new(vec![(0.0, 500_000)], vec![]);
    let (tempo_events, time_signatures, measure_boundaries) =
        timeline_tempo_fields(&resumed_tempo_map, total_dur);

    let metadata = save_data.get("metadata");
    let track_count = metadata
        .and_then(|m| m.get("track_details"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let pedal_count = metadata
        .and_then(|m| m.get("compiled_pedal_count"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let saved_curves = saved_pedal_curves(metadata.and_then(|m| m.get("playback_settings")));

    session.final_notes = Some(final_notes.clone());
    session.humanized_notes = None;
    session.note_events = None;
    session.pedal_events = None;
    session.merged_events = Some(events);
    session.tempo_map = Some(resumed_tempo_map);
    session.total_dur = total_dur;
    session.midi_pedal_events = Vec::new();
    session.notes_config_snapshot = None;
    session.pedal_config_snapshot = None;

    Ok(ResumedSession {
        played_notes: final_notes.clone(),
        final_notes,
        total_dur,
        tempo_events,
        time_signatures,
        measure_boundaries,
        track_count,
        pedal_count,
        pedal_curve_on: saved_curves.on,
        pedal_curve_off: saved_curves.off,
        legacy_pedal_thresholds: saved_curves.legacy,
    })
}

#[tauri::command]
pub async fn resume_from_save<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    filepath: String,
) -> Result<ResumedSession, String> {
    run_blocking(move || {
        let data = load_save_file_logic(Path::new(&filepath))?;
        let state = app.state::<AppState>();
        let mut session = state.playback_session.lock().map_err(|e| e.to_string())?;
        resume_from_save_logic(&data, &mut session)
    })
    .await
}

fn clear_loaded_song_logic(state: &AppState) -> Result<(), String> {
    *state.playback_session.lock().map_err(|e| e.to_string())? = PlaybackSession::default();
    *state.parsed_tracks.lock().map_err(|e| e.to_string())? = None;
    *state.parsed_tempo_map.lock().map_err(|e| e.to_string())? = None;
    *state.loaded_pedal_count.lock().map_err(|e| e.to_string())? = 0;
    state.midi_pedal_events.lock().map_err(|e| e.to_string())?.clear();
    Ok(())
}

#[tauri::command]
pub async fn clear_loaded_song<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        clear_loaded_song_logic(&state)
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SaveSummary {
    pub path: String,
    pub filename: String,
    pub song_name: String,
    pub created: String,
    pub last_accessed: String,
    pub track_count: usize,
    pub note_count: usize,
    pub pedal_count: i64,
    pub tempo: f64,
    pub pedal_style: String,
    pub use_88_key_layout: bool,
    pub humanization: Vec<String>,
}

fn parse_save_timestamp(s: &str) -> chrono::DateTime<chrono::Utc> {
    if let Ok(d) = chrono::DateTime::parse_from_rfc3339(s) {
        return d.with_timezone(&chrono::Utc);
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return naive.and_utc();
    }
    chrono::DateTime::<chrono::Utc>::MIN_UTC
}

fn humanization_labels(settings: &Value) -> Vec<String> {
    const FLAGS: &[(&str, &str)] = &[
        ("vary_timing", "Vary Timing"),
        ("vary_articulation", "Vary Articulation"),
        ("simulate_hands", "Simulate Hands"),
        ("enable_chord_roll", "Chord Roll"),
        ("enable_tempo_sway", "Tempo Sway"),
        ("enable_drift_correction", "Hand Drift"),
        ("enable_mistakes", "Mistake Chance"),
    ];
    FLAGS
        .iter()
        .filter(|(key, _)| settings.get(*key).and_then(|v| v.as_bool()).unwrap_or(false))
        .map(|(_, label)| label.to_string())
        .collect()
}

fn build_save_summary(path: &Path, data: &Value) -> Option<SaveSummary> {
    let metadata = data.get("metadata")?.as_object()?;
    let song_name = metadata
        .get("source_midi_filename")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown MIDI")
        .to_string();
    let created = metadata
        .get("creation_timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let last_accessed = metadata
        .get("last_accessed")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| created.clone());
    let track_details = metadata.get("track_details").and_then(|v| v.as_array());
    let track_count = track_details.map(|a| a.len()).unwrap_or(0);
    let note_count = track_details
        .map(|a| {
            a.iter()
                .filter_map(|t| t.get("note_count").and_then(|v| v.as_u64()))
                .sum::<u64>() as usize
        })
        .unwrap_or(0);
    let pedal_count = metadata
        .get("compiled_pedal_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let empty_settings = Value::Object(serde_json::Map::new());
    let settings = metadata.get("playback_settings").unwrap_or(&empty_settings);
    let tempo = settings.get("tempo").and_then(|v| v.as_f64()).unwrap_or(1.0) * 100.0;
    let pedal_style = settings
        .get("pedal_style")
        .and_then(|v| v.as_str())
        .unwrap_or("none")
        .to_string();
    let use_88_key_layout = settings
        .get("use_88_key_layout")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let humanization = humanization_labels(settings);

    Some(SaveSummary {
        path: path.to_string_lossy().to_string(),
        filename: path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default(),
        song_name,
        created,
        last_accessed,
        track_count,
        note_count,
        pedal_count,
        tempo,
        pedal_style,
        use_88_key_layout,
        humanization,
    })
}

pub fn list_saves_logic(dir: &Path) -> Vec<SaveSummary> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        if validate_save_data(&data).is_err() {
            continue;
        }
        if let Some(summary) = build_save_summary(&path, &data) {
            out.push(summary);
        }
    }

    out.sort_by(|a, b| {
        parse_save_timestamp(&b.last_accessed).cmp(&parse_save_timestamp(&a.last_accessed))
    });
    out
}

#[tauri::command]
pub fn list_saves(dir: String) -> Vec<SaveSummary> {
    list_saves_logic(Path::new(&dir))
}

pub fn delete_save_logic(path: &Path) -> Result<(), String> {
    if !path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        return Err("Refusing to delete a file that isn't a save (.json).".to_string());
    }
    if !path.exists() {
        return Err("Save file does not exist.".to_string());
    }
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_save(filepath: String) -> Result<(), String> {
    delete_save_logic(Path::new(&filepath))
}

fn sanitize_save_name(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let without_ext = if trimmed.len() >= 5
        && trimmed.is_char_boundary(trimmed.len() - 5)
        && trimmed[trimmed.len() - 5..].eq_ignore_ascii_case(".json")
    {
        trimmed[..trimmed.len() - 5].trim_end()
    } else {
        trimmed
    };
    let cleaned = without_ext.trim_end_matches(['.', ' ']);
    if cleaned.is_empty() {
        return Err("New name cannot be empty.".to_string());
    }
    const INVALID: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    if cleaned.chars().any(|c| INVALID.contains(&c) || c.is_control()) {
        return Err(
            "New name contains characters that aren't allowed in a filename (< > : \" / \\ | ? *)."
                .to_string(),
        );
    }
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
        "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.iter().any(|r| cleaned.eq_ignore_ascii_case(r)) {
        return Err(format!("'{cleaned}' is a reserved name on Windows and can't be used."));
    }
    Ok(cleaned.to_string())
}

pub fn rename_save_logic(path: &Path, new_name: &str) -> Result<String, String> {
    if !path.exists() {
        return Err("Save file does not exist.".to_string());
    }
    if !path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json")) {
        return Err("Refusing to rename a file that isn't a save (.json).".to_string());
    }
    let cleaned = sanitize_save_name(new_name)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let new_path = parent.join(format!("{cleaned}.json"));
    if new_path.exists() && new_path != path {
        let same_file = std::fs::canonicalize(&new_path)
            .ok()
            .zip(std::fs::canonicalize(path).ok())
            .is_some_and(|(a, b)| a == b);
        if !same_file {
            return Err("A save with that name already exists.".to_string());
        }
    }
    std::fs::rename(path, &new_path).map_err(|e| e.to_string())?;
    Ok(new_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn rename_save(filepath: String, new_name: String) -> Result<String, String> {
    rename_save_logic(Path::new(&filepath), &new_name)
}

#[tauri::command]
pub fn translate_sheet_to_notes(
    sheet_text: String,
    bpm: f64,
    use_88_key_layout: bool,
) -> Result<Vec<Note>, String> {
    let key_mapper = KeyMapper::new(use_88_key_layout);
    Ok(VirtualPianoFormat::parse(&sheet_text, bpm, &key_mapper))
}

#[tauri::command]
pub fn notes_to_sheet(
    notes: Vec<Note>,
    use_88_key_layout: bool,
    tempo_events: Vec<(f64, u32)>,
    time_signatures: Vec<(f64, u8, u8)>,
) -> Result<String, String> {
    let key_mapper = KeyMapper::new(use_88_key_layout);
    let tempo_map = TempoMap::new(tempo_events, time_signatures);
    Ok(VirtualPianoFormat::serialize(&notes, &key_mapper, &tempo_map))
}

fn hotkey_event_to_channel_payload(event: &HotkeyEvent) -> (&'static str, serde_json::Value) {
    match event {
        HotkeyEvent::BoundUpdated(s) => ("hotkey_bound_updated", serde_json::json!(s)),
        HotkeyEvent::BoundSaveUpdated(s) => ("hotkey_bound_save_updated", serde_json::json!(s)),
        HotkeyEvent::ToggleRequested => ("hotkey_toggle_requested", Value::Null),
        HotkeyEvent::SaveRequested => ("hotkey_save_requested", Value::Null),
    }
}

pub fn hotkey_config_patch(event: &HotkeyEvent) -> Option<Value> {
    match event {
        HotkeyEvent::BoundUpdated(text) => Some(serde_json::json!({ PLAYBACK_HOTKEY_CONFIG_KEY: text })),
        HotkeyEvent::BoundSaveUpdated(text) => Some(serde_json::json!({ SAVE_HOTKEY_CONFIG_KEY: text })),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    GlobalHook,
    Page,
}

pub fn key_from_rdev_event(event_type: &rdev::EventType) -> Option<(RawKey, bool)> {
    match event_type {
        rdev::EventType::KeyPress(k) => rdev_key_to_raw(*k).map(|r| (r, true)),
        rdev::EventType::KeyRelease(k) => rdev_key_to_raw(*k).map(|r| (r, false)),
        _ => None,
    }
}

pub fn deliver_key(
    state: &AppState,
    raw: RawKey,
    is_press: bool,
    source: KeySource,
) -> Vec<HotkeyEvent> {
    match source {
        KeySource::GlobalHook => state.hook_seen.record(raw, is_press),
        KeySource::Page => {
            if state.hook_seen.is_duplicate(raw, is_press) {
                return if state.hook_seen.take_suppressed_save(raw, is_press) {
                    vec![HotkeyEvent::SaveRequested]
                } else {
                    Vec::new()
                };
            }
        }
    }
    if state.injected_echoes.consume(raw, is_press) {
        return Vec::new();
    }
    let Ok(mut mgr) = state.hotkey_manager.lock() else {
        return Vec::new();
    };
    if !is_press {
        mgr.on_release(raw);
        return Vec::new();
    }
    let mut events = mgr.on_press(raw);
    drop(mgr);
    if source == KeySource::GlobalHook && events.contains(&HotkeyEvent::SaveRequested) {
        events.retain(|event| *event != HotkeyEvent::SaveRequested);
        state.hook_seen.mark_save_suppressed(raw, is_press);
    }
    for event in &events {
        if let Some(patch) = hotkey_config_patch(event) {
            let _ = patch_app_config(state, &patch);
        }
    }
    events
}

pub fn emit_hotkey_events<R: tauri::Runtime>(app: &tauri::AppHandle<R>, events: &[HotkeyEvent]) {
    for event in events {
        let (channel, payload) = hotkey_event_to_channel_payload(event);
        let _ = app.emit(channel, payload);
    }
}

#[tauri::command]
pub fn forward_key<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    state: State<AppState>,
    code: String,
    pressed: bool,
) -> Result<(), String> {
    let Some(raw) = dom_code_to_raw(&code) else {
        return Ok(());
    };
    let events = deliver_key(&state, raw, pressed, KeySource::Page);
    emit_hotkey_events(&app, &events);
    Ok(())
}

#[tauri::command]
pub fn get_hotkeys(state: State<AppState>) -> Result<(String, String), String> {
    let mgr = state.hotkey_manager.lock().map_err(|e| e.to_string())?;
    Ok((mgr.format_hotkey_string(), mgr.format_save_hotkey_string()))
}

pub fn handle_hook_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event_type: &rdev::EventType,
) {
    let Some((raw, is_press)) = key_from_rdev_event(event_type) else {
        return;
    };
    let state = app.state::<AppState>();
    let events = deliver_key(&state, raw, is_press, KeySource::GlobalHook);
    emit_hotkey_events(app, &events);
}

pub fn spawn_hotkey_listener(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let _ = rdev::listen(move |event| handle_hook_event(&app, &event.event_type));
    });
}

#[tauri::command]
pub fn start_binding(state: State<AppState>) -> Result<(), String> {
    state
        .hotkey_manager
        .lock()
        .map_err(|e| e.to_string())?
        .start_binding();
    Ok(())
}

#[tauri::command]
pub fn start_save_binding(state: State<AppState>) -> Result<(), String> {
    state
        .hotkey_manager
        .lock()
        .map_err(|e| e.to_string())?
        .start_save_binding();
    Ok(())
}

#[cfg(test)]
mod tests;
