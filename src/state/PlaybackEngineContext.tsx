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
  type TrackSummary,
  type SelectedTrackInfo,
  type BackendNote,
  type TempoEvent,
  type TimeSignatureEvent,
  type MeasureBoundary,
} from "../lib/tauri";
import type { HandRole } from "../dialogs/TrackSelectionDialog";
import type { TrackPart } from "../pages/playback/LoadedParts";
import type { PedalAiStats } from "../pages/playback/PedalAiCard";

interface PlaybackEngineValue {
  fileName: string;
  midiFilePath: string;
  tracks: TrackSummary[];
  parts: TrackPart[];
  originalBpm: number;
  totalDuration: number;
  finalNotes: BackendNote[];
  tempoEvents: TempoEvent[];
  timeSignatures: TimeSignatureEvent[];
  measureBoundaries: MeasureBoundary[];
  hasCompiledNotes: boolean;
  hasCompiledPedal: boolean;
  isGeneratingPedal: boolean;
  pedalIntervals: [number, number][];
  aiThresholds: [number, number] | null;
  defaultAiThresholds: [number, number] | null;
  aiStats: PedalAiStats | null;
  isPlaying: boolean;
  isPaused: boolean;
  currentTime: number;
  activePitches: Set<number>;
  pedalActive: boolean;
  saveDirConfigured: boolean;

  openFileBrowser: () => Promise<void>;
  loadFile: (path: string, name: string) => Promise<void>;
  confirmTrackSelection: (selection: { index: number; role: HandRole }[]) => Promise<void>;
  generatePedal: () => Promise<void>;
  play: () => Promise<void>;
  togglePause: () => Promise<void>;
  stop: () => Promise<void>;
  seek: (time: number) => Promise<void>;
  save: () => Promise<string | null>;
  resumeSave: (filepath: string, label: string) => Promise<void>;
  playTranslatedSheet: (sheetText: string, bpm: number) => Promise<void>;
  clearSong: () => Promise<void>;
}

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
  const [tempoEvents, setTempoEvents] = useState<TempoEvent[]>([]);
  const [timeSignatures, setTimeSignatures] = useState<TimeSignatureEvent[]>([]);
  const [measureBoundaries, setMeasureBoundaries] = useState<MeasureBoundary[]>([]);
  const [hasCompiledNotes, setHasCompiledNotes] = useState(false);
  const [hasCompiledPedal, setHasCompiledPedal] = useState(false);
  const [isGeneratingPedal, setIsGeneratingPedal] = useState(false);
  const [pedalIntervals, setPedalIntervals] = useState<[number, number][]>([]);
  const [aiThresholds, setAiThresholds] = useState<[number, number] | null>(null);
  const [defaultAiThresholds, setDefaultAiThresholds] = useState<[number, number] | null>(null);
  const defaultAiThresholdsRef = useRef(defaultAiThresholds);
  defaultAiThresholdsRef.current = defaultAiThresholds;
  const [isPlaying, setIsPlaying] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [activePitches, setActivePitches] = useState<Set<number>>(new Set());
  const [pedalActive, setPedalActive] = useState(false);
  const [saveDirConfigured, setSaveDirConfigured] = useState(false);

  const configRef = useRef(config);
  configRef.current = config;
  const isPlayingRef = useRef(isPlaying);
  isPlayingRef.current = isPlaying;
  const isGeneratingPedalRef = useRef(false);
  const songVersionRef = useRef(0);
  const playRef = useRef(play);
  playRef.current = play;
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
    if (!isTauri()) return;
    const unlisten: Promise<() => void>[] = [
      onEvent("status_updated", (msg) => appendLog(msg)),
      onEvent("progress_updated", (t) => setCurrentTime(t)),
      onEvent("playback_finished", () => {
        setIsPlaying(false);
        setIsPaused(false);
        appendLog("Playback finished");
      }),
      onEvent("visualizer_updated", (pitches) => setActivePitches(new Set(pitches))),
      onEvent("pedal_updated", (active) => setPedalActive(active)),
      onEvent("auto_paused", () => setIsPaused(true)),
      onEvent("error_occurred", (msg) => appendLog(`Error: ${msg}`)),
      onEvent("playback_started", () => setIsPlaying(true)),
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
  }, [appendLog]);

  const loadFile = useCallback(async (path: string, name: string) => {
    const version = ++songVersionRef.current;
    setMidiFilePath(path);
    setFileName(name);
    setHasCompiledNotes(false);
    setHasCompiledPedal(false);
    setPedalIntervals([]);
    setAiThresholds(null);
    setDefaultAiThresholds(null);
    setParts([]);
    setSelectedTracksInfo([]);
    setFinalNotes([]);
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    const version = songVersionRef.current;
    const info: SelectedTrackInfo[] = selection.map((s) => [s.index, s.role]);
    setSelectedTracksInfo(info);
    setParts(
      selection.map(({ index, role }) => {
        const t = tracks.find((tr) => tr.index === index)!;
        const roleLabel = role === "Auto-Detect" ? "Auto" : role === "Left Hand" ? "Left" : "Right";
        return { name: t.name, meta: `♩ ${t.note_count} · ${roleLabel}` };
      }),
    );
    if (!isTauri()) {
      const previewDuration = 60;
      setTotalDuration(previewDuration);
      setTempoEvents([[0, 500_000]]);
      setTimeSignatures([]);
      const previewBoundaries: MeasureBoundary[] = [];
      for (let start = 0; start < previewDuration; start += 2) {
        previewBoundaries.push([start, Math.min(start + 2, previewDuration)]);
      }
      setMeasureBoundaries(previewBoundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(false);
      appendLog("Notes compiled successful (preview mode)");
      return;
    }
    try {
      const timeline = await invokeCompileNotes(configRef.current, midiFilePath, info);
      if (version !== songVersionRef.current) return;
      setTotalDuration(timeline.total_dur);
      setFinalNotes(timeline.final_notes);
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
      appendLog("Notes compiled successful");
    } catch (e) {
      if (version !== songVersionRef.current) return;
      appendLog(`Failed to compile notes: ${String(e)}`);
    }
  }

  function captureDefaultThresholds(thresholds: [number, number]) {
    if (defaultAiThresholdsRef.current === null) {
      setDefaultAiThresholds(thresholds);
    }
  }

  async function generatePedal() {
    if (!isTauri()) {
      setPedalIntervals([[0, 1.8], [3, 4.8], [6, 7.8]]);
      setAiThresholds([0.5, 0.5]);
      captureDefaultThresholds([0.5, 0.5]);
      setHasCompiledPedal(true);
      appendLog("Pedal compiled successful (preview mode)");
      return;
    }
    if (isGeneratingPedalRef.current) return;
    isGeneratingPedalRef.current = true;
    setIsGeneratingPedal(true);
    try {
      const data = await invokeCompilePedal(configRef.current, midiFilePath);
      setPedalIntervals(data.pedal_intervals);
      setAiThresholds(data.ai_thresholds);
      if (data.ai_thresholds) captureDefaultThresholds(data.ai_thresholds);
      setHasCompiledPedal(true);
      if (data.ai_thresholds) {
        updateConfig({ pedal_threshold_on: data.ai_thresholds[0], pedal_threshold_off: data.ai_thresholds[1] });
      }
      updateSnapshot({ pedal: `${data.pedal_intervals.length} presses` });
      appendLog("Pedal compiled successful");
    } catch (e) {
      appendLog(`Failed to compile pedal: ${String(e)}`);
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
    }
  }

  async function togglePause() {
    if (!isTauri()) {
      setIsPlaying((p) => !p);
      return;
    }
    setIsPaused((p) => !p);
    await invokeTogglePause();
  }

  async function stop() {
    setIsPaused(false);
    if (!isTauri()) {
      setIsPlaying(false);
      setCurrentTime(0);
      return;
    }
    await invokeStopPlayback();
    setCurrentTime(0);
  }

  async function seek(time: number) {
    if (!isTauri()) {
      setCurrentTime(time);
      return;
    }
    await invokeSeekPlayback(time);
  }

  async function save(): Promise<string | null> {
    if (!isTauri()) {
      appendLog("Save unavailable outside the desktop app");
      return null;
    }
    try {
      const path = await invokeSavePlayback(configRef.current, midiFilePath, selectedTracksInfo, fileName);
      appendLog(`Saved to ${path}`);
      return path;
    } catch (e) {
      appendLog(`Failed to save: ${String(e)}`);
      return null;
    }
  }

  async function resumeSave(filepath: string, label: string) {
    if (!isTauri()) {
      appendLog("Resume unavailable outside the desktop app");
      return;
    }
    try {
      const result = await invokeResumeFromSave(filepath);
      setFinalNotes(result.final_notes);
      setTotalDuration(result.total_dur);
      setTempoEvents(result.tempo_events);
      setTimeSignatures(result.time_signatures);
      setMeasureBoundaries(result.measure_boundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(true);
      setPedalIntervals([]);
      setAiThresholds(null);
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
      setTotalDuration(timeline.total_dur);
      setTempoEvents(timeline.tempo_events);
      setTimeSignatures(timeline.time_signatures);
      setMeasureBoundaries(timeline.measure_boundaries);
      setHasCompiledNotes(true);
      setHasCompiledPedal(false);
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
    setTempoEvents([]);
    setTimeSignatures([]);
    setMeasureBoundaries([]);
    setHasCompiledNotes(false);
    setHasCompiledPedal(false);
    setPedalIntervals([]);
    setAiThresholds(null);
    setDefaultAiThresholds(null);
    setCurrentTime(0);
    setActivePitches(new Set());
    setPedalActive(false);
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
    tempoEvents,
    timeSignatures,
    measureBoundaries,
    hasCompiledNotes,
    hasCompiledPedal,
    isGeneratingPedal,
    pedalIntervals,
    aiThresholds,
    defaultAiThresholds,
    aiStats,
    isPlaying,
    isPaused,
    currentTime,
    activePitches,
    pedalActive,
    saveDirConfigured,
    openFileBrowser,
    loadFile,
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
  };

  return <PlaybackEngineContext.Provider value={value}>{children}</PlaybackEngineContext.Provider>;
}

export function usePlaybackEngine() {
  const ctx = useContext(PlaybackEngineContext);
  if (!ctx) throw new Error("usePlaybackEngine must be used within PlaybackEngineProvider");
  return ctx;
}
