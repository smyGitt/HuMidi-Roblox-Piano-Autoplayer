import type { CSSProperties, InputHTMLAttributes, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";

const PERCENT = 100;
const DEFAULT_RANGE_MAX = 100;

function join(base: string, className?: string) {
  return className ? `${base} ${className}` : base;
}

export function Select({ className, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={join("field", className)} {...rest} />;
}

export function TextInput({ className, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input type="text" className={join("field", className)} {...rest} />;
}

export function NumberInput({ className, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input type="number" className={join("field field--number", className)} {...rest} />;
}

export function TextArea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={join("field", className)} {...rest} />;
}

export function rangeFillPercent(value: number, min: number, max: number): number {
  const width = max - min;
  if (!Number.isFinite(value) || !Number.isFinite(width) || width <= 0) return 0;
  return Math.min(PERCENT, Math.max(0, ((value - min) / width) * PERCENT));
}

export function RangeInput({ className, style, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  const fill = rangeFillPercent(Number(rest.value), Number(rest.min ?? 0), Number(rest.max ?? DEFAULT_RANGE_MAX));
  const fillStyle = { "--range-fill": `${fill}%`, ...style } as CSSProperties;
  return <input type="range" className={join("range", className)} style={fillStyle} {...rest} />;
}

export function Checkbox({ className, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input type="checkbox" className={join("checkbox", className)} {...rest} />;
}

interface ColorInputProps {
  value: string;
  onChange: (value: string) => void;
}

export function ColorInput({ value, onChange }: ColorInputProps) {
  return (
    <label className="color-input">
      <input type="color" value={value} onChange={(e) => onChange(e.target.value)} />
      <span style={{ background: value }} />
    </label>
  );
}
