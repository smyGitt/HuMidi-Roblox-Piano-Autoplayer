import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { DEFAULT_CONFIG, type PlaybackConfig } from "../pages/playback/types";

interface PlaybackConfigContextValue {
  config: PlaybackConfig;
  setConfig: (updater: PlaybackConfig | ((c: PlaybackConfig) => PlaybackConfig)) => void;
  updateConfig: (patch: Partial<PlaybackConfig>) => void;
}

const PlaybackConfigContext = createContext<PlaybackConfigContextValue | null>(null);

export function PlaybackConfigProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<PlaybackConfig>(DEFAULT_CONFIG);

  const value = useMemo(
    () => ({
      config,
      setConfig,
      updateConfig: (patch: Partial<PlaybackConfig>) => setConfig((c) => ({ ...c, ...patch })),
    }),
    [config],
  );

  return <PlaybackConfigContext.Provider value={value}>{children}</PlaybackConfigContext.Provider>;
}

export function usePlaybackConfig() {
  const ctx = useContext(PlaybackConfigContext);
  if (!ctx) throw new Error("usePlaybackConfig must be used within PlaybackConfigProvider");
  return ctx;
}
