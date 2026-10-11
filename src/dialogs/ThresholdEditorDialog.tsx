import { useState } from "react";
import { Modal } from "../components/Modal";
import { ToggleSwitch } from "../components/ToggleSwitch";
import type { PedalConfidence } from "../lib/pedalPreview";
import type { ThresholdCurves } from "../lib/thresholdCurve";
import { ThresholdChart } from "../pages/playback/ThresholdChart";

const DIALOG_WIDTH = 820;
const DIALOG_HEIGHT = 520;

interface ThresholdEditorDialogProps {
  confidence: PedalConfidence;
  curves: ThresholdCurves;
  onCurvesChange: (curves: ThresholdCurves) => void;
  onClose: () => void;
}

export function ThresholdEditorDialog({ confidence, curves, onCurvesChange, onClose }: ThresholdEditorDialogProps) {
  const [showValues, setShowValues] = useState(false);

  return (
    <Modal title="Edit threshold curves" onClose={onClose} width={DIALOG_WIDTH} height={DIALOG_HEIGHT}>
      <div className="threshold-editor">
        <ToggleSwitch checked={showValues} onChange={setShowValues} label="Show exact numbers" />
        <ThresholdChart confidence={confidence} curves={curves} onCurvesChange={onCurvesChange} showValues={showValues} />
      </div>
    </Modal>
  );
}
