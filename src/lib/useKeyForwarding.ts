import { useEffect } from "react";
import { forwardKey, isTauri } from "./tauri";

export function useKeyForwarding(): void {
  useEffect(() => {
    if (!isTauri()) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.repeat) return;
      void forwardKey(event.code, true).catch(() => {});
    };
    const onKeyUp = (event: KeyboardEvent) => {
      void forwardKey(event.code, false).catch(() => {});
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
    };
  }, []);
}
