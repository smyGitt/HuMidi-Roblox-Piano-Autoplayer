import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "../../theme/ThemeProvider";
import { AppSettingsProvider, MAX_VISIBLE_SAVES } from "../../state/AppSettingsContext";
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
  it("the Customize button is in the Theme row, right after the dropdown and before the platform logos", () => {
    renderSettings();
    const row = screen.getByText("Theme").closest(".control-row") as HTMLElement;
    const select = row.querySelector("select") as HTMLElement;
    const button = screen.getByText("Customize...").closest("button") as HTMLElement;
    expect(button.closest(".control-row")).toBe(row);
    expect(select.nextElementSibling).toBe(button);
    expect(button.nextElementSibling?.className).toBe("platform-icons");
  });

  it("the Appearance card has no footer any more", () => {
    renderSettings();
    const card = screen.getByText("Theme").closest(".card") as HTMLElement;
    expect(card.querySelector(".card__footer")).toBeNull();
  });

  it("the Customize button still opens the theme editor", async () => {
    renderSettings();
    fireEvent.click(screen.getByText("Customize..."));
    await waitFor(() => expect(screen.getByText("Theme Editor")).toBeTruthy());
  });

  it("Max visible saves is a plain number box with no slider", () => {
    renderSettings();
    const row = screen.getByText("Max visible saves").closest(".control-row") as HTMLElement;
    expect(row.querySelector("input[type='range']")).toBeNull();
    expect(row.querySelector("input[type='number']")).not.toBeNull();
  });

  it("Max visible saves clamps a typed value into its allowed range", async () => {
    renderSettings();
    const row = screen.getByText("Max visible saves").closest(".control-row") as HTMLElement;
    const box = row.querySelector("input[type='number']") as HTMLInputElement;
    fireEvent.change(box, { target: { value: "100000" } });
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ max_visible_saves: MAX_VISIBLE_SAVES }));
  });

  it("Max visible saves shows the default 20 and persists a change under max_visible_saves", async () => {
    renderSettings();
    const row = screen.getByText("Max visible saves").closest(".control-row") as HTMLElement;
    const spinbox = row.querySelector("input[type='number']") as HTMLInputElement;
    expect(spinbox.value).toBe("20");
    fireEvent.change(spinbox, { target: { value: "35" } });
    await waitFor(() => expect(tauriMocks.saveAppConfig).toHaveBeenCalledWith({ max_visible_saves: 35 }));
  });

  it("the Window card has no notes about Wayland or transparency", () => {
    renderSettings();
    expect(screen.queryByText("Always on top is not supported on Wayland.")).toBeNull();
    expect(screen.queryByText("Transparency on Linux is currently not supported.")).toBeNull();
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
const KEYBOARD_SWITCH_LABEL = "Always listen to keys (Linux)";
const KEYBOARD_WARNING =
  "HuMidi reads your keyboard directly, so hotkeys work in every window, even when another app has focus. This also means HuMidi can see every key you press.";

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

  it("puts the explanation behind an info icon right after the switch, in the same card", () => {
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    expect(screen.queryByText(KEYBOARD_WARNING)).toBeNull();
    expect(toggle.nextElementSibling?.className).toBe("info-tip");
    fireEvent.mouseEnter(toggle.nextElementSibling as HTMLElement);
    const warning = screen.getByText(KEYBOARD_WARNING);
    expect(toggle.closest(".card")?.contains(warning)).toBe(true);
    expect(document.querySelector(".settings-tab__note")).toBeNull();
  });

  it("explains the switch in at most two sentences, using only letters, spaces, periods and commas", () => {
    openHotkeyPage();
    const toggle = screen.getByText(KEYBOARD_SWITCH_LABEL).closest("[role='switch']") as HTMLElement;
    fireEvent.mouseEnter(toggle.nextElementSibling as HTMLElement);
    const text = screen.getByText(KEYBOARD_WARNING).textContent ?? "";
    expect(text.match(/[.!?]/g)?.length).toBeLessThanOrEqual(2);
    expect(text).toMatch(/^[A-Za-z ,.]+$/);
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

vi.mock("./settingsPlatforms", async () => {
  const actual = await vi.importActual<typeof import("./settingsPlatforms")>("./settingsPlatforms");
  const everywhere = Object.fromEntries(
    actual.SETTING_IDS.map((id) => [
      id,
      {
        windows: true,
        linux: id === "alwaysOnTop" ? "partial" : true,
        linuxWorks: ["System A"],
        linuxFails: id === "alwaysOnTop" ? ["System B"] : [],
      },
    ]),
  );
  return { ...actual, SETTING_PLATFORMS: everywhere };
});

describe("SettingsTab > platform icons", () => {
  const PAGES = [
    { page: "Display", controls: 10 },
    { page: "Files", controls: 3 },
    { page: "Hotkey", controls: 3 },
    { page: "System", controls: 4 },
    { page: "Privacy", controls: 1 },
  ];

  for (const { page, controls } of PAGES) {
    it(`${page} shows the logos on each of its ${controls} controls`, async () => {
      renderSettings();
      fireEvent.click(screen.getByText(page));
      await waitFor(() => expect(screen.getAllByLabelText("Works on Windows")).toHaveLength(controls));
      const partial = page === "Display" ? 1 : 0;
      expect(screen.queryAllByLabelText("Works on Linux")).toHaveLength(controls - partial);
      expect(screen.queryAllByLabelText("Partly works on Linux")).toHaveLength(partial);
    });
  }

  it("the logos are the last thing in the row, on the right of the control", () => {
    renderSettings();
    const row = screen.getByText("Always on Top").closest(".control-row") as HTMLElement;
    const first = row.lastElementChild as HTMLElement;
    expect(first.className).toBe("platform-icons");
    expect(first.querySelector("[aria-label='Works on Windows']")).not.toBeNull();
  });

  it("every control row ends with the logos", async () => {
    renderSettings();
    for (const page of ["Display", "Files", "Hotkey", "System", "Privacy"]) {
      fireEvent.click(screen.getByText(page));
      await waitFor(() => expect(document.querySelectorAll(".platform-icons").length).toBeGreaterThan(0));
      for (const slot of document.querySelectorAll(".platform-icons")) {
        expect(slot.parentElement?.lastElementChild).toBe(slot);
      }
    }
  });
});

describe("SettingsTab > explanations behind info icons", () => {
  function tipText(row: HTMLElement): string | null {
    const tip = row.querySelector(".info-tip") as HTMLElement;
    expect(tip).not.toBeNull();
    fireEvent.mouseEnter(tip);
    const text = screen.getByRole("tooltip").textContent;
    fireEvent.mouseLeave(tip);
    return text;
  }

  it("Privacy: Redact file paths has its explanation behind an icon beside the switch", () => {
    renderSettings();
    fireEvent.click(screen.getByText("Privacy"));
    const row = screen.getByText("Redact file paths").closest(".control-row") as HTMLElement;
    expect(tipText(row)).toBe("Collapses absolute file paths in the Debug log to just their filename.");
    expect(document.querySelectorAll(".page-placeholder__hint")).toHaveLength(0);
    expect(document.querySelector(".toggle-switch__description")).toBeNull();
  });

  it("Files: each folder card has its explanation behind an icon next to the card title", () => {
    renderSettings();
    fireEvent.click(screen.getByText("Files"));
    const expected: [string, string][] = [
      ["Save Directory", 'Where "Save Playback" writes practice-run files.'],
      ["MIDI Directory", "Default directory the MIDI file picker opens to."],
      ["Themes File", "JSON file where custom themes are stored."],
    ];
    for (const [title, text] of expected) {
      const row = screen.getByText(title).closest(".card__title-row") as HTMLElement;
      expect(tipText(row)).toBe(text);
    }
    expect(document.querySelectorAll(".page-placeholder__hint")).toHaveLength(0);
  });

  it("System: Pedal Prompt Threshold and Reset have their explanations behind icons", () => {
    renderSettings();
    fireEvent.click(screen.getByText("System"));
    const threshold = screen.getByText("Pedal Prompt Threshold").closest(".control-row__label") as HTMLElement;
    expect(tipText(threshold)).toMatch(/^Minimum number of embedded MIDI sustain-pedal events/);
    const reset = screen.getByText("Reset All Settings").closest(".control-row") as HTMLElement;
    expect(tipText(reset)).toMatch(/^Restore all playback and humanization settings to their defaults/);
    expect(document.querySelectorAll(".page-placeholder__hint")).toHaveLength(0);
  });
});

describe("SettingsTab > Linux logo hover", () => {
  it("shows the systems a setting works on when its Linux logo is hovered", () => {
    renderSettings();
    const icon = screen.getAllByLabelText("Works on Linux")[0] as HTMLElement;
    fireEvent.mouseEnter(icon.closest(".platform-icons__linux") as HTMLElement);
    const items = [...screen.getByRole("tooltip").querySelectorAll("ul li")].map((item) => item.textContent);
    expect(items).toEqual(["System A"]);
  });

  it("shows both lists on the triangle of Always on Top", () => {
    renderSettings();
    const icon = screen.getByLabelText("Partly works on Linux") as HTMLElement;
    fireEvent.mouseEnter(icon.closest(".platform-icons__linux") as HTMLElement);
    const lists = [...screen.getByRole("tooltip").querySelectorAll("ul")].map((list) =>
      [...list.querySelectorAll("li")].map((item) => item.textContent),
    );
    expect(lists).toEqual([["System A"], ["System B"]]);
  });
});
