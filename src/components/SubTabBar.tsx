const LABELS = ["File", "Playback", "Humanize"];

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
          {label}
        </button>
      ))}
    </div>
  );
}
