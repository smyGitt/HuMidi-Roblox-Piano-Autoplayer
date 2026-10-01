import { createContext, useContext, useMemo, type ReactNode } from "react";
import { DEFAULT_CONFIG, type PlaybackConfig } from "../pages/playback/types";
import { configFromSaved, configToPatch } from "../pages/playback/persistedConfig";
import { usePersistedPatch } from "./usePersistedState";

interface PlaybackConfigContextValue {
  config: PlaybackConfig;
  setConfig: (updater: PlaybackConfig | ((c: PlaybackConfig) => PlaybackConfig)) => void;
  updateConfig: (patch: Partial<PlaybackConfig>) => void;
}

const PlaybackConfigContext = createContext<PlaybackConfigContextValue | null>(null);

export function PlaybackConfigProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = usePersistedPatch<PlaybackConfig>("playback settings", DEFAULT_CONFIG, {
    fromConfig: configFromSaved,
    toPatch: configToPatch,
  });

  const value = useMemo(
    () => ({
      config,
      setConfig,
      updateConfig: (patch: Partial<PlaybackConfig>) => setConfig((c) => ({ ...c, ...patch })),
    }),
    [config, setConfig],
  );

  return <PlaybackConfigContext.Provider value={value}>{children}</PlaybackConfigContext.Provider>;
}

export function usePlaybackConfig() {
  const ctx = useContext(PlaybackConfigContext);
  if (!ctx) throw new Error("usePlaybackConfig must be used within PlaybackConfigProvider");
  return ctx;
}
