import { describe, expect, it } from "vitest";
import { LINUX_SYSTEMS, SETTING_IDS, SETTING_PLATFORMS, type SettingId } from "./settingsPlatforms";

describe("SETTING_PLATFORMS", () => {
  it("has an entry with the flags and the Linux lists for every setting id", () => {
    for (const id of SETTING_IDS) {
      const entry = SETTING_PLATFORMS[id];
      expect(typeof entry.windows).toBe("boolean");
      expect(typeof entry.linux).toMatch(/boolean|string/);
      expect(Array.isArray(entry.linuxWorks)).toBe(true);
      expect(Array.isArray(entry.linuxFails)).toBe(true);
    }
  });

  it("has no entries beyond the listed ids", () => {
    expect(Object.keys(SETTING_PLATFORMS).sort()).toEqual([...SETTING_IDS].sort());
  });
});

const PASSED_ON_ALL_SYSTEMS: SettingId[] = [
  "showStatusText",
  "showTimeline",
  "visualizerHorizontal",
  "showPiano",
  "showPianoPedal",
  "switchToVisualizerOnPlay",
  "maxVisibleSaves",
  "theme",
  "saveDirectory",
  "midiDirectory",
  "themesFile",
  "playbackHotkey",
  "linuxKeyboardHotkeys",
  "autoCheckUpdates",
  "checkNow",
  "pedalPromptThreshold",
  "redactPaths",
  "resetAll",
];

describe("the tested Linux systems", () => {
  it("are the three the suite ran on", () => {
    expect(LINUX_SYSTEMS).toEqual(["Mint XFCE (X11)", "Mint Wayland", "Fedora Wayland"]);
  });
});

describe("Linux flags", () => {
  it("are true exactly for the settings the Linux suite passed on every system", () => {
    const flagged = SETTING_IDS.filter((id) => SETTING_PLATFORMS[id].linux === true);
    expect([...flagged].sort()).toEqual([...PASSED_ON_ALL_SYSTEMS].sort());
  });

  it("list all three systems as working, and none as failing, for those settings", () => {
    for (const id of PASSED_ON_ALL_SYSTEMS) {
      expect(SETTING_PLATFORMS[id].linuxWorks).toEqual(LINUX_SYSTEMS);
      expect(SETTING_PLATFORMS[id].linuxFails).toEqual([]);
    }
  });

  it("show a triangle where it works on some systems and not on others: Always on Top, X11 only", () => {
    const entry = SETTING_PLATFORMS.alwaysOnTop;
    expect(entry.linux).toBe("partial");
    expect(entry.linuxWorks).toEqual(["X11"]);
    expect(entry.linuxFails).toEqual(["Wayland (Mint, Fedora)"]);
  });

  it("are derived from the lists, so a setting with both works and fails is partial and one with neither is false", () => {
    for (const id of SETTING_IDS) {
      const { linux, linuxWorks, linuxFails } = SETTING_PLATFORMS[id];
      const expected = linuxWorks.length === 0 ? false : linuxFails.length === 0 ? true : "partial";
      expect(linux).toBe(expected);
    }
  });

  it("show nothing for Opacity, which cannot work on Linux, and for the untested Save hotkey", () => {
    for (const id of ["opacity", "saveHotkey"] as const) {
      expect(SETTING_PLATFORMS[id].linux).toBe(false);
      expect(SETTING_PLATFORMS[id].linuxWorks).toEqual([]);
    }
  });
});
