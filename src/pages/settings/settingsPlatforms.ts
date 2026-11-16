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

export interface SettingPlatforms {
  windows: boolean;
  linux: LinuxSupport;
}

const WINDOWS_ONLY: SettingPlatforms = { windows: true, linux: false };
const LINUX_ONLY: SettingPlatforms = { windows: false, linux: true };
const BOTH: SettingPlatforms = { windows: true, linux: true };
const WINDOWS_PARTIAL_LINUX: SettingPlatforms = { windows: true, linux: "partial" };

export const SETTING_PLATFORMS: Record<SettingId, SettingPlatforms> = {
  alwaysOnTop: WINDOWS_PARTIAL_LINUX,
  opacity: WINDOWS_ONLY,
  showStatusText: BOTH,
  showTimeline: BOTH,
  visualizerHorizontal: BOTH,
  showPiano: BOTH,
  showPianoPedal: BOTH,
  switchToVisualizerOnPlay: BOTH,
  maxVisibleSaves: BOTH,
  theme: BOTH,
  saveDirectory: BOTH,
  midiDirectory: BOTH,
  themesFile: BOTH,
  playbackHotkey: BOTH,
  saveHotkey: WINDOWS_ONLY,
  linuxKeyboardHotkeys: LINUX_ONLY,
  autoCheckUpdates: BOTH,
  checkNow: BOTH,
  pedalPromptThreshold: BOTH,
  resetAll: WINDOWS_ONLY,
  redactPaths: BOTH,
};
