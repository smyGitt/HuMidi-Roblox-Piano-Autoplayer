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
  getHotkeys: vi.fn<() => Promise<[string, string]>>(() => Promise.resolve(["F6", "Ctrl+S"])),
  setSaveDir: vi.fn(() => Promise.resolve()),
  getSaveDir: vi.fn<() => Promise<string>>(() => Promise.resolve("")),
  setMidiDir: vi.fn(() => Promise.resolve()),
  getMidiDir: vi.fn<() => Promise<string>>(() => Promise.resolve("")),
  getThemesFile: vi.fn<() => Promise<string>>(() => Promise.resolve("C:\\Users\\me\\.humidi\\themes.json")),
  setThemesDir: vi.fn<(p: string) => Promise<string>>((p) => Promise.resolve(`${p}\\themes.json`)),
  getAppVersion: vi.fn<() => Promise<string>>(() => Promise.resolve("9.8.7")),
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

const platformMocks = vi.hoisted(() => ({ isLinux: vi.fn(() => true) }));
vi.mock("../../lib/platform", () => platformMocks);

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
      <LogProvider>
        <AppSettingsProvider>
          <PlaybackConfigProvider>
            <ConfigSpy />
            <SettingsTab />
          </PlaybackConfigProvider>
        </AppSettingsProvider>
      </LogProvider>
    </ThemeProvider>,
  );
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(true);
  platformMocks.isLinux.mockReturnValue(true);
  tauriMocks.getSaveDir.mockReset().mockReturnValue(Promise.resolve(""));
  tauriMocks.getMidiDir.mockReset().mockReturnValue(Promise.resolve(""));
  tauriMocks.getThemesFile.mockReset().mockReturnValue(Promise.resolve("C:\\Users\\me\\.humidi\\themes.json"));
  tauriMocks.setSaveDir.mockReset().mockReturnValue(Promise.resolve());
  tauriMocks.setMidiDir.mockReset().mockReturnValue(Promise.resolve());
  tauriMocks.getAppVersion.mockClear();
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

describe("SettingsTab > Display page", () => {
  it("Max visible saves shows the default 20 and persists a change under max_visible_saves", async () => {
    renderSettings();
    const row = screen.getByText("Max visible saves").closest(".control-row") as HTMLElement;
    const spinbox = row.querySelector(".slider-spinbox__spinbox") as HTMLInputElement;
    expect(spinbox.value).toBe("20");
    fireEvent.change(spinbox, { target: { value: "35" } });
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ max_visible_saves: 35 }));
  });

  it("the Window card, next to Opacity, says transparency on Linux is not supported", () => {
    renderSettings();
    const note = screen.getByText("Transparency on Linux is currently not supported.");
    const card = note.closest(".card") as HTMLElement;
    expect(card.querySelector(".card__title")?.textContent).toBe("Window");
    expect(card.contains(screen.getByText("Opacity"))).toBe(true);
    expect(note.querySelector("input, button, [role='switch']")).toBeNull();
  });

  it("Max visible saves is inside a Saved Songs card", () => {
    renderSettings();
    const card = screen.getByText("Max visible saves").closest(".card") as HTMLElement;
    expect(card.querySelector(".card__title")?.textContent).toBe("Saved Songs");
  });

  it("Horizontal Scrolling sits in the Visualizer card, starts on, and persists under visualizer_horizontal", async () => {
    renderSettings();
    const toggle = screen.getByText("Horizontal Scrolling").closest("[role='switch']") as HTMLElement;
    const card = toggle.closest(".card") as HTMLElement;
    expect(card.querySelector(".card__title")?.textContent).toBe("Visualizer");
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(toggle);
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ visualizer_horizontal: false }));
    expect(toggle.getAttribute("aria-checked")).toBe("false");
  });
});

describe("SettingsTab > System page", () => {
  it("Reset All Settings restores DEFAULT_CONFIG but preserves the current pedal AI threshold curves", async () => {
    renderSettings();
    act(() => {
      capturedConfig.current!.setConfig({
        ...DEFAULT_CONFIG,
        tempo: 250,
        simulate_hands: true,
        pedal_curve_on: [{ time: 0, value: 0.71 }],
        pedal_curve_off: [{ time: 0, value: 0.22 }],
      });
    });
    fireEvent.click(screen.getByText("System"));
    fireEvent.click(screen.getByText("Reset All Settings"));

    expect(capturedConfig.current!.config.tempo).toBe(DEFAULT_CONFIG.tempo);
    expect(capturedConfig.current!.config.simulate_hands).toBe(false);
    expect(capturedConfig.current!.config.pedal_curve_on).toEqual([{ time: 0, value: 0.71 }]);
    expect(capturedConfig.current!.config.pedal_curve_off).toEqual([{ time: 0, value: 0.22 }]);
  });

  it("pedal prompt threshold spinbox reflects and updates the shared setting", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    const spinbox = document.querySelector(".slider-spinbox__spinbox") as HTMLInputElement;
    expect(spinbox.value).toBe("8");
    fireEvent.change(spinbox, { target: { value: "42" } });
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ pedal_prompt_threshold: 42 }));
  });

  it("shows the app version in a card at the top of the System page with no card title", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    await waitFor(() => expect(screen.getByText("App version: 9.8.7")).toBeTruthy());
    expect(screen.getByText("App version: 9.8.7").closest(".card")).not.toBeNull();
    expect(screen.queryByText("App info")).toBeNull();
  });

  it("shows no version and never calls the backend outside the desktop app", async () => {
    tauriMocks.isTauri.mockReturnValue(false);
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    expect(document.querySelector(".settings-tab__version")).toBeNull();
    expect(tauriMocks.getAppVersion).not.toHaveBeenCalled();
  });

  it("auto-check-updates toggle persists via save_app_config", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    fireEvent.click(screen.getByText("Automatically check for updates"));
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ auto_check_updates: true }));
  });
});

const KEYBOARD_CARD_TITLE = "Keyboard access (Linux)";
const KEYBOARD_SWITCH_LABEL = "Read the keyboard directly";
const KEYBOARD_WARNING =
  "Lets the hotkeys work in every window, including on Wayland. HuMidi can then see every key you press. It needs access to your keyboard devices, which the package sets up; log out and back in once after installing.";

describe("SettingsTab > Hotkey page > Linux keyboard access", () => {
  function openHotkeyPage() {
    renderSettings();
    fireEvent.click(screen.getByText("Hotkey"));
  }

  it("shows a switch for reading the keyboard directly, on by default, in its own card", () => {
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    const card = toggle.closest(".card") as HTMLElement;
    expect(card.querySelector(".card__title")?.textContent).toBe(KEYBOARD_CARD_TITLE);
  });

  it("warns, in plain text right under the switch, that HuMidi can then see every key pressed", () => {
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    const warning = screen.getByText(KEYBOARD_WARNING);
    expect(warning).toBeVisible();
    expect(toggle.closest(".card")?.contains(warning)).toBe(true);
    expect(toggle.compareDocumentPosition(warning) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(warning.querySelector("input, button, [role='switch']")).toBeNull();
  });

  it("sits right under the two hotkey cards", () => {
    openHotkeyPage();
    const cards = Array.from(document.querySelectorAll(".settings-tab__display > .card")).map(
      (card) => card.querySelector(".card__title")?.textContent,
    );
    expect(cards).toEqual(["Playback Toggle", "Save Playback", KEYBOARD_CARD_TITLE]);
  });

  it("turning it off persists linux_keyboard_hotkeys and flips the switch", async () => {
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    fireEvent.click(toggle);
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ linux_keyboard_hotkeys: false }));
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(toggle);
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenLastCalledWith({ linux_keyboard_hotkeys: true }));
  });

  it("starts off when the saved setting is off", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ linux_keyboard_hotkeys: false });
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    await waitFor(() => expect(toggle.getAttribute("aria-checked")).toBe("false"));
  });

  it("is not shown on other platforms", () => {
    platformMocks.isLinux.mockReturnValue(false);
    openHotkeyPage();
    expect(screen.queryByText(KEYBOARD_SWITCH_LABEL)).toBeNull();
    expect(screen.queryByText(KEYBOARD_WARNING)).toBeNull();
    expect(screen.queryByText(KEYBOARD_CARD_TITLE)).toBeNull();
  });
});

describe("SettingsTab > Hotkey page and remembered page", () => {
  it("shows the hotkeys the backend has bound instead of the defaults", async () => {
    tauriMocks.getHotkeys.mockResolvedValue(["F9", "Alt+W"]);
    renderSettings();
    fireEvent.click(screen.getByText("Hotkey"));
    await waitFor(() => expect(screen.getByText("F9")).toBeTruthy());
    expect(screen.getByText("Alt+W")).toBeTruthy();
  });

  it("reopens on the page that was open last time", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ ui_settings_page: "System" });
    renderSettings();
    await waitFor(() => expect(screen.getByText("Automatically check for updates")).toBeTruthy());
  });

  it("remembers the page that was opened", async () => {
    vi.useFakeTimers();
    renderSettings();
    await act(async () => {
      await Promise.resolve();
    });
    fireEvent.click(screen.getByText("Privacy"));
    await act(async () => {
      vi.advanceTimersByTime(1000);
    });
    expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ ui_settings_page: "Privacy" });
    vi.useRealTimers();
  });
});
