import { Button } from "./Button";

const LABELS = ["File", "Playback", "Humanize"];

interface SubTabBarProps {
  active: number;
  onChange: (index: number) => void;
}

export function SubTabBar({ active, onChange }: SubTabBarProps) {
  return (
    <div className="sub-tab-bar">
      {LABELS.map((label, i) => (
        <Button
          key={label}
          variant="tab" active={active === i}
          onClick={() => onChange(i)}
        >
          {label}
        </Button>
      ))}
    </div>
  );
}
