import { InfoIcon } from "@phosphor-icons/react";
import { useEffect, useRef, useState } from "react";
import { Button } from "./Button";
import { useKeepOnScreen } from "./useKeepOnScreen";

export function InfoTip({ text }: { text: string }) {
  const [hovered, setHovered] = useState(false);
  const [pinned, setPinned] = useState(false);
  const rootRef = useRef<HTMLSpanElement>(null);
  const open = hovered || pinned;
  const { ref: panelRef, style: panelStyle, aboveClass } = useKeepOnScreen<HTMLSpanElement>(open);

  useEffect(() => {
    if (!pinned) return;
    const closeOutside = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setPinned(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setPinned(false);
    };
    document.addEventListener("mousedown", closeOutside);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("mousedown", closeOutside);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [pinned]);

  return (
    <span
      ref={rootRef}
      className="info-tip"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <Button
        variant="icon"
        subtle
        size="sm"
        aria-label="More information"
        aria-expanded={open}
        onClick={() => setPinned((value) => !value)}
      >
        <InfoIcon weight="fill" />
      </Button>
      {open && (
        <span role="tooltip" ref={panelRef} style={panelStyle} className={`info-tip__text${aboveClass}`}>
          {text}
        </span>
      )}
    </span>
  );
}
