import { ToggleSwitch } from "../../components/ToggleSwitch";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { Label } from "../../components/Label";

interface HumRowProps {
  name: string;
  desc: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  min: number;
  max: number;
  value: number;
  suffix: string;
  decimals?: number;
  onValueChange: (value: number) => void;
}

export function HumRow({
  name,
  desc,
  checked,
  onCheckedChange,
  min,
  max,
  value,
  suffix,
  decimals,
  onValueChange,
}: HumRowProps) {
  return (
    <div className="hum-row">
      <div className="hum-row__top">
        <div className="hum-row__check">
          <ToggleSwitch checked={checked} onChange={onCheckedChange} label={name} />
          {desc && <Label className="hum-row__desc">{desc}</Label>}
        </div>
      </div>
      <SliderSpinbox
        min={min}
        max={max}
        value={value}
        suffix={suffix}
        decimals={decimals}
        disabled={!checked}
        onChange={onValueChange}
      />
    </div>
  );
}
