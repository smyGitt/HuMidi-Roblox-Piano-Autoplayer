import type { CSSProperties } from "react";
import {
  ArrowsInSimpleIcon,
  ArrowsOutSimpleIcon,
  PauseIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StopIcon,
} from "@phosphor-icons/react";
import { Button } from "./Button";
import { RangeInput } from "./Field";
import { StatusIndicator, type PlaybackStatus } from "./StatusIndicator";
import type { PlaybackClock } from "../state/playbackClock";
import { useClockValue } from "../state/useClockValue";
import { DEFAULT_IMAGE_OFFSET, DEFAULT_IMAGE_ZOOM } from "../theme/tokens";

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
  image?: string;
  imageZoom?: number;
  imageX?: number;
  imageY?: number;
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
  image,
  imageZoom = DEFAULT_IMAGE_ZOOM,
  imageX = DEFAULT_IMAGE_OFFSET,
  imageY = DEFAULT_IMAGE_OFFSET,
  onScrub,
  onPlayPause,
  onStop,
  onSeekStart,
  onSeekEnd,
  onToggleCollapsed,
}: TransportBarProps) {
  return (
    <div className={`transport-bar${isCollapsed ? " transport-bar--collapsed" : ""}`}>
      {image && (
        <img
          className="transport-bar__image"
          src={image}
          alt=""
          style={{ "--image-zoom": imageZoom, "--image-x": `${imageX}px`, "--image-y": `${imageY}px` } as CSSProperties}
        />
      )}
      <ScrubberRow clock={clock} totalTime={totalTime} onScrub={onScrub} />

      <div className="transport-bar__btn-row">
        <div className="transport-bar__side">
          <StatusIndicator status={status} label={statusLabel} showLabel={showStatusText} />
          {isCollapsed && <CollapsedTime clock={clock} totalTime={totalTime} />}
        </div>

        <div className="transport-bar__controls">
        <Button variant="icon" subtle size="lg" className="transport-bar__skip" onClick={onSeekStart} disabled={!playEnabled} title="To start">
          <SkipBackIcon weight="fill" />
        </Button>
        <Button
          variant="icon" subtle size="lg"
          onClick={onPlayPause}
          disabled={!playEnabled}
          title={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? <PauseIcon weight="fill" /> : <PlayIcon weight="fill" />}
        </Button>
        <Button variant="icon" subtle size="lg" onClick={onStop} disabled={status === "unloaded"} title="Stop">
          <StopIcon weight="fill" />
        </Button>
        <Button variant="icon" subtle size="lg" className="transport-bar__skip" onClick={onSeekEnd} disabled={!playEnabled} title="To end">
          <SkipForwardIcon weight="fill" />
        </Button>
        </div>

        <div className="transport-bar__side transport-bar__side--end">
          <Button
            variant="icon" subtle size="md"
            onClick={onToggleCollapsed}
            title={isCollapsed ? "Expand" : "Collapse"}
          >
            {isCollapsed ? <ArrowsOutSimpleIcon weight="fill" /> : <ArrowsInSimpleIcon weight="fill" />}
          </Button>
        </div>
      </div>
    </div>
  );
}
