import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { AppSettingsProvider, useAppSettings } from "./AppSettingsContext";

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
  return <AppSettingsProvider>{children}</AppSettingsProvider>;
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
    expect(result.current.pedalPromptThreshold).toBe(8);
    expect(result.current.autoCheckUpdates).toBe(false);
    expect(result.current.showStatusText).toBe(false);
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
        pedal_prompt_threshold: 42,
        auto_check_updates: false,
        show_status_text: true,
        max_visible_saves: 7,
      }),
    );
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await waitFor(() => expect(result.current.pedalPromptThreshold).toBe(42));
    expect(result.current.showStatusText).toBe(true);
    expect(result.current.maxVisibleSaves).toBe(7);
    expect(result.current.alwaysOnTop).toBe(true);
    expect(result.current.opacity).toBe(70);
    expect(result.current.showTimeline).toBe(false);
    expect(result.current.showPiano).toBe(false);
    expect(result.current.showPianoPedal).toBe(false);
    expect(result.current.autoCheckUpdates).toBe(false);
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

  it("setShowStatusText persists under show_status_text", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setShowStatusText(true));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ show_status_text: true });
  });

  it("setAlwaysOnTop persists to config AND calls the real Tauri window API", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setAlwaysOnTop(true));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ always_on_top: true });
    expect(setAlwaysOnTop).toHaveBeenCalledWith(true);
  });

  it("setAutoCheckUpdates persists under the exact Python key name", async () => {
    const { result } = renderHook(() => useAppSettings(), { wrapper });
    await act(async () => result.current.setAutoCheckUpdates(false));
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ auto_check_updates: false });
  });
});
