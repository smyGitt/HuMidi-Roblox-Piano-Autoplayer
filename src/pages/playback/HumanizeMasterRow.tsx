import { Card } from "../../components/Card";
import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { Button } from "../../components/Button";

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
        <Button variant="icon" subtle danger onClick={onReset} title="Reset">
          <ArrowCounterClockwiseIcon weight="duotone" />
        </Button>
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
