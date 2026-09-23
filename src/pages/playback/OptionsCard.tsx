import { Card } from "../../components/Card";
import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import type { PlaybackConfig } from "./types";

interface OptionsCardProps {
  config: PlaybackConfig;
  onChange: (patch: Partial<PlaybackConfig>) => void;
  onReset: () => void;
}

export function OptionsCard({ config, onChange, onReset }: OptionsCardProps) {
  return (
    <Card
      title="OPTIONS"
      titleButtons={
        <button className="icon-btn icon-btn--danger" onClick={onReset} title="Reset">
          <ArrowCounterClockwiseIcon size={16} weight="duotone" />
        </button>
      }
      className="options-card"
    >
      <div className="control-row">
        <ToggleSwitch
          checked={config.use_88_key_layout}
          onChange={(v) => onChange({ use_88_key_layout: v })}
          label="88-Key Layout"
        />
      </div>
      <div className="control-row">
        <ToggleSwitch checked={config.countdown} onChange={(v) => onChange({ countdown: v })} label="Countdown" />
      </div>
      <div className="control-row">
        <ToggleSwitch
          checked={config.debug_mode}
          onChange={(v) => onChange({ debug_mode: v })}
          label="Debug Output"
        />
      </div>
    </Card>
  );
}
