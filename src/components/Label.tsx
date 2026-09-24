import { useLayoutEffect, useRef, type ReactNode } from "react";

const SLIDE_PX_PER_SECOND = 40;
const SLIDE_HOLD_MS = 5000;
const MS_PER_SECOND = 1000;

interface LabelProps {
  className?: string;
  slide?: boolean;
  children: ReactNode;
}

export function Label({ className, slide, children }: LabelProps) {
  const outerRef = useRef<HTMLSpanElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);

  useLayoutEffect(() => {
    const outer = outerRef.current;
    const text = textRef.current;
    if (!slide || !outer || !text) return;
    let animation: Animation | null = null;
    function measure() {
      if (!outer || !text) return;
      animation?.cancel();
      animation = null;
      const shift = Math.max(0, Math.ceil(text.scrollWidth - outer.clientWidth));
      if (shift === 0 || typeof text.animate !== "function") return;
      const travelMs = (shift / SLIDE_PX_PER_SECOND) * MS_PER_SECOND;
      const totalMs = SLIDE_HOLD_MS + travelMs + SLIDE_HOLD_MS;
      const start = "translateX(0)";
      const end = `translateX(${-shift}px)`;
      animation = text.animate(
        [
          { transform: start, offset: 0 },
          { transform: start, offset: SLIDE_HOLD_MS / totalMs },
          { transform: end, offset: (SLIDE_HOLD_MS + travelMs) / totalMs },
          { transform: end, offset: 1 },
        ],
        { duration: totalMs, iterations: Infinity, easing: "linear" },
      );
    }
    measure();
    if (typeof ResizeObserver === "undefined") return () => animation?.cancel();
    const observer = new ResizeObserver(measure);
    observer.observe(outer);
    observer.observe(text);
    return () => {
      observer.disconnect();
      animation?.cancel();
    };
  }, [children, slide]);

  return (
    <span ref={outerRef} className={className ? `label ${className}` : "label"}>
      <span ref={textRef} className="label__text">
        {children}
      </span>
    </span>
  );
}
