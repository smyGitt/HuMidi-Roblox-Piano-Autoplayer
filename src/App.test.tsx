import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { toBackendConfig } from "./lib/tauri";
import { DEFAULT_CONFIG } from "./pages/playback/types";

const FORWARD_COMMAND = "forward_key";

const tauriApi = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
  listen: vi.fn(async () => () => {}),
  getVersion: vi.fn(async () => "0.0.0"),
  dialogOpen: vi.fn<() => Promise<string | null>>(),
  windowHandle: {
    setAlwaysOnTop: vi.fn(async () => {}),
    setSize: vi.fn(async () => {}),
    innerSize: vi.fn(async () => ({ width: 0, height: 0 })),
    scaleFactor: vi.fn(async () => 1),
    onResized: vi.fn(async () => () => {}),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriApi.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauriApi.listen }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: tauriApi.getVersion }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriApi.windowHandle,
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: tauriApi.dialogOpen, save: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn(), openPath: vi.fn() }));

function press(code: string) {
  window.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true }));
}

function release(code: string) {
  window.dispatchEvent(new KeyboardEvent("keyup", { code, bubbles: true, cancelable: true }));
}

function forwarded() {
  return tauriApi.invoke.mock.calls.filter(([command]) => command === FORWARD_COMMAND);
}

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  tauriApi.dialogOpen.mockReset();
  tauriApi.invoke.mockReset().mockImplementation(async (command: string) => (command === "load_app_config" ? {} : undefined));
});

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("App key forwarding", () => {
  it("sends every key press and release in the window to the backend as hotkey input", async () => {
    render(<App />);
    await act(async () => {
      await Promise.resolve();
    });
    act(() => {
      press("KeyG");
      release("KeyG");
    });
    expect(forwarded()).toEqual([
      [FORWARD_COMMAND, { code: "KeyG", pressed: true }],
      [FORWARD_COMMAND, { code: "KeyG", pressed: false }],
    ]);
  });

  it("stops sending keys once the app is unmounted", async () => {
    const { unmount } = render(<App />);
    await act(async () => {
      await Promise.resolve();
    });
    unmount();
    act(() => {
      press("KeyG");
    });
    expect(forwarded()).toEqual([]);
  });
});

const SAVE_PATH = "C:/saves/first.json";
const MIDI_PATH = "C:/music/song.mid";
const SAVE_DIR_COMMAND = "get_save_dir";
const LIST_SAVES_COMMAND = "list_saves";
const LOAD_FOR_EDIT_COMMAND = "load_save_for_edit";
const COMPILE_NOTES_COMMAND = "compile_notes";
const COMPILE_PEDAL_COMMAND = "compile_pedal";
const PARSE_COMMAND = "parse_midi_structure";
const SAVED_TEMPO = 80;
const NOTE_COUNT = 12;
const PEDAL_FPS = 50;

const SUMMARY = {
  path: SAVE_PATH,
  filename: "first.json",
  song_name: "song.mid",
  created: "2026-10-01T10:00:00+00:00",
  last_accessed: "2026-10-01T10:00:00+00:00",
  track_count: 1,
  note_count: NOTE_COUNT,
  pedal_count: 3,
  tempo: 100,
  pedal_style: "ai",
  use_88_key_layout: false,
  humanization: [],
};

const PLAN = {
  midi_file: MIDI_PATH,
  settings: toBackendConfig({ ...DEFAULT_CONFIG, tempo: SAVED_TEMPO }, MIDI_PATH),
  tracks: [{ index: 1, role: "Right Hand", name: "Piano", note_count: NOTE_COUNT }],
  legacy_pedal_thresholds: false,
};

const PARSED = {
  tracks: [{ index: 1, name: "Piano", note_count: NOTE_COUNT, instrument_name: "Piano", is_drum: false }],
  initial_bpm: 120,
};

const TIMELINE = {
  final_notes: [],
  played_notes: [],
  total_dur: 10,
  tempo_events: [[0, 500_000]],
  time_signatures: [],
  measure_boundaries: [[0, 2]],
};

const PEDAL = { pedal_intervals: [[0, 1]], curve_on: [], curve_off: [], confidence: [], confidence_fps: PEDAL_FPS };

function answers(overrides: Record<string, () => unknown> = {}) {
  const table: Record<string, () => unknown> = {
    load_app_config: () => ({}),
    [SAVE_DIR_COMMAND]: () => "C:/saves",
    [LIST_SAVES_COMMAND]: () => [SUMMARY],
    [LOAD_FOR_EDIT_COMMAND]: () => PLAN,
    [PARSE_COMMAND]: () => PARSED,
    [COMPILE_NOTES_COMMAND]: () => TIMELINE,
    [COMPILE_PEDAL_COMMAND]: () => PEDAL,
    ...overrides,
  };
  tauriApi.invoke.mockImplementation(async (command: string) => table[command]?.());
}

function invoked(command: string) {
  return tauriApi.invoke.mock.calls.filter(([name]) => name === command);
}

async function openSavedSong() {
  render(<App />);
  fireEvent.click(await screen.findByText("first"));
  return screen.findByRole("button", { name: "Load and Edit" });
}

describe("App Load and Edit", () => {
  it("asks the backend to open the picked save for editing", async () => {
    answers();
    fireEvent.click(await openSavedSong());
    await waitFor(() => expect(invoked(LOAD_FOR_EDIT_COMMAND)).toEqual([[LOAD_FOR_EDIT_COMMAND, { filepath: SAVE_PATH }]]));
  });

  it("closes the Load Save window and compiles the saved tracks without asking which tracks to use", async () => {
    answers();
    fireEvent.click(await openSavedSong());
    await waitFor(() => expect(invoked(COMPILE_PEDAL_COMMAND)).toHaveLength(1));
    expect(screen.queryByText("Load Save")).toBeNull();
    expect(screen.queryByText("Select Tracks")).toBeNull();
    expect((invoked(COMPILE_NOTES_COMMAND)[0][1] as { selectedTracksInfo: unknown }).selectedTracksInfo).toEqual([
      [1, "Right Hand"],
    ]);
  });

  it("tells the user why a save cannot be opened and goes away with OK", async () => {
    const reason = "The MIDI file for this save was not found: C:/music/song.mid.";
    answers({
      [LOAD_FOR_EDIT_COMMAND]: () => {
        throw reason;
      },
    });
    fireEvent.click(await openSavedSong());
    expect(await screen.findByText("Cannot open for editing")).toBeTruthy();
    expect(screen.getByText(reason)).toBeTruthy();
    expect(screen.queryByText("Load Save")).toBeNull();
    fireEvent.click(screen.getByText("OK"));
    expect(screen.queryByText("Cannot open for editing")).toBeNull();
  });

  it("still asks which tracks to use when an ordinary MIDI file is opened", async () => {
    answers();
    tauriApi.dialogOpen.mockResolvedValue(MIDI_PATH);
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Replace" }));
    expect(await screen.findByText("Select Tracks")).toBeTruthy();
    expect(invoked(LOAD_FOR_EDIT_COMMAND)).toHaveLength(0);
  });

  it("tells the user why playback could not start and goes away with OK", async () => {
    const reason = "HuMidi cannot send key presses because it has no access to /dev/uinput. Details: no display";
    answers({
      start_playback: () => {
        throw reason;
      },
    });
    fireEvent.click(await openSavedSong());
    await waitFor(() => expect(invoked(COMPILE_PEDAL_COMMAND)).toHaveLength(1));
    const play = screen.getByTitle("Play") as HTMLButtonElement;
    await waitFor(() => expect(play.disabled).toBe(false));
    fireEvent.click(play);
    expect(await screen.findByText("Cannot start playback")).toBeTruthy();
    expect(screen.getByText(reason)).toBeTruthy();
    fireEvent.click(screen.getByText("OK"));
    expect(screen.queryByText("Cannot start playback")).toBeNull();
  });
});

describe("App theme image", () => {
  const IMAGE = "data:image/png;base64,AAAA";
  const THEME_NAME = "Picture";

  function themeColors(image: string, imageScale = 1, imageX = 0, imageY = 0) {
    return {
      name: THEME_NAME,
      bg_primary: "#000000",
      bg_surface: "#111111",
      bg_input: "#222222",
      text_primary: "#ffffff",
      text_muted: "#aaaaaa",
      border: "#333333",
      accent: "#ff8800",
      accent_play: "#00ff00",
      accent_stop: "#ff0000",
      pedal_color: "#ffaa00",
      accent_loaded: "#ffff00",
      knob_color: "#ffffff",
      viz_note_left: "",
      viz_note_right: "",
      viz_note_other: "",
      viz_progress_line: "",
      viz_measure_line: "",
      viz_key_white: "",
      viz_key_black: "",
      viz_key_pressed: "",
      image,
      image_scale: imageScale,
      image_x: imageX,
      image_y: imageY,
      builtin: false,
    };
  }

  function answerWithTheme(image: string, imageScale = 1, imageX = 0, imageY = 0) {
    tauriApi.invoke.mockImplementation(async (command: string) => {
      if (command === "load_app_config") return {};
      if (command === "get_active_theme_name") return THEME_NAME;
      if (command === "get_custom_themes") return [themeColors(image, imageScale, imageX, imageY)];
      return undefined;
    });
  }

  it("shows the active theme's image in the transport bar", async () => {
    answerWithTheme(IMAGE);
    render(<App />);
    await waitFor(() => expect(document.querySelector(".transport-bar__image")?.getAttribute("src")).toBe(IMAGE));
  });

  it("scales and moves the image by the active theme's zoom and offsets", async () => {
    answerWithTheme(IMAGE, 6.7, -12, 7);
    render(<App />);
    await waitFor(() => expect(document.querySelector(".transport-bar__image")).not.toBeNull());
    const style = (document.querySelector(".transport-bar__image") as HTMLElement).style;
    expect([style.getPropertyValue("--image-zoom"), style.getPropertyValue("--image-x"), style.getPropertyValue("--image-y")]).toEqual([
      "6.7",
      "-12px",
      "7px",
    ]);
  });

  it("shows no image when the active theme has none", async () => {
    answerWithTheme("");
    render(<App />);
    await act(async () => {
      await Promise.resolve();
    });
    await waitFor(() => expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#ff8800"));
    expect(document.querySelector(".transport-bar__image")).toBeNull();
  });
});
