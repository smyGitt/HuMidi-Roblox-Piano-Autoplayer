import { Label } from "../../components/Label";
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
      <Label className="part-card__title">{title}</Label>
      <Label className="part-card__meta">{meta}</Label>
      {timeStr && <Label className="part-card__meta part-card__time">{timeStr}</Label>}
    </div>
  );
}
