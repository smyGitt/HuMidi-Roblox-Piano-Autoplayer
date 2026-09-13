const LABELS = ["File", "Playback", "Humanize"];
const ORDINALS = ["I", "II", "III"];

interface SubTabBarProps {
  active: number;
  onChange: (index: number) => void;
}

export function SubTabBar({ active, onChange }: SubTabBarProps) {
  return (
    <div className="sub-tab-bar">
      {LABELS.map((label, i) => (
        <button
          key={label}
          className={`sub-tab-bar__btn${active === i ? " sub-tab-bar__btn--active" : ""}`}
          onClick={() => onChange(i)}
        >
          {ORDINALS[i]}. {label}
        </button>
      ))}
    </div>
  );
}
