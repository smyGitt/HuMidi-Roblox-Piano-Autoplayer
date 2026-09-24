use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::core::atomic_write::write_json_atomic;
use crate::core::compiler::{compile_events, compile_note_events, compile_pedal_events, merge_compiled};
use crate::core::config::PlaybackConfig;
use crate::core::keyboard_driver::{EnigoDriver, KeyboardDriver};
use crate::core::midi::{KeyMapper, MidiParser, TempoMap};
use crate::core::models::{KeyEvent, MidiTrack, Note};
use crate::core::pedal::model::{PedalModel, FEATURES};
use crate::core::pedal::AiThresholds;
use crate::core::player::{PlaybackEngine, PlayerEvent};
use crate::core::section_analyzer::{assign_hands, SectionAnalyzer};
use crate::core::session_cache;
use crate::core::translator::VirtualPianoFormat;
use crate::managers::hotkey_manager::{rdev_key_to_raw, HotkeyEvent};
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
pub fn run_pedal_smoketest(state: State<AppState>, app: tauri::AppHandle) -> Result<Vec<f32>, String> {
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
    for ev in events {
        if ev.action != "pedal" {
            continue;
        }
        match ev.key_char.as_str() {
            "down" => down_time = Some(ev.time),
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
    pub ai_thresholds: Option<(f32, f32)>,
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
    let (note_events, humanized_notes) = compile_note_events(config, &final_notes, &sections);
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
    let (note_events, humanized_notes) = compile_note_events(config, &final_notes, &sections);
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

    let sections = SectionAnalyzer::new(final_notes.clone(), &tempo_map).analyze();
    let midi_pedal_opt = if midi_pedal_events.is_empty() {
        None
    } else {
        Some(midi_pedal_events.as_slice())
    };
    let (pedal_events, ai_meta) = compile_pedal_events(
        model,
        config,
        &humanized_notes,
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

    Ok(PedalData {
        pedal_intervals,
        ai_thresholds: ai_meta.map(|t: AiThresholds| (t.threshold_on, t.threshold_off)),
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
        start_playback_blocking(&state, &app, config)?;
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

    engine.start(now_fn());
    let mut finished_emitted = false;

    loop {
        let outcome = engine.tick(driver, now_fn());
        for ev in &outcome.events {
            if matches!(ev, PlayerEvent::Finished) {
                finished_emitted = true;
            }
            emit(ev);
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

fn start_playback_blocking<R: tauri::Runtime>(
    state: &AppState,
    app: &tauri::AppHandle<R>,
    config: PlaybackConfig,
) -> Result<(), String> {
    let (merged_events, total_dur) = {
        let session = state.playback_session.lock().map_err(|e| e.to_string())?;
        let merged_events = session
            .merged_events
            .clone()
            .ok_or("Pedal must be compiled before playback can start.")?;
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

    let mut driver = EnigoDriver::new().map_err(|e| e.to_string())?;
    let mut engine = PlaybackEngine::new(config, Vec::new());
    engine.load_compiled_events(merged_events, total_dur);

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
) -> Result<(), String> {
    run_blocking(move || start_playback_blocking(&app.state::<AppState>(), &app, config)).await
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

#[tauri::command]
pub fn save_app_config(state: State<AppState>, patch: Value) -> Result<(), String> {
    let mut app_config = state.app_config.lock().map_err(|e| e.to_string())?;
    *app_config = merge_app_config(&app_config, &patch);
    let config_manager = state.config_manager.lock().map_err(|e| e.to_string())?;
    config_manager.save(&app_config);
    Ok(())
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
pub async fn check_for_updates_now(
    app: tauri::AppHandle,
) -> Result<crate::managers::update_manager::UpdateCheckOutcome, String> {
    crate::managers::update_manager::check_for_updates(&app).await
}

#[tauri::command]
pub async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    crate::managers::update_manager::download_and_install_update(&app).await
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

pub fn save_playback_logic(
    config: &PlaybackConfig,
    model: Option<&PedalModel>,
    tracks: &[MidiTrack],
    selected_tracks_info: &[(i32, String)],
    save_dir: &Path,
    original_filename: &str,
) -> Result<String, String> {
    let (final_notes, tempo_map, midi_pedal_events) =
        prepare_notes(config, selected_tracks_info).map_err(|e| e.to_string())?;
    let sections = SectionAnalyzer::new(final_notes.clone(), &tempo_map).analyze();
    let midi_pedal_opt = if midi_pedal_events.is_empty() {
        None
    } else {
        Some(midi_pedal_events.as_slice())
    };
    let (events, _ai_meta) = compile_events(model, config, &final_notes, &sections, midi_pedal_opt);

    if events.is_empty() {
        return Err(
            "Compilation produced zero events -- nothing to save. Verify that the selected \
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
    for ev in &events {
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

    let timestamp_str = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let stem = Path::new(original_filename)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| original_filename.to_string());
    let output_path = save_dir.join(format!("{stem}_{timestamp_str}.json"));

    let serialized = serde_json::to_string_pretty(&save_data).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(save_dir).map_err(|e| e.to_string())?;
    std::fs::write(&output_path, serialized).map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn save_playback(
    state: State<AppState>,
    app: tauri::AppHandle,
    config: PlaybackConfig,
    selected_tracks_info: Vec<(i32, String)>,
    original_filename: String,
) -> Result<String, String> {
    let model = pedal_model_for(&state, &config, || pedal_resource_path(&app))?;
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

    save_playback_logic(
        &config,
        model.as_deref(),
        &tracks,
        &selected_tracks_info,
        &save_dir,
        &original_filename,
    )
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResumedSession {
    pub final_notes: Vec<Note>,
    pub total_dur: f64,
    pub tempo_events: Vec<(f64, u32)>,
    pub time_signatures: Vec<(f64, u8, u8)>,
    pub measure_boundaries: Vec<(f64, f64)>,
    pub track_count: usize,
    pub pedal_count: i64,
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
        final_notes,
        total_dur,
        tempo_events,
        time_signatures,
        measure_boundaries,
        track_count,
        pedal_count,
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

pub fn spawn_hotkey_listener(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let callback = move |event: rdev::Event| {
            let mapped = match event.event_type {
                rdev::EventType::KeyPress(k) => rdev_key_to_raw(k).map(|r| (r, true)),
                rdev::EventType::KeyRelease(k) => rdev_key_to_raw(k).map(|r| (r, false)),
                _ => None,
            };
            let Some((raw, is_press)) = mapped else {
                return;
            };

            let state = app.state::<AppState>();
            let Ok(mut mgr) = state.hotkey_manager.lock() else {
                return;
            };

            if is_press {
                let events = mgr.on_press(raw);
                drop(mgr);
                for ev in events {
                    let (channel, payload) = hotkey_event_to_channel_payload(&ev);
                    let _ = app.emit(channel, payload);
                }
            } else {
                mgr.on_release(raw);
            }
        };
        let _ = rdev::listen(callback);
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
mod tests {
    use super::*;

    fn pedal_ev(t: f64, direction: &str) -> KeyEvent {
        KeyEvent::new(t, 1, "pedal", direction)
    }

    fn press_ev(t: f64) -> KeyEvent {
        KeyEvent::new(t, 0, "press", "a")
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

    fn write_temp_midi_with_tempo(tempo_us: u32) -> tempfile::TempPath {
        use midly::num::{u15, u24, u28, u4, u7};
        use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

        let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
        let track = vec![
            TrackEvent {
                delta: u28::from(0),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::from(tempo_us))),
            },
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

    mod test_merge_app_config {
        use super::*;

        #[test]
        fn test_merges_new_keys_into_existing_object() {
            let current = serde_json::json!({"always_on_top": false, "opacity": 100});
            let patch = serde_json::json!({"opacity": 80});
            let merged = merge_app_config(&current, &patch);
            assert_eq!(merged, serde_json::json!({"always_on_top": false, "opacity": 80}));
        }

        #[test]
        fn test_patch_keys_not_present_in_current_are_added() {
            let current = serde_json::json!({"always_on_top": false});
            let patch = serde_json::json!({"pedal_prompt_threshold": 8});
            let merged = merge_app_config(&current, &patch);
            assert_eq!(
                merged,
                serde_json::json!({"always_on_top": false, "pedal_prompt_threshold": 8})
            );
        }

        #[test]
        fn test_empty_current_object_takes_every_patch_key() {
            let current = serde_json::json!({});
            let patch = serde_json::json!({"opacity": 50, "auto_check_updates": true});
            let merged = merge_app_config(&current, &patch);
            assert_eq!(merged, serde_json::json!({"opacity": 50, "auto_check_updates": true}));
        }

        #[test]
        fn test_non_object_current_is_replaced_wholesale_by_patch() {
            let current = Value::Null;
            let patch = serde_json::json!({"opacity": 50});
            let merged = merge_app_config(&current, &patch);
            assert_eq!(merged, serde_json::json!({"opacity": 50}));
        }

        #[test]
        fn test_independent_callers_patching_different_keys_do_not_clobber_each_other() {
            let mut current = serde_json::json!({});
            current = merge_app_config(&current, &serde_json::json!({"always_on_top": true}));
            current = merge_app_config(&current, &serde_json::json!({"redact_debug_paths": false}));
            assert_eq!(
                current,
                serde_json::json!({"always_on_top": true, "redact_debug_paths": false})
            );
        }
    }

    mod test_parse_midi_structure_logic {
        use super::*;

        #[test]
        fn test_default_tempo_when_no_tempo_event() {
            let path = write_temp_midi_single_note();
            let (_, _, _, _, result) =
                parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
            assert!((result.initial_bpm - 120.0).abs() < 1e-6);
        }

        #[test]
        fn test_explicit_tempo_event_reflected() {
            let path = write_temp_midi_with_tempo(400_000);
            let (_, _, _, _, result) =
                parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
            assert!((result.initial_bpm - 150.0).abs() < 1e-6);
        }

        #[test]
        fn test_tracks_field_matches_summaries() {
            let path = write_temp_midi_single_note();
            let (_, _, _, _, result) =
                parse_midi_structure_logic(&path.to_string_lossy()).unwrap();
            assert_eq!(result.tracks.len(), 1);
            assert_eq!(result.tracks[0].note_count, 1);
        }

        #[test]
        fn test_missing_file_errors() {
            assert!(parse_midi_structure_logic("does_not_exist.mid").is_err());
        }
    }

    mod test_compute_total_dur {
        use super::*;

        fn note(start_time: f64, duration: f64) -> Note {
            Note::new(0, 60, 64, start_time, duration)
        }

        #[test]
        fn test_empty_defaults_to_one() {
            assert_eq!(compute_total_dur(&[]), 1.0);
        }

        #[test]
        fn test_uses_max_end_time() {
            let notes = vec![note(0.0, 1.0), note(2.0, 5.0), note(1.0, 0.5)];
            assert_eq!(compute_total_dur(&notes), 7.0);
        }
    }

    mod test_compile_notes_logic {
        use super::*;
        use crate::state::PlaybackSession;

        #[test]
        fn test_populates_session_and_returns_timeline() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let mut session = PlaybackSession::default();

            let result =
                compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session)
                    .unwrap();

            assert_eq!(result.final_notes.len(), 1);
            assert!(session.notes_config_snapshot.is_some());
            assert!(session.final_notes.is_some());
            assert!(session.humanized_notes.is_some());
            assert!(session.note_events.is_some());
            assert!(session.tempo_map.is_some());
        }

        #[test]
        fn test_returns_tempo_events_and_measure_boundaries_matching_session_tempo_map() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let mut session = PlaybackSession::default();

            let result =
                compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session)
                    .unwrap();

            let session_tempo_map = session.tempo_map.as_ref().unwrap();
            assert_eq!(result.tempo_events, session_tempo_map.events());
            assert_eq!(result.time_signatures, session_tempo_map.time_signatures());
            assert_eq!(
                result.measure_boundaries,
                session_tempo_map.get_measure_boundaries(result.total_dur)
            );
            assert!(!result.measure_boundaries.is_empty());
        }

        #[test]
        fn test_invalidates_previously_compiled_pedal_state() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let mut session = PlaybackSession::default();
            session.pedal_events = Some(vec![]);
            session.merged_events = Some(vec![]);
            session.pedal_config_snapshot = Some(serde_json::json!({}));

            compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).unwrap();

            assert!(session.pedal_events.is_none());
            assert!(session.merged_events.is_none());
            assert!(session.pedal_config_snapshot.is_none());
        }

        #[test]
        fn test_empty_selection_defaults_total_dur_to_one() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let mut session = PlaybackSession::default();

            let result = compile_notes_logic(&config, &[], &mut session).unwrap();

            assert!(result.final_notes.is_empty());
            assert_eq!(result.total_dur, 1.0);
        }

        #[test]
        fn test_missing_file_returns_error() {
            let mut config = PlaybackConfig::default();
            config.midi_file = "does_not_exist.mid".to_string();
            let mut session = PlaybackSession::default();

            assert!(compile_notes_logic(&config, &[], &mut session).is_err());
        }

        #[test]
        fn test_zero_tempo_errors_instead_of_panicking() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            config.tempo = 0.0;
            let mut session = PlaybackSession::default();

            assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
        }

        #[test]
        fn test_negative_tempo_errors() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            config.tempo = -1.0;
            let mut session = PlaybackSession::default();

            assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
        }

        #[test]
        fn test_nan_tempo_errors() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            config.tempo = f64::NAN;
            let mut session = PlaybackSession::default();

            assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
        }

        #[test]
        fn test_infinite_tempo_errors() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            config.tempo = f64::INFINITY;
            let mut session = PlaybackSession::default();

            assert!(compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).is_err());
        }
    }

    mod test_compile_notes_from_sheet {
        use super::*;
        use crate::state::PlaybackSession;

        fn km() -> KeyMapper {
            KeyMapper::new(false)
        }

        #[test]
        fn test_empty_sheet_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result = compile_notes_from_sheet_logic(&config, "", 120.0, &mut session);
            assert!(result.is_err());
        }

        #[test]
        fn test_whitespace_only_sheet_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result = compile_notes_from_sheet_logic(&config, "   \n  ", 120.0, &mut session);
            assert!(result.is_err());
        }

        #[test]
        fn test_unmapped_chars_only_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result = compile_notes_from_sheet_logic(&config, "~~~", 120.0, &mut session);
            assert!(result.is_err());
        }

        #[test]
        fn test_single_note_matches_translate_sheet_timing() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let raw = translate_sheet_to_notes(ch.to_string(), 120.0, false).unwrap();

            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result =
                compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
                    .unwrap();

            assert_eq!(result.final_notes.len(), 1);
            assert_eq!(result.final_notes[0].pitch, raw[0].pitch);
            assert!((result.final_notes[0].start_time - raw[0].start_time).abs() < 1e-9);
            assert!((result.final_notes[0].duration - raw[0].duration).abs() < 1e-9);
        }

        #[test]
        fn test_returns_single_synthetic_tempo_event_matching_bpm_and_no_time_signatures() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result =
                compile_notes_from_sheet_logic(&config, &ch.to_string(), 100.0, &mut session)
                    .unwrap();

            assert_eq!(result.tempo_events.len(), 1);
            assert_eq!(result.tempo_events[0].0, 0.0);
            let expected_tempo_us = (60_000_000.0f64 / 100.0).round() as u32;
            assert_eq!(result.tempo_events[0].1, expected_tempo_us);
            assert!(result.time_signatures.is_empty());
            assert!(!result.measure_boundaries.is_empty());
        }

        #[test]
        fn test_chord_notes_share_start_time() {
            let mapper = km();
            let p1 = mapper.get_key_for_pitch(60).unwrap();
            let p2 = mapper.get_key_for_pitch(62).unwrap();
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result = compile_notes_from_sheet_logic(
                &config,
                &format!("[{p1}{p2}]"),
                120.0,
                &mut session,
            )
            .unwrap();
            assert_eq!(result.final_notes.len(), 2);
            assert_eq!(result.final_notes[0].start_time, result.final_notes[1].start_time);
        }

        #[test]
        fn test_hand_assignment_by_pitch_threshold_when_simulate_off() {
            let mapper = km();
            let low = mapper.get_key_for_pitch(40).unwrap();
            let high = mapper.get_key_for_pitch(81).unwrap();
            let mut session = PlaybackSession::default();
            let mut config = PlaybackConfig::default();
            config.simulate_hands = false;
            let result = compile_notes_from_sheet_logic(
                &config,
                &format!("{low} {high}"),
                120.0,
                &mut session,
            )
            .unwrap();
            let low_note = result.final_notes.iter().find(|n| n.pitch == 40).unwrap();
            let high_note = result.final_notes.iter().find(|n| n.pitch == 81).unwrap();
            assert_eq!(low_note.hand, "left");
            assert_eq!(high_note.hand, "right");
        }

        #[test]
        fn test_hand_assignment_uses_group_average_when_simulate_on() {
            let mapper = km();
            let p1 = mapper.get_key_for_pitch(40).unwrap();
            let p2 = mapper.get_key_for_pitch(50).unwrap();
            let mut session = PlaybackSession::default();
            let mut config = PlaybackConfig::default();
            config.simulate_hands = true;
            let result = compile_notes_from_sheet_logic(
                &config,
                &format!("[{p1}{p2}]"),
                120.0,
                &mut session,
            )
            .unwrap();
            assert!(result.final_notes.iter().all(|n| n.hand == "left"));
        }

        #[test]
        fn test_zero_bpm_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            assert!(compile_notes_from_sheet_logic(&config, "a", 0.0, &mut session).is_err());
        }

        #[test]
        fn test_negative_bpm_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            assert!(compile_notes_from_sheet_logic(&config, "a", -10.0, &mut session).is_err());
        }

        #[test]
        fn test_nan_bpm_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            assert!(
                compile_notes_from_sheet_logic(&config, "a", f64::NAN, &mut session).is_err()
            );
        }

        #[test]
        fn test_infinite_bpm_errors() {
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            assert!(compile_notes_from_sheet_logic(&config, "a", f64::INFINITY, &mut session)
                .is_err());
        }

        #[test]
        fn test_extremely_high_bpm_does_not_panic() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result = compile_notes_from_sheet_logic(
                &config,
                &ch.to_string(),
                100_000_000.0,
                &mut session,
            );
            assert!(result.is_ok());
        }

        #[test]
        fn test_large_sheet_note_count_matches_token_count() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let sheet = std::iter::repeat(ch.to_string())
                .take(500)
                .collect::<Vec<_>>()
                .join(" ");
            let mut session = PlaybackSession::default();
            let config = PlaybackConfig::default();
            let result =
                compile_notes_from_sheet_logic(&config, &sheet, 120.0, &mut session).unwrap();
            assert_eq!(result.final_notes.len(), 500);
        }

        #[test]
        fn test_invalidates_previously_compiled_pedal_state() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let mut session = PlaybackSession::default();
            session.pedal_events = Some(vec![]);
            session.merged_events = Some(vec![]);
            session.pedal_config_snapshot = Some(serde_json::json!({}));
            let config = PlaybackConfig::default();
            compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
                .unwrap();
            assert!(session.pedal_events.is_none());
            assert!(session.merged_events.is_none());
            assert!(session.pedal_config_snapshot.is_none());
        }

        #[test]
        fn test_full_pipeline_reuse_pedal_compile_succeeds_after_sheet_compile() {
            let mapper = km();
            let ch = mapper.get_key_for_pitch(60).unwrap();
            let mut session = PlaybackSession::default();
            let mut config = PlaybackConfig::default();
            config.pedal_style = "none".to_string();
            compile_notes_from_sheet_logic(&config, &ch.to_string(), 120.0, &mut session)
                .unwrap();

            let pedal_result = compile_pedal_logic(&config, None, &mut session).unwrap();
            assert!(pedal_result.pedal_intervals.is_empty());
            assert!(session.merged_events.is_some());
        }
    }

    mod test_compile_pedal_logic {
        use super::*;
        use crate::state::PlaybackSession;

        fn compiled_session() -> (PlaybackConfig, PlaybackSession) {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            config.pedal_style = "none".to_string();
            let mut session = PlaybackSession::default();
            compile_notes_logic(&config, &[(0, "Right Hand".to_string())], &mut session).unwrap();
            (config, session)
        }

        #[test]
        fn test_errors_when_notes_not_compiled_first() {
            let config = PlaybackConfig::default();
            let mut session = PlaybackSession::default();
            assert!(compile_pedal_logic(&config, None, &mut session).is_err());
        }

        #[test]
        fn test_none_style_produces_no_pedal_events() {
            let (config, mut session) = compiled_session();
            let result = compile_pedal_logic(&config, None, &mut session).unwrap();
            assert!(result.pedal_intervals.is_empty());
            assert_eq!(session.pedal_events, Some(vec![]));
        }

        #[test]
        fn test_populates_merged_and_snapshot() {
            let (config, mut session) = compiled_session();
            compile_pedal_logic(&config, None, &mut session).unwrap();
            assert!(session.merged_events.is_some());
            assert_eq!(
                session.pedal_config_snapshot,
                Some(session_cache::extract_pedal_config(&config))
            );
        }

        #[test]
        fn test_merged_events_include_all_note_events() {
            let (config, mut session) = compiled_session();
            compile_pedal_logic(&config, None, &mut session).unwrap();
            let note_events = session.note_events.clone().unwrap();
            let merged = session.merged_events.clone().unwrap();
            assert_eq!(merged.len(), note_events.len());
        }

        #[test]
        fn test_harmonic_style_does_not_panic_without_model() {
            let (mut config, mut session) = compiled_session();
            config.pedal_style = "harmonic".to_string();
            compile_pedal_logic(&config, None, &mut session).unwrap();
            assert_eq!(
                session.pedal_config_snapshot,
                Some(session_cache::extract_pedal_config(&config))
            );
        }

        #[test]
        fn test_ai_style_without_model_returns_no_pedal_events_but_does_not_error() {
            let (mut config, mut session) = compiled_session();
            config.pedal_style = "ai".to_string();
            let result = compile_pedal_logic(&config, None, &mut session);
            assert!(result.is_ok());
        }
    }

    mod test_event_to_channel_payload {
        use super::*;

        #[test]
        fn test_status() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Status("hi".to_string()));
            assert_eq!(ch, "status_updated");
            assert_eq!(payload, serde_json::json!("hi"));
        }

        #[test]
        fn test_progress() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Progress(1.5));
            assert_eq!(ch, "progress_updated");
            assert_eq!(payload, serde_json::json!(1.5));
        }

        #[test]
        fn test_finished() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Finished);
            assert_eq!(ch, "playback_finished");
            assert_eq!(payload, serde_json::Value::Null);
        }

        #[test]
        fn test_visualizer() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Visualizer(vec![60, 64]));
            assert_eq!(ch, "visualizer_updated");
            assert_eq!(payload, serde_json::json!([60, 64]));
        }

        #[test]
        fn test_pedal() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Pedal(true));
            assert_eq!(ch, "pedal_updated");
            assert_eq!(payload, serde_json::json!(true));
        }

        #[test]
        fn test_auto_paused() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::AutoPaused);
            assert_eq!(ch, "auto_paused");
            assert_eq!(payload, serde_json::Value::Null);
        }

        #[test]
        fn test_error() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Error("oops".to_string()));
            assert_eq!(ch, "error_occurred");
            assert_eq!(payload, serde_json::json!("oops"));
        }

        #[test]
        fn test_section() {
            let (ch, payload) = event_to_channel_payload(&PlayerEvent::Section(3));
            assert_eq!(ch, "section_changed");
            assert_eq!(payload, serde_json::json!(3));
        }
    }

    mod test_run_playback_loop {
        use super::*;
        use crate::core::keyboard_driver::{Key, KeyActionResult};
        use std::cell::RefCell;

        struct NoopDriver;
        impl KeyboardDriver for NoopDriver {
            fn press(&mut self, _key: Key) -> KeyActionResult {
                Ok(())
            }
            fn release(&mut self, _key: Key) -> KeyActionResult {
                Ok(())
            }
        }

        #[test]
        fn test_stop_command_breaks_loop_and_emits_final_finished() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Stop).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {},
            );

            let events = events.into_inner();
            assert!(events.iter().any(|e| matches!(e, PlayerEvent::Finished)));
        }

        #[test]
        fn test_stop_clears_visualizer_pitches_and_pedal_indicator() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Stop).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {},
            );

            let events = events.into_inner();
            assert!(events
                .iter()
                .any(|e| matches!(e, PlayerEvent::Visualizer(pitches) if pitches.is_empty())));
            assert!(events
                .iter()
                .any(|e| matches!(e, PlayerEvent::Pedal(false))));
            let visualizer_clear_idx = events
                .iter()
                .position(|e| matches!(e, PlayerEvent::Visualizer(p) if p.is_empty()))
                .unwrap();
            let finished_idx = events
                .iter()
                .position(|e| matches!(e, PlayerEvent::Finished))
                .unwrap();
            assert!(
                visualizer_clear_idx <= finished_idx,
                "visualizer/pedal clear must be emitted no later than Finished so the \
                 frontend never renders a stale highlighted key/pedal after playback ends"
            );
        }

        #[test]
        fn test_shutdown_command_also_clears_visualizer_pitches_and_pedal_indicator() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Shutdown).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {},
            );

            let events = events.into_inner();
            assert!(events
                .iter()
                .any(|e| matches!(e, PlayerEvent::Visualizer(pitches) if pitches.is_empty())));
            assert!(events.iter().any(|e| matches!(e, PlayerEvent::Pedal(false))));
        }

        #[test]
        fn test_shutdown_command_also_breaks_loop_and_emits_final_finished() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Shutdown).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {},
            );

            assert!(events
                .into_inner()
                .iter()
                .any(|e| matches!(e, PlayerEvent::Finished)));
        }

        #[test]
        fn test_countdown_emits_four_status_messages_with_second_long_sleeps() {
            let mut config = PlaybackConfig::default();
            config.countdown = true;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Stop).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            let sleeps: RefCell<Vec<f64>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |s| sleeps.borrow_mut().push(s),
            );

            let events = events.into_inner();
            let status_msgs: Vec<String> = events
                .iter()
                .filter_map(|e| match e {
                    PlayerEvent::Status(s) => Some(s.clone()),
                    _ => None,
                })
                .take(4)
                .collect();
            assert_eq!(status_msgs, PlaybackEngine::countdown_messages());
            assert_eq!(sleeps.into_inner().iter().filter(|&&s| s == 1.0).count(), 4);
        }

        #[test]
        fn test_no_countdown_skips_status_messages() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();
            tx.send(PlaybackCommand::Stop).unwrap();

            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {},
            );

            let has_countdown_status = events
                .into_inner()
                .iter()
                .any(|e| matches!(e, PlayerEvent::Status(s) if s == "Get ready..."));
            assert!(!has_countdown_status);
        }

        #[test]
        fn test_no_pending_command_does_not_break_loop_immediately() {
            let mut config = PlaybackConfig::default();
            config.countdown = false;
            let engine = PlaybackEngine::new(config, Vec::new());
            let mut driver = NoopDriver;
            let (tx, rx) = mpsc::channel();

            let tick_count = RefCell::new(0);
            let events: RefCell<Vec<PlayerEvent>> = RefCell::new(Vec::new());
            run_playback_loop(
                engine,
                &mut driver,
                &rx,
                |ev| events.borrow_mut().push(ev.clone()),
                || 0.0,
                |_| {
                    *tick_count.borrow_mut() += 1;
                    if *tick_count.borrow() >= 3 {
                        let _ = tx.send(PlaybackCommand::Stop);
                    }
                },
            );

            assert!(*tick_count.borrow() >= 3);
            assert!(events
                .into_inner()
                .iter()
                .any(|e| matches!(e, PlayerEvent::Finished)));
        }
    }

    fn track_fixture() -> MidiTrack {
        MidiTrack {
            index: 0,
            name: "Melody".to_string(),
            program_change: 0,
            is_drum: false,
            notes: vec![Note::new(0, 60, 64, 0.0, 1.0)],
        }
    }

    mod test_save_playback_logic {
        use super::*;

        #[test]
        fn test_writes_file_and_returns_path() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let save_dir = tempfile::tempdir().unwrap();
            let tracks = vec![track_fixture()];

            let result = save_playback_logic(
                &config,
                None,
                &tracks,
                &[(0, "Right Hand".to_string())],
                save_dir.path(),
                "song.mid",
            )
            .unwrap();

            assert!(std::path::Path::new(&result).exists());
            assert!(result.contains("song_"));
        }

        #[test]
        fn test_saved_file_contains_expected_schema() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let save_dir = tempfile::tempdir().unwrap();
            let tracks = vec![track_fixture()];

            let result_path = save_playback_logic(
                &config,
                None,
                &tracks,
                &[(0, "Right Hand".to_string())],
                save_dir.path(),
                "song.mid",
            )
            .unwrap();

            let data: Value =
                serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
            assert!(data["metadata"]["track_details"].is_array());
            assert_eq!(data["metadata"]["source_midi_filename"], "song.mid");
            assert!(!data["compiled_events"].as_array().unwrap().is_empty());
            let first_event = &data["compiled_events"][0];
            assert!(first_event.get("velocity").is_some());
        }

        #[test]
        fn test_track_details_include_pitch_range_and_role() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let save_dir = tempfile::tempdir().unwrap();
            let tracks = vec![track_fixture()];

            let result_path = save_playback_logic(
                &config,
                None,
                &tracks,
                &[(0, "Left Hand".to_string())],
                save_dir.path(),
                "song.mid",
            )
            .unwrap();
            let data: Value =
                serde_json::from_str(&std::fs::read_to_string(result_path).unwrap()).unwrap();
            let detail = &data["metadata"]["track_details"][0];
            assert_eq!(detail["role"], "Left Hand");
            assert_eq!(detail["pitch_min"], 60);
            assert_eq!(detail["pitch_max"], 60);
            assert_eq!(detail["note_count"], 1);
        }

        #[test]
        fn test_empty_selection_errors_with_zero_events_message() {
            let path = write_temp_midi_single_note();
            let mut config = PlaybackConfig::default();
            config.midi_file = path.to_string_lossy().to_string();
            let save_dir = tempfile::tempdir().unwrap();
            let tracks = vec![track_fixture()];

            let result = save_playback_logic(&config, None, &tracks, &[], save_dir.path(), "song.mid");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("zero events"));
        }
    }

    mod test_validate_save_data {
        use super::*;

        fn valid_data() -> Value {
            serde_json::json!({
                "metadata": {"track_details": [], "compiled_pedal_count": 0},
                "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
            })
        }

        #[test]
        fn test_valid_data_passes() {
            assert!(validate_save_data(&valid_data()).is_ok());
        }

        #[test]
        fn test_missing_metadata_fails() {
            let mut data = valid_data();
            data.as_object_mut().unwrap().remove("metadata");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_empty_events_fails() {
            let mut data = valid_data();
            data["compiled_events"] = serde_json::json!([]);
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_missing_events_field_fails() {
            let mut data = valid_data();
            data.as_object_mut().unwrap().remove("compiled_events");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_first_event_missing_time_fails() {
            let mut data = valid_data();
            data["compiled_events"][0].as_object_mut().unwrap().remove("time");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_first_event_missing_priority_fails() {
            let mut data = valid_data();
            data["compiled_events"][0]
                .as_object_mut()
                .unwrap()
                .remove("priority");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_first_event_missing_action_fails() {
            let mut data = valid_data();
            data["compiled_events"][0].as_object_mut().unwrap().remove("action");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_first_event_missing_key_char_fails() {
            let mut data = valid_data();
            data["compiled_events"][0]
                .as_object_mut()
                .unwrap()
                .remove("key_char");
            assert!(validate_save_data(&data).is_err());
        }

        #[test]
        fn test_older_version_missing_track_details_fails() {
            let mut data = valid_data();
            data["metadata"].as_object_mut().unwrap().remove("track_details");
            let err = validate_save_data(&data).unwrap_err();
            assert!(err.contains("older"));
        }

        #[test]
        fn test_older_version_missing_compiled_pedal_count_fails() {
            let mut data = valid_data();
            data["metadata"]
                .as_object_mut()
                .unwrap()
                .remove("compiled_pedal_count");
            let err = validate_save_data(&data).unwrap_err();
            assert!(err.contains("older"));
        }

        #[test]
        fn test_only_first_event_is_validated() {
            let mut data = valid_data();
            data["compiled_events"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"garbage": true}));
            assert!(validate_save_data(&data).is_ok());
        }

        #[test]
        fn test_float_valued_priority_on_first_event_still_passes() {
            let mut data = valid_data();
            data["compiled_events"][0]["priority"] = serde_json::json!(0.0);
            assert!(validate_save_data(&data).is_ok());
        }
    }

    mod test_load_save_file_logic {
        use super::*;

        fn valid_save_json() -> Value {
            serde_json::json!({
                "metadata": {
                    "track_details": [],
                    "compiled_pedal_count": 0,
                    "last_accessed": "old-value",
                },
                "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
            })
        }

        #[test]
        fn test_loads_and_returns_parsed_data() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("save.json");
            std::fs::write(&path, valid_save_json().to_string()).unwrap();

            let data = load_save_file_logic(&path).unwrap();
            assert_eq!(data["metadata"]["compiled_pedal_count"], 0);
        }

        #[test]
        fn test_stamps_last_accessed_in_returned_data() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("save.json");
            std::fs::write(&path, valid_save_json().to_string()).unwrap();

            let data = load_save_file_logic(&path).unwrap();
            assert_ne!(data["metadata"]["last_accessed"], serde_json::json!("old-value"));
        }

        #[test]
        fn test_invalid_save_data_is_rejected() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("save.json");
            std::fs::write(&path, serde_json::json!({"nope": true}).to_string()).unwrap();

            assert!(load_save_file_logic(&path).is_err());
        }

        #[test]
        fn test_missing_file_returns_error() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("does_not_exist.json");
            assert!(load_save_file_logic(&path).is_err());
        }

        #[test]
        fn test_eventually_persists_stamped_last_accessed_to_disk() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("save.json");
            std::fs::write(&path, valid_save_json().to_string()).unwrap();

            let data = load_save_file_logic(&path).unwrap();
            let expected = data["metadata"]["last_accessed"].clone();

            for _ in 0..50 {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(on_disk) = serde_json::from_str::<Value>(&content) {
                        if on_disk["metadata"]["last_accessed"] == expected {
                            return;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            panic!("stamped last_accessed was not persisted to disk in time");
        }
    }

    mod test_list_saves_logic {
        use super::*;

        fn save_json(
            song_name: &str,
            created: &str,
            last_accessed: &str,
            note_counts: &[i64],
            pedal_count: i64,
        ) -> Value {
            save_json_with_settings(
                song_name,
                created,
                last_accessed,
                note_counts,
                pedal_count,
                serde_json::json!({}),
            )
        }

        fn save_json_with_settings(
            song_name: &str,
            created: &str,
            last_accessed: &str,
            note_counts: &[i64],
            pedal_count: i64,
            playback_settings: Value,
        ) -> Value {
            let track_details: Vec<Value> = note_counts
                .iter()
                .map(|&n| {
                    serde_json::json!({
                        "name": "t", "note_count": n, "pitch_min": 0, "pitch_max": 0, "role": "Right Hand"
                    })
                })
                .collect();
            serde_json::json!({
                "metadata": {
                    "creation_timestamp": created,
                    "last_accessed": last_accessed,
                    "source_midi_filename": song_name,
                    "track_details": track_details,
                    "compiled_pedal_count": pedal_count,
                    "playback_settings": playback_settings,
                },
                "compiled_events": [{"time": 0.0, "priority": 0, "action": "press", "key_char": "a"}],
            })
        }

        #[test]
        fn test_missing_directory_returns_empty() {
            let dir = tempfile::tempdir().unwrap();
            let missing = dir.path().join("does_not_exist");
            assert!(list_saves_logic(&missing).is_empty());
        }

        #[test]
        fn test_empty_directory_returns_empty() {
            let dir = tempfile::tempdir().unwrap();
            assert!(list_saves_logic(dir.path()).is_empty());
        }

        #[test]
        fn test_single_valid_save_returns_one_summary() {
            let dir = tempfile::tempdir().unwrap();
            let data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[5],
                2,
            );
            std::fs::write(dir.path().join("song_20260101_000000.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 1);
            assert_eq!(saves[0].song_name, "song.mid");
            assert_eq!(saves[0].note_count, 5);
            assert_eq!(saves[0].pedal_count, 2);
            assert_eq!(saves[0].track_count, 1);
            assert_eq!(saves[0].filename, "song_20260101_000000.json");
            assert_eq!(saves[0].tempo, 100.0);
            assert_eq!(saves[0].pedal_style, "none");
            assert!(!saves[0].use_88_key_layout);
            assert!(saves[0].humanization.is_empty());
        }

        #[test]
        fn test_playback_settings_reflected_in_summary() {
            let dir = tempfile::tempdir().unwrap();
            let settings = serde_json::json!({
                "tempo": 0.8,
                "pedal_style": "ai",
                "use_88_key_layout": true,
                "vary_timing": true,
                "enable_tempo_sway": true,
            });
            let data = save_json_with_settings(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
                settings,
            );
            std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].tempo, 80.0);
            assert_eq!(saves[0].pedal_style, "ai");
            assert!(saves[0].use_88_key_layout);
            assert_eq!(
                saves[0].humanization,
                vec!["Vary Timing".to_string(), "Tempo Sway".to_string()]
            );
        }

        #[test]
        fn test_missing_playback_settings_uses_safe_defaults() {
            let dir = tempfile::tempdir().unwrap();
            let mut data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            data["metadata"].as_object_mut().unwrap().remove("playback_settings");
            std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].tempo, 100.0);
            assert_eq!(saves[0].pedal_style, "none");
            assert!(!saves[0].use_88_key_layout);
            assert!(saves[0].humanization.is_empty());
        }

        #[test]
        fn test_non_json_files_ignored() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("notes.txt"), "hello").unwrap();
            assert!(list_saves_logic(dir.path()).is_empty());
        }

        #[test]
        fn test_malformed_json_skipped_others_still_returned() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("bad.json"), "{not valid json").unwrap();
            let data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            std::fs::write(dir.path().join("good.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 1);
            assert_eq!(saves[0].song_name, "song.mid");
        }

        #[test]
        fn test_directory_named_dot_json_skipped_gracefully() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir(dir.path().join("weird.json")).unwrap();
            assert!(list_saves_logic(dir.path()).is_empty());
        }

        #[test]
        fn test_valid_json_failing_schema_validation_skipped() {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join("old.json"),
                serde_json::json!({"metadata": {}, "compiled_events": []}).to_string(),
            )
            .unwrap();
            assert!(list_saves_logic(dir.path()).is_empty());
        }

        #[test]
        fn test_older_version_missing_track_details_skipped() {
            let dir = tempfile::tempdir().unwrap();
            let mut data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            data["metadata"].as_object_mut().unwrap().remove("track_details");
            std::fs::write(dir.path().join("old.json"), data.to_string()).unwrap();
            assert!(list_saves_logic(dir.path()).is_empty());
        }

        #[test]
        fn test_sorted_newest_last_accessed_first() {
            let dir = tempfile::tempdir().unwrap();
            let older = save_json(
                "a.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            let newer = save_json(
                "b.mid",
                "2026-01-02T00:00:00+00:00",
                "2026-01-02T00:00:00+00:00",
                &[1],
                0,
            );
            std::fs::write(dir.path().join("a.json"), older.to_string()).unwrap();
            std::fs::write(dir.path().join("b.json"), newer.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 2);
            assert_eq!(saves[0].song_name, "b.mid");
            assert_eq!(saves[1].song_name, "a.mid");
        }

        #[test]
        fn test_sorts_by_actual_instant_not_lexicographic_string() {
            let dir = tempfile::tempdir().unwrap();
            let actually_later = save_json("later.mid", "x", "2026-01-01T23:00:00-05:00", &[1], 0);
            let actually_earlier =
                save_json("earlier.mid", "x", "2026-01-02T00:30:00+00:00", &[1], 0);
            std::fs::write(dir.path().join("later.json"), actually_later.to_string()).unwrap();
            std::fs::write(dir.path().join("earlier.json"), actually_earlier.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].song_name, "later.mid");
            assert_eq!(saves[1].song_name, "earlier.mid");
        }

        #[test]
        fn test_missing_last_accessed_falls_back_to_creation_timestamp() {
            let dir = tempfile::tempdir().unwrap();
            let mut data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            data["metadata"].as_object_mut().unwrap().remove("last_accessed");
            std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].last_accessed, "2026-01-01T00:00:00+00:00");
        }

        #[test]
        fn test_note_count_sums_across_multiple_tracks() {
            let dir = tempfile::tempdir().unwrap();
            let data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[3, 7],
                0,
            );
            std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].note_count, 10);
            assert_eq!(saves[0].track_count, 2);
        }

        #[test]
        fn test_multiple_valid_saves_all_returned() {
            let dir = tempfile::tempdir().unwrap();
            for i in 0..5 {
                let data = save_json(
                    &format!("song{i}.mid"),
                    "2026-01-01T00:00:00+00:00",
                    "2026-01-01T00:00:00+00:00",
                    &[1],
                    0,
                );
                std::fs::write(dir.path().join(format!("song{i}.json")), data.to_string()).unwrap();
            }
            assert_eq!(list_saves_logic(dir.path()).len(), 5);
        }

        #[test]
        fn test_python_naive_isoformat_timestamps_sort_correctly_not_as_min_utc() {
            let dir = tempfile::tempdir().unwrap();
            let older = save_json("a.mid", "2026-01-01T00:00:00.123456", "2026-01-01T00:00:00.123456", &[1], 0);
            let newer = save_json("b.mid", "2026-01-02T00:00:00", "2026-01-02T00:00:00", &[1], 0);
            std::fs::write(dir.path().join("a.json"), older.to_string()).unwrap();
            std::fs::write(dir.path().join("b.json"), newer.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 2);
            assert_eq!(saves[0].song_name, "b.mid");
            assert_eq!(saves[1].song_name, "a.mid");
        }

        #[test]
        fn test_python_naive_timestamp_sorts_above_unparseable_garbage() {
            let dir = tempfile::tempdir().unwrap();
            let naive = save_json("a.mid", "2026-01-01T00:00:00", "2026-01-01T00:00:00", &[1], 0);
            let garbage = save_json("b.mid", "not-a-timestamp", "not-a-timestamp", &[1], 0);
            std::fs::write(dir.path().join("a.json"), naive.to_string()).unwrap();
            std::fs::write(dir.path().join("b.json"), garbage.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves[0].song_name, "a.mid");
            assert_eq!(saves[1].song_name, "b.mid");
        }

        #[test]
        fn test_missing_source_midi_filename_defaults_instead_of_dropping_save() {
            let dir = tempfile::tempdir().unwrap();
            let mut data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            data["metadata"].as_object_mut().unwrap().remove("source_midi_filename");
            std::fs::write(dir.path().join("song.json"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 1);
            assert_eq!(saves[0].song_name, "Unknown MIDI");
        }

        #[test]
        fn test_uppercase_json_extension_included_case_insensitively() {
            let dir = tempfile::tempdir().unwrap();
            let data = save_json(
                "song.mid",
                "2026-01-01T00:00:00+00:00",
                "2026-01-01T00:00:00+00:00",
                &[1],
                0,
            );
            std::fs::write(dir.path().join("song.JSON"), data.to_string()).unwrap();

            let saves = list_saves_logic(dir.path());
            assert_eq!(saves.len(), 1);
        }
    }

    mod test_delete_save_logic {
        use super::*;

        #[test]
        fn test_deletes_existing_file() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("song.json");
            std::fs::write(&path, "{}").unwrap();

            assert!(delete_save_logic(&path).is_ok());
            assert!(!path.exists());
        }

        #[test]
        fn test_missing_file_errors() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("does_not_exist.json");
            assert!(delete_save_logic(&path).is_err());
        }

        #[test]
        fn test_refuses_non_json_extension() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("song.mid");
            std::fs::write(&path, "not a save").unwrap();

            assert!(delete_save_logic(&path).is_err());
            assert!(path.exists());
        }

        #[test]
        fn test_uppercase_json_extension_accepted() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("song.JSON");
            std::fs::write(&path, "{}").unwrap();

            assert!(delete_save_logic(&path).is_ok());
            assert!(!path.exists());
        }

        #[test]
        fn test_refuses_file_with_no_extension() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("noext");
            std::fs::write(&path, "x").unwrap();

            assert!(delete_save_logic(&path).is_err());
            assert!(path.exists());
        }

        #[test]
        fn test_directory_named_dot_json_does_not_panic() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("weird.json");
            std::fs::create_dir(&path).unwrap();

            assert!(delete_save_logic(&path).is_err());
        }
    }

    mod test_sanitize_save_name {
        use super::*;

        #[test]
        fn test_simple_name_passes_through() {
            assert_eq!(sanitize_save_name("My Practice Run").unwrap(), "My Practice Run");
        }

        #[test]
        fn test_empty_name_errors() {
            assert!(sanitize_save_name("").is_err());
        }

        #[test]
        fn test_whitespace_only_errors() {
            assert!(sanitize_save_name("   ").is_err());
        }

        #[test]
        fn test_trailing_json_extension_stripped_case_insensitively() {
            assert_eq!(sanitize_save_name("song.json").unwrap(), "song");
            assert_eq!(sanitize_save_name("song.JSON").unwrap(), "song");
            assert_eq!(sanitize_save_name("song.Json").unwrap(), "song");
        }

        #[test]
        fn test_only_extension_after_stripping_errors() {
            assert!(sanitize_save_name(".json").is_err());
        }

        #[test]
        fn test_forward_slash_rejected() {
            assert!(sanitize_save_name("a/b").is_err());
        }

        #[test]
        fn test_backslash_rejected() {
            assert!(sanitize_save_name("a\\b").is_err());
        }

        #[test]
        fn test_parent_traversal_rejected() {
            assert!(sanitize_save_name("../evil").is_err());
            assert!(sanitize_save_name("..\\evil").is_err());
        }

        #[test]
        fn test_bare_dot_and_dotdot_rejected() {
            assert!(sanitize_save_name(".").is_err());
            assert!(sanitize_save_name("..").is_err());
        }

        #[test]
        fn test_windows_reserved_characters_rejected() {
            for ch in ['<', '>', ':', '"', '|', '?', '*'] {
                let name = format!("bad{ch}name");
                assert!(sanitize_save_name(&name).is_err(), "expected '{ch}' to be rejected");
            }
        }

        #[test]
        fn test_control_characters_rejected() {
            assert!(sanitize_save_name("bad\u{0000}name").is_err());
            assert!(sanitize_save_name("bad\tname").is_err());
        }

        #[test]
        fn test_trailing_dot_trimmed_away() {
            assert_eq!(sanitize_save_name("name.").unwrap(), "name");
        }

        #[test]
        fn test_trailing_space_trimmed_away() {
            assert_eq!(sanitize_save_name("name ").unwrap(), "name");
        }

        #[test]
        fn test_all_trailing_dots_and_spaces_trimmed() {
            assert_eq!(sanitize_save_name("name. . .").unwrap(), "name");
        }

        #[test]
        fn test_name_that_is_only_dots_and_spaces_errors() {
            assert!(sanitize_save_name(". . .").is_err());
        }

        #[test]
        fn test_unicode_name_accepted() {
            assert_eq!(sanitize_save_name("Clair de Lune \u{1f3b9}").unwrap(), "Clair de Lune \u{1f3b9}");
        }

        #[test]
        fn test_leading_and_trailing_whitespace_trimmed() {
            assert_eq!(sanitize_save_name("  song  ").unwrap(), "song");
        }

        #[test]
        fn test_multibyte_name_near_extension_check_does_not_panic() {
            assert_eq!(sanitize_save_name("\u{c9}tude").unwrap(), "\u{c9}tude");
            assert_eq!(sanitize_save_name("\u{65e5}\u{672c}\u{8a9e}").unwrap(), "\u{65e5}\u{672c}\u{8a9e}");
            assert_eq!(
                sanitize_save_name("\u{41f}\u{440}\u{435}\u{43b}\u{44e}\u{434}\u{438}\u{44f}").unwrap(),
                "\u{41f}\u{440}\u{435}\u{43b}\u{44e}\u{434}\u{438}\u{44f}"
            );
            assert_eq!(sanitize_save_name("\u{1f3b9}\u{1f3b9}").unwrap(), "\u{1f3b9}\u{1f3b9}");
        }

        #[test]
        fn test_multibyte_name_with_json_suffix_still_strips_extension() {
            assert_eq!(sanitize_save_name("\u{65e5}\u{672c}.json").unwrap(), "\u{65e5}\u{672c}");
        }

        #[test]
        fn test_windows_reserved_device_names_rejected() {
            for name in ["CON", "con", "NUL", "PRN", "AUX", "COM1", "LPT9"] {
                assert!(sanitize_save_name(name).is_err(), "expected '{name}' to be rejected");
            }
        }

        #[test]
        fn test_reserved_device_name_as_substring_is_allowed() {
            assert_eq!(sanitize_save_name("CONcerto").unwrap(), "CONcerto");
        }
    }

    mod test_rename_save_logic {
        use super::*;

        #[test]
        fn test_renames_and_returns_new_path() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            std::fs::write(&path, "{\"a\":1}").unwrap();

            let new_path = rename_save_logic(&path, "new name").unwrap();
            assert!(new_path.ends_with("new name.json"));
            assert!(!path.exists());
            assert!(std::path::Path::new(&new_path).exists());
        }

        #[test]
        fn test_preserves_file_content() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            std::fs::write(&path, "{\"marker\":\"hello\"}").unwrap();

            let new_path = rename_save_logic(&path, "renamed").unwrap();
            let content = std::fs::read_to_string(new_path).unwrap();
            assert!(content.contains("hello"));
        }

        #[test]
        fn test_missing_source_file_errors() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("does_not_exist.json");
            assert!(rename_save_logic(&path, "whatever").is_err());
        }

        #[test]
        fn test_invalid_new_name_errors_and_leaves_original_untouched() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            std::fs::write(&path, "{}").unwrap();

            assert!(rename_save_logic(&path, "bad/name").is_err());
            assert!(path.exists());
        }

        #[test]
        fn test_target_name_already_taken_errors_and_leaves_both_files_untouched() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            let taken_path = dir.path().join("taken.json");
            std::fs::write(&path, "{\"marker\":\"old\"}").unwrap();
            std::fs::write(&taken_path, "{\"marker\":\"taken\"}").unwrap();

            assert!(rename_save_logic(&path, "taken").is_err());
            assert!(path.exists());
            assert_eq!(std::fs::read_to_string(&taken_path).unwrap(), "{\"marker\":\"taken\"}");
        }

        #[test]
        fn test_renaming_to_its_own_current_name_succeeds() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("same.json");
            std::fs::write(&path, "{}").unwrap();

            assert!(rename_save_logic(&path, "same").is_ok());
        }

        #[test]
        fn test_new_name_with_json_extension_does_not_double_up() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            std::fs::write(&path, "{}").unwrap();

            let new_path = rename_save_logic(&path, "renamed.json").unwrap();
            assert!(new_path.ends_with("renamed.json"));
            assert!(!new_path.ends_with("renamed.json.json"));
        }

        #[test]
        fn test_refuses_to_rename_a_non_json_source_file() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("song.mid");
            std::fs::write(&path, "not a save").unwrap();

            assert!(rename_save_logic(&path, "renamed").is_err());
            assert!(path.exists());
        }

        #[test]
        fn test_reserved_device_name_rejected_and_leaves_original_untouched() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("old.json");
            std::fs::write(&path, "{}").unwrap();

            assert!(rename_save_logic(&path, "CON").is_err());
            assert!(path.exists());
        }

        #[test]
        fn test_case_only_rename_succeeds_instead_of_reporting_name_taken() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("song.json");
            std::fs::write(&path, "{\"marker\":\"hi\"}").unwrap();

            let new_path = rename_save_logic(&path, "Song").unwrap();
            assert!(new_path.ends_with("Song.json"));
            assert!(std::path::Path::new(&new_path).exists());
            assert_eq!(std::fs::read_to_string(&new_path).unwrap(), "{\"marker\":\"hi\"}");
        }
    }

    mod test_hotkey_event_to_channel_payload {
        use super::*;

        #[test]
        fn test_bound_updated() {
            let (ch, payload) =
                hotkey_event_to_channel_payload(&HotkeyEvent::BoundUpdated("Alt+P".to_string()));
            assert_eq!(ch, "hotkey_bound_updated");
            assert_eq!(payload, serde_json::json!("Alt+P"));
        }

        #[test]
        fn test_bound_save_updated() {
            let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::BoundSaveUpdated(
                "Shift+K".to_string(),
            ));
            assert_eq!(ch, "hotkey_bound_save_updated");
            assert_eq!(payload, serde_json::json!("Shift+K"));
        }

        #[test]
        fn test_toggle_requested() {
            let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::ToggleRequested);
            assert_eq!(ch, "hotkey_toggle_requested");
            assert_eq!(payload, Value::Null);
        }

        #[test]
        fn test_save_requested() {
            let (ch, payload) = hotkey_event_to_channel_payload(&HotkeyEvent::SaveRequested);
            assert_eq!(ch, "hotkey_save_requested");
            assert_eq!(payload, Value::Null);
        }
    }

    mod test_reconstruct_notes_for_visualizer {
        use super::*;

        fn press(t: f64, pitch: i32, vel: i32) -> KeyEvent {
            let mut e = KeyEvent::new(t, 2, "press", "a");
            e.pitch = Some(pitch);
            e.velocity = Some(vel);
            e
        }

        fn release(t: f64, pitch: i32) -> KeyEvent {
            let mut e = KeyEvent::new(t, 4, "release", "a");
            e.pitch = Some(pitch);
            e
        }

        #[test]
        fn test_empty_events_produce_no_notes() {
            assert!(reconstruct_notes_for_visualizer(&[]).is_empty());
        }

        #[test]
        fn test_simple_press_release_pair() {
            let events = [press(1.0, 60, 80), release(1.5, 60)];
            let notes = reconstruct_notes_for_visualizer(&events);
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].pitch, 60);
            assert_eq!(notes[0].velocity, 80);
            assert!((notes[0].start_time - 1.0).abs() < 1e-9);
            assert!((notes[0].duration - 0.5).abs() < 1e-9);
        }

        #[test]
        fn test_dangling_press_without_release_dropped() {
            let events = [press(1.0, 60, 80)];
            assert!(reconstruct_notes_for_visualizer(&events).is_empty());
        }

        #[test]
        fn test_release_without_prior_press_ignored() {
            let events = [release(1.0, 60)];
            assert!(reconstruct_notes_for_visualizer(&events).is_empty());
        }

        #[test]
        fn test_fifo_pairing_for_repeated_same_pitch() {
            let events = [
                press(0.0, 60, 64),
                press(1.0, 60, 90),
                release(0.5, 60),
                release(2.0, 60),
            ];
            let notes = reconstruct_notes_for_visualizer(&events);
            assert_eq!(notes.len(), 2);
            assert!((notes[0].start_time - 0.0).abs() < 1e-9);
            assert!((notes[0].duration - 0.5).abs() < 1e-9);
            assert!((notes[1].start_time - 1.0).abs() < 1e-9);
            assert!((notes[1].duration - 1.0).abs() < 1e-9);
        }

        #[test]
        fn test_events_without_pitch_skipped() {
            let down = KeyEvent::new(0.0, 1, "pedal", "down");
            let up = KeyEvent::new(1.0, 0, "pedal", "up");
            assert!(reconstruct_notes_for_visualizer(&[down, up]).is_empty());
        }

        #[test]
        fn test_missing_velocity_defaults_to_64() {
            let mut p = press(0.0, 60, 0);
            p.velocity = None;
            let notes = reconstruct_notes_for_visualizer(&[p, release(0.5, 60)]);
            assert_eq!(notes[0].velocity, 64);
        }

        #[test]
        fn test_zero_duration_note_dropped() {
            let events = [press(1.0, 60, 64), release(1.0, 60)];
            assert!(reconstruct_notes_for_visualizer(&events).is_empty());
        }

        #[test]
        fn test_pedal_events_ignored_for_note_reconstruction() {
            let pedal_down = KeyEvent::new(0.0, 1, "pedal", "down");
            let events = [pedal_down, press(0.0, 60, 64), release(0.5, 60)];
            let notes = reconstruct_notes_for_visualizer(&events);
            assert_eq!(notes.len(), 1);
        }

        #[test]
        fn test_note_ids_sequential() {
            let events = [
                press(0.0, 60, 64),
                release(0.5, 60),
                press(1.0, 62, 64),
                release(1.5, 62),
            ];
            let notes = reconstruct_notes_for_visualizer(&events);
            assert_eq!(notes[0].id, 0);
            assert_eq!(notes[1].id, 1);
        }
    }

    mod test_resume_from_save_logic {
        use super::*;
        use crate::state::PlaybackSession;

        fn events_value(events: Vec<Value>) -> Value {
            serde_json::json!({
                "metadata": {"track_details": [], "compiled_pedal_count": 0},
                "compiled_events": events,
            })
        }

        fn press_v(t: f64, pitch: i32) -> Value {
            serde_json::json!({"time": t, "priority": 2, "action": "press", "key_char": "a", "pitch": pitch, "velocity": 64})
        }

        fn release_v(t: f64, pitch: i32) -> Value {
            serde_json::json!({"time": t, "priority": 4, "action": "release", "key_char": "a", "pitch": pitch, "velocity": null})
        }

        #[test]
        fn test_valid_save_populates_session_for_playback() {
            let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();

            assert_eq!(result.final_notes.len(), 1);
            assert!((result.total_dur - 0.5).abs() < 1e-9);
            assert_eq!(session.merged_events.as_ref().unwrap().len(), 2);
            assert!(session.tempo_map.is_some());
            assert!((session.total_dur - 0.5).abs() < 1e-9);
        }

        #[test]
        fn test_returns_track_count_and_pedal_count_from_save_metadata() {
            let mut data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
            data["metadata"]["track_details"] = serde_json::json!([{"name": "Melody"}, {"name": "Bass"}]);
            data["metadata"]["compiled_pedal_count"] = serde_json::json!(7);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();

            assert_eq!(result.track_count, 2);
            assert_eq!(result.pedal_count, 7);
        }

        #[test]
        fn test_missing_metadata_counts_default_to_zero() {
            let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();

            assert_eq!(result.track_count, 0);
            assert_eq!(result.pedal_count, 0);
        }

        #[test]
        fn test_returns_default_tempo_fields_matching_session_tempo_map() {
            let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();

            let session_tempo_map = session.tempo_map.as_ref().unwrap();
            assert_eq!(result.tempo_events, session_tempo_map.events());
            assert_eq!(result.time_signatures, session_tempo_map.time_signatures());
            assert_eq!(
                result.measure_boundaries,
                session_tempo_map.get_measure_boundaries(result.total_dur)
            );
            assert!(!result.measure_boundaries.is_empty());
        }

        #[test]
        fn test_notes_config_snapshot_left_none_so_pedal_recompile_still_gated() {
            let data = events_value(vec![press_v(0.0, 60), release_v(0.5, 60)]);
            let mut session = PlaybackSession::default();
            resume_from_save_logic(&data, &mut session).unwrap();
            assert!(session.notes_config_snapshot.is_none());

            let config = PlaybackConfig::default();
            assert!(compile_pedal_logic(&config, None, &mut session).is_err());
        }

        #[test]
        fn test_empty_compiled_events_errors() {
            let data = events_value(vec![]);
            let mut session = PlaybackSession::default();
            assert!(resume_from_save_logic(&data, &mut session).is_err());
        }

        #[test]
        fn test_malformed_event_deep_in_array_errors_with_index() {
            let mut events = vec![press_v(0.0, 60), release_v(0.5, 60)];
            let mut bad = press_v(1.0, 62);
            bad.as_object_mut().unwrap().remove("time");
            events.push(bad);
            let data = events_value(events);
            let mut session = PlaybackSession::default();
            let err = resume_from_save_logic(&data, &mut session).unwrap_err();
            assert!(err.contains("#2"));
        }

        #[test]
        fn test_out_of_order_events_are_sorted_before_playback() {
            let data = events_value(vec![release_v(0.5, 60), press_v(0.0, 60)]);
            let mut session = PlaybackSession::default();
            resume_from_save_logic(&data, &mut session).unwrap();
            let merged = session.merged_events.unwrap();
            assert_eq!(merged[0].action, "press");
            assert_eq!(merged[1].action, "release");
        }

        #[test]
        fn test_total_dur_uses_max_time_not_last_array_element() {
            let data = events_value(vec![
                press_v(5.0, 60),
                release_v(5.5, 60),
                press_v(0.0, 62),
                release_v(0.2, 62),
            ]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();
            assert!((result.total_dur - 5.5).abs() < 1e-9);
        }

        #[test]
        fn test_dangling_press_still_included_in_merged_events_but_not_visualized() {
            let data = events_value(vec![press_v(0.0, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();
            assert!(result.final_notes.is_empty());
            assert_eq!(session.merged_events.unwrap().len(), 1);
        }

        #[test]
        fn test_pedal_events_pass_through_to_merged_events() {
            let pedal_down =
                serde_json::json!({"time": 0.1, "priority": 1, "action": "pedal", "key_char": "down"});
            let pedal_up =
                serde_json::json!({"time": 0.9, "priority": 0, "action": "pedal", "key_char": "up"});
            let data = events_value(vec![press_v(0.0, 60), pedal_down, release_v(0.5, 60), pedal_up]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();
            assert_eq!(result.final_notes.len(), 1);
            assert_eq!(session.merged_events.unwrap().len(), 4);
        }

        #[test]
        fn test_invalid_save_data_rejected_before_touching_session() {
            let data = serde_json::json!({"nope": true});
            let mut session = PlaybackSession::default();
            assert!(resume_from_save_logic(&data, &mut session).is_err());
            assert!(session.merged_events.is_none());
        }

        #[test]
        fn test_all_events_at_time_zero_total_dur_floors_to_one() {
            let data = events_value(vec![press_v(0.0, 60), release_v(0.0, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();
            assert_eq!(result.total_dur, 1.0);
            assert_eq!(session.total_dur, 1.0);
        }

        #[test]
        fn test_negative_time_events_total_dur_floors_to_one_not_negative() {
            let data = events_value(vec![press_v(-5.0, 60), release_v(-3.0, 60)]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session).unwrap();
            assert_eq!(result.total_dur, 1.0);
        }

        #[test]
        fn test_float_valued_priority_and_pitch_are_coerced_not_rejected() {
            let event = serde_json::json!({
                "time": 0.0, "priority": 2.0, "action": "press", "key_char": "a", "pitch": 60.0, "velocity": 64.0
            });
            let data = events_value(vec![event]);
            let mut session = PlaybackSession::default();
            let result = resume_from_save_logic(&data, &mut session);
            assert!(result.is_ok(), "float-valued numeric fields should be coerced, got {result:?}");
            assert_eq!(session.merged_events.unwrap()[0].pitch, Some(60));
        }

        #[test]
        fn test_out_of_i32_range_pitch_dropped_not_silently_truncated() {
            let event = serde_json::json!({
                "time": 0.0, "priority": 2, "action": "press", "key_char": "a",
                "pitch": 5_000_000_000i64, "velocity": 64
            });
            let data = events_value(vec![event]);
            let mut session = PlaybackSession::default();
            resume_from_save_logic(&data, &mut session).unwrap();
            let merged = session.merged_events.unwrap();
            assert_eq!(merged[0].pitch, None);
        }
    }

    mod test_translate_sheet_to_notes {
        use super::*;

        #[test]
        fn test_maps_char_to_pitch_via_key_mapper() {
            let km = KeyMapper::new(false);
            let ch = km.get_key_for_pitch(60).unwrap();
            let notes = translate_sheet_to_notes(ch.to_string(), 60.0, false).unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].pitch, 60);
        }

        #[test]
        fn test_empty_sheet_produces_no_notes() {
            let notes = translate_sheet_to_notes(String::new(), 120.0, false).unwrap();
            assert!(notes.is_empty());
        }
    }

    mod test_notes_to_sheet {
        use super::*;

        #[test]
        fn test_round_trips_through_key_mapper_and_tempo_map() {
            let km = KeyMapper::new(false);
            let ch = km.get_key_for_pitch(60).unwrap();
            let note = Note::new(0, 60, 64, 0.0, 0.25);
            let sheet = notes_to_sheet(vec![note], false, vec![(0.0, 500_000)], vec![]).unwrap();
            assert!(sheet.starts_with(ch));
        }

        #[test]
        fn test_empty_notes_produces_empty_string() {
            let sheet = notes_to_sheet(vec![], false, vec![], vec![]).unwrap();
            assert_eq!(sheet, "");
        }
    }

    mod test_extract_pedal_intervals {
        use super::*;

        #[test]
        fn test_empty_returns_empty() {
            assert_eq!(extract_pedal_intervals(&[]), vec![]);
        }

        #[test]
        fn test_single_down_up_pair() {
            let events = [pedal_ev(1.0, "down"), pedal_ev(2.0, "up")];
            assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
        }

        #[test]
        fn test_multiple_pairs() {
            let events = [
                pedal_ev(0.0, "down"),
                pedal_ev(0.5, "up"),
                pedal_ev(1.0, "down"),
                pedal_ev(1.8, "up"),
            ];
            assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 0.5), (1.0, 1.8)]);
        }

        #[test]
        fn test_unpaired_down_dropped() {
            assert_eq!(extract_pedal_intervals(&[pedal_ev(1.0, "down")]), vec![]);
        }

        #[test]
        fn test_up_without_prior_down_ignored() {
            let events = [pedal_ev(1.0, "up"), pedal_ev(2.0, "down"), pedal_ev(3.0, "up")];
            assert_eq!(extract_pedal_intervals(&events), vec![(2.0, 3.0)]);
        }

        #[test]
        fn test_non_pedal_events_ignored() {
            let events = [
                press_ev(0.0),
                pedal_ev(1.0, "down"),
                press_ev(1.5),
                pedal_ev(2.0, "up"),
            ];
            assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
        }

        #[test]
        fn test_interval_values_preserved() {
            let events = [pedal_ev(0.123, "down"), pedal_ev(4.567, "up")];
            let intervals = extract_pedal_intervals(&events);
            assert!((intervals[0].0 - 0.123).abs() < 1e-9);
            assert!((intervals[0].1 - 4.567).abs() < 1e-9);
        }

        #[test]
        fn test_consecutive_downs_last_wins() {
            let events = [pedal_ev(1.0, "down"), pedal_ev(1.5, "down"), pedal_ev(2.0, "up")];
            assert_eq!(extract_pedal_intervals(&events), vec![(1.5, 2.0)]);
        }

        #[test]
        fn test_consecutive_ups_only_first_consumed() {
            let events = [pedal_ev(1.0, "down"), pedal_ev(2.0, "up"), pedal_ev(3.0, "up")];
            assert_eq!(extract_pedal_intervals(&events), vec![(1.0, 2.0)]);
        }

        #[test]
        fn test_only_pedal_actions_inspected() {
            let weird = KeyEvent::new(1.0, 1, "pedal", "wiggle");
            let events = [pedal_ev(0.0, "down"), weird, pedal_ev(2.0, "up")];
            assert_eq!(extract_pedal_intervals(&events), vec![(0.0, 2.0)]);
        }
    }

    mod test_apply_hand_assignment {
        use super::*;

        fn note(id: i32, pitch: i32, start_time: f64, hand: &str) -> Note {
            Note {
                id,
                pitch,
                velocity: 64,
                start_time,
                duration: 0.5,
                hand: hand.to_string(),
                original_track_index: -1,
                channel: -1,
            }
        }

        #[test]
        fn test_simulate_hands_off_defaults_by_pitch_threshold() {
            let mut notes = vec![note(0, 40, 0.0, "unknown"), note(1, 70, 1.0, "unknown")];
            let mut cfg = PlaybackConfig::default();
            cfg.simulate_hands = false;
            apply_hand_assignment(&mut notes, &cfg);
            assert_eq!(notes[0].hand, "left");
            assert_eq!(notes[1].hand, "right");
        }

        #[test]
        fn test_simulate_hands_off_never_touches_already_assigned() {
            let mut notes = vec![note(0, 90, 0.0, "left")];
            let mut cfg = PlaybackConfig::default();
            cfg.simulate_hands = false;
            apply_hand_assignment(&mut notes, &cfg);
            assert_eq!(notes[0].hand, "left");
        }

        #[test]
        fn test_simulate_hands_on_uses_group_average() {
            let mut notes = vec![
                note(0, 40, 0.0, "unknown"),
                note(1, 50, 0.0, "unknown"),
            ];
            let mut cfg = PlaybackConfig::default();
            cfg.simulate_hands = true;
            apply_hand_assignment(&mut notes, &cfg);
            assert_eq!(notes[0].hand, "left");
            assert_eq!(notes[1].hand, "left");
        }

        #[test]
        fn test_pitch_60_defaults_right_when_simulate_off() {
            let mut notes = vec![note(0, 60, 0.0, "unknown")];
            let mut cfg = PlaybackConfig::default();
            cfg.simulate_hands = false;
            apply_hand_assignment(&mut notes, &cfg);
            assert_eq!(notes[0].hand, "right");
        }
    }

    mod test_prepare_notes_from_bytes {
        use super::*;
        use midly::num::{u15, u28, u4, u7};
        use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

        fn simple_midi_two_tracks() -> Vec<u8> {
            let header = Header::new(Format::Parallel, Timing::Metrical(u15::from(480)));
            let track_a = vec![
                TrackEvent {
                    delta: u28::from(0),
                    kind: TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(40),
                            vel: u7::from(64),
                        },
                    },
                },
                TrackEvent {
                    delta: u28::from(480),
                    kind: TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(40),
                            vel: u7::from(0),
                        },
                    },
                },
            ];
            let track_b = vec![
                TrackEvent {
                    delta: u28::from(0),
                    kind: TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOn {
                            key: u7::from(72),
                            vel: u7::from(64),
                        },
                    },
                },
                TrackEvent {
                    delta: u28::from(480),
                    kind: TrackEventKind::Midi {
                        channel: u4::from(0),
                        message: MidiMessage::NoteOff {
                            key: u7::from(72),
                            vel: u7::from(0),
                        },
                    },
                },
            ];
            let smf = Smf {
                header,
                tracks: vec![track_a, track_b],
            };
            let mut buf = Vec::new();
            smf.write(&mut buf).unwrap();
            buf
        }

        #[test]
        fn test_only_selected_tracks_included() {
            let bytes = simple_midi_two_tracks();
            let cfg = PlaybackConfig::default();
            let (notes, _, _) =
                prepare_notes_from_bytes(&bytes, &cfg, &[(0, "Right Hand".to_string())]).unwrap();
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].pitch, 40);
        }

        #[test]
        fn test_role_assignment_sets_hand() {
            let bytes = simple_midi_two_tracks();
            let cfg = PlaybackConfig::default();
            let (notes, _, _) = prepare_notes_from_bytes(
                &bytes,
                &cfg,
                &[(0, "Left Hand".to_string()), (1, "Right Hand".to_string())],
            )
            .unwrap();
            let left: Vec<&Note> = notes.iter().filter(|n| n.pitch == 40).collect();
            let right: Vec<&Note> = notes.iter().filter(|n| n.pitch == 72).collect();
            assert_eq!(left[0].hand, "left");
            assert_eq!(right[0].hand, "right");
        }

        #[test]
        fn test_notes_sorted_by_start_time_across_tracks() {
            let bytes = simple_midi_two_tracks();
            let cfg = PlaybackConfig::default();
            let (notes, _, _) = prepare_notes_from_bytes(
                &bytes,
                &cfg,
                &[(0, "Left Hand".to_string()), (1, "Right Hand".to_string())],
            )
            .unwrap();
            for w in notes.windows(2) {
                assert!(w[0].start_time <= w[1].start_time);
            }
        }

        #[test]
        fn test_no_selected_tracks_produces_no_notes() {
            let bytes = simple_midi_two_tracks();
            let cfg = PlaybackConfig::default();
            let (notes, _, _) = prepare_notes_from_bytes(&bytes, &cfg, &[]).unwrap();
            assert!(notes.is_empty());
        }

        #[test]
        fn test_unrecognized_role_leaves_hand_for_later_assignment() {
            let bytes = simple_midi_two_tracks();
            let mut cfg = PlaybackConfig::default();
            cfg.simulate_hands = false;
            let (notes, _, _) =
                prepare_notes_from_bytes(&bytes, &cfg, &[(0, "Unassigned".to_string())]).unwrap();
            assert_eq!(notes[0].hand, "left");
        }
    }

    struct NoopWake;

    impl std::task::Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    type BoxedFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

    fn poll_once<T>(fut: &mut BoxedFuture<T>) -> std::task::Poll<T> {
        let waker = std::task::Waker::from(Arc::new(NoopWake));
        let mut cx = std::task::Context::from_waker(&waker);
        std::future::Future::poll(fut.as_mut(), &mut cx)
    }

    fn hermetic_state(dir: &Path) -> AppState {
        use crate::managers::config_manager::ConfigManager;
        use crate::managers::hotkey_manager::HotkeyManager;
        use crate::managers::theme_manager::ThemeManager;
        use std::sync::Mutex;

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

    mod test_clear_loaded_song {
        use super::*;

        fn populated_state(dir: &Path) -> AppState {
            let state = hermetic_state(dir);
            {
                let mut session = state.playback_session.lock().unwrap();
                session.final_notes = Some(Vec::new());
                session.note_events = Some(Vec::new());
                session.pedal_events = Some(Vec::new());
                session.merged_events = Some(Vec::new());
                session.total_dur = 12.5;
                session.midi_pedal_events.push((1.0, true));
            }
            *state.parsed_tracks.lock().unwrap() = Some(Vec::new());
            *state.loaded_pedal_count.lock().unwrap() = 3;
            state.midi_pedal_events.lock().unwrap().push((1.0, true));
            state
        }

        #[test]
        fn test_resets_the_session_to_its_default() {
            let dir = tempfile::tempdir().unwrap();
            let state = populated_state(dir.path());
            clear_loaded_song_logic(&state).unwrap();
            let session = state.playback_session.lock().unwrap();
            assert!(session.final_notes.is_none());
            assert!(session.note_events.is_none());
            assert!(session.pedal_events.is_none());
            assert!(session.merged_events.is_none());
            assert_eq!(session.total_dur, 0.0);
            assert!(session.midi_pedal_events.is_empty());
        }

        #[test]
        fn test_clears_the_parsed_file_and_midi_pedal_state() {
            let dir = tempfile::tempdir().unwrap();
            let state = populated_state(dir.path());
            clear_loaded_song_logic(&state).unwrap();
            assert!(state.parsed_tracks.lock().unwrap().is_none());
            assert!(state.parsed_tempo_map.lock().unwrap().is_none());
            assert_eq!(*state.loaded_pedal_count.lock().unwrap(), 0);
            assert!(state.midi_pedal_events.lock().unwrap().is_empty());
        }

        #[test]
        fn test_clearing_an_already_empty_state_succeeds() {
            let dir = tempfile::tempdir().unwrap();
            let state = hermetic_state(dir.path());
            clear_loaded_song_logic(&state).unwrap();
            clear_loaded_song_logic(&state).unwrap();
        }
    }

    mod test_run_blocking {
        use super::*;

        #[test]
        fn test_returns_the_ok_value() {
            let result = tauri::async_runtime::block_on(run_blocking(|| Ok(41 + 1)));
            assert_eq!(result, Ok(42));
        }

        #[test]
        fn test_returns_the_err_string_unchanged() {
            let result: Result<(), String> =
                tauri::async_runtime::block_on(run_blocking(|| Err("boom".to_string())));
            assert_eq!(result, Err("boom".to_string()));
        }

        #[test]
        fn test_closure_runs_on_a_different_thread_than_the_caller() {
            let caller = std::thread::current().id();
            let worker = tauri::async_runtime::block_on(run_blocking(|| {
                Ok(std::thread::current().id())
            }))
            .unwrap();
            assert_ne!(caller, worker);
        }

        #[test]
        fn test_polling_does_not_run_the_closure_inline() {
            let (release_tx, release_rx) = mpsc::channel::<()>();
            let (started_tx, started_rx) = mpsc::channel::<()>();
            let mut fut: BoxedFuture<Result<u8, String>> = Box::pin(run_blocking(move || {
                started_tx.send(()).unwrap();
                let _ = release_rx.recv_timeout(Duration::from_secs(5));
                Ok(7)
            }));

            assert!(poll_once(&mut fut).is_pending());
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(poll_once(&mut fut).is_pending());

            release_tx.send(()).unwrap();
            assert_eq!(tauri::async_runtime::block_on(fut), Ok(7));
        }

        #[test]
        fn test_a_panicking_closure_becomes_an_err_instead_of_unwinding_into_the_caller() {
            let result: Result<(), String> = tauri::async_runtime::block_on(run_blocking(
                || -> Result<(), String> { panic!("worker exploded") },
            ));
            let message = result.unwrap_err();
            assert!(message.starts_with("Background task failed"), "{message}");
        }

        #[test]
        fn test_the_pool_keeps_working_after_a_panicking_closure() {
            let _: Result<(), String> = tauri::async_runtime::block_on(run_blocking(
                || -> Result<(), String> { panic!("worker exploded") },
            ));
            let result = tauri::async_runtime::block_on(run_blocking(|| Ok("still alive")));
            assert_eq!(result, Ok("still alive"));
        }

        #[test]
        fn test_many_closures_run_concurrently_rather_than_one_after_another() {
            use std::sync::atomic::{AtomicUsize, Ordering};

            let running = Arc::new(AtomicUsize::new(0));
            let peak = Arc::new(AtomicUsize::new(0));
            let futures: Vec<_> = (0..4)
                .map(|i| {
                    let running = Arc::clone(&running);
                    let peak = Arc::clone(&peak);
                    run_blocking(move || {
                        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(300));
                        running.fetch_sub(1, Ordering::SeqCst);
                        Ok(i)
                    })
                })
                .collect();
            let results = tauri::async_runtime::block_on(async move {
                let handles: Vec<_> = futures
                    .into_iter()
                    .map(tauri::async_runtime::spawn)
                    .collect();
                let mut out = Vec::new();
                for h in handles {
                    out.push(h.await.unwrap().unwrap());
                }
                out
            });
            assert_eq!(results, vec![0, 1, 2, 3]);
            assert!(peak.load(Ordering::SeqCst) >= 2);
        }
    }

    mod test_pedal_model_sharing {
        use super::*;

        fn model_path() -> std::path::PathBuf {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("pedal_bilstm.safetensors")
        }

        fn real_model() -> Arc<PedalModel> {
            Arc::new(PedalModel::load(model_path()).unwrap())
        }

        fn config_with_style(style: &str) -> PlaybackConfig {
            let mut config = PlaybackConfig::default();
            config.pedal_style = style.to_string();
            config
        }

        fn forbidden_resolver() -> Result<std::path::PathBuf, String> {
            panic!("the resource path must not be resolved here")
        }

        fn state_in(dir: &tempfile::TempDir) -> AppState {
            hermetic_state(dir.path())
        }

        #[test]
        fn test_styles_that_do_not_use_the_model_never_resolve_a_path_or_load_it() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            for style in ["none", "harmonic", "legato"] {
                let model =
                    pedal_model_for(&state, &config_with_style(style), forbidden_resolver).unwrap();
                assert!(model.is_none(), "{style}");
            }
            assert!(state.pedal_model.lock().unwrap().is_none());
        }

        #[test]
        fn test_ai_and_hybrid_styles_return_the_cached_model_without_reloading() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            let stored = real_model();
            *state.pedal_model.lock().unwrap() = Some(Arc::clone(&stored));
            for style in ["ai", "hybrid"] {
                let got = pedal_model_for(&state, &config_with_style(style), forbidden_resolver)
                    .unwrap()
                    .unwrap();
                assert!(Arc::ptr_eq(&got, &stored), "{style}");
            }
        }

        #[test]
        fn test_the_first_call_loads_the_model_once_and_every_later_call_shares_it() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            let first = pedal_model_for(&state, &config_with_style("ai"), || Ok(model_path()))
                .unwrap()
                .unwrap();
            let second = pedal_model_for(&state, &config_with_style("hybrid"), forbidden_resolver)
                .unwrap()
                .unwrap();
            assert!(Arc::ptr_eq(&first, &second));
            let stored = state.pedal_model.lock().unwrap().clone().unwrap();
            assert!(Arc::ptr_eq(&first, &stored));
        }

        #[test]
        fn test_the_mutex_is_free_while_a_caller_keeps_using_the_returned_model() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            *state.pedal_model.lock().unwrap() = Some(real_model());
            let in_use = pedal_model_for(&state, &config_with_style("ai"), forbidden_resolver)
                .unwrap()
                .unwrap();
            assert!(
                state.pedal_model.try_lock().is_ok(),
                "a caller using the model must not keep the AppState mutex locked"
            );
            drop(in_use);
        }

        #[test]
        fn test_the_model_stays_usable_from_another_thread_while_the_mutex_is_held_elsewhere() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            *state.pedal_model.lock().unwrap() = Some(real_model());
            let in_use = pedal_model_for(&state, &config_with_style("ai"), forbidden_resolver)
                .unwrap()
                .unwrap();

            std::thread::scope(|scope| {
                let (locked_tx, locked_rx) = mpsc::channel();
                let (release_tx, release_rx) = mpsc::channel::<()>();
                let state_ref = &state;
                let holder = scope.spawn(move || {
                    let _guard = state_ref.pedal_model.lock().unwrap();
                    locked_tx.send(()).unwrap();
                    let _ = release_rx.recv_timeout(Duration::from_secs(5));
                });
                locked_rx.recv_timeout(Duration::from_secs(5)).unwrap();

                let preds = in_use.forward(&vec![0.0f32; 10 * FEATURES], 10).unwrap();
                assert_eq!(preds.len(), 10);

                release_tx.send(()).unwrap();
                holder.join().unwrap();
            });
        }

        #[test]
        fn test_a_failing_path_resolver_is_an_error_and_leaves_the_cache_empty() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            let result = pedal_model_for(&state, &config_with_style("ai"), || {
                Err("no such resource".to_string())
            });
            assert_eq!(result.err(), Some("no such resource".to_string()));
            assert!(state.pedal_model.lock().unwrap().is_none());
        }

        #[test]
        fn test_an_unreadable_model_file_is_an_error_and_leaves_the_cache_empty() {
            let dir = tempfile::tempdir().unwrap();
            let state = state_in(&dir);
            let junk = dir.path().join("junk.safetensors");
            std::fs::write(&junk, b"not a safetensors file").unwrap();
            let result = pedal_model_for(&state, &config_with_style("hybrid"), || Ok(junk));
            assert!(result.is_err());
            assert!(state.pedal_model.lock().unwrap().is_none());
        }
    }
}
