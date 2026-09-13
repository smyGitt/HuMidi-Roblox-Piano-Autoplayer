import { Card } from "../../components/Card";
import { Icon } from "../../components/Icon";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { PEDAL_MAPPING, type PlaybackConfig } from "./types";

const PEDAL_LABELS = Object.keys(PEDAL_MAPPING);
const PEDAL_MAPPING_INV: Record<string, string> = Object.fromEntries(
  Object.entries(PEDAL_MAPPING).map(([label, value]) => [value, label]),
);

interface PerformanceCardProps {
  config: PlaybackConfig;
  onChange: (patch: Partial<PlaybackConfig>) => void;
  onReset: () => void;
  midiPedalAvailable: boolean;
}

export function PerformanceCard({ config, onChange, onReset, midiPedalAvailable }: PerformanceCardProps) {
  return (
    <Card
      title="PERFORMANCE"
      titleButtons={
        <button className="icon-btn icon-btn--danger" onClick={onReset} title="Reset">
          <Icon name="arrow-counter-clockwise" size={16} />
        </button>
      }
      className="performance-card"
    >
      <div className="control-row">
        <div className="control-row__label">
          <span>Pedal Style</span>
        </div>
        <select
          className="control-row__select"
          value={PEDAL_MAPPING_INV[config.pedal_style] ?? "PedalAI"}
          onChange={(e) => onChange({ pedal_style: PEDAL_MAPPING[e.target.value] })}
        >
          {PEDAL_LABELS.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
      </div>

      <div className="control-row">
        <div className="control-row__label">
          <span>Transpose</span>
        </div>
        <input
          className="control-row__number"
          type="number"
          min={-24}
          max={24}
          value={config.transpose}
          onChange={(e) => onChange({ transpose: Number(e.target.value) })}
        />
        <span className="control-row__suffix">st</span>
      </div>

      {midiPedalAvailable && (
        <div className="control-row">
          <ToggleSwitch
            checked={config.use_midi_pedal}
            onChange={(v) => onChange({ use_midi_pedal: v })}
            label="Use MIDI Pedal"
          />
        </div>
      )}

      <div className="control-row">
        <ToggleSwitch
          checked={config.use_velocity_accent}
          onChange={(v) => onChange({ use_velocity_accent: v })}
          label="Use Velocity"
        />
      </div>
    </Card>
  );
}
