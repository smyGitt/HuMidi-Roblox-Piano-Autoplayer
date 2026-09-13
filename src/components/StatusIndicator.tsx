import { useEffect, useState } from "react";

export type PlaybackStatus = "unloaded" | "loading" | "loaded" | "ready";

const HOURGLASS_FRAMES = ["◳", "◴", "◵", "◶"];

interface StatusIndicatorProps {
  status: PlaybackStatus;
  label?: string;
}

export function StatusIndicator({ status, label }: StatusIndicatorProps) {
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    if (status !== "loading") return;
    const id = setInterval(() => setFrame((f) => (f + 1) % HOURGLASS_FRAMES.length), 350);
    return () => clearInterval(id);
  }, [status]);

  const color =
    status === "unloaded"
      ? "var(--accent_stop)"
      : status === "loaded"
        ? "var(--accent_loaded)"
        : status === "ready"
          ? "var(--accent_play)"
          : "var(--accent)";

  return (
    <div className="status-indicator" title={label}>
      {status === "loading" ? (
        <span className="status-indicator__hourglass">{HOURGLASS_FRAMES[frame]}</span>
      ) : (
        <span className="status-indicator__dot" style={{ background: color }} />
      )}
      {label && <span className="status-indicator__label">{label}</span>}
    </div>
  );
}
