interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
}

export function ToggleSwitch({ checked, onChange, label, disabled }: ToggleSwitchProps) {
  return (
    <button
      type="button"
      className={`toggle-switch${disabled ? " toggle-switch--disabled" : ""}`}
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span className={`toggle-switch__track${checked ? " toggle-switch__track--on" : ""}`}>
        <span className={`toggle-switch__knob${checked ? " toggle-switch__knob--on" : ""}`} />
      </span>
      {label && <span className="toggle-switch__label">{label}</span>}
    </button>
  );
}
