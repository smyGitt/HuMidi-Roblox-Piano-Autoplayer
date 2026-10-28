import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { AppSettingsProvider, useAppSettings } from "./AppSettingsContext";
import { LogProvider, useLog } from "./LogContext";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(() => Promise.resolve({})),
  saveAppConfig: vi.fn(() => Promise.resolve()),
  onEvent: vi.fn(() => Promise.resolve(() => {})),
}));

vi.mock("../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri")>("../lib/tauri");
  return { ...actual, ...tauriMocks };
});

const setAlwaysOnTop = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setAlwaysOnTop }),
}));

function wrapper({ children }: { children: ReactNode }) {
  return (
    <LogProvider>
      <AppSettingsProvider>{children}</AppSettingsProvider>
    </LogProvider>
  );
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  tauriMocks.loadAppConfig.mockReset().mockReturnValue(Promise.resolve({}));
  tauriMocks.saveAppConfig.mockReset().mockReturnValue(Promise.resolve());
  setAlwaysOnTop.mockReset().mockReturnValue(Promise.resolve());
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("AppSettingsContext (preview mode, isTauri() = false)", () => {
  it("starts at documented defaults and never touches the backend", () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    expect(result.current.alwaysOnTop).toBe(false);
    expect(result.current.opacity).toBe(100);
    expect(result.current.showTimeline).toBe(true);
    expect(result.current.showPiano).toBe(true);
    expect(result.current.showPianoPedal).toBe(true);
    expect(result.current.visualizerHorizontal).toBe(true);
    expect(result.current.pedalPromptThreshold).toBe(8);
    expect(result.current.autoCheckUpdates).toBe(false);
    expect(result.current.showStatusText).toBe(false);
    expect(result.current.switchToVisualizerOnPlay).toBe(true);
    expect(result.current.maxVisibleSaves).toBe(20);
    expect(tauriMocks.loadAppConfig).not.toHaveBeenCalled();
  });

  it("setters mutate local state without calling saveAppConfig or the window API", () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    act(() => result.current.setPedalPromptThreshold(20));
    act(() => result.current.setAlwaysOnTop(true));
    expect(result.current.pedalPromptThreshold).toBe(20);
    expect(result.current.alwaysOnTop).toBe(true);
    expect(tauriMocks.saveAppConfig).not.toHaveBeenCalled();
    expect(setAlwaysOnTop).not.toHaveBeenCalled();
  });
});

describe("AppSettingsContext (Tauri mode, isTauri() = true)", () => {
  beforeEach(() => {
    tauriMocks.isTauri.mockReturnValue(true);
  });

  it("hydrates every field from load_app_config on mount", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(
      Promise.resolve({
        always_on_top: true,
        opacity: 70,
        show_timeline_visualizer: false,
        show_piano_visualizer: false,
        show_piano_pedal_visualizer: false,
        visualizer_horizontal: false,
        pedal_prompt_threshold: 42,
        auto_check_updates: false,
        show_status_text: true,
        switch_to_visualizer_on_play: false,
        max_visible_saves: 7,
      }),
    );
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(result.current.pedalPromptThreshold).toBe(42));
    expect(result.current.showStatusText).toBe(true);
    expect(result.current.switchToVisualizerOnPlay).toBe(false);
    expect(result.current.maxVisibleSaves).toBe(7);
    expect(result.current.alwaysOnTop).toBe(true);
    expect(result.current.opacity).toBe(70);
    expect(result.current.showTimeline).toBe(false);
    expect(result.current.showPiano).toBe(false);
    expect(result.current.showPianoPedal).toBe(false);
    expect(result.current.visualizerHorizontal).toBe(false);
    expect(result.current.autoCheckUpdates).toBe(false);
  });

  it("setVisualizerHorizontal persists under visualizer_horizontal", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setVisualizerHorizontal(false));
    expect(result.current.visualizerHorizontal).toBe(false);
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ visualizer_horizontal: false });
  });

  it("an empty persisted config leaves every field at its default", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(tauriMocks.loadAppConfig).toHaveBeenCalled());
    expect(result.current.pedalPromptThreshold).toBe(8);
    expect(result.current.autoCheckUpdates).toBe(false);
  });

  it("setPedalPromptThreshold persists under the exact Python key name", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setPedalPromptThreshold(15));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ pedal_prompt_threshold: 15 });
  });

  it("setMaxVisibleSaves persists under max_visible_saves and clamps to 1..100", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setMaxVisibleSaves(35));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ max_visible_saves: 35 });
    expect(result.current.maxVisibleSaves).toBe(35);

    await act(async () => result.current.setMaxVisibleSaves(0));
    expect(result.current.maxVisibleSaves).toBe(1);
    expect(tauriMocks.saveAppConfig).toHaveBeenLastCalledWith({ max_visible_saves: 1 });

    await act(async () => result.current.setMaxVisibleSaves(5000));
    expect(result.current.maxVisibleSaves).toBe(100);
    expect(tauriMocks.saveAppConfig).toHaveBeenLastCalledWith({ max_visible_saves: 100 });
  });

  it("a persisted max_visible_saves outside 1..100 is clamped on load", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ max_visible_saves: 9999 }));
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(result.current.maxVisibleSaves).toBe(100));
  });

  it("keyboard hotkeys are on by default and an empty config keeps them on", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(tauriMocks.loadAppConfig).toHaveBeenCalled());
    expect(result.current.linuxKeyboardHotkeys).toBe(true);
  });

  it("hydrates the keyboard hotkeys setting from linux_keyboard_hotkeys", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ linux_keyboard_hotkeys: false }));
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(result.current.linuxKeyboardHotkeys).toBe(false));
  });

  it("ignores a keyboard hotkeys setting of the wrong type", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ linux_keyboard_hotkeys: "no" }));
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(tauriMocks.loadAppConfig).toHaveBeenCalled());
    expect(result.current.linuxKeyboardHotkeys).toBe(true);
  });

  it("setLinuxKeyboardHotkeys persists under linux_keyboard_hotkeys", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setLinuxKeyboardHotkeys(false));
    expect(result.current.linuxKeyboardHotkeys).toBe(false);
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ linux_keyboard_hotkeys: false });
    await act(async () => result.current.setLinuxKeyboardHotkeys(true));
    expect(tauriMocks.saveAppConfig).toHaveBeenLastCalledWith({ linux_keyboard_hotkeys: true });
  });

  it("setShowStatusText persists under show_status_text", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setShowStatusText(true));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ show_status_text: true });
  });

  it("setSwitchToVisualizerOnPlay persists under switch_to_visualizer_on_play", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setSwitchToVisualizerOnPlay(false));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ switch_to_visualizer_on_play: false });
  });

  it("setAlwaysOnTop persists to config AND calls the real Tauri window API", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setAlwaysOnTop(true));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ always_on_top: true });
    expect(setAlwaysOnTop).toHaveBeenCalledWith(true);
  });

  it("applies a saved always_on_top = true to the window when the config loads", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ always_on_top: true }));
    renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(setAlwaysOnTop).toHaveBeenCalledWith(true));
  });

  it("does not touch the window when the saved always_on_top is false or absent", async () => {
    tauriMocks.loadAppConfig.mockReturnValue(Promise.resolve({ always_on_top: false }));
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(tauriMocks.loadAppConfig).toHaveBeenCalled());
    await act(async () => {});
    expect(result.current.alwaysOnTop).toBe(false);
    expect(setAlwaysOnTop).not.toHaveBeenCalled();
  });

  it("writes a rejected setAlwaysOnTop call to the app log instead of swallowing it", async () => {
    setAlwaysOnTop.mockReturnValue(Promise.reject(new Error("not allowed")));
    const { result } = renderHook(() => ({ settings: useAppSettings(), log: useLog() }), { wrapper });
    await act(async () => result.current.settings.setAlwaysOnTop(true));
    await waitFor(() =>
      expect(result.current.log.entries.some((entry) => entry.line.includes("Always on top failed: Error: not allowed"))).toBe(true),
    );
  });

  it("setAutoCheckUpdates persists under the exact Python key name", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setAutoCheckUpdates(false));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ auto_check_updates: false });
  });
});
