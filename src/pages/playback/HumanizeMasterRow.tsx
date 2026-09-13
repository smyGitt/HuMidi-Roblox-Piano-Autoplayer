import { Card } from "../../components/Card";
import { Icon } from "../../components/Icon";
import { ToggleSwitch } from "../../components/ToggleSwitch";

interface HumanizeMasterRowProps {
  humanizeAll: boolean;
  simulateHands: boolean;
  chordRoll: boolean;
  onHumanizeAllChange: (v: boolean) => void;
  onSimulateHandsChange: (v: boolean) => void;
  onChordRollChange: (v: boolean) => void;
  onReset: () => void;
}

export function HumanizeMasterRow({
  humanizeAll,
  simulateHands,
  chordRoll,
  onHumanizeAllChange,
  onSimulateHandsChange,
  onChordRollChange,
  onReset,
}: HumanizeMasterRowProps) {
  return (
    <Card
      title="GENERAL SETTINGS"
      titleButtons={
        <button className="icon-btn icon-btn--danger" onClick={onReset} title="Reset">
          <Icon name="arrow-counter-clockwise" size={16} />
        </button>
      }
    >
      <div className="check-pair">
        <ToggleSwitch checked={humanizeAll} onChange={onHumanizeAllChange} label="Humanize all" />
        <span className="check-pair__desc">enable or disable all humanization</span>
      </div>
      <div className="check-pair">
        <ToggleSwitch checked={simulateHands} onChange={onSimulateHandsChange} label="Simulate Hands" />
        <span className="check-pair__desc">separate timing per hand</span>
      </div>
      <div className="check-pair">
        <ToggleSwitch checked={chordRoll} onChange={onChordRollChange} label="Chord Roll" />
        <span className="check-pair__desc">slight arpeggiation of simultaneous notes</span>
      </div>
    </Card>
  );
}
