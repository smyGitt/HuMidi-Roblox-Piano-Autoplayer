import { describe, expect, it, vi } from "vitest";
import { LiveKeys } from "./liveKeys";

describe("LiveKeys", () => {
  it("starts with no keys and the pedal up", () => {
    const keys = new LiveKeys();
    expect(keys.snapshot().pitches.size).toBe(0);
    expect(keys.snapshot().pedal).toBe(false);
  });

  it("setPitches replaces the pressed set and notifies", () => {
    const keys = new LiveKeys();
    const listener = vi.fn();
    keys.subscribe(listener);
    keys.setPitches([60, 64]);
    expect([...keys.snapshot().pitches].sort()).toEqual([60, 64]);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("keeps the same snapshot object and stays quiet when nothing actually changed", () => {
    const keys = new LiveKeys();
    keys.setPitches([60, 64]);
    const before = keys.snapshot();
    const listener = vi.fn();
    keys.subscribe(listener);
    keys.setPitches([64, 60]);
    keys.setPedal(false);
    expect(keys.snapshot()).toBe(before);
    expect(listener).not.toHaveBeenCalled();
  });

  it("setPedal keeps the pressed keys", () => {
    const keys = new LiveKeys();
    keys.setPitches([60]);
    keys.setPedal(true);
    expect(keys.snapshot().pedal).toBe(true);
    expect(keys.snapshot().pitches.has(60)).toBe(true);
  });

  it("clear releases everything once and is a no-op when already clear", () => {
    const keys = new LiveKeys();
    keys.setPitches([60]);
    keys.setPedal(true);
    const listener = vi.fn();
    keys.subscribe(listener);
    keys.clear();
    keys.clear();
    expect(keys.snapshot().pitches.size).toBe(0);
    expect(keys.snapshot().pedal).toBe(false);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("stops notifying after unsubscribe", () => {
    const keys = new LiveKeys();
    const listener = vi.fn();
    const unsubscribe = keys.subscribe(listener);
    unsubscribe();
    keys.setPitches([60]);
    expect(listener).not.toHaveBeenCalled();
  });
});
