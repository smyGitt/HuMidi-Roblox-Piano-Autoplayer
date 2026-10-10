import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { usePlaybackConfig } from "./PlaybackConfigContext";
import { useLog } from "./LogContext";
import {
  isTauri,
  onEvent,
  parseMidiStructure,
  compileNotes as invokeCompileNotes,
  compileNotesFromSheet as invokeCompileNotesFromSheet,
  compilePedal as invokeCompilePedal,
  startPlayback as invokeStartPlayback,
  togglePause as invokeTogglePause,
  stopPlayback as invokeStopPlayback,
  seekPlayback as invokeSeekPlayback,
  clearLoadedSong as invokeClearLoadedSong,
  savePlayback as invokeSavePlayback,
  resumeFromSave as invokeResumeFromSave,
  getSaveDir,
  getMidiDir,
  blockedInputMessage,
  type TrackSummary,
  type SelectedTrackInfo,
  type PedalData,
  type BackendNote,
  type TempoEvent,
  type TimeSignatureEvent,
  type MeasureBoundary,
} from "../lib/tauri";
import { defaultSaveName } from "../lib/saveName";
import { activePitchesAt, buildNoteTimeline, pedalActiveAt } from "../lib/noteTimeline";
import type { HandRole } from "../dialogs/TrackSelectionDialog";
import type { TrackPart } from "../pages/playback/LoadedParts";
import type { PedalAiStats } from "../pages/playback/PedalAiCard";
import { PlaybackClock } from "./playbackClock";
import { LiveKeys } from "./liveKeys";
import { PREVIEW_CONFIDENCE, PREVIEW_SONG } from "./previewSong";
import type { PedalConfidence } from "../lib/pedalPreview";
import { DEFAULT_CONFIG, type PlaybackConfig } from "../pages/playback/types";
import { NOTES_CONFIG_KEYS, PEDAL_CONFIG_KEYS, configDiffers, pickConfig } from "../pages/playback/compileConfig";

interface PlaybackEngineValue {
  fileName: string;
  midiFilePath: string;
  tracks: TrackSummary[];
  parts: TrackPart[];
  originalBpm: number;
  totalDuration: number;
  finalNotes: BackendNote[];
  playedNotes: BackendNote[];
  tempoEvents: TempoEvent[];
  timeSignatures: TimeSignatureEvent[];
  measureBoundaries: MeasureBoundary[];
  hasCompiledNotes: boolean;
  hasCompiledPedal: boolean;
  isGeneratingPedal: boolean;
  pedalIntervals: [number, number][];
  aiThresholds: [number, number] | null;
  pedalConfidence: PedalConfidence | null;
  configDirty: boolean;
  defaultAiThresholds: [number, number] | null;
  aiStats: PedalAiStats | null;
  isPlaying: boolean;
  isPaused: boolean;
  clock: PlaybackClock;
  liveKeys: LiveKeys;
  saveDirConfigured: boolean;

  openFileBrowser: () => Promise<void>;
  loadFile: (path: string, name: string) => Promise<void>;
  applyConfigChanges: () => Promise<void>;
  discardConfigChanges: () => void;
  confirmTrackSelection: (selection: { index: number; role: HandRole }[]) => Promise<void>;
  generatePedal: () => Promise<void>;
  play: () => Promise<void>;
  togglePause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (time: number) => Promise<void>;
  isSaving: boolean;
  saveCount: number;
  save: (name?: string) => Promise<SaveResult>;
  resumeSave: (filepath: string, label: string) => Promise<void>;
  playTranslatedSheet: (sheetText: string, bpm: number) => Promise<void>;
  clearSong: () => Promise<void>;
  blockedInput: string | null;
  dismissBlockedInput: () => void;
}

export type SaveResult = { path: string } | { error: string };

const NO_AI_THRESHOLDS = {
  pedal_threshold_on: DEFAULT_CONFIG.pedal_threshold_on,
  pedal_threshold_off: DEFAULT_CONFIG.pedal_threshold_off,
};

const PlaybackEngineContext = createContext<PlaybackEngineValue | null>(null);

function formatSnapshotDuration(totalDur: number): string {
  const minutes = Math.floor(totalDur / 60);
  const seconds = Math.floor(totalDur % 60);
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

function statsFromIntervals(intervals: [number, number][], totalDuration: number): PedalAiStats | null {
  if (intervals.length === 0) return null;
  const durations = intervals.map(([s, e]) => e - s);
  const avgDur = durations.reduce((a, b) => a + b, 0) / durations.length;
  return {
    avgDur,
    minDur: Math.min(...durations),
    maxDur: Math.max(...durations),
    pressesPerMin: totalDuration > 0 ? (intervals.length / totalDuration) * 60 : 0,
  };
}

export function PlaybackEngineProvider({ children }: { children: ReactNode }) {
  const { config, updateConfig } = usePlaybackConfig();
  const { appendLog, updateSnapshot, clearSnapshot } = useLog();

  const [fileName, setFileName] = useState("");
  const [midiFilePath, setMidiFilePath] = useState("");
  const [tracks, setTracks] = useState<TrackSummary[]>([]);
  const [parts, setParts] = useState<TrackPart[]>([]);
  const [selectedTracksInfo, setSelectedTracksInfo] = useState<SelectedTrackInfo[]>([]);
  const [originalBpm, setOriginalBpm] = useState(0);
  const [totalDuration, setTotalDuration] = useState(0);
  const [finalNotes, setFinalNotes] = useState<BackendNote[]>([]);
  const [playedNotes, setPlayedNotes] = useState<BackendNote[]>([]);
  const [tempoEvents, setTempoEvents] = useState<TempoEvent[]>([]);
  const [timeSignatures, setTimeSignatures] = useState<TimeSignatureEvent[]>([]);
  const [measureBoundaries, setMeasureBoundaries] = useState<MeasureBoundary[]>([]);
  const [hasCompiledNotes, setHasCompiledNotes] = useState(false);
  const [hasCompiledPedal, setHasCompiledPedal] = useState(false);
  const [isGeneratingPedal, setIsGeneratingPedal] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [blockedInput, setBlockedInput] = useState<string | null>(null);
  const [saveCount, setSaveCount] = useState(0);
  const [pedalIntervals, setPedalIntervals] = useState<[number, number][]>([]);
  const [aiThresholds, setAiThresholds] = useState<[number, number] | null>(null);
  const [pedalConfidence, setPedalConfidence] = useState<PedalConfidence | null>(null);
  const [compiledNotesConfig, setCompiledNotesConfig] = useState<Partial<PlaybackConfig> | null>(null);
  const [compiledPedalConfig, setCompiledPedalConfig] = useState<Partial<PlaybackConfig> | null>(null);
  const [defaultAiThresholds, setDefaultAiThresholds] = useState<[number, number] | null>(null);
  const defaultAiThresholdsRef = useRef(defaultAiThresholds);
  defaultAiThresholdsRef.current = defaultAiThresholds;
  const [isPlaying, setIsPlaying] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [clock] = useState(() => new PlaybackClock({ selfTimed: !isTauri() }));
  const [liveKeys] = useState(() => new LiveKeys());
  const [saveDirConfigured, setSaveDirConfigured] = useState(false);
  const running = isPlaying && !isPaused;

  const configRef = useRef(config);
  configRef.current = config;
  const isPlayingRef = useRef(isPlaying);
  isPlayingRef.current = isPlaying;
  const isGeneratingPedalRef = useRef(false);
  const atEndRef = useRef(false);
  const isSavingRef = useRef(false);
  const songVersionRef = useRef(0);
  const playRef = useRef(play);
  playRef.current = play;
  const stopRef = useRef(stop);
  stopRef.current = stop;
  const togglePauseRef = useRef(togglePause);
  togglePauseRef.current = togglePause;
  const saveRef = useRef(save);
  saveRef.current = save;

  useEffect(() => {
    if (!isTauri()) return;
    getSaveDir()
      .then((dir) => setSaveDirConfigured(!!dir))
      .catch(() => {});
  }, []);

  useEffect(() => {
    clock.setRunning(running);
  }, [clock, running]);

  useEffect(() => {
    clock.setDuration(totalDuration);
  }, [clock, totalDuration]);

  const previewTimeline = useMemo(() => buildNoteTimeline(playedNotes), [playedNotes]);

  useEffect(() => {
    if (isTauri()) return;
    if (!running) {
      liveKeys.clear();
      return;
    }
    const simulateKeys = () => {
      const time = clock.time();
      liveKeys.setPitches(activePitchesAt(previewTimeline, time));
      liveKeys.setPedal(pedalActiveAt(pedalIntervals, time));
    };
    simulateKeys();
    return clock.subscribe(simulateKeys);
  }, [clock, liveKeys, running, previewTimeline, pedalIntervals]);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten: Promise<() => void>[] = [
      onEvent("status_updated", (msg) => appendLog(msg)),
      onEvent("progress_updated", (t) => clock.report(t)),
      onEvent("playback_finished", () => {
        atEndRef.current = false;
        setIsPlaying(false);
        setIsPaused(false);
        appendLog("Playback finished");
      }),
      onEvent("visualizer_updated", (pitches) => liveKeys.setPitches(pitches)),
      onEvent("pedal_updated", (active) => liveKeys.setPedal(active)),
      onEvent("auto_paused", () => {
        atEndRef.current = true;
        setIsPaused(true);
      }),
      onEvent("error_occurred", (msg) => appendLog(`Error: ${msg}`)),
      onEvent("playback_started", () => {
        atEndRef.current = false;
        setIsPlaying(true);
      }),
      onEvent("hotkey_toggle_requested", () => {
        void (isPlayingRef.current ? togglePauseRef.current() : playRef.current());
      }),
      onEvent("hotkey_save_requested", () => {
        void saveRef.current();
      }),
    ];
    return () => {
      unlisten.forEach((p) => p.then((fn) => fn()));
    };
  }, [appendLog, clock, liveKeys]);

  const loadFile = useCallback(async (path: string, name: string) => {
    if (isPlayingRef.current) await stopRef.current();
    const version = ++songVersionRef.current;
    setMidiFilePath(path);
    setFileName(name);
    setHasCompiledNotes(false);
    setHasCompiledPedal(false);
    setPedalIntervals([]);
    setAiThresholds(null);
    setPedalConfidence(null);
    setCompiledNotesConfig(null);
    setCompiledPedalConfig(null);
    setDefaultAiThresholds(null);
    setParts([]);
    setSelectedTracksInfo([]);
    setFinalNotes([]);
    setPlayedNotes([]);
    setTempoEvents([]);
    setTimeSignatures([]);
    setMeasureBoundaries([]);
    setOriginalBpm(0);
    appendLog(`Loading ${name}`);
    clearSnapshot();
    updateSnapshot({ file: name, source: "MIDI file" });
    if (!isTauri()) {
      setTracks([
        { index: 0, name: "Piano Right", note_count: 214, instrument_name: "Acoustic Grand Piano", is_drum: false },
        { index: 1, name: "Piano Left", note_count: 128, instrument_name: "Acoustic Grand Piano", is_drum: false },
        { index: 2, name: "Click Track", note_count: 96, instrument_name: "Drum Kit", is_drum: true },
      ]);
      setOriginalBpm(120);
      appendLog("Parsed 3 track(s) successful (preview mode, not a real MIDI parse)");
      return;
    }
    try {
      const parsed = await parseMidiStructure(path);
      if (version !== songVersionRef.current) return;
      setTracks(parsed.tracks);
      setOriginalBpm(parsed.initial_bpm);
      updateSnapshot({ tempo: `${Math.round(parsed.initial_bpm)} BPM` });
      appendLog(`Parsed ${parsed.tracks.length} track(s) successful`);
    } catch (e) {
      if (version !== songVersionRef.current) return;
      appendLog(`Failed to parse MIDI: ${String(e)}`);
    }
  }, [appendLog]);

  async function openFileBrowser() {
    if (!isTauri()) {
      appendLog("File dialog unavailable outside the desktop app");
      return;
    }
    const midiDir = await getMidiDir().catch(() => "");
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "MIDI", extensions: ["mid", "midi"] }],
      defaultPath: midiDir || undefined,
    });
    if (typeof selected === "string") {
      await loadFile(selected, selected.split(/[\\/]/).pop() ?? selected);
    }
  }

  async function confirmTrackSelection(selection: { index: number; role: HandRole }[]) {
    const info: SelectedTrackInfo[] = selection.map((s) => [s.index, s.role]);
    setSelectedTracksInfo(info);
    setParts(
      selection.map(({ index, role }) => {
        const t = tracks.find((tr) => tr.index === index)!;
        const roleLabel = role === "Auto-Detect" ? "Auto" : role === "Left Hand" ? "Left" : "Right";
        return { name: t.name, meta: `♩ ${t.note_count} · ${roleLabel}` };
      }),
    );
    await compileNotesNow(info);
  }

  async function compileNotesNow(info: SelectedTrackInfo[]): Promise<boolean> {
    const version = songVersionRef.current;
    const configUsed = configRef.current;
    if (!isTauri()) {
      setTotalDuration(PREVIEW_SONG.totalDuration);
      setFinalNotes(PREVIEW_SONG.notes);
      setPlayedNotes(PREVIEW_SONG.notes);
      setTempoEvents(PREVIEW_SONG.tempoEvents);
      setTimeSignatures([]);
      setMeasureBoundaries(PREVIEW_SONG.measureBoundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(false);
      setCompiledNotesConfig(pickConfig(configUsed, NOTES_CONFIG_KEYS));
      setCompiledPedalConfig(null);
      appendLog("Notes compiled successful (preview mode)");
      return true;
    }
    try {
      const timeline = await invokeCompileNotes(configUsed, midiFilePath, info);
      if (version !== songVersionRef.current) return false;
      setTotalDuration(timeline.total_dur);
      setFinalNotes(timeline.final_notes);
      setPlayedNotes(timeline.played_notes);
      setTempoEvents(timeline.tempo_events);
      setTimeSignatures(timeline.time_signatures);
      setMeasureBoundaries(timeline.measure_boundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(false);
      updateSnapshot({
        tracks: info.length,
        notes: timeline.final_notes.length,
        duration: formatSnapshotDuration(timeline.total_dur),
      });
      setCompiledNotesConfig(pickConfig(configUsed, NOTES_CONFIG_KEYS));
      setCompiledPedalConfig(null);
      appendLog("Notes compiled successful");
      return true;
    } catch (e) {
      if (version !== songVersionRef.current) return false;
      appendLog(`Failed to compile notes: ${String(e)}`);
      const blocked = blockedInputMessage(e);
      if (blocked !== null) {
        setBlockedInput(blocked);
        await clearSong();
      }
      return false;
    }
  }

  function captureDefaultThresholds(thresholds: [number, number]) {
    if (defaultAiThresholdsRef.current === null) {
      setDefaultAiThresholds(thresholds);
    }
  }

  async function generatePedal() {
    await compilePedalNow();
  }

  const notesDirty = hasCompiledNotes && configDiffers(config, compiledNotesConfig, NOTES_CONFIG_KEYS);
  const pedalDirty = hasCompiledPedal && configDiffers(config, compiledPedalConfig, PEDAL_CONFIG_KEYS);
  const configDirty = notesDirty || pedalDirty;

  async function applyConfigChanges() {
    if (isPlayingRef.current) await stop();
    if (notesDirty) {
      const compiled = await compileNotesNow(selectedTracksInfo);
      if (compiled) await compilePedalNow();
    } else if (pedalDirty) {
      await compilePedalNow();
    }
  }

  function discardConfigChanges() {
    updateConfig({ ...compiledNotesConfig, ...compiledPedalConfig });
  }

  async function compilePedalNow(): Promise<PedalData | null> {
    if (!isTauri()) {
      setPedalIntervals(PREVIEW_SONG.pedalIntervals);
      setAiThresholds([0.5, 0.5]);
      setPedalConfidence(PREVIEW_CONFIDENCE);
      setCompiledPedalConfig(pickConfig(configRef.current, PEDAL_CONFIG_KEYS));
      captureDefaultThresholds([0.5, 0.5]);
      setHasCompiledPedal(true);
      appendLog("Pedal compiled successful (preview mode)");
      return null;
    }
    if (isGeneratingPedalRef.current) return null;
    isGeneratingPedalRef.current = true;
    setIsGeneratingPedal(true);
    try {
      const configUsed = configRef.current;
      const data = await invokeCompilePedal(configUsed, midiFilePath);
      setPedalIntervals(data.pedal_intervals);
      setAiThresholds(data.ai_thresholds);
      setPedalConfidence(
        data.confidence.length > 0 ? { values: data.confidence, fps: data.confidence_fps } : null,
      );
      if (data.ai_thresholds) captureDefaultThresholds(data.ai_thresholds);
      setHasCompiledPedal(true);
      setCompiledPedalConfig({
        ...pickConfig(configUsed, PEDAL_CONFIG_KEYS),
        ...(data.ai_thresholds
          ? { pedal_threshold_on: data.ai_thresholds[0], pedal_threshold_off: data.ai_thresholds[1] }
          : NO_AI_THRESHOLDS),
      });
      if (data.ai_thresholds) {
        updateConfig({ pedal_threshold_on: data.ai_thresholds[0], pedal_threshold_off: data.ai_thresholds[1] });
      } else {
        setDefaultAiThresholds(null);
        updateConfig(NO_AI_THRESHOLDS);
      }
      updateSnapshot({ pedal: `${data.pedal_intervals.length} presses` });
      appendLog("Pedal compiled successful");
      return data;
    } catch (e) {
      appendLog(`Failed to compile pedal: ${String(e)}`);
      return null;
    } finally {
      isGeneratingPedalRef.current = false;
      setIsGeneratingPedal(false);
    }
  }

  async function play() {
    if (isGeneratingPedalRef.current) return;
    setIsPaused(false);
    if (!isTauri()) {
      setIsPlaying((p) => !p);
      return;
    }
    try {
      if (!hasCompiledPedal) await generatePedal();
      await invokeStartPlayback(configRef.current, midiFilePath);
    } catch (e) {
      appendLog(`Failed to start playback: ${String(e)}`);
      const blocked = blockedInputMessage(e);
      if (blocked !== null) setBlockedInput(blocked);
    }
  }

  async function togglePause() {
    if (!isTauri()) {
      setIsPlaying((p) => !p);
      return;
    }
    if (atEndRef.current) {
      atEndRef.current = false;
      clock.jump(0);
      await invokeSeekPlayback(0);
    }
    setIsPaused((p) => !p);
    await invokeTogglePause();
  }

  async function stop() {
    setIsPaused(false);
    if (!isTauri()) {
      setIsPlaying(false);
      clock.jump(0);
      return;
    }
    atEndRef.current = false;
    await invokeStopPlayback();
    clock.jump(0);
  }

  async function seek(time: number) {
    if (!isTauri()) {
      clock.jump(time);
      return;
    }
    atEndRef.current = false;
    if (isPlayingRef.current) clock.jump(time);
    await invokeSeekPlayback(time);
  }

  async function save(name?: string): Promise<SaveResult> {
    if (!isTauri()) {
      appendLog("Save unavailable outside the desktop app");
      return { error: "Save unavailable outside the desktop app" };
    }
    if (isSavingRef.current) return { error: "A save is already in progress." };
    if (isGeneratingPedalRef.current) return { error: "Pedal is still generating." };
    isSavingRef.current = true;
    setIsSaving(true);
    try {
      let saveConfig = configRef.current;
      if (!hasCompiledPedal) {
        const data = await compilePedalNow();
        if (data?.ai_thresholds) {
          saveConfig = {
            ...saveConfig,
            pedal_threshold_on: data.ai_thresholds[0],
            pedal_threshold_off: data.ai_thresholds[1],
          };
        }
      }
      const path = await invokeSavePlayback(
        saveConfig,
        midiFilePath,
        selectedTracksInfo,
        fileName,
        name ?? defaultSaveName(fileName),
      );
      appendLog(`Saved to ${path}`);
      setSaveCount((c) => c + 1);
      return { path };
    } catch (e) {
      appendLog(`Failed to save: ${String(e)}`);
      return { error: String(e) };
    } finally {
      isSavingRef.current = false;
      setIsSaving(false);
    }
  }

  async function resumeSave(filepath: string, label: string) {
    if (!isTauri()) {
      appendLog("Resume unavailable outside the desktop app");
      return;
    }
    if (isPlayingRef.current) await stop();
    try {
      const result = await invokeResumeFromSave(filepath);
      setFinalNotes(result.final_notes);
      setPlayedNotes(result.played_notes);
      setTotalDuration(result.total_dur);
      setTempoEvents(result.tempo_events);
      setTimeSignatures(result.time_signatures);
      setMeasureBoundaries(result.measure_boundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(true);
      setPedalIntervals([]);
      setAiThresholds(null);
      setPedalConfidence(null);
      setCompiledNotesConfig(null);
      setCompiledPedalConfig(null);
      setDefaultAiThresholds(null);
      setFileName(label);
      setMidiFilePath("");
      setParts([]);
      setSelectedTracksInfo([]);
      clearSnapshot();
      updateSnapshot({
        file: label,
        source: "Save file",
        tracks: result.track_count,
        pedal: `${result.pedal_count} presses`,
      });
      appendLog(`Resumed save: ${label}`);
    } catch (e) {
      appendLog(`Failed to resume save: ${String(e)}`);
      const blocked = blockedInputMessage(e);
      if (blocked !== null) setBlockedInput(blocked);
    }
  }

  async function playTranslatedSheet(sheetText: string, bpm: number) {
    if (!isTauri()) {
      appendLog("Sheet playback unavailable outside the desktop app");
      return;
    }
    try {
      const timeline = await invokeCompileNotesFromSheet(configRef.current, sheetText, bpm);
      setFinalNotes(timeline.final_notes);
      setPlayedNotes(timeline.played_notes);
      setTotalDuration(timeline.total_dur);
      setTempoEvents(timeline.tempo_events);
      setTimeSignatures(timeline.time_signatures);
      setMeasureBoundaries(timeline.measure_boundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(false);
      setCompiledNotesConfig(null);
      setCompiledPedalConfig(null);
      setFileName("Translated sheet");
      setMidiFilePath("");
      setParts([]);
      setSelectedTracksInfo([]);
      clearSnapshot();
      updateSnapshot({ file: "(pasted sheet)", source: "Virtual Piano import", tempo: `${bpm} BPM` });
      appendLog(`Sheet compiled successful: ${timeline.final_notes.length} note(s)`);
      await generatePedal();
      setIsPaused(false);
      await invokeStartPlayback(configRef.current, "");
    } catch (e) {
      appendLog(`Failed to play translated sheet: ${String(e)}`);
    }
  }

  async function clearSong() {
    if (isGeneratingPedalRef.current) {
      appendLog("Cannot clear the song while the pedal is generating");
      return;
    }
    if (isPlayingRef.current) await stop();
    if (isTauri()) {
      try {
        await invokeClearLoadedSong();
      } catch (e) {
        appendLog(`Failed to clear the song: ${String(e)}`);
        return;
      }
    }
    songVersionRef.current += 1;
    setFileName("");
    setMidiFilePath("");
    setTracks([]);
    setParts([]);
    setSelectedTracksInfo([]);
    setOriginalBpm(0);
    setTotalDuration(0);
    setFinalNotes([]);
    setPlayedNotes([]);
    setTempoEvents([]);
    setTimeSignatures([]);
    setMeasureBoundaries([]);
    setHasCompiledNotes(false);
    setHasCompiledPedal(false);
    setPedalIntervals([]);
    setAiThresholds(null);
    setPedalConfidence(null);
    setCompiledNotesConfig(null);
    setCompiledPedalConfig(null);
    setDefaultAiThresholds(null);
    clock.jump(0);
    liveKeys.clear();
    clearSnapshot();
    appendLog("Cleared the loaded song");
  }

  const aiStats = useMemo(() => statsFromIntervals(pedalIntervals, totalDuration), [pedalIntervals, totalDuration]);

  const value: PlaybackEngineValue = {
    fileName,
    midiFilePath,
    tracks,
    parts,
    originalBpm,
    totalDuration,
    finalNotes,
    playedNotes,
    tempoEvents,
    timeSignatures,
    measureBoundaries,
    hasCompiledNotes,
    hasCompiledPedal,
    isGeneratingPedal,
    isSaving,
    saveCount,
    pedalIntervals,
    aiThresholds,
    pedalConfidence,
    configDirty,
    defaultAiThresholds,
    aiStats,
    isPlaying,
    isPaused,
    clock,
    liveKeys,
    saveDirConfigured,
    openFileBrowser,
    loadFile,
    applyConfigChanges,
    discardConfigChanges,
    confirmTrackSelection,
    generatePedal,
    play,
    togglePause,
    stop,
    seek,
    save,
    resumeSave,
    playTranslatedSheet,
    clearSong,
    blockedInput,
    dismissBlockedInput: () => setBlockedInput(null),
  };

  return <PlaybackEngineContext.Provider value={value}>{children}</PlaybackEngineContext.Provider>;
}

export function usePlaybackEngine() {
  const ctx = useContext(PlaybackEngineContext);
  if (!ctx) throw new Error("usePlaybackEngine must be used within PlaybackEngineProvider");
  return ctx;
}
