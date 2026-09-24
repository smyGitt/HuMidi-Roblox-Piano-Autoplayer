import type { ReactNode } from "react";

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
          <span className="card__title">{title}</span>
          {titleButtons}
        </div>
      )}
      <div className="card__body">{children}</div>
      {footer && <div className="card__footer">{footer}</div>}
    </div>
  );
}
