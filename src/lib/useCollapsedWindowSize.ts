import { useEffect, useRef, type RefObject } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { isTauri, loadAppConfig, saveAppConfig } from "./tauri";
import { FULL_SIZE_KEY, MINI_SIZE_KEY, readSavedSize, sameSize, type WindowSize } from "./windowSizes";

const STRIP_SELECTORS = [".file-header", ".transport-bar"];
const RESIZE_SAVE_DELAY_MS = 400;

function stripHeight(root: HTMLElement): number {
  return STRIP_SELECTORS.reduce((total, selector) => {
    const element = root.querySelector(selector);
    return total + (element ? element.getBoundingClientRect().height : 0);
  }, 0);
}

export function useCollapsedWindowSize(
  isCollapsed: boolean,
  rootRef: RefObject<HTMLElement | null>,
  report: (message: string) => void,
) {
  const previousCollapsed = useRef(isCollapsed);
  const collapsedNow = useRef(isCollapsed);
  const sizes = useRef<{ full: WindowSize | null; mini: WindowSize | null }>({ full: null, mini: null });
  const loaded = useRef(false);
  const applied = useRef<WindowSize | null>(null);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const reportRef = useRef(report);
  reportRef.current = report;

  function enqueue(task: () => Promise<void>) {
    queue.current = queue.current
      .then(task)
      .catch((error) => reportRef.current(`Window resize failed: ${String(error)}`));
  }

  async function applySize(size: WindowSize) {
    applied.current = size;
    await getCurrentWindow().setSize(new LogicalSize(size.width, size.height));
  }

  useEffect(() => {
    if (!isTauri()) return;
    const appWindow = getCurrentWindow();
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let unlisten: (() => void) | undefined;

    enqueue(async () => {
      const config = await loadAppConfig();
      sizes.current.full = readSavedSize(config, FULL_SIZE_KEY);
      sizes.current.mini = readSavedSize(config, MINI_SIZE_KEY);
      loaded.current = true;
      if (!cancelled && !collapsedNow.current && sizes.current.full) await applySize(sizes.current.full);
    });

    async function saveResize(physical: { toLogical: (factor: number) => WindowSize }) {
      const logical = physical.toLogical(await appWindow.scaleFactor());
      const size = { width: logical.width, height: logical.height };
      if (!loaded.current || sameSize(size, applied.current)) return;
      const mini = collapsedNow.current;
      sizes.current[mini ? "mini" : "full"] = size;
      await saveAppConfig({ [mini ? MINI_SIZE_KEY : FULL_SIZE_KEY]: size });
    }

    appWindow
      .onResized(({ payload }) => {
        clearTimeout(timer);
        timer = setTimeout(() => {
          saveResize(payload).catch((error) => reportRef.current(`Saving the window size failed: ${String(error)}`));
        }, RESIZE_SAVE_DELAY_MS);
      })
      .then((stop) => {
        if (cancelled) stop();
        else unlisten = stop;
      });

    return () => {
      cancelled = true;
      clearTimeout(timer);
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauri() || previousCollapsed.current === isCollapsed) return;
    previousCollapsed.current = isCollapsed;
    collapsedNow.current = isCollapsed;
    const root = rootRef.current;
    if (!root) return;
    const appWindow = getCurrentWindow();

    if (isCollapsed) {
      const height = stripHeight(root);
      enqueue(async () => {
        const [inner, factor] = await Promise.all([appWindow.innerSize(), appWindow.scaleFactor()]);
        const logical = inner.toLogical(factor);
        const full = { width: logical.width, height: logical.height };
        sizes.current.full = full;
        await applySize(sizes.current.mini ?? { width: full.width, height });
      });
    } else {
      enqueue(async () => {
        if (sizes.current.full) await applySize(sizes.current.full);
      });
    }
  }, [isCollapsed, rootRef]);
}
