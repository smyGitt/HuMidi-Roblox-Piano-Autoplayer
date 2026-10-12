import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const FORWARD_COMMAND = "forward_key";

const tauriApi = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
  listen: vi.fn(async () => () => {}),
  getVersion: vi.fn(async () => "0.0.0"),
  windowHandle: {
    setAlwaysOnTop: vi.fn(async () => {}),
    setSize: vi.fn(async () => {}),
    innerSize: vi.fn(async () => ({ width: 0, height: 0 })),
    scaleFactor: vi.fn(async () => 1),
    onResized: vi.fn(async () => () => {}),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: tauriApi.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: tauriApi.listen }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: tauriApi.getVersion }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriApi.windowHandle,
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn(), openPath: vi.fn() }));

function press(code: string) {
  window.dispatchEvent(new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true }));
}

function release(code: string) {
  window.dispatchEvent(new KeyboardEvent("keyup", { code, bubbles: true, cancelable: true }));
}

function forwarded() {
  return tauriApi.invoke.mock.calls.filter(([command]) => command === FORWARD_COMMAND);
}

beforeEach(() => {
  (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
  tauriApi.invoke.mockReset().mockImplementation(async (command: string) => (command === "load_app_config" ? {} : undefined));
});

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe("App key forwarding", () => {
  it("sends every key press and release in the window to the backend as hotkey input", async () => {
    render(<App />);
    await act(async () => {
      await Promise.resolve();
    });
    act(() => {
      press("KeyG");
      release("KeyG");
    });
    expect(forwarded()).toEqual([
      [FORWARD_COMMAND, { code: "KeyG", pressed: true }],
      [FORWARD_COMMAND, { code: "KeyG", pressed: false }],
    ]);
  });

  it("stops sending keys once the app is unmounted", async () => {
    const { unmount } = render(<App />);
    await act(async () => {
      await Promise.resolve();
    });
    unmount();
    act(() => {
      press("KeyG");
    });
    expect(forwarded()).toEqual([]);
  });
});
