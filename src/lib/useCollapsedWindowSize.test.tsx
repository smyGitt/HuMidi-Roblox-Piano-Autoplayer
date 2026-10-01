import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const WINDOW_WIDTH = 800;
const WINDOW_HEIGHT = 600;
const SCALE_FACTOR = 2;
const HEADER_HEIGHT = 48;
const TRANSPORT_HEIGHT = 96;
const SAVED_FULL = { width: 1000, height: 700 };
const SAVED_MINI = { width: 500, height: 200 };
const USER_FULL = { width: 1200, height: 900 };
const USER_MINI = { width: 640, height: 180 };
const SAVE_TIMEOUT_MS = 2000;

type ResizeHandler = (event: { payload: ReturnType<typeof physical> }) => void;

const mocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(),
  saveAppConfig: vi.fn<(patch: Record<string, unknown>) => Promise<void>>(),
  setSize: vi.fn<(size: unknown) => Promise<void>>(),
  innerSize: vi.fn(),
  scaleFactor: vi.fn(),
  resizeHandler: null as unknown,
}));

vi.mock("./tauri", () => ({
  isTauri: mocks.isTauri,
  loadAppConfig: mocks.loadAppConfig,
  saveAppConfig: mocks.saveAppConfig,
}));

vi.mock("@tauri-apps/api/window", () => {
  class LogicalSize {
    constructor(
      public width: number,
      public height: number,
    ) {}
  }
  return {
    LogicalSize,
    getCurrentWindow: () => ({
      innerSize: mocks.innerSize,
      scaleFactor: mocks.scaleFactor,
      setSize: mocks.setSize,
      onResized: (handler: unknown) => {
        mocks.resizeHandler = handler;
        return Promise.resolve(() => {});
      },
    }),
  };
});

import { useCollapsedWindowSize } from "./useCollapsedWindowSize";

function buildRoot() {
  const root = document.createElement("div");
  const header = document.createElement("div");
  header.className = "file-header";
  header.getBoundingClientRect = () => ({ height: HEADER_HEIGHT }) as DOMRect;
  const transport = document.createElement("div");
  transport.className = "transport-bar";
  transport.getBoundingClientRect = () => ({ height: TRANSPORT_HEIGHT }) as DOMRect;
  root.append(header, transport);
  return root;
}

function physical(width: number, height: number) {
  return {
    toLogical: (factor: number) => ({ width: (width * SCALE_FACTOR) / factor, height: (height * SCALE_FACTOR) / factor }),
  };
}

function userResizesTo(size: { width: number; height: number }) {
  act(() => (mocks.resizeHandler as ResizeHandler)({ payload: physical(size.width, size.height) }));
}

beforeEach(() => {
  mocks.isTauri.mockReturnValue(true);
  mocks.loadAppConfig.mockReset().mockReturnValue(Promise.resolve({}));
  mocks.saveAppConfig.mockReset().mockReturnValue(Promise.resolve());
  mocks.setSize.mockReset().mockReturnValue(Promise.resolve());
  mocks.innerSize.mockReset().mockReturnValue(Promise.resolve(physical(WINDOW_WIDTH, WINDOW_HEIGHT)));
  mocks.scaleFactor.mockReset().mockReturnValue(Promise.resolve(SCALE_FACTOR));
  mocks.resizeHandler = null;
});

function renderResize(report = vi.fn()) {
  const rootRef = { current: buildRoot() };
  const view = renderHook(({ collapsed }) => useCollapsedWindowSize(collapsed, rootRef, report), {
    initialProps: { collapsed: false },
  });
  return { ...view, report };
}

describe("useCollapsedWindowSize: collapsing and expanding", () => {
  it("leaves the window alone on launch when no size is saved", async () => {
    renderResize();
    await waitFor(() => expect(mocks.loadAppConfig).toHaveBeenCalled());
    expect(mocks.setSize).not.toHaveBeenCalled();
  });

  it("applies the saved full size on launch", async () => {
    mocks.loadAppConfig.mockReturnValue(Promise.resolve({ window_size_full: SAVED_FULL }));
    renderResize();
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledWith(expect.objectContaining(SAVED_FULL)));
  });

  it("shrinks to the header plus transport height at the current width when no mini size is saved", async () => {
    const { rerender } = renderResize();
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    expect(mocks.setSize).toHaveBeenCalledWith(
      expect.objectContaining({ width: WINDOW_WIDTH, height: HEADER_HEIGHT + TRANSPORT_HEIGHT }),
    );
  });

  it("uses the saved mini size when there is one", async () => {
    mocks.loadAppConfig.mockReturnValue(Promise.resolve({ window_size_mini: SAVED_MINI }));
    const { rerender } = renderResize();
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    expect(mocks.setSize).toHaveBeenCalledWith(expect.objectContaining(SAVED_MINI));
  });

  it("restores the size it had before collapsing", async () => {
    const { rerender } = renderResize();
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    rerender({ collapsed: false });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(2));
    expect(mocks.setSize).toHaveBeenLastCalledWith(
      expect.objectContaining({ width: WINDOW_WIDTH, height: WINDOW_HEIGHT }),
    );
  });

  it("does not touch the window outside the desktop app", () => {
    mocks.isTauri.mockReturnValue(false);
    const { rerender } = renderResize();
    rerender({ collapsed: true });
    expect(mocks.setSize).not.toHaveBeenCalled();
    expect(mocks.innerSize).not.toHaveBeenCalled();
    expect(mocks.loadAppConfig).not.toHaveBeenCalled();
  });

  it("reports a failed resize instead of swallowing it", async () => {
    mocks.setSize.mockReturnValue(Promise.reject(new Error("not allowed")));
    const { rerender, report } = renderResize();
    rerender({ collapsed: true });
    await waitFor(() => expect(report).toHaveBeenCalledWith("Window resize failed: Error: not allowed"));
  });
});

describe("useCollapsedWindowSize: saving the size the user chooses", () => {
  it("saves a resize in full mode under the full key", async () => {
    renderResize();
    await waitFor(() => expect(mocks.resizeHandler).not.toBeNull());
    userResizesTo(USER_FULL);
    await waitFor(() => expect(mocks.saveAppConfig).toHaveBeenCalledWith({ window_size_full: USER_FULL }), {
      timeout: SAVE_TIMEOUT_MS,
    });
  });

  it("saves a resize in mini mode under the mini key, separately from the full one", async () => {
    const { rerender } = renderResize();
    await waitFor(() => expect(mocks.resizeHandler).not.toBeNull());
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    userResizesTo(USER_MINI);
    await waitFor(() => expect(mocks.saveAppConfig).toHaveBeenCalledWith({ window_size_mini: USER_MINI }), {
      timeout: SAVE_TIMEOUT_MS,
    });
    expect(mocks.saveAppConfig).not.toHaveBeenCalledWith(expect.objectContaining({ window_size_full: expect.anything() }));
  });

  it("restores the size the user chose in each mode on the next toggle", async () => {
    const { rerender } = renderResize();
    await waitFor(() => expect(mocks.resizeHandler).not.toBeNull());
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    userResizesTo(USER_MINI);
    await waitFor(() => expect(mocks.saveAppConfig).toHaveBeenCalledTimes(1), { timeout: SAVE_TIMEOUT_MS });
    rerender({ collapsed: false });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(2));
    rerender({ collapsed: true });
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(3));
    expect(mocks.setSize).toHaveBeenLastCalledWith(expect.objectContaining(USER_MINI));
  });

  it("does not save the size the app itself applied", async () => {
    mocks.loadAppConfig.mockReturnValue(Promise.resolve({ window_size_full: SAVED_FULL }));
    renderResize();
    await waitFor(() => expect(mocks.setSize).toHaveBeenCalledTimes(1));
    userResizesTo(SAVED_FULL);
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, SAVE_TIMEOUT_MS / 2));
    });
    expect(mocks.saveAppConfig).not.toHaveBeenCalled();
  });
});
