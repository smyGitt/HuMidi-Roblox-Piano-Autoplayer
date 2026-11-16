import { useEffect, useState, type InputHTMLAttributes } from "react";
import { NumberInput } from "./Field";

interface DraftNumberInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "onChange"> {
  value: number;
  onCommit: (value: number) => void;
}

export function DraftNumberInput({ value, onCommit, onFocus, onBlur, ...rest }: DraftNumberInputProps) {
  const [text, setText] = useState(String(value));
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    if (!focused) setText(String(value));
  }, [value, focused]);

  return (
    <NumberInput
      {...rest}
      value={text}
      onFocus={(event) => {
        setFocused(true);
        onFocus?.(event);
      }}
      onBlur={(event) => {
        setFocused(false);
        onBlur?.(event);
      }}
      onChange={(event) => {
        const raw = event.target.value;
        setText(raw);
        if (raw === "") return;
        const parsed = Number(raw);
        if (!Number.isNaN(parsed)) onCommit(parsed);
      }}
    />
  );
}
