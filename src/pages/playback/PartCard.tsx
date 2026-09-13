interface PartCardProps {
  title: string;
  meta: string;
  onClick?: () => void;
  clickable?: boolean;
  timeStr?: string;
}

export function PartCard({ title, meta, onClick, clickable, timeStr }: PartCardProps) {
  return (
    <div
      className={`part-card${clickable ? " part-card--clickable" : ""}`}
      onClick={onClick}
      role={clickable ? "button" : undefined}
    >
      <span className="part-card__title">{title}</span>
      <span className="part-card__meta">{meta}</span>
      {timeStr && <span className="part-card__meta part-card__time">{timeStr}</span>}
    </div>
  );
}
