import type { ReactNode } from "react";
import { Label } from "./Label";

interface CardProps {
  title: string;
  titleButtons?: ReactNode;
  footer?: ReactNode;
  side?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function Card({ title, titleButtons, footer, side, children, className }: CardProps) {
  const content = (
    <>
      {title && (
        <div className="card__title-row">
          <Label className="card__title">{title}</Label>
          {titleButtons}
        </div>
      )}
      <div className="card__body">{children}</div>
      {footer && <div className="card__footer">{footer}</div>}
    </>
  );
  const classes = ["card", className, side && "card--with-side"].filter(Boolean).join(" ");
  return (
    <div className={classes}>
      {side ? (
        <>
          <div className="card__main">{content}</div>
          <div className="card__side">{side}</div>
        </>
      ) : (
        content
      )}
    </div>
  );
}
