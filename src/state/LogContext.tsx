import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { isTauri, loadAppConfig, saveAppConfig } from "../lib/tauri";

export type LogLevel = "INFO" | "DEBUG" | "WARN" | "OK";

export interface LogEntry {
  level: LogLevel;
  line: string;
}

const MAX_ENTRIES = 5000;
const WARN_MARKERS = ["error", "failed", "failure", "rejected", "aborted", "crashed", "cancelled", "not found"];
const OK_MARKERS = ["successful", "complete", "accepted", "finished"];

function classify(message: string): LogLevel {
  const lower = message.toLowerCase();
  if (WARN_MARKERS.some((m) => lower.includes(m))) return "WARN";
  if (OK_MARKERS.some((m) => lower.includes(m))) return "OK";
  if (message.startsWith("[")) return "DEBUG";
  return "INFO";
}

function timestamp(): string {
  return new Date().toTimeString().slice(0, 8);
}

export type SnapshotKey = "file" | "source" | "tracks" | "notes" | "duration" | "pedal" | "tempo" | "pedal_style";

export type Snapshot = Record<SnapshotKey, string | null>;

const EMPTY_SNAPSHOT: Snapshot = {
  file: null,
  source: null,
  tracks: null,
  notes: null,
  duration: null,
  pedal: null,
  tempo: null,
  pedal_style: null,
};

interface LogContextValue {
  entries: LogEntry[];
  appendLog: (message: string) => void;
  clear: () => void;
  redactPaths: boolean;
  setRedactPaths: (v: boolean) => void;
  snapshot: Snapshot;
  updateSnapshot: (fields: Partial<Record<SnapshotKey, string | number | null>>) => void;
  clearSnapshot: () => void;
}

const WINDOWS_PATH_PATTERN = /(?:[a-zA-Z]:[\\/]|\\\\)[^\s"']*[\\/]([^\\/:\s"']+)/g;

function redact(text: string): string {
  return text.replace(WINDOWS_PATH_PATTERN, (_match, lastSegment: string) => lastSegment);
}

const LogContext = createContext<LogContextValue | null>(null);

export function LogProvider({ children }: { children: ReactNode }) {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [redactPaths, setRedactPathsState] = useState(true);
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY_SNAPSHOT);

  const updateSnapshot = useCallback((fields: Partial<Record<SnapshotKey, string | number | null>>) => {
    setSnapshot((prev) => {
      const next = { ...prev };
      for (const [key, value] of Object.entries(fields) as [SnapshotKey, string | number | null][]) {
        next[key] = value === null ? null : String(value);
      }
      return next;
    });
  }, []);

  const clearSnapshot = useCallback(() => setSnapshot(EMPTY_SNAPSHOT), []);

  useEffect(() => {
    if (!isTauri()) return;
    loadAppConfig()
      .then((cfg) => {
        if (typeof cfg.redact_debug_paths === "boolean") setRedactPathsState(cfg.redact_debug_paths);
      })
      .catch(() => {});
  }, []);

  function setRedactPaths(v: boolean) {
    setRedactPathsState(v);
    if (isTauri()) void saveAppConfig({ redact_debug_paths: v });
  }

  const appendLog = useCallback(
    (message: string) => {
      const trimmed = message.trim();
      if (!trimmed) return;
      const text = redactPaths ? redact(trimmed) : trimmed;
      const level = classify(text);
      const line = `[${timestamp()}] ${level} ${text}`;
      setEntries((prev) => [...prev.slice(-(MAX_ENTRIES - 1)), { level, line }]);
    },
    [redactPaths],
  );

  const value = useMemo(
    () => ({
      entries,
      appendLog,
      clear: () => setEntries([]),
      redactPaths,
      setRedactPaths,
      snapshot,
      updateSnapshot,
      clearSnapshot,
    }),
    [entries, appendLog, redactPaths, snapshot, updateSnapshot, clearSnapshot],
  );

  return <LogContext.Provider value={value}>{children}</LogContext.Provider>;
}

export function useLog() {
  const ctx = useContext(LogContext);
  if (!ctx) throw new Error("useLog must be used within LogProvider");
  return ctx;
}
