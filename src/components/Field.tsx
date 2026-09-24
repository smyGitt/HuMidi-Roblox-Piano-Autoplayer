import type { InputHTMLAttributes, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";

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

export function RangeInput({ className, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input type="range" className={join("range", className)} {...rest} />;
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
