import type { ReactNode } from "react";

interface CardProps {
  title: string;
  titleButtons?: ReactNode;
  footer?: ReactNode;
  dashed?: boolean;
  dragActive?: boolean;
  children: ReactNode;
  className?: string;
}

export function Card({ title, titleButtons, footer, dashed, dragActive, children, className }: CardProps) {
  return (
    <div
      className={`card${dashed ? " card--dashed" : ""}${dragActive ? " card--drag-active" : ""}${className ? ` ${className}` : ""}`}
    >
      {title && (
        <div className="card__title-row">
          <span className="card__title">{title}</span>
          {titleButtons}
        </div>
      )}
      <div className="card__body">{children}</div>
      {footer && <div className="card__footer">{footer}</div>}
    </div>
  );
}
