import { FloppyDiskIcon, PauseIcon, PlayIcon, ResizeIcon, StopIcon } from "@phosphor-icons/react";

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
  playEnabled: boolean;
  saveEnabled: boolean;
  onScrub: (value: number) => void;
  onSeekCommit: (value: number) => void;
  onPlayPause: () => void;
  onStop: () => void;
  onSave: () => void;
  onToggleCollapsed: () => void;
}

export function TransportBar({
  isPlaying,
  currentTime,
  totalTime,
  isCollapsed,
  playEnabled,
  saveEnabled,
  onScrub,
  onSeekCommit,
  onPlayPause,
  onStop,
  onSave,
  onToggleCollapsed,
}: TransportBarProps) {
  return (
    <div className="transport-bar">
      {!isCollapsed && (
        <div className="transport-bar__scrubber-row">
          <span className="transport-bar__time">{formatTime(currentTime)}</span>
          <input
            className="transport-bar__scrubber"
            type="range"
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
        <button
          className="transport-bar__btn"
          onClick={onPlayPause}
          disabled={!playEnabled}
          title={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? <PauseIcon size={20} weight="duotone" /> : <PlayIcon size={20} weight="duotone" />}
        </button>
        <button className="transport-bar__btn" onClick={onStop} title="Stop">
          <StopIcon size={20} weight="duotone" />
        </button>

        {isCollapsed && (
          <span className="transport-bar__time transport-bar__time--collapsed">
            {formatTime(currentTime)} / {formatTime(totalTime)}
          </span>
        )}

        <div className="transport-bar__stretch" />

        <button
          className="transport-bar__btn"
          onClick={onSave}
          disabled={!saveEnabled}
          title="Save playback"
        >
          <FloppyDiskIcon size={20} weight="duotone" />
        </button>
        <button
          className="transport-bar__btn"
          onClick={onToggleCollapsed}
          title={isCollapsed ? "Expand" : "Collapse"}
        >
          <ResizeIcon size={20} weight="duotone" style={isCollapsed ? { transform: "rotate(180deg)" } : undefined} />
        </button>
      </div>
    </div>
  );
}
