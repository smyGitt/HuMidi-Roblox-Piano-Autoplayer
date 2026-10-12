import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useKeyForwarding } from "./useKeyForwarding";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  forwardKey: vi.fn<(code: string, pressed: boolean) => Promise<void>>(),
}));

vi.mock("./tauri", () => tauriMocks);

function press(code: string, init: KeyboardEventInit = {}, target: EventTarget = window) {
  const event = new KeyboardEvent("keydown", { code, bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

function release(code: string, target: EventTarget = window) {
  const event = new KeyboardEvent("keyup", { code, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
}

async function flush() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.forwardKey.mockReset().mockResolvedValue(undefined);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("useKeyForwarding", () => {
  it("forwards a key press with its physical key code", () => {
    renderHook(() => useKeyForwarding());
    press("KeyG");
    expect(tauriMocks.forwardKey).toHaveBeenCalledTimes(1);
    expect(tauriMocks.forwardKey).toHaveBeenCalledWith("KeyG", true);
  });

  it("forwards a key release", () => {
    renderHook(() => useKeyForwarding());
    release("KeyG");
    expect(tauriMocks.forwardKey).toHaveBeenCalledTimes(1);
    expect(tauriMocks.forwardKey).toHaveBeenCalledWith("KeyG", false);
  });

  it("forwards modifiers and function keys the same way", () => {
    renderHook(() => useKeyForwarding());
    press("ControlLeft");
    press("F6");
    release("ControlLeft");
    expect(tauriMocks.forwardKey.mock.calls).toEqual([
      ["ControlLeft", true],
      ["F6", true],
      ["ControlLeft", false],
    ]);
  });

  it("ignores the repeats of a held key so one press is one event", () => {
    renderHook(() => useKeyForwarding());
    press("KeyG");
    press("KeyG", { repeat: true });
    press("KeyG", { repeat: true });
    expect(tauriMocks.forwardKey).toHaveBeenCalledTimes(1);
  });

  it("still forwards a key pressed again after it was released", () => {
    renderHook(() => useKeyForwarding());
    press("KeyG");
    release("KeyG");
    press("KeyG");
    expect(tauriMocks.forwardKey.mock.calls).toEqual([
      ["KeyG", true],
      ["KeyG", false],
      ["KeyG", true],
    ]);
  });

  it("forwards keys typed into an input and even when the input stops the event", () => {
    renderHook(() => useKeyForwarding());
    const input = document.createElement("input");
    input.addEventListener("keydown", (event) => event.stopPropagation());
    document.body.appendChild(input);
    press("KeyG", {}, input);
    document.body.removeChild(input);
    expect(tauriMocks.forwardKey).toHaveBeenCalledWith("KeyG", true);
  });

  it("never blocks the key from doing what it normally does", () => {
    renderHook(() => useKeyForwarding());
    expect(press("KeyG").defaultPrevented).toBe(false);
    expect(release("KeyG").defaultPrevented).toBe(false);
  });

  it("forwards the code the browser reports even when it is empty or unidentified", () => {
    renderHook(() => useKeyForwarding());
    press("");
    press("Unidentified");
    expect(tauriMocks.forwardKey.mock.calls).toEqual([
      ["", true],
      ["Unidentified", true],
    ]);
  });

  it("does nothing outside the desktop app", () => {
    tauriMocks.isTauri.mockReturnValue(false);
    renderHook(() => useKeyForwarding());
    press("KeyG");
    release("KeyG");
    expect(tauriMocks.forwardKey).not.toHaveBeenCalled();
  });

  it("stops forwarding when the component unmounts", () => {
    const { unmount } = renderHook(() => useKeyForwarding());
    unmount();
    press("KeyG");
    release("KeyG");
    expect(tauriMocks.forwardKey).not.toHaveBeenCalled();
  });

  it("registers its two listeners once, however often it re-renders", () => {
    const add = vi.spyOn(window, "addEventListener");
    const { rerender } = renderHook(() => useKeyForwarding());
    rerender();
    rerender();
    const keyListeners = add.mock.calls.filter(([type]) => type === "keydown" || type === "keyup");
    expect(keyListeners.map(([type]) => type).sort()).toEqual(["keydown", "keyup"]);
  });

  it("removes exactly the listeners it added", () => {
    const add = vi.spyOn(window, "addEventListener");
    const remove = vi.spyOn(window, "removeEventListener");
    const { unmount } = renderHook(() => useKeyForwarding());
    unmount();
    const added = add.mock.calls.filter(([type]) => type === "keydown" || type === "keyup");
    const removed = remove.mock.calls.filter(([type]) => type === "keydown" || type === "keyup");
    expect(removed.map(([type, listener]) => [type, listener]).sort()).toEqual(
      added.map(([type, listener]) => [type, listener]).sort(),
    );
  });

  it("survives the backend refusing a key without an unhandled rejection", async () => {
    const rejected = Promise.reject(new Error("backend gone"));
    const handled = vi.spyOn(rejected, "catch");
    tauriMocks.forwardKey.mockReturnValue(rejected);
    renderHook(() => useKeyForwarding());
    press("KeyG");
    await flush();
    expect(handled).toHaveBeenCalledTimes(1);
    expect(tauriMocks.forwardKey).toHaveBeenCalledTimes(1);
  });

  it("survives the backend refusing a key release without an unhandled rejection", async () => {
    const rejected = Promise.reject(new Error("backend gone"));
    const handled = vi.spyOn(rejected, "catch");
    tauriMocks.forwardKey.mockReturnValue(rejected);
    renderHook(() => useKeyForwarding());
    release("KeyG");
    await flush();
    expect(handled).toHaveBeenCalledTimes(1);
    expect(tauriMocks.forwardKey).toHaveBeenCalledTimes(1);
  });
});
