import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { ThemeProvider, useTheme } from "./ThemeProvider";
import { DEFAULT_THEME_NAME } from "./tokens";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  getActiveThemeName: vi.fn<() => Promise<string | null>>(() => Promise.resolve(null)),
  setActiveThemeName: vi.fn(() => Promise.resolve()),
  getCustomThemes: vi.fn<() => Promise<import("../lib/tauri").CustomThemeColors[]>>(() => Promise.resolve([])),
}));

vi.mock("../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri")>("../lib/tauri");
  return { ...actual, ...tauriMocks };
});

function wrapper({ children }: { children: ReactNode }) {
  return <ThemeProvider>{children}</ThemeProvider>;
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  tauriMocks.getActiveThemeName.mockReset().mockReturnValue(Promise.resolve(null));
  tauriMocks.setActiveThemeName.mockReset().mockReturnValue(Promise.resolve());
  tauriMocks.getCustomThemes.mockReset().mockReturnValue(Promise.resolve([]));
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("ThemeProvider (preview mode, isTauri() = false)", () => {
  it("starts at the default theme and never calls the backend", async () => {
    const { result } = renderHook(() => useTheme(), { wrapper });
    expect(result.current.themeName).toBe(DEFAULT_THEME_NAME);
    await act(async () => result.current.setThemeName("Light"));
    expect(result.current.themeName).toBe("Light");
    expect(tauriMocks.setActiveThemeName).not.toHaveBeenCalled();
    expect(tauriMocks.getActiveThemeName).not.toHaveBeenCalled();
  });
});

describe("ThemeProvider (Tauri mode, isTauri() = true)", () => {
  beforeEach(() => {
    tauriMocks.isTauri.mockReturnValue(true);
  });

  it("hydrates from the persisted active theme name on mount", async () => {
    tauriMocks.getActiveThemeName.mockReturnValue(Promise.resolve("Hatsune Miku"));
    const { result } = renderHook(() => useTheme(), { wrapper });
    await waitFor(() => expect(result.current.themeName).toBe("Hatsune Miku"));
  });

  it("ignores a persisted name that matches no built-in or custom theme", async () => {
    tauriMocks.getActiveThemeName.mockReturnValue(Promise.resolve("Nonexistent Theme"));
    const { result } = renderHook(() => useTheme(), { wrapper });
    await waitFor(() => expect(tauriMocks.getActiveThemeName).toHaveBeenCalled());
    expect(result.current.themeName).toBe(DEFAULT_THEME_NAME);
  });

  it("accepts a persisted name that matches a custom theme", async () => {
    tauriMocks.getActiveThemeName.mockReturnValue(Promise.resolve("Sunset"));
    tauriMocks.getCustomThemes.mockReturnValue(
      Promise.resolve([
        {
          name: "Sunset",
          bg_primary: "#000",
          bg_surface: "#111",
          bg_input: "#222",
          text_primary: "#fff",
          text_muted: "#aaa",
          border: "#333",
          accent: "#f80",
          accent_play: "#0f0",
          accent_stop: "#f00",
          pedal_color: "#fa0",
          accent_loaded: "#ff0",
          knob_color: "#fff",
          builtin: false,
        },
      ]),
    );
    const { result } = renderHook(() => useTheme(), { wrapper });
    await waitFor(() => expect(result.current.themeName).toBe("Sunset"));
  });

  it("setThemeName persists the choice via set_active_theme_name", async () => {
    const { result } = renderHook(() => useTheme(), { wrapper });
    await act(async () => result.current.setThemeName("Light"));
    expect(result.current.themeName).toBe("Light");
    expect(tauriMocks.setActiveThemeName).toHaveBeenCalledWith("Light");
  });
});
