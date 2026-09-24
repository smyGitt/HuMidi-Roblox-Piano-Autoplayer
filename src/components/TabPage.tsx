import type { ReactNode } from "react";
import { Button } from "./Button";

interface TabPageProps {
  title?: string;
  tabs: readonly string[];
  active: number;
  onChange: (index: number) => void;
  lead?: ReactNode;
  overlay?: ReactNode;
  children: ReactNode;
}

export function TabPage({ title, tabs, active, onChange, lead, overlay, children }: TabPageProps) {
  return (
    <div className="tab-page">
      <div className="tab-page__header">
        {title && <h1 className="tab-page__title">{title}</h1>}
        {lead}
        <div className="tab-page__tabs">
          {tabs.map((label, i) => (
            <Button key={label} variant="tab" active={active === i} onClick={() => onChange(i)}>
              {label}
            </Button>
          ))}
        </div>
      </div>
      <div className="tab-page__body">
        <div className={`tab-page__page${overlay ? " tab-page__page--with-overlay" : ""}`}>{children}</div>
        {overlay}
      </div>
    </div>
  );
}
