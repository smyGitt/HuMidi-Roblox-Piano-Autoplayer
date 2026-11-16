import { ListIcon } from "@phosphor-icons/react";
import type { ReactNode } from "react";

export function SidePanel({ children }: { children: ReactNode }) {
  return (
    <aside className="side-panel">
      <div className="side-panel__handle">
        <ListIcon weight="fill" />
      </div>
      <div className="side-panel__content">{children}</div>
    </aside>
  );
}
