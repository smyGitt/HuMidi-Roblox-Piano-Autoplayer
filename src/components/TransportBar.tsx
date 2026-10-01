import {
  PauseCircleIcon,
  PlayCircleIcon,
  ResizeIcon,
  SkipBackCircleIcon,
  SkipForwardCircleIcon,
  StopCircleIcon,
} from "@phosphor-icons/react";
import { Button } from "./Button";
import { RangeInput } from "./Field";
import { StatusIndicator, type PlaybackStatus } from "./StatusIndicator";
import type { PlaybackClock } from "../state/playbackClock";
import { useClockValue } from "../state/useClockValue";

const SCRUBBER_STEPS = 10000;

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

const exactTime = (time: number) => time;

interface TransportBarProps {
  isPlaying: boolean;
  clock: PlaybackClock;
  totalTime: number;
  isCollapsed: boolean;
  status: PlaybackStatus;
  statusLabel: string;
  showStatusText: boolean;
  playEnabled: boolean;
  onScrub: (time: number) => void;
  onPlayPause: () => void;
  onStop: () => void;
  onSeekStart: () => void;
  onSeekEnd: () => void;
  onToggleCollapsed: () => void;
}

function ScrubberRow({
  clock,
  totalTime,
  onScrub,
}: {
  clock: PlaybackClock;
  totalTime: number;
  onScrub: (time: number) => void;
}) {
  const time = useClockValue(clock, exactTime);
  return (
    <div className="transport-bar__scrubber-row">
      <span className="transport-bar__time">{formatTime(time)}</span>
      <RangeInput
        className="transport-bar__scrubber"
        min={0}
        max={SCRUBBER_STEPS}
        value={totalTime > 0 ? (time / totalTime) * SCRUBBER_STEPS : 0}
        onChange={(e) => onScrub(totalTime > 0 ? (Number(e.target.value) / SCRUBBER_STEPS) * totalTime : 0)}
      />
      <span className="transport-bar__time">{formatTime(totalTime)}</span>
    </div>
  );
}

function CollapsedTime({ clock, totalTime }: { clock: PlaybackClock; totalTime: number }) {
  const seconds = useClockValue(clock, Math.floor);
  return (
    <span className="transport-bar__time">
      {formatTime(seconds)} / {formatTime(totalTime)}
    </span>
  );
}

export function TransportBar({
  isPlaying,
  clock,
  totalTime,
  isCollapsed,
  status,
  statusLabel,
  showStatusText,
  playEnabled,
  onScrub,
  onPlayPause,
  onStop,
  onSeekStart,
  onSeekEnd,
  onToggleCollapsed,
}: TransportBarProps) {
  return (
    <div className={`transport-bar${isCollapsed ? " transport-bar--collapsed" : ""}`}>
      <ScrubberRow clock={clock} totalTime={totalTime} onScrub={onScrub} />

      <div className="transport-bar__btn-row">
        <div className="transport-bar__side">
          <StatusIndicator status={status} label={statusLabel} showLabel={showStatusText} />
          {isCollapsed && <CollapsedTime clock={clock} totalTime={totalTime} />}
        </div>

        <div className="transport-bar__controls">
        <Button variant="icon" subtle size="lg" onClick={onSeekStart} disabled={!playEnabled} title="To start">
          <SkipBackCircleIcon weight="duotone" />
        </Button>
        <Button
          variant="icon" subtle size="lg"
          onClick={onPlayPause}
          disabled={!playEnabled}
          title={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? <PauseCircleIcon weight="duotone" /> : <PlayCircleIcon weight="duotone" />}
        </Button>
        <Button variant="icon" subtle size="lg" onClick={onStop} disabled={status === "unloaded"} title="Stop">
          <StopCircleIcon weight="duotone" />
        </Button>
        <Button variant="icon" subtle size="lg" onClick={onSeekEnd} disabled={!playEnabled} title="To end">
          <SkipForwardCircleIcon weight="duotone" />
        </Button>
        </div>

        <div className="transport-bar__side transport-bar__side--end">
          <Button
            variant="icon" size="md"
            onClick={onToggleCollapsed}
            title={isCollapsed ? "Expand" : "Collapse"}
          >
            <ResizeIcon weight="duotone" style={isCollapsed ? { transform: "rotate(180deg)" } : undefined} />
          </Button>
        </div>
      </div>
    </div>
  );
}
