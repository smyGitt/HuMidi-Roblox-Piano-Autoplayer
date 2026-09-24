import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri, loadAppConfig, saveAppConfig, onEvent } from "../lib/tauri";

interface AppSettings {
  alwaysOnTop: boolean;
  opacity: number;
  showTimeline: boolean;
  showPiano: boolean;
  showPianoPedal: boolean;
  pedalPromptThreshold: number;
  autoCheckUpdates: boolean;
  showStatusText: boolean;
}

interface AppSettingsContextValue extends AppSettings {
  setAlwaysOnTop: (v: boolean) => void;
  setOpacity: (v: number) => void;
  setShowTimeline: (v: boolean) => void;
  setShowPiano: (v: boolean) => void;
  setShowPianoPedal: (v: boolean) => void;
  setPedalPromptThreshold: (v: number) => void;
  setAutoCheckUpdates: (v: boolean) => void;
  setShowStatusText: (v: boolean) => void;
  showUpdatePrompt: boolean;
  resolveUpdatePrompt: (v: boolean) => void;
  updateAvailable: { tag: string; url: string; currentVersion: string } | null;
  dismissUpdateAvailable: () => void;
}

const DEFAULTS: AppSettings = {
  alwaysOnTop: false,
  opacity: 100,
  showTimeline: true,
  showPiano: true,
  showPianoPedal: true,
  pedalPromptThreshold: 8,
  autoCheckUpdates: false,
  showStatusText: false,
};

const AppSettingsContext = createContext<AppSettingsContextValue | null>(null);

export function AppSettingsProvider({ children }: { children: ReactNode }) {
  const [alwaysOnTop, setAlwaysOnTopState] = useState(DEFAULTS.alwaysOnTop);
  const [opacity, setOpacityState] = useState(DEFAULTS.opacity);
  const [showTimeline, setShowTimelineState] = useState(DEFAULTS.showTimeline);
  const [showPiano, setShowPianoState] = useState(DEFAULTS.showPiano);
  const [showPianoPedal, setShowPianoPedalState] = useState(DEFAULTS.showPianoPedal);
  const [pedalPromptThreshold, setPedalPromptThresholdState] = useState(DEFAULTS.pedalPromptThreshold);
  const [autoCheckUpdates, setAutoCheckUpdatesState] = useState(DEFAULTS.autoCheckUpdates);
  const [showStatusText, setShowStatusTextState] = useState(DEFAULTS.showStatusText);
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
        if (typeof cfg.always_on_top === "boolean") setAlwaysOnTopState(cfg.always_on_top);
        if (typeof cfg.opacity === "number") setOpacityState(cfg.opacity);
        if (typeof cfg.show_timeline_visualizer === "boolean") setShowTimelineState(cfg.show_timeline_visualizer);
        if (typeof cfg.show_piano_visualizer === "boolean") setShowPianoState(cfg.show_piano_visualizer);
        if (typeof cfg.show_piano_pedal_visualizer === "boolean") {
          setShowPianoPedalState(cfg.show_piano_pedal_visualizer);
        }
        if (typeof cfg.pedal_prompt_threshold === "number") {
          setPedalPromptThresholdState(cfg.pedal_prompt_threshold);
        }
        if (typeof cfg.show_status_text === "boolean") setShowStatusTextState(cfg.show_status_text);
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
    getCurrentWindow()
      .setAlwaysOnTop(v)
      .catch(() => {});
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
      pedalPromptThreshold,
      autoCheckUpdates,
      showStatusText,
      setAlwaysOnTop,
      setOpacity,
      setShowTimeline,
      setShowPiano,
      setShowPianoPedal,
      setPedalPromptThreshold,
      setAutoCheckUpdates,
      setShowStatusText,
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
      pedalPromptThreshold,
      autoCheckUpdates,
      showStatusText,
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
