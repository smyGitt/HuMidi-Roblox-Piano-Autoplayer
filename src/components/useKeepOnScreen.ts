import { useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { NO_SHIFT, fitToViewport, type PopupFit } from "../lib/popupFit";

export function useKeepOnScreen<T extends HTMLElement>(open: boolean) {
  const ref = useRef<T>(null);
  const [fit, setFit] = useState<PopupFit>(NO_SHIFT);

  useLayoutEffect(() => {
    if (!open) {
      setFit(NO_SHIFT);
      return;
    }
    const element = ref.current;
    if (!element) return;
    setFit(fitToViewport(element.getBoundingClientRect(), { width: window.innerWidth, height: window.innerHeight }));
  }, [open]);

  const style: CSSProperties | undefined = fit.dx === 0 ? undefined : { transform: `translateX(${fit.dx}px)` };
  return { ref, style, aboveClass: fit.above ? " info-tip__text--above" : "" };
}
