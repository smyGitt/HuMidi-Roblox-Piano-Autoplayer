import { NumberInput, RangeInput } from "./Field";

interface SliderSpinboxProps {
  min: number;
  max: number;
  value: number;
  step?: number;
  suffix?: string;
  decimals?: number;
  disabled?: boolean;
  onChange: (value: number) => void;
}

export function SliderSpinbox({
  min,
  max,
  value,
  step,
  suffix = "",
  decimals = 0,
  disabled,
  onChange,
}: SliderSpinboxProps) {
  const resolvedStep = step ?? (decimals > 0 ? 1 / 10 ** decimals : 1);
  return (
    <div className="slider-spinbox">
      <RangeInput
        className="slider-spinbox__slider"
        min={min}
        max={max}
        step={resolvedStep}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <NumberInput
        className="slider-spinbox__spinbox"
        min={min}
        max={max}
        step={resolvedStep}
        value={Number(value.toFixed(decimals))}
        disabled={disabled}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (!Number.isNaN(v)) onChange(Math.min(max, Math.max(min, v)));
        }}
      />
      {suffix && <span className="slider-spinbox__suffix">{suffix}</span>}
    </div>
  );
}
