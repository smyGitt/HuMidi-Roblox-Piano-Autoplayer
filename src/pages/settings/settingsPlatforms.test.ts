import { describe, expect, it } from "vitest";
import { SETTING_IDS, SETTING_PLATFORMS } from "./settingsPlatforms";

describe("SETTING_PLATFORMS", () => {
  it("has an entry with both flags for every setting id", () => {
    for (const id of SETTING_IDS) {
      const entry = SETTING_PLATFORMS[id];
      expect(typeof entry.windows).toBe("boolean");
      expect(typeof entry.linux).toMatch(/boolean|string/);
    }
  });

  it("has no entries beyond the listed ids", () => {
    expect(Object.keys(SETTING_PLATFORMS).sort()).toEqual([...SETTING_IDS].sort());
  });
});

const LINUX_VERIFIED = [
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
];

describe("Linux flags", () => {
  it("are true exactly for the settings the Linux test suite passed", () => {
    const flagged = SETTING_IDS.filter((id) => SETTING_PLATFORMS[id].linux === true);
    expect([...flagged].sort()).toEqual([...LINUX_VERIFIED].sort());
  });

  it("are partial for Always on Top, which works on X11 only, and false for Opacity", () => {
    expect(SETTING_PLATFORMS.alwaysOnTop.linux).toBe("partial");
    expect(SETTING_PLATFORMS.opacity.linux).toBe(false);
  });
});
