import type { LinuxSupport } from "../../components/PlatformIcons";

export const SETTING_IDS = [
  "alwaysOnTop",
  "opacity",
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
  "saveHotkey",
  "linuxKeyboardHotkeys",
  "autoCheckUpdates",
  "checkNow",
  "pedalPromptThreshold",
  "resetAll",
  "redactPaths",
] as const;

export type SettingId = (typeof SETTING_IDS)[number];

export const LINUX_SYSTEMS = ["Mint XFCE (X11)", "Mint Wayland", "Fedora Wayland"] as const;

export interface SettingPlatforms {
  windows: boolean;
  linux: LinuxSupport;
  linuxWorks: readonly string[];
  linuxFails: readonly string[];
}

interface LinuxCompat {
  works: readonly string[];
  fails: readonly string[];
}

const NOT_ON_LINUX: LinuxCompat = { works: [], fails: [] };
const ALL_TESTED: LinuxCompat = { works: LINUX_SYSTEMS, fails: [] };
const X11_ONLY: LinuxCompat = { works: ["X11"], fails: ["Wayland (Mint, Fedora)"] };

function support({ works, fails }: LinuxCompat): LinuxSupport {
  if (works.length === 0) return false;
  return fails.length === 0 ? true : "partial";
}

function platforms(windows: boolean, linux: LinuxCompat): SettingPlatforms {
  return { windows, linux: support(linux), linuxWorks: linux.works, linuxFails: linux.fails };
}

export const SETTING_PLATFORMS: Record<SettingId, SettingPlatforms> = {
  alwaysOnTop: platforms(true, X11_ONLY),
  opacity: platforms(true, NOT_ON_LINUX),
  showStatusText: platforms(true, ALL_TESTED),
  showTimeline: platforms(true, ALL_TESTED),
  visualizerHorizontal: platforms(true, ALL_TESTED),
  showPiano: platforms(true, ALL_TESTED),
  showPianoPedal: platforms(true, ALL_TESTED),
  switchToVisualizerOnPlay: platforms(true, ALL_TESTED),
  maxVisibleSaves: platforms(true, ALL_TESTED),
  theme: platforms(true, ALL_TESTED),
  saveDirectory: platforms(true, ALL_TESTED),
  midiDirectory: platforms(true, ALL_TESTED),
  themesFile: platforms(true, ALL_TESTED),
  playbackHotkey: platforms(true, ALL_TESTED),
  saveHotkey: platforms(true, ALL_TESTED),
  linuxKeyboardHotkeys: platforms(false, ALL_TESTED),
  autoCheckUpdates: platforms(true, ALL_TESTED),
  checkNow: platforms(true, ALL_TESTED),
  pedalPromptThreshold: platforms(true, ALL_TESTED),
  resetAll: platforms(true, ALL_TESTED),
  redactPaths: platforms(true, ALL_TESTED),
};
