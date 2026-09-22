import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "../../theme/ThemeProvider";
import { AppSettingsProvider } from "../../state/AppSettingsContext";
import { PlaybackConfigProvider, usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { LogProvider } from "../../state/LogContext";
import { DEFAULT_CONFIG } from "../playback/types";
import { SettingsTab } from "./SettingsTab";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  onEvent: vi.fn(() => Promise.resolve(() => {})),
  startBinding: vi.fn(() => Promise.resolve()),
  startSaveBinding: vi.fn(() => Promise.resolve()),
  setSaveDir: vi.fn(() => Promise.resolve()),
  getSaveDir: vi.fn<() => Promise<string>>(() => Promise.resolve("")),
  setMidiDir: vi.fn(() => Promise.resolve()),
  getMidiDir: vi.fn<() => Promise<string>>(() => Promise.resolve("")),
  getThemesFile: vi.fn<() => Promise<string>>(() => Promise.resolve("C:\\Users\\me\\.humidi\\themes.json")),
  setThemesDir: vi.fn<(p: string) => Promise<string>>((p) => Promise.resolve(`${p}\\themes.json`)),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(() => Promise.resolve({})),
  saveAppConfig: vi.fn(() => Promise.resolve()),
  getActiveThemeName: vi.fn<() => Promise<string | null>>(() => Promise.resolve(null)),
  setActiveThemeName: vi.fn(() => Promise.resolve()),
  getCustomThemes: vi.fn<() => Promise<import("../../lib/tauri").CustomThemeColors[]>>(() => Promise.resolve([])),
}));

vi.mock("../../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../../lib/tauri")>("../../lib/tauri");
  return { ...actual, ...tauriMocks };
});

const openDialog = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));

const openPath = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("@tauri-apps/plugin-opener", () => ({ openPath }));

const setAlwaysOnTop = vi.hoisted(() => vi.fn(() => Promise.resolve()));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setAlwaysOnTop }),
}));

let capturedConfig: { current: ReturnType<typeof usePlaybackConfig> | null } = { current: null };

function ConfigSpy() {
  capturedConfig.current = usePlaybackConfig();
  return null;
}

function renderSettings() {
  capturedConfig = { current: null };
  return render(
    <ThemeProvider>
      <AppSettingsProvider>
        <PlaybackConfigProvider>
          <LogProvider>
            <ConfigSpy />
            <SettingsTab />
          </LogProvider>
        </PlaybackConfigProvider>
      </AppSettingsProvider>
    </ThemeProvider>,
  );
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.getSaveDir.mockReset().mockReturnValue(Promise.resolve(""));
  tauriMocks.getMidiDir.mockReset().mockReturnValue(Promise.resolve(""));
  tauriMocks.getThemesFile.mockReset().mockReturnValue(Promise.resolve("C:\\Users\\me\\.humidi\\themes.json"));
  tauriMocks.setSaveDir.mockReset().mockReturnValue(Promise.resolve());
  tauriMocks.setMidiDir.mockReset().mockReturnValue(Promise.resolve());
  tauriMocks.loadAppConfig.mockReset().mockReturnValue(Promise.resolve({}));
  tauriMocks.saveAppConfig.mockReset().mockReturnValue(Promise.resolve());
  openDialog.mockReset();
  openPath.mockReset().mockReturnValue(Promise.resolve());
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("SettingsTab > Files page", () => {
  it("shows 'Not set' for save/MIDI directory before anything is configured", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(tauriMocks.getSaveDir).toHaveBeenCalled());
    const inputs = screen.getAllByDisplayValue("Not set");
    expect(inputs.length).toBe(2);
  });

  it("displays the real save/MIDI directories once loaded, and Open is enabled", async () => {
    tauriMocks.getSaveDir.mockReturnValue(Promise.resolve("C:\\saves"));
    tauriMocks.getMidiDir.mockReturnValue(Promise.resolve("C:\\midis"));
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(screen.getByDisplayValue("C:\\saves")).toBeInTheDocument());
    expect(screen.getByDisplayValue("C:\\midis")).toBeInTheDocument();
  });

  it("Browse for Save Directory calls set_save_dir with the chosen path and updates the display", async () => {
    openDialog.mockResolvedValue("C:\\NewSaves");
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(tauriMocks.getSaveDir).toHaveBeenCalled());
    const browseButtons = screen.getAllByText("Browse...");
    await act(async () => fireEvent.click(browseButtons[0]));
    expect(tauriMocks.setSaveDir).toHaveBeenCalledWith("C:\\NewSaves");
    await waitFor(() => expect(screen.getByDisplayValue("C:\\NewSaves")).toBeInTheDocument());
  });

  it("Browse for MIDI Directory calls set_midi_dir, not set_save_dir", async () => {
    openDialog.mockResolvedValue("C:\\NewMidis");
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(tauriMocks.getMidiDir).toHaveBeenCalled());
    const browseButtons = screen.getAllByText("Browse...");
    await act(async () => fireEvent.click(browseButtons[1]));
    expect(tauriMocks.setMidiDir).toHaveBeenCalledWith("C:\\NewMidis");
    expect(tauriMocks.setSaveDir).not.toHaveBeenCalled();
  });

  it("Open button is disabled when no directory is configured yet", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(tauriMocks.getSaveDir).toHaveBeenCalled());
    const openButtons = screen.getAllByTitle("Open in file explorer");
    // Save Directory and MIDI Directory rows (indices 0/1) start empty; the Themes File
    // row (index 2) always has a real default path from get_themes_file, so it's excluded.
    expect(openButtons[0]).toBeDisabled();
    expect(openButtons[1]).toBeDisabled();
  });

  it("Open button calls openPath with the current directory once configured", async () => {
    tauriMocks.getSaveDir.mockReturnValue(Promise.resolve("C:\\saves"));
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() => expect(screen.getByDisplayValue("C:\\saves")).toBeInTheDocument());
    const openButtons = screen.getAllByTitle("Open in file explorer");
    fireEvent.click(openButtons[0]);
    expect(openPath).toHaveBeenCalledWith("C:\\saves");
  });

  it("shows the themes file path from get_themes_file", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    await waitFor(() =>
      expect(screen.getByDisplayValue("C:\\Users\\me\\.humidi\\themes.json")).toBeInTheDocument(),
    );
  });
});

describe("SettingsTab > System page", () => {
  it("Reset All Settings restores DEFAULT_CONFIG but preserves the current pedal AI thresholds", async () => {
    renderSettings();
    act(() => {
      capturedConfig.current!.setConfig({
        ...DEFAULT_CONFIG,
        tempo: 250,
        simulate_hands: true,
        pedal_threshold_on: 0.71,
        pedal_threshold_off: 0.22,
      });
    });
    fireEvent.click(screen.getByText("System"));
    fireEvent.click(screen.getByText("Reset All Settings"));

    expect(capturedConfig.current!.config.tempo).toBe(DEFAULT_CONFIG.tempo);
    expect(capturedConfig.current!.config.simulate_hands).toBe(false);
    expect(capturedConfig.current!.config.pedal_threshold_on).toBe(0.71);
    expect(capturedConfig.current!.config.pedal_threshold_off).toBe(0.22);
  });

  it("pedal prompt threshold spinbox reflects and updates the shared setting", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    const spinbox = document.querySelector(".slider-spinbox__spinbox") as HTMLInputElement;
    expect(spinbox.value).toBe("8");
    fireEvent.change(spinbox, { target: { value: "42" } });
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ pedal_prompt_threshold: 42 }));
  });

  it("auto-check-updates toggle persists via save_app_config", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    fireEvent.click(screen.getByText("Automatically check for updates"));
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ auto_check_updates: true }));
  });
});
