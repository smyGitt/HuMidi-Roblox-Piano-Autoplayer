import { ToggleSwitch } from "../../components/ToggleSwitch";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { InfoTip } from "../../components/InfoTip";
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
      <ToggleSwitch checked={checked} onChange={onCheckedChange} ariaLabel={name} />
      <div className="control-row">
        <span className="hum-row__title" onClick={() => onCheckedChange(!checked)}>
          <Label>{name}</Label>
        </span>
        {desc && <InfoTip text={desc} />}
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
