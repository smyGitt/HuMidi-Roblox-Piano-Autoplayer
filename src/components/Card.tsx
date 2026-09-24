import type { ReactNode } from "react";
import { Label } from "./Label";

interface CardProps {
  title: string;
  titleButtons?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function Card({ title, titleButtons, footer, children, className }: CardProps) {
  return (
    <div
      className={`card${className ? ` ${className}` : ""}`}
    >
      {title && (
        <div className="card__title-row">
          <Label className="card__title">{title}</Label>
          {titleButtons}
        </div>
      )}
      <div className="card__body">{children}</div>
      {footer && <div className="card__footer">{footer}</div>}
    </div>
  );
}
