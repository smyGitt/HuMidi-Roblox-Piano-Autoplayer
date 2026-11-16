import { Card } from "../../components/Card";
import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { InfoTip } from "../../components/InfoTip";
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
          <ArrowCounterClockwiseIcon weight="fill" />
        </Button>
      }
    >
      <div className="control-row">
        <ToggleSwitch checked={humanizeAll} onChange={onHumanizeAllChange} label="Humanize all" />
        <InfoTip text="turn all the human touches on or off at once" />
      </div>
      <div className="control-row">
        <ToggleSwitch checked={simulateHands} onChange={onSimulateHandsChange} label="Simulate Hands" />
        <InfoTip text="each hand gets its own slightly different timing" />
      </div>
      <div className="control-row">
        <ToggleSwitch checked={chordRoll} onChange={onChordRollChange} label="Chord Roll" />
        <InfoTip text="notes in a chord start just a hair apart, like a real hand" />
      </div>
    </Card>
  );
}
