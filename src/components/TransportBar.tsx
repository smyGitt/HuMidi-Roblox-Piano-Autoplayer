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

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

interface TransportBarProps {
  isPlaying: boolean;
  currentTime: number;
  totalTime: number;
  isCollapsed: boolean;
  status: PlaybackStatus;
  statusLabel: string;
  showStatusText: boolean;
  playEnabled: boolean;
  onScrub: (value: number) => void;
  onSeekCommit: (value: number) => void;
  onPlayPause: () => void;
  onStop: () => void;
  onSeekStart: () => void;
  onSeekEnd: () => void;
  onToggleCollapsed: () => void;
}

export function TransportBar({
  isPlaying,
  currentTime,
  totalTime,
  isCollapsed,
  status,
  statusLabel,
  showStatusText,
  playEnabled,
  onScrub,
  onSeekCommit,
  onPlayPause,
  onStop,
  onSeekStart,
  onSeekEnd,
  onToggleCollapsed,
}: TransportBarProps) {
  return (
    <div className="transport-bar">
      {!isCollapsed && (
        <div className="transport-bar__scrubber-row">
          <span className="transport-bar__time">{formatTime(currentTime)}</span>
          <RangeInput
            className="transport-bar__scrubber"
            min={0}
            max={10000}
            value={totalTime > 0 ? (currentTime / totalTime) * 10000 : 0}
            onChange={(e) => onScrub(Number(e.target.value))}
            onMouseUp={(e) => onSeekCommit(Number((e.target as HTMLInputElement).value))}
          />
          <span className="transport-bar__time">{formatTime(totalTime)}</span>
        </div>
      )}

      <div className="transport-bar__btn-row">
        <div className="transport-bar__side">
          <StatusIndicator status={status} label={statusLabel} showLabel={showStatusText} />
          {isCollapsed && (
            <span className="transport-bar__time">
              {formatTime(currentTime)} / {formatTime(totalTime)}
            </span>
          )}
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
