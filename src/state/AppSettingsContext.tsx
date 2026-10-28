import { createContext, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri, loadAppConfig, saveAppConfig, onEvent } from "../lib/tauri";
import { useLog } from "./LogContext";

interface AppSettings {
  alwaysOnTop: boolean;
  opacity: number;
  showTimeline: boolean;
  showPiano: boolean;
  showPianoPedal: boolean;
  visualizerHorizontal: boolean;
  pedalPromptThreshold: number;
  autoCheckUpdates: boolean;
  showStatusText: boolean;
  switchToVisualizerOnPlay: boolean;
  maxVisibleSaves: number;
  linuxKeyboardHotkeys: boolean;
}

interface AppSettingsContextValue extends AppSettings {
  setAlwaysOnTop: (v: boolean) => void;
  setOpacity: (v: number) => void;
  setShowTimeline: (v: boolean) => void;
  setShowPiano: (v: boolean) => void;
  setShowPianoPedal: (v: boolean) => void;
  setVisualizerHorizontal: (v: boolean) => void;
  setPedalPromptThreshold: (v: number) => void;
  setAutoCheckUpdates: (v: boolean) => void;
  setShowStatusText: (v: boolean) => void;
  setSwitchToVisualizerOnPlay: (v: boolean) => void;
  setMaxVisibleSaves: (v: number) => void;
  setLinuxKeyboardHotkeys: (v: boolean) => void;
  showUpdatePrompt: boolean;
  resolveUpdatePrompt: (v: boolean) => void;
  updateAvailable: { tag: string; url: string; currentVersion: string } | null;
  dismissUpdateAvailable: () => void;
}

export const MIN_VISIBLE_SAVES = 1;
export const MAX_VISIBLE_SAVES = 100;
const DEFAULT_MAX_VISIBLE_SAVES = 20;

function clampVisibleSaves(v: number): number {
  return Math.min(MAX_VISIBLE_SAVES, Math.max(MIN_VISIBLE_SAVES, Math.round(v)));
}

function applyAlwaysOnTop(value: boolean, report: (message: string) => void) {
  getCurrentWindow()
    .setAlwaysOnTop(value)
    .catch((error) => report(`Always on top failed: ${String(error)}`));
}

const DEFAULTS: AppSettings = {
  alwaysOnTop: false,
  opacity: 100,
  showTimeline: true,
  showPiano: true,
  showPianoPedal: true,
  visualizerHorizontal: true,
  pedalPromptThreshold: 8,
  autoCheckUpdates: false,
  showStatusText: false,
  switchToVisualizerOnPlay: true,
  maxVisibleSaves: DEFAULT_MAX_VISIBLE_SAVES,
  linuxKeyboardHotkeys: true,
};

const AppSettingsContext = createContext<AppSettingsContextValue | null>(null);

export function AppSettingsProvider({ children }: { children: ReactNode }) {
  const { appendLog } = useLog();
  const appendLogRef = useRef(appendLog);
  appendLogRef.current = appendLog;
  const [alwaysOnTop, setAlwaysOnTopState] = useState(DEFAULTS.alwaysOnTop);
  const [opacity, setOpacityState] = useState(DEFAULTS.opacity);
  const [showTimeline, setShowTimelineState] = useState(DEFAULTS.showTimeline);
  const [showPiano, setShowPianoState] = useState(DEFAULTS.showPiano);
  const [showPianoPedal, setShowPianoPedalState] = useState(DEFAULTS.showPianoPedal);
  const [visualizerHorizontal, setVisualizerHorizontalState] = useState(DEFAULTS.visualizerHorizontal);
  const [pedalPromptThreshold, setPedalPromptThresholdState] = useState(DEFAULTS.pedalPromptThreshold);
  const [autoCheckUpdates, setAutoCheckUpdatesState] = useState(DEFAULTS.autoCheckUpdates);
  const [showStatusText, setShowStatusTextState] = useState(DEFAULTS.showStatusText);
  const [switchToVisualizerOnPlay, setSwitchToVisualizerOnPlayState] = useState(DEFAULTS.switchToVisualizerOnPlay);
  const [maxVisibleSaves, setMaxVisibleSavesState] = useState(DEFAULTS.maxVisibleSaves);
  const [linuxKeyboardHotkeys, setLinuxKeyboardHotkeysState] = useState(DEFAULTS.linuxKeyboardHotkeys);
  const [showUpdatePrompt, setShowUpdatePrompt] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState<{ tag: string; url: string; currentVersion: string } | null>(
    null,
  );

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | null = null;
    onEvent("update-available", (payload) => setUpdateAvailable(payload)).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    loadAppConfig()
      .then((cfg) => {
        if (typeof cfg.always_on_top === "boolean") {
          setAlwaysOnTopState(cfg.always_on_top);
          if (cfg.always_on_top) applyAlwaysOnTop(true, appendLogRef.current);
        }
        if (typeof cfg.opacity === "number") setOpacityState(cfg.opacity);
        if (typeof cfg.show_timeline_visualizer === "boolean") setShowTimelineState(cfg.show_timeline_visualizer);
        if (typeof cfg.show_piano_visualizer === "boolean") setShowPianoState(cfg.show_piano_visualizer);
        if (typeof cfg.show_piano_pedal_visualizer === "boolean") {
          setShowPianoPedalState(cfg.show_piano_pedal_visualizer);
        }
        if (typeof cfg.visualizer_horizontal === "boolean") setVisualizerHorizontalState(cfg.visualizer_horizontal);
        if (typeof cfg.pedal_prompt_threshold === "number") {
          setPedalPromptThresholdState(cfg.pedal_prompt_threshold);
        }
        if (typeof cfg.show_status_text === "boolean") setShowStatusTextState(cfg.show_status_text);
        if (typeof cfg.switch_to_visualizer_on_play === "boolean") {
          setSwitchToVisualizerOnPlayState(cfg.switch_to_visualizer_on_play);
        }
        if (typeof cfg.max_visible_saves === "number") setMaxVisibleSavesState(clampVisibleSaves(cfg.max_visible_saves));
        if (typeof cfg.linux_keyboard_hotkeys === "boolean") setLinuxKeyboardHotkeysState(cfg.linux_keyboard_hotkeys);
        if (typeof cfg.auto_check_updates === "boolean") {
          setAutoCheckUpdatesState(cfg.auto_check_updates);
        } else {
          setShowUpdatePrompt(true);
        }
      })
      .catch(() => {});
  }, []);

  function setAlwaysOnTop(v: boolean) {
    setAlwaysOnTopState(v);
    if (!isTauri()) return;
    void saveAppConfig({ always_on_top: v });
    applyAlwaysOnTop(v, appendLog);
  }

  function setOpacity(v: number) {
    setOpacityState(v);
    if (isTauri()) void saveAppConfig({ opacity: v });
  }

  function setShowTimeline(v: boolean) {
    setShowTimelineState(v);
    if (isTauri()) void saveAppConfig({ show_timeline_visualizer: v });
  }

  function setShowPiano(v: boolean) {
    setShowPianoState(v);
    if (isTauri()) void saveAppConfig({ show_piano_visualizer: v });
  }

  function setShowPianoPedal(v: boolean) {
    setShowPianoPedalState(v);
    if (isTauri()) void saveAppConfig({ show_piano_pedal_visualizer: v });
  }

  function setVisualizerHorizontal(v: boolean) {
    setVisualizerHorizontalState(v);
    if (isTauri()) void saveAppConfig({ visualizer_horizontal: v });
  }

  function setPedalPromptThreshold(v: number) {
    setPedalPromptThresholdState(v);
    if (isTauri()) void saveAppConfig({ pedal_prompt_threshold: v });
  }

  function setAutoCheckUpdates(v: boolean) {
    setAutoCheckUpdatesState(v);
    if (isTauri()) void saveAppConfig({ auto_check_updates: v });
  }

  function setShowStatusText(v: boolean) {
    setShowStatusTextState(v);
    if (isTauri()) void saveAppConfig({ show_status_text: v });
  }

  function setSwitchToVisualizerOnPlay(v: boolean) {
    setSwitchToVisualizerOnPlayState(v);
    if (isTauri()) void saveAppConfig({ switch_to_visualizer_on_play: v });
  }

  function setMaxVisibleSaves(v: number) {
    const clamped = clampVisibleSaves(v);
    setMaxVisibleSavesState(clamped);
    if (isTauri()) void saveAppConfig({ max_visible_saves: clamped });
  }

  function setLinuxKeyboardHotkeys(v: boolean) {
    setLinuxKeyboardHotkeysState(v);
    if (isTauri()) void saveAppConfig({ linux_keyboard_hotkeys: v });
  }

  function resolveUpdatePrompt(v: boolean) {
    setShowUpdatePrompt(false);
    setAutoCheckUpdates(v);
  }

  function dismissUpdateAvailable() {
    setUpdateAvailable(null);
  }

  const value = useMemo(
    () => ({
      alwaysOnTop,
      opacity,
      showTimeline,
      showPiano,
      showPianoPedal,
      visualizerHorizontal,
      pedalPromptThreshold,
      autoCheckUpdates,
      showStatusText,
      switchToVisualizerOnPlay,
      maxVisibleSaves,
      linuxKeyboardHotkeys,
      setAlwaysOnTop,
      setOpacity,
      setShowTimeline,
      setShowPiano,
      setShowPianoPedal,
      setVisualizerHorizontal,
      setPedalPromptThreshold,
      setAutoCheckUpdates,
      setShowStatusText,
      setSwitchToVisualizerOnPlay,
      setMaxVisibleSaves,
      setLinuxKeyboardHotkeys,
      showUpdatePrompt,
      resolveUpdatePrompt,
      updateAvailable,
      dismissUpdateAvailable,
    }),
    [
      alwaysOnTop,
      opacity,
      showTimeline,
      showPiano,
      showPianoPedal,
      visualizerHorizontal,
      pedalPromptThreshold,
      autoCheckUpdates,
      showStatusText,
      switchToVisualizerOnPlay,
      maxVisibleSaves,
      linuxKeyboardHotkeys,
      showUpdatePrompt,
      updateAvailable,
    ],
  );

  return <AppSettingsContext.Provider value={value}>{children}</AppSettingsContext.Provider>;
}

export function useAppSettings() {
  const ctx = useContext(AppSettingsContext);
  if (!ctx) throw new Error("useAppSettings must be used within AppSettingsProvider");
  return ctx;
}
