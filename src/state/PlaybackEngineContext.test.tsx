import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { PlaybackConfigProvider, usePlaybackConfig } from "./PlaybackConfigContext";
import { LogProvider, useLog } from "./LogContext";
import { PlaybackEngineProvider, usePlaybackEngine } from "./PlaybackEngineContext";

const eventHandlers = vi.hoisted(() => new Map<string, (payload: unknown) => void>());

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  onEvent: vi.fn((name: string, handler: (payload: unknown) => void) => {
    eventHandlers.set(name, handler);
    return Promise.resolve(() => {});
  }),
  parseMidiStructure: vi.fn(),
  compileNotes: vi.fn(),
  compileNotesFromSheet: vi.fn(),
  compilePedal: vi.fn(),
  startPlayback: vi.fn(),
  togglePause: vi.fn(),
  stopPlayback: vi.fn(),
  seekPlayback: vi.fn(),
  savePlayback: vi.fn(),
  resumeFromSave: vi.fn(),
  getSaveDir: vi.fn(() => Promise.resolve(null)),
}));

vi.mock("../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri")>("../lib/tauri");
  return { ...actual, ...tauriMocks };
});

const dialogOpen = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: dialogOpen }));

const onDragDropEvent = vi.hoisted(() => vi.fn(() => Promise.resolve(() => {})));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent }),
}));

function wrapper({ children }: { children: ReactNode }) {
  return (
    <PlaybackConfigProvider>
      <LogProvider>
        <PlaybackEngineProvider>{children}</PlaybackEngineProvider>
      </LogProvider>
    </PlaybackConfigProvider>
  );
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function renderEngine() {
  return renderHook(
    () => ({ engine: usePlaybackEngine(), log: useLog(), playbackConfig: usePlaybackConfig() }),
    { wrapper },
  );
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  eventHandlers.clear();
  tauriMocks.parseMidiStructure.mockReset();
  tauriMocks.compileNotes.mockReset();
  tauriMocks.compileNotesFromSheet.mockReset();
  tauriMocks.compilePedal.mockReset();
  tauriMocks.startPlayback.mockReset();
  tauriMocks.togglePause.mockReset();
  tauriMocks.stopPlayback.mockReset();
  tauriMocks.seekPlayback.mockReset();
  tauriMocks.savePlayback.mockReset();
  tauriMocks.resumeFromSave.mockReset();
  tauriMocks.getSaveDir.mockReturnValue(Promise.resolve(null));
  dialogOpen.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("PlaybackEngineContext (preview mode, isTauri() = false)", () => {
  it("loadFile fabricates 3 demo tracks and logs it as preview mode", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("test.mid", "test.mid"));
    expect(result.current.engine.fileName).toBe("test.mid");
    expect(result.current.engine.tracks).toHaveLength(3);
    expect(result.current.engine.tracks[2].is_drum).toBe(true);
    expect(result.current.log.entries.some((e) => e.line.includes("preview mode"))).toBe(true);
    expect(tauriMocks.parseMidiStructure).not.toHaveBeenCalled();
    expect(result.current.engine.originalBpm).toBeGreaterThan(0);
  });

  it("resumeSave logs unavailability and does not call resume_from_save", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.resumeSave("/saves/x.json", "x"));
    expect(result.current.log.entries.some((e) => e.line.toLowerCase().includes("unavailable"))).toBe(true);
    expect(tauriMocks.resumeFromSave).not.toHaveBeenCalled();
    expect(result.current.engine.hasCompiledPedal).toBe(false);
  });

  it("playTranslatedSheet logs unavailability and does not call compile_notes_from_sheet", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.playTranslatedSheet("a b c", 120));
    expect(result.current.log.entries.some((e) => e.line.toLowerCase().includes("unavailable"))).toBe(true);
    expect(tauriMocks.compileNotesFromSheet).not.toHaveBeenCalled();
    expect(tauriMocks.startPlayback).not.toHaveBeenCalled();
  });

  it("confirmTrackSelection sets hasCompiledNotes without calling the real compile command", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("test.mid", "test.mid"));
    await act(async () => result.current.engine.confirmTrackSelection([{ index: 0, role: "Left Hand" }]));
    expect(result.current.engine.hasCompiledNotes).toBe(true);
    expect(result.current.engine.totalDuration).toBeGreaterThan(0);
    expect(result.current.engine.parts).toHaveLength(1);
    expect(result.current.engine.parts[0].meta).toContain("Left");
    expect(tauriMocks.compileNotes).not.toHaveBeenCalled();
  });

  it("confirmTrackSelection fabricates a non-empty default measure grid in preview mode", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("test.mid", "test.mid"));
    await act(async () => result.current.engine.confirmTrackSelection([{ index: 0, role: "Left Hand" }]));
    expect(result.current.engine.tempoEvents).toEqual([[0, 500_000]]);
    expect(result.current.engine.timeSignatures).toEqual([]);
    expect(result.current.engine.measureBoundaries.length).toBeGreaterThan(0);
    expect(result.current.engine.measureBoundaries[0]).toEqual([0, 2]);
  });

  it("generatePedal fabricates pedal intervals and derives non-null aiStats from them", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.hasCompiledPedal).toBe(true);
    expect(result.current.engine.pedalIntervals.length).toBeGreaterThan(0);
    expect(result.current.engine.aiStats).not.toBeNull();
    expect(result.current.engine.aiStats!.pressesPerMin).toBeGreaterThanOrEqual(0);
    expect(tauriMocks.compilePedal).not.toHaveBeenCalled();
  });

  it("play/stop/seek/togglePause mutate local state without any backend call", async () => {
    const { result } = renderEngine();
    expect(result.current.engine.isPlaying).toBe(false);
    await act(async () => result.current.engine.play());
    expect(result.current.engine.isPlaying).toBe(true);
    await act(async () => result.current.engine.togglePause());
    expect(result.current.engine.isPlaying).toBe(false);
    await act(async () => result.current.engine.seek(42));
    expect(result.current.engine.currentTime).toBe(42);
    await act(async () => result.current.engine.stop());
    expect(result.current.engine.currentTime).toBe(0);
    expect(tauriMocks.startPlayback).not.toHaveBeenCalled();
    expect(tauriMocks.stopPlayback).not.toHaveBeenCalled();
    expect(tauriMocks.seekPlayback).not.toHaveBeenCalled();
  });

  it("save() logs unavailability and resolves to null instead of calling the backend", async () => {
    const { result } = renderEngine();
    let saveResult: string | null = "unset";
    await act(async () => {
      saveResult = await result.current.engine.save();
    });
    expect(saveResult).toBeNull();
    expect(result.current.log.entries.some((e) => e.line.toLowerCase().includes("unavailable"))).toBe(true);
    expect(tauriMocks.savePlayback).not.toHaveBeenCalled();
  });

  it("openFileBrowser logs unavailability instead of opening a dialog", async () => {
    const { result } = renderEngine();
    await act(async () => result.current.engine.openFileBrowser());
    expect(dialogOpen).not.toHaveBeenCalled();
    expect(result.current.log.entries.some((e) => e.line.toLowerCase().includes("unavailable"))).toBe(true);
  });
});

describe("PlaybackEngineContext (Tauri mode, isTauri() = true)", () => {
  beforeEach(() => {
    tauriMocks.isTauri.mockReturnValue(true);
  });

  it("loadFile calls the real parse_midi_structure command and stores its result", async () => {
    tauriMocks.parseMidiStructure.mockResolvedValue({
      tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
      initial_bpm: 142.5,
    });
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/path/song.mid", "song.mid"));
    await waitFor(() => expect(result.current.engine.tracks).toHaveLength(1));
    expect(tauriMocks.parseMidiStructure).toHaveBeenCalledWith("/real/path/song.mid");
    expect(result.current.engine.midiFilePath).toBe("/real/path/song.mid");
    expect(result.current.engine.originalBpm).toBe(142.5);
  });

  it("loadFile resets originalBpm to 0 while a new file is loading, discarding the previous file's value", async () => {
    tauriMocks.parseMidiStructure.mockResolvedValue({
      tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
      initial_bpm: 90,
    });
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/path/a.mid", "a.mid"));
    expect(result.current.engine.originalBpm).toBe(90);

    tauriMocks.parseMidiStructure.mockReturnValue(new Promise(() => {}));
    act(() => {
      void result.current.engine.loadFile("/real/path/b.mid", "b.mid");
    });
    await waitFor(() => expect(result.current.engine.originalBpm).toBe(0));
  });

  it("loadFile logs a failure message when parse_midi_structure rejects, without crashing", async () => {
    tauriMocks.parseMidiStructure.mockRejectedValue(new Error("bad file"));
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/path/broken.mid", "broken.mid"));
    expect(result.current.log.entries.some((e) => e.level === "WARN" && e.line.includes("Failed"))).toBe(true);
    expect(result.current.engine.tracks).toHaveLength(0);
  });

  it("confirmTrackSelection calls compile_notes with the converted backend config and stores real notes/duration", async () => {
    tauriMocks.parseMidiStructure.mockResolvedValue({
      tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
      initial_bpm: 120,
    });
    tauriMocks.compileNotes.mockResolvedValue({
      final_notes: [{ id: 0, pitch: 60, velocity: 80, start_time: 0, duration: 1, hand: "left", original_track_index: 0, channel: 0 }],
      total_dur: 12.5,
      tempo_events: [[0, 545_454]],
      time_signatures: [[0, 3, 4]],
      measure_boundaries: [[0, 1.5], [1.5, 3.0]],
    });
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/song.mid", "song.mid"));
    await act(async () => result.current.engine.confirmTrackSelection([{ index: 0, role: "Auto-Detect" }]));
    expect(tauriMocks.compileNotes).toHaveBeenCalledTimes(1);
    const [config, midiFile, selectedTracksInfo] = tauriMocks.compileNotes.mock.calls[0];
    expect(config.tempo).toBe(100);
    expect(midiFile).toBe("/real/song.mid");
    expect(selectedTracksInfo).toEqual([[0, "Auto-Detect"]]);
    expect(result.current.engine.totalDuration).toBe(12.5);
    expect(result.current.engine.finalNotes).toHaveLength(1);
    expect(result.current.engine.hasCompiledNotes).toBe(true);
    expect(result.current.engine.tempoEvents).toEqual([[0, 545_454]]);
    expect(result.current.engine.timeSignatures).toEqual([[0, 3, 4]]);
    expect(result.current.engine.measureBoundaries).toEqual([[0, 1.5], [1.5, 3.0]]);
  });

  it("loadFile resets tempoEvents/timeSignatures/measureBoundaries from a previously compiled song", async () => {
    tauriMocks.parseMidiStructure.mockResolvedValue({
      tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
      initial_bpm: 120,
    });
    tauriMocks.compileNotes.mockResolvedValue({
      final_notes: [],
      total_dur: 1,
      tempo_events: [[0, 400_000]],
      time_signatures: [[0, 6, 8]],
      measure_boundaries: [[0, 1]],
    });
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/a.mid", "a.mid"));
    await act(async () => result.current.engine.confirmTrackSelection([{ index: 0, role: "Auto-Detect" }]));
    expect(result.current.engine.measureBoundaries.length).toBe(1);

    await act(async () => result.current.engine.loadFile("/real/b.mid", "b.mid"));
    expect(result.current.engine.tempoEvents).toEqual([]);
    expect(result.current.engine.timeSignatures).toEqual([]);
    expect(result.current.engine.measureBoundaries).toEqual([]);
  });

  it("generatePedal calls compile_pedal and writes the returned AI thresholds back into shared PlaybackConfig", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [[0, 1], [2, 3]], ai_thresholds: [0.4, 0.6] });
    const { result } = renderEngine();
    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.hasCompiledPedal).toBe(true);
    expect(result.current.engine.aiThresholds).toEqual([0.4, 0.6]);
    expect(result.current.playbackConfig.config.pedal_threshold_on).toBe(0.4);
    expect(result.current.playbackConfig.config.pedal_threshold_off).toBe(0.6);
  });

  it("play() compiles pedal first if it hasn't been compiled yet, then starts playback", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [], ai_thresholds: null });
    tauriMocks.startPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();
    expect(result.current.engine.hasCompiledPedal).toBe(false);
    await act(async () => result.current.engine.play());
    expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
    expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
  });

  it("defaultAiThresholds captures only the FIRST generation's thresholds, not later regenerations", async () => {
    tauriMocks.compilePedal
      .mockResolvedValueOnce({ pedal_intervals: [[0, 1]], ai_thresholds: [0.62, 0.38] })
      .mockResolvedValueOnce({ pedal_intervals: [[0, 1], [2, 3]], ai_thresholds: [0.71, 0.29] });
    const { result } = renderEngine();

    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.defaultAiThresholds).toEqual([0.62, 0.38]);
    expect(result.current.engine.aiThresholds).toEqual([0.62, 0.38]);

    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.aiThresholds).toEqual([0.71, 0.29]);
    expect(result.current.engine.defaultAiThresholds).toEqual([0.62, 0.38]);
  });

  it("defaultAiThresholds resets on loadFile so the next song's first generation becomes the new default", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [[0, 1]], ai_thresholds: [0.62, 0.38] });
    tauriMocks.parseMidiStructure.mockResolvedValue({ tracks: [], initial_bpm: 120 });
    const { result } = renderEngine();

    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.defaultAiThresholds).toEqual([0.62, 0.38]);

    await act(async () => result.current.engine.loadFile("/real/next.mid", "next.mid"));
    expect(result.current.engine.defaultAiThresholds).toBeNull();
  });

  it("defaultAiThresholds resets on resumeSave", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [[0, 1]], ai_thresholds: [0.62, 0.38] });
    tauriMocks.resumeFromSave.mockResolvedValue({
      final_notes: [],
      total_dur: 1,
      tempo_events: [],
      time_signatures: [],
      measure_boundaries: [],
    });
    const { result } = renderEngine();

    await act(async () => result.current.engine.generatePedal());
    expect(result.current.engine.defaultAiThresholds).toEqual([0.62, 0.38]);

    await act(async () => result.current.engine.resumeSave("/saves/song.json", "song"));
    expect(result.current.engine.defaultAiThresholds).toBeNull();
  });

  it("play() skips re-compiling pedal if it's already compiled", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [[0, 1]], ai_thresholds: [0.5, 0.5] });
    tauriMocks.startPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();
    await act(async () => result.current.engine.generatePedal());
    tauriMocks.compilePedal.mockClear();
    await act(async () => result.current.engine.play());
    expect(tauriMocks.compilePedal).not.toHaveBeenCalled();
    expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
  });

  describe("pedal generation state", () => {
    const pedalResult = { pedal_intervals: [[0, 1]], ai_thresholds: null };

    it("isGeneratingPedal is true only while compile_pedal is in flight", async () => {
      const compile = deferred<typeof pedalResult>();
      tauriMocks.compilePedal.mockReturnValue(compile.promise);
      const { result } = renderEngine();
      expect(result.current.engine.isGeneratingPedal).toBe(false);

      let running!: Promise<void>;
      act(() => {
        running = result.current.engine.generatePedal();
      });
      await waitFor(() => expect(result.current.engine.isGeneratingPedal).toBe(true));
      expect(result.current.engine.hasCompiledPedal).toBe(false);

      await act(async () => {
        compile.resolve(pedalResult);
        await running;
      });
      expect(result.current.engine.isGeneratingPedal).toBe(false);
      expect(result.current.engine.hasCompiledPedal).toBe(true);
    });

    it("isGeneratingPedal returns to false when compile_pedal rejects", async () => {
      const compile = deferred<unknown>();
      tauriMocks.compilePedal.mockReturnValue(compile.promise);
      const { result } = renderEngine();

      let running!: Promise<void>;
      act(() => {
        running = result.current.engine.generatePedal();
      });
      await waitFor(() => expect(result.current.engine.isGeneratingPedal).toBe(true));

      await act(async () => {
        compile.reject(new Error("boom"));
        await running;
      });
      expect(result.current.engine.isGeneratingPedal).toBe(false);
      expect(result.current.engine.hasCompiledPedal).toBe(false);
      expect(result.current.log.entries.some((e) => e.level === "WARN" && e.line.includes("Failed to compile pedal"))).toBe(
        true,
      );
    });

    it("a second generatePedal call while one is running is ignored and does not end the first one's spinner early", async () => {
      const compile = deferred<typeof pedalResult>();
      tauriMocks.compilePedal.mockReturnValue(compile.promise);
      const { result } = renderEngine();

      let first!: Promise<void>;
      act(() => {
        first = result.current.engine.generatePedal();
      });
      await waitFor(() => expect(result.current.engine.isGeneratingPedal).toBe(true));

      await act(async () => result.current.engine.generatePedal());
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
      expect(result.current.engine.isGeneratingPedal).toBe(true);

      await act(async () => {
        compile.resolve(pedalResult);
        await first;
      });
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
      expect(result.current.engine.isGeneratingPedal).toBe(false);
    });

    it("generatePedal can run again after a finished run, including after a failed one", async () => {
      tauriMocks.compilePedal.mockRejectedValueOnce(new Error("boom")).mockResolvedValueOnce(pedalResult);
      const { result } = renderEngine();

      await act(async () => result.current.engine.generatePedal());
      expect(result.current.engine.hasCompiledPedal).toBe(false);
      expect(result.current.engine.isGeneratingPedal).toBe(false);

      await act(async () => result.current.engine.generatePedal());
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(2);
      expect(result.current.engine.hasCompiledPedal).toBe(true);
    });

    it("play() does nothing while a pedal generation is running, then works once it has finished", async () => {
      const compile = deferred<typeof pedalResult>();
      tauriMocks.compilePedal.mockReturnValue(compile.promise);
      tauriMocks.startPlayback.mockResolvedValue(undefined);
      const { result } = renderEngine();

      let running!: Promise<void>;
      act(() => {
        running = result.current.engine.generatePedal();
      });
      await waitFor(() => expect(result.current.engine.isGeneratingPedal).toBe(true));

      await act(async () => result.current.engine.play());
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
      expect(tauriMocks.startPlayback).not.toHaveBeenCalled();

      await act(async () => {
        compile.resolve(pedalResult);
        await running;
      });
      await act(async () => result.current.engine.play());
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
      expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
    });

    it("hotkey_toggle_requested during a running generation neither starts playback nor a second compile", async () => {
      const compile = deferred<typeof pedalResult>();
      tauriMocks.compilePedal.mockReturnValue(compile.promise);
      tauriMocks.startPlayback.mockResolvedValue(undefined);
      const { result } = renderEngine();
      await waitFor(() => expect(eventHandlers.has("hotkey_toggle_requested")).toBe(true));

      let running!: Promise<void>;
      act(() => {
        running = result.current.engine.generatePedal();
      });
      await waitFor(() => expect(result.current.engine.isGeneratingPedal).toBe(true));

      await act(async () => eventHandlers.get("hotkey_toggle_requested")!(null));
      expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
      expect(tauriMocks.startPlayback).not.toHaveBeenCalled();

      await act(async () => {
        compile.resolve(pedalResult);
        await running;
      });
    });
  });

  it("togglePause calls the real command and optimistically flips isPaused", async () => {
    tauriMocks.togglePause.mockResolvedValue(undefined);
    const { result } = renderEngine();
    expect(result.current.engine.isPaused).toBe(false);
    await act(async () => result.current.engine.togglePause());
    expect(tauriMocks.togglePause).toHaveBeenCalledTimes(1);
    expect(result.current.engine.isPaused).toBe(true);
    await act(async () => result.current.engine.togglePause());
    expect(result.current.engine.isPaused).toBe(false);
  });

  it("stop calls the real command and resets currentTime", async () => {
    tauriMocks.stopPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();
    await act(async () => result.current.engine.seek(30));
    tauriMocks.seekPlayback.mockResolvedValue(undefined);
    await act(async () => result.current.engine.stop());
    expect(tauriMocks.stopPlayback).toHaveBeenCalledTimes(1);
    expect(result.current.engine.currentTime).toBe(0);
  });

  it("seek calls the real command with the given time", async () => {
    tauriMocks.seekPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();
    await act(async () => result.current.engine.seek(17.5));
    expect(tauriMocks.seekPlayback).toHaveBeenCalledWith(17.5);
  });

  it("save calls save_playback with the current file name and returns the output path", async () => {
    tauriMocks.savePlayback.mockResolvedValue("/saves/song_20260101_000000.json");
    const { result } = renderEngine();
    await act(async () => result.current.engine.loadFile("/real/song.mid", "song.mid"));
    let path: string | null = null;
    await act(async () => {
      path = await result.current.engine.save();
    });
    expect(path).toBe("/saves/song_20260101_000000.json");
    expect(tauriMocks.savePlayback.mock.calls[0][3]).toBe("song.mid");
  });

  it("save logs a failure message and returns null when the backend call rejects", async () => {
    tauriMocks.savePlayback.mockRejectedValue(new Error("No save directory is configured."));
    const { result } = renderEngine();
    let path: string | null = "unset";
    await act(async () => {
      path = await result.current.engine.save();
    });
    expect(path).toBeNull();
    expect(result.current.log.entries.some((e) => e.level === "WARN" && e.line.includes("Failed to save"))).toBe(
      true,
    );
  });

  it("hotkey_save_requested uses the CURRENT fileName, not whichever one existed when the listener was registered (stale-closure regression guard)", async () => {
    tauriMocks.savePlayback.mockResolvedValue("/saves/out.json");
    const { result } = renderEngine();

    await waitFor(() => expect(eventHandlers.has("hotkey_save_requested")).toBe(true));

    await act(async () => result.current.engine.loadFile("/real/first.mid", "first.mid"));
    await act(async () => result.current.engine.loadFile("/real/second.mid", "second.mid"));
    expect(result.current.engine.fileName).toBe("second.mid");

    await act(async () => eventHandlers.get("hotkey_save_requested")!(null));

    expect(tauriMocks.savePlayback).toHaveBeenCalledTimes(1);
    expect(tauriMocks.savePlayback.mock.calls[0][3]).toBe("second.mid");
  });

  it("hotkey_toggle_requested calls play() when not playing, togglePause() when already playing", async () => {
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [], ai_thresholds: null });
    tauriMocks.startPlayback.mockResolvedValue(undefined);
    tauriMocks.togglePause.mockResolvedValue(undefined);
    renderEngine();

    await waitFor(() => expect(eventHandlers.has("hotkey_toggle_requested")).toBe(true));

    await act(async () => eventHandlers.get("hotkey_toggle_requested")!(null));
    expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
    expect(tauriMocks.togglePause).not.toHaveBeenCalled();

    await act(async () => {
      eventHandlers.get("playback_started")?.(null);
    });

    await act(async () => eventHandlers.get("hotkey_toggle_requested")!(null));
    expect(tauriMocks.togglePause).toHaveBeenCalledTimes(1);
  });

  it("resumeSave calls resume_from_save and populates notes/duration, marking both notes and pedal as compiled", async () => {
    tauriMocks.resumeFromSave.mockResolvedValue({
      final_notes: [
        { id: 0, pitch: 60, velocity: 80, start_time: 0, duration: 1, hand: "unknown", original_track_index: -1, channel: -1 },
      ],
      total_dur: 5.5,
      tempo_events: [[0, 500_000]],
      time_signatures: [],
      measure_boundaries: [[0, 2], [2, 4]],
    });
    const { result } = renderEngine();
    await act(async () => result.current.engine.resumeSave("/saves/song.json", "song"));
    expect(tauriMocks.resumeFromSave).toHaveBeenCalledWith("/saves/song.json");
    expect(result.current.engine.finalNotes).toHaveLength(1);
    expect(result.current.engine.totalDuration).toBe(5.5);
    expect(result.current.engine.hasCompiledNotes).toBe(true);
    expect(result.current.engine.hasCompiledPedal).toBe(true);
    expect(result.current.engine.fileName).toBe("song");
    expect(result.current.engine.measureBoundaries).toEqual([[0, 2], [2, 4]]);
  });

  it("resumeSave leaves hasCompiledPedal false so a stale prior pedal state can't be replayed after a failed resume", async () => {
    tauriMocks.resumeFromSave.mockRejectedValue(new Error("Save file has no compiled events."));
    const { result } = renderEngine();
    await act(async () => result.current.engine.resumeSave("/saves/bad.json", "bad"));
    expect(result.current.engine.hasCompiledPedal).toBe(false);
    expect(result.current.log.entries.some((e) => e.level === "WARN" && e.line.includes("Failed to resume"))).toBe(
      true,
    );
  });

  it("resumeSave, once played, does NOT trigger a redundant compile_pedal call from play()", async () => {
    tauriMocks.resumeFromSave.mockResolvedValue({ final_notes: [], total_dur: 1 });
    tauriMocks.startPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();
    await act(async () => result.current.engine.resumeSave("/saves/song.json", "song"));
    await act(async () => result.current.engine.play());
    expect(tauriMocks.compilePedal).not.toHaveBeenCalled();
    expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
  });

  it("playTranslatedSheet compiles from the sheet, compiles pedal, then starts playback", async () => {
    tauriMocks.compileNotesFromSheet.mockResolvedValue({
      final_notes: [
        { id: 0, pitch: 60, velocity: 64, start_time: 0, duration: 0.25, hand: "right", original_track_index: 0, channel: 0 },
      ],
      total_dur: 0.25,
      tempo_events: [[0, 428_571]],
      time_signatures: [],
      measure_boundaries: [[0, 0.25]],
    });
    tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [], ai_thresholds: null });
    tauriMocks.startPlayback.mockResolvedValue(undefined);
    const { result } = renderEngine();

    await act(async () => result.current.engine.playTranslatedSheet("y y y", 140));

    expect(tauriMocks.compileNotesFromSheet).toHaveBeenCalledTimes(1);
    const [config, sheetText, bpm] = tauriMocks.compileNotesFromSheet.mock.calls[0];
    expect(sheetText).toBe("y y y");
    expect(bpm).toBe(140);
    expect(config.tempo).toBe(100);
    expect(result.current.engine.finalNotes).toHaveLength(1);
    expect(result.current.engine.totalDuration).toBe(0.25);
    expect(result.current.engine.hasCompiledNotes).toBe(true);
    expect(tauriMocks.compilePedal).toHaveBeenCalledTimes(1);
    expect(tauriMocks.startPlayback).toHaveBeenCalledTimes(1);
    expect(result.current.engine.tempoEvents).toEqual([[0, 428_571]]);
  });

  it("playTranslatedSheet logs a failure and never reaches startPlayback when compile_notes_from_sheet rejects", async () => {
    tauriMocks.compileNotesFromSheet.mockRejectedValue(new Error("Sheet produced zero playable notes."));
    const { result } = renderEngine();
    await act(async () => result.current.engine.playTranslatedSheet("~~~", 120));
    expect(tauriMocks.compilePedal).not.toHaveBeenCalled();
    expect(tauriMocks.startPlayback).not.toHaveBeenCalled();
    expect(
      result.current.log.entries.some((e) => e.level === "WARN" && e.line.includes("Failed to play translated sheet")),
    ).toBe(true);
  });

  describe("Debug tab Session Snapshot wiring", () => {
    it("loadFile clears the snapshot then sets file/source, and tempo once the parse resolves", async () => {
      tauriMocks.parseMidiStructure.mockResolvedValue({
        tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
        initial_bpm: 128.6,
      });
      const { result } = renderEngine();
      await act(async () => result.current.engine.loadFile("/real/song.mid", "song.mid"));
      expect(result.current.log.snapshot.file).toBe("song.mid");
      expect(result.current.log.snapshot.source).toBe("MIDI file");
      expect(result.current.log.snapshot.tempo).toBe("129 BPM");
    });

    it("confirmTrackSelection sets tracks/notes/duration on the snapshot", async () => {
      tauriMocks.parseMidiStructure.mockResolvedValue({
        tracks: [{ index: 0, name: "Melody", note_count: 50, instrument_name: "Piano", is_drum: false }],
        initial_bpm: 120,
      });
      tauriMocks.compileNotes.mockResolvedValue({
        final_notes: [
          { id: 0, pitch: 60, velocity: 80, start_time: 0, duration: 1, hand: "left", original_track_index: 0, channel: 0 },
        ],
        total_dur: 125,
        tempo_events: [],
        time_signatures: [],
        measure_boundaries: [],
      });
      const { result } = renderEngine();
      await act(async () => result.current.engine.loadFile("/real/song.mid", "song.mid"));
      await act(async () => result.current.engine.confirmTrackSelection([{ index: 0, role: "Auto-Detect" }]));
      expect(result.current.log.snapshot.tracks).toBe("1");
      expect(result.current.log.snapshot.notes).toBe("1");
      expect(result.current.log.snapshot.duration).toBe("2:05");
    });

    it("generatePedal sets the pedal press count on the snapshot", async () => {
      tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [[0, 1], [2, 3], [4, 5]], ai_thresholds: null });
      const { result } = renderEngine();
      await act(async () => result.current.engine.generatePedal());
      expect(result.current.log.snapshot.pedal).toBe("3 presses");
    });

    it("resumeSave clears the snapshot then sets file/source/tracks/pedal from the resumed save", async () => {
      tauriMocks.resumeFromSave.mockResolvedValue({
        final_notes: [],
        total_dur: 5.5,
        tempo_events: [],
        time_signatures: [],
        measure_boundaries: [],
        track_count: 4,
        pedal_count: 9,
      });
      const { result } = renderEngine();
      await act(async () => result.current.engine.resumeSave("/saves/song.json", "song"));
      expect(result.current.log.snapshot.file).toBe("song");
      expect(result.current.log.snapshot.source).toBe("Save file");
      expect(result.current.log.snapshot.tracks).toBe("4");
      expect(result.current.log.snapshot.pedal).toBe("9 presses");
    });

    it("playTranslatedSheet clears the snapshot then sets file/source/tempo for the pasted sheet", async () => {
      tauriMocks.compileNotesFromSheet.mockResolvedValue({
        final_notes: [],
        total_dur: 1,
        tempo_events: [],
        time_signatures: [],
        measure_boundaries: [],
      });
      tauriMocks.compilePedal.mockResolvedValue({ pedal_intervals: [], ai_thresholds: null });
      tauriMocks.startPlayback.mockResolvedValue(undefined);
      const { result } = renderEngine();
      await act(async () => result.current.engine.playTranslatedSheet("y y y", 140));
      expect(result.current.log.snapshot.file).toBe("(pasted sheet)");
      expect(result.current.log.snapshot.source).toBe("Virtual Piano import");
      expect(result.current.log.snapshot.tempo).toBe("140 BPM");
    });
  });
});
