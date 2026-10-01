import { Button } from "../../components/Button";
import { NumberInput } from "../../components/Field";

export const MIN_THRESHOLD = 0;
export const MAX_THRESHOLD = 1;
export const THRESHOLD_INPUT_STEP = 0.001;
export const THRESHOLD_STEPS = [0.05, 0.005];
const THRESHOLD_PRECISION = 1000;

export function stepThreshold(value: number, delta: number): number {
  const stepped = Math.round((value + delta) * THRESHOLD_PRECISION) / THRESHOLD_PRECISION;
  return Math.min(MAX_THRESHOLD, Math.max(MIN_THRESHOLD, stepped));
}

function stepLabel(delta: number): string {
  return delta > 0 ? `+${delta}` : String(delta);
}

interface ThresholdStepperProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
}

export function ThresholdStepper({ label, value, onChange }: ThresholdStepperProps) {
  const decreases = THRESHOLD_STEPS.map((step) => -step);
  const increases = [...THRESHOLD_STEPS].reverse();

  function stepButton(delta: number) {
    return (
      <Button
        key={delta}
        className="pedal-ai-card__step"
        title={`${label} ${stepLabel(delta)}`}
        onClick={() => onChange(stepThreshold(value, delta))}
      >
        {stepLabel(delta)}
      </Button>
    );
  }

  return (
    <div className="pedal-ai-card__threshold">
      <span>{label}</span>
      <div className="pedal-ai-card__stepper">
        {decreases.map(stepButton)}
        <NumberInput
          aria-label={label}
          min={MIN_THRESHOLD}
          max={MAX_THRESHOLD}
          step={THRESHOLD_INPUT_STEP}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
        />
        {increases.map(stepButton)}
      </div>
    </div>
  );
}
