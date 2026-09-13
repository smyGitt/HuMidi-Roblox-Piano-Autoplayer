import { useEffect, useMemo, useRef, useState } from "react";
import { Card } from "../../components/Card";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { useLog, type LogLevel, type SnapshotKey } from "../../state/LogContext";

const LEVELS: LogLevel[] = ["INFO", "DEBUG", "WARN", "OK"];
const FILTER_OPTIONS = ["All", ...LEVELS];

const SNAPSHOT_ROWS: { key: SnapshotKey; label: string }[] = [
  { key: "file", label: "File" },
  { key: "source", label: "Source" },
  { key: "tracks", label: "Tracks" },
  { key: "notes", label: "Notes" },
  { key: "duration", label: "Duration" },
  { key: "pedal", label: "Pedal" },
  { key: "tempo", label: "Tempo" },
  { key: "pedal_style", label: "Pedal Style" },
];

export function DebugTab() {
  const { entries, clear, snapshot } = useLog();
  const [filter, setFilter] = useState("All");
  const [autoScroll, setAutoScroll] = useState(true);
  const consoleRef = useRef<HTMLDivElement>(null);

  const filtered = useMemo(
    () => (filter === "All" ? entries : entries.filter((e) => e.level === filter)),
    [entries, filter],
  );

  useEffect(() => {
    if (autoScroll && consoleRef.current) consoleRef.current.scrollTop = consoleRef.current.scrollHeight;
  }, [filtered, autoScroll]);

  const counts = useMemo(() => {
    const c: Record<LogLevel, number> = { INFO: 0, DEBUG: 0, WARN: 0, OK: 0 };
    for (const e of entries) c[e.level]++;
    return c;
  }, [entries]);

  return (
    <div className="debug-tab">
      <div className="debug-tab__body">
        <Card title="" className="debug-tab__console-card">
          <div ref={consoleRef} className="debug-tab__console">
            {filtered.length === 0 ? (
              <span className="page-placeholder__hint">No log entries.</span>
            ) : (
              filtered.map((e, i) => (
                <div key={i} className={`debug-tab__line debug-tab__line--${e.level.toLowerCase()}`}>
                  {e.line}
                </div>
              ))
            )}
          </div>
        </Card>

        <div className="debug-tab__sidebar">
          <Card title="Filter">
            <select className="control-row__select" value={filter} onChange={(e) => setFilter(e.target.value)}>
              {FILTER_OPTIONS.map((f) => (
                <option key={f}>{f}</option>
              ))}
            </select>
            <ToggleSwitch checked={autoScroll} onChange={setAutoScroll} label="Auto-scroll" />
          </Card>

          <Card title="Levels">
            <div className="debug-tab__levels-grid">
              {LEVELS.map((l) => (
                <div key={l} className="debug-tab__level-row">
                  <span>{l}</span>
                  <span>{counts[l]}</span>
                </div>
              ))}
            </div>
          </Card>

          <Card title="Session Snapshot">
            <div className="debug-tab__snapshot-grid">
              {SNAPSHOT_ROWS.map(({ key, label }) => (
                <div key={key} className="debug-tab__snapshot-row">
                  <span>{label}</span>
                  <span>{snapshot[key] ?? "-"}</span>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>

      <div className="debug-tab__footer">
        <button className="modal__btn" onClick={clear}>
          Clear
        </button>
        <button
          className="modal__btn"
          onClick={() => navigator.clipboard.writeText(filtered.map((e) => e.line).join("\n"))}
        >
          Copy Log
        </button>
        <button
          className="modal__btn"
          onClick={() => {
            const blob = new Blob([entries.map((e) => e.line).join("\n")], { type: "text/plain" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = `humidi_log_${Date.now()}.txt`;
            a.click();
            URL.revokeObjectURL(url);
          }}
        >
          Export Log
        </button>
      </div>
    </div>
  );
}
