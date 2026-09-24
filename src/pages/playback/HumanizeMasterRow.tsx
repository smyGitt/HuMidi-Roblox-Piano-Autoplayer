import { Card } from "../../components/Card";
import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { Button } from "../../components/Button";
import { Label } from "../../components/Label";

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
        <Label className="check-pair__desc">enable or disable all humanization</Label>
      </div>
      <div className="check-pair">
        <ToggleSwitch checked={simulateHands} onChange={onSimulateHandsChange} label="Simulate Hands" />
        <Label className="check-pair__desc">separate timing per hand</Label>
      </div>
      <div className="check-pair">
        <ToggleSwitch checked={chordRoll} onChange={onChordRollChange} label="Chord Roll" />
        <Label className="check-pair__desc">slight arpeggiation of simultaneous notes</Label>
      </div>
    </Card>
  );
}
