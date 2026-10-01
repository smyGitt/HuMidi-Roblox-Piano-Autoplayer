import { useEffect, useRef, useState } from "react";
import { isTauri, loadAppConfig, saveAppConfig } from "../lib/tauri";
import { useOptionalLog } from "./LogContext";
import type { Parser } from "./persistedParsers";

const PERSIST_DELAY_MS = 300;

type AppConfig = Record<string, unknown>;

interface PersistedPatchOptions<T> {
  fromConfig: (config: AppConfig) => T | undefined;
  toPatch: (value: T) => AppConfig;
}

export function usePersistedPatch<T>(name: string, fallback: T, options: PersistedPatchOptions<T>) {
  const log = useOptionalLog();
  const optionsRef = useRef(options);
  optionsRef.current = options;
  const logRef = useRef(log?.appendLog);
  logRef.current = log?.appendLog;
  const [value, setValue] = useState<T>(fallback);
  const lastPersisted = useRef(JSON.stringify(options.toPatch(fallback)));
  const loaded = useRef(false);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    loadAppConfig()
      .then((config) => {
        if (cancelled) return;
        const restored = optionsRef.current.fromConfig(config);
        if (restored !== undefined) {
          lastPersisted.current = JSON.stringify(optionsRef.current.toPatch(restored));
          setValue(restored);
        }
        loaded.current = true;
      })
      .catch((error) => {
        loaded.current = true;
        logRef.current?.(`Loading ${name} failed: ${String(error)}`);
      });
    return () => {
      cancelled = true;
    };
  }, [name]);

  useEffect(() => {
    if (!isTauri() || !loaded.current) return;
    const patch = optionsRef.current.toPatch(value);
    const json = JSON.stringify(patch);
    if (json === lastPersisted.current) return;
    const timer = setTimeout(() => {
      lastPersisted.current = json;
      saveAppConfig(patch).catch((error) => logRef.current?.(`Saving ${name} failed: ${String(error)}`));
    }, PERSIST_DELAY_MS);
    return () => clearTimeout(timer);
  }, [value, name]);

  return [value, setValue] as const;
}

interface PersistedStateOptions<T> {
  parse: Parser<T>;
  serialize?: (value: T) => unknown;
}

export function usePersistedState<T>(key: string, fallback: T, options: PersistedStateOptions<T>) {
  return usePersistedPatch<T>(key, fallback, {
    fromConfig: (config) => options.parse(config[key]),
    toPatch: (value) => ({ [key]: options.serialize ? options.serialize(value) : value }),
  });
}
