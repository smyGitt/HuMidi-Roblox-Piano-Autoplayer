import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { PlaybackConfig } from "../pages/playback/types";

export interface BackendPlaybackConfig {
  midi_file: string;
  tempo: number;
  transpose: number;
  countdown: boolean;
  use_88_key_layout: boolean;
  pedal_style: string;
  pedal_threshold_on: number;
  pedal_threshold_off: number;
  debug_mode: boolean;
  simulate_hands: boolean;
  vary_velocity: boolean;
  enable_chord_roll: boolean;
  vary_timing: boolean;
  timing_variance: number;
  vary_articulation: boolean;
  articulation: number;
  enable_drift_correction: boolean;
  drift_decay_factor: number;
  enable_mistakes: boolean;
  mistake_chance: number;
  enable_tempo_sway: boolean;
  tempo_sway_intensity: number;
  invert_tempo_sway: boolean;
  use_midi_pedal: boolean;
  use_velocity_accent: boolean;
  use_ai_pedal: boolean;
}

export function toBackendConfig(config: PlaybackConfig, midiFile: string): BackendPlaybackConfig {
  return {
    midi_file: midiFile,
    tempo: config.tempo / 100,
    transpose: config.transpose,
    countdown: config.countdown,
    use_88_key_layout: config.use_88_key_layout,
    pedal_style: config.pedal_style,
    pedal_threshold_on: config.pedal_threshold_on,
    pedal_threshold_off: config.pedal_threshold_off,
    debug_mode: config.debug_mode,
    simulate_hands: config.simulate_hands,
    vary_velocity: false,
    enable_chord_roll: config.enable_chord_roll,
    vary_timing: config.vary_timing,
    timing_variance: config.timing_variance,
    vary_articulation: config.vary_articulation,
    articulation: config.articulation / 100,
    enable_drift_correction: config.enable_drift_correction,
    drift_decay_factor: config.drift_decay_factor / 100,
    enable_mistakes: config.enable_mistakes,
    mistake_chance: config.mistake_chance,
    enable_tempo_sway: config.enable_tempo_sway,
    tempo_sway_intensity: config.tempo_sway_intensity,
    invert_tempo_sway: config.invert_tempo_sway,
    use_midi_pedal: config.use_midi_pedal,
    use_velocity_accent: config.use_velocity_accent,
    use_ai_pedal: false,
  };
}

export interface BackendNote {
  id: number;
  pitch: number;
  velocity: number;
  start_time: number;
  duration: number;
  hand: string;
  original_track_index: number;
  channel: number;
}

export interface TrackSummary {
  index: number;
  name: string;
  note_count: number;
  instrument_name: string;
  is_drum: boolean;
}

export interface ParsedMidiStructure {
  tracks: TrackSummary[];
  initial_bpm: number;
}

export interface SaveSummary {
  path: string;
  filename: string;
  song_name: string;
  created: string;
  last_accessed: string;
  track_count: number;
  note_count: number;
  pedal_count: number;
  tempo: number;
  pedal_style: string;
  use_88_key_layout: boolean;
  humanization: string[];
}

export type TempoEvent = [number, number];
export type TimeSignatureEvent = [number, number, number];
export type MeasureBoundary = [number, number];

export interface ResumedSession {
  final_notes: BackendNote[];
  total_dur: number;
  tempo_events: TempoEvent[];
  time_signatures: TimeSignatureEvent[];
  measure_boundaries: MeasureBoundary[];
  track_count: number;
  pedal_count: number;
}

export interface TimelineData {
  final_notes: BackendNote[];
  total_dur: number;
  tempo_events: TempoEvent[];
  time_signatures: TimeSignatureEvent[];
  measure_boundaries: MeasureBoundary[];
}

export interface PedalData {
  pedal_intervals: [number, number][];
  ai_thresholds: [number, number] | null;
}

export type SelectedTrackInfo = [number, string];

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function parseMidiStructure(filepath: string): Promise<ParsedMidiStructure> {
  return invoke("parse_midi_structure", { filepath });
}

export async function compileNotes(
  config: PlaybackConfig,
  midiFile: string,
  selectedTracksInfo: SelectedTrackInfo[],
): Promise<TimelineData> {
  return invoke("compile_notes", {
    config: toBackendConfig(config, midiFile),
    selectedTracksInfo,
  });
}

export async function compileNotesFromSheet(
  config: PlaybackConfig,
  sheetText: string,
  bpm: number,
): Promise<TimelineData> {
  return invoke("compile_notes_from_sheet", {
    config: toBackendConfig(config, ""),
    sheetText,
    bpm,
  });
}

export async function compilePedal(config: PlaybackConfig, midiFile: string): Promise<PedalData> {
  return invoke("compile_pedal", { config: toBackendConfig(config, midiFile) });
}

export async function compilePedalAndPlay(config: PlaybackConfig, midiFile: string): Promise<PedalData> {
  return invoke("compile_pedal_and_play", { config: toBackendConfig(config, midiFile) });
}

export async function startPlayback(config: PlaybackConfig, midiFile: string): Promise<void> {
  return invoke("start_playback", { config: toBackendConfig(config, midiFile) });
}

export async function togglePause(): Promise<void> {
  return invoke("toggle_pause");
}

export async function stopPlayback(): Promise<void> {
  return invoke("stop_playback");
}

export async function clearLoadedSong(): Promise<void> {
  return invoke("clear_loaded_song");
}

export async function seekPlayback(targetTime: number): Promise<void> {
  return invoke("seek_playback", { targetTime });
}

export async function setSaveDir(path: string): Promise<void> {
  return invoke("set_save_dir", { path });
}

export async function getSaveDir(): Promise<string> {
  return invoke("get_save_dir");
}

export async function setMidiDir(path: string): Promise<void> {
  return invoke("set_midi_dir", { path });
}

export async function getMidiDir(): Promise<string> {
  return invoke("get_midi_dir");
}

export async function loadAppConfig(): Promise<Record<string, unknown>> {
  return invoke("load_app_config");
}

export async function saveAppConfig(patch: Record<string, unknown>): Promise<void> {
  return invoke("save_app_config", { patch });
}

export interface CustomThemeColors {
  name: string;
  bg_primary: string;
  bg_surface: string;
  bg_input: string;
  text_primary: string;
  text_muted: string;
  border: string;
  accent: string;
  accent_play: string;
  accent_stop: string;
  pedal_color: string;
  accent_loaded: string;
  knob_color: string;
  builtin: boolean;
}

export async function getThemesFile(): Promise<string> {
  return invoke("get_themes_file");
}

export async function setThemesDir(path: string): Promise<string> {
  return invoke("set_themes_dir", { path });
}

export async function getActiveThemeName(): Promise<string | null> {
  return invoke("get_active_theme_name");
}

export async function setActiveThemeName(name: string): Promise<void> {
  return invoke("set_active_theme_name", { name });
}

export async function getCustomThemes(): Promise<CustomThemeColors[]> {
  return invoke("get_custom_themes");
}

export async function saveCustomTheme(theme: CustomThemeColors): Promise<void> {
  return invoke("save_custom_theme", { theme });
}

export async function deleteCustomTheme(name: string): Promise<void> {
  return invoke("delete_custom_theme", { name });
}

export async function exportThemeFile(path: string, theme: CustomThemeColors): Promise<void> {
  return invoke("export_theme_file", { path, theme });
}

export async function importThemeFile(path: string): Promise<CustomThemeColors> {
  return invoke("import_theme_file", { path });
}

export async function savePlayback(
  config: PlaybackConfig,
  midiFile: string,
  selectedTracksInfo: SelectedTrackInfo[],
  originalFilename: string,
  saveName: string,
): Promise<string> {
  return invoke("save_playback", {
    config: toBackendConfig(config, midiFile),
    selectedTracksInfo,
    originalFilename,
    saveName,
  });
}

export async function checkSaveName(name: string): Promise<string> {
  return invoke("check_save_name", { name });
}

export async function loadSaveFile(filepath: string): Promise<unknown> {
  return invoke("load_save_file", { filepath });
}

export async function listSaves(dir: string): Promise<SaveSummary[]> {
  return invoke("list_saves", { dir });
}

export async function resumeFromSave(filepath: string): Promise<ResumedSession> {
  return invoke("resume_from_save", { filepath });
}

export async function renameSave(filepath: string, newName: string): Promise<string> {
  return invoke("rename_save", { filepath, newName });
}

export async function deleteSave(filepath: string): Promise<void> {
  return invoke("delete_save", { filepath });
}

export async function translateSheetToNotes(
  sheetText: string,
  bpm: number,
  use88KeyLayout: boolean,
): Promise<BackendNote[]> {
  return invoke("translate_sheet_to_notes", { sheetText, bpm, use88KeyLayout });
}

export async function notesToSheet(
  notes: BackendNote[],
  use88KeyLayout: boolean,
  tempoEvents: TempoEvent[],
  timeSignatures: TimeSignatureEvent[],
): Promise<string> {
  return invoke("notes_to_sheet", { notes, use88KeyLayout, tempoEvents, timeSignatures });
}

export async function startBinding(): Promise<void> {
  return invoke("start_binding");
}

export async function startSaveBinding(): Promise<void> {
  return invoke("start_save_binding");
}

export type UpdateCheckOutcome =
  | { status: "update_available"; tag: string; url: string }
  | { status: "no_update" }
  | { status: "indeterminate" };

export async function checkForUpdatesNow(): Promise<UpdateCheckOutcome> {
  return invoke("check_for_updates_now");
}

export async function getAppVersion(): Promise<string> {
  return getVersion();
}

export async function downloadAndInstallUpdate(): Promise<void> {
  return invoke("download_and_install_update");
}

export type PlayerEventMap = {
  status_updated: string;
  progress_updated: number;
  playback_finished: null;
  visualizer_updated: number[];
  pedal_updated: boolean;
  auto_paused: null;
  error_occurred: string;
  section_changed: number;
  playback_started: null;
  hotkey_bound_updated: string;
  hotkey_bound_save_updated: string;
  hotkey_toggle_requested: null;
  hotkey_save_requested: null;
  "update-available": { tag: string; url: string; currentVersion: string };
};

export function onEvent<K extends keyof PlayerEventMap>(
  name: K,
  handler: (payload: PlayerEventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<PlayerEventMap[K]>(name, (e) => handler(e.payload));
}
