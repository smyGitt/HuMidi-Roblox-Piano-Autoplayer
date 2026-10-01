import { describe, expect, it, vi } from "vitest";
import { MAX_EXTRAPOLATION_SECONDS, PlaybackClock, type FrameScheduler } from "./playbackClock";

function harness(selfTimed: boolean) {
  let nowMs = 0;
  const queued = new Map<number, () => void>();
  let nextHandle = 1;
  const frames: FrameScheduler = {
    request: (callback) => {
      const handle = nextHandle++;
      queued.set(handle, callback);
      return handle;
    },
    cancel: (handle) => {
      queued.delete(handle);
    },
  };
  const clock = new PlaybackClock({ selfTimed, nowMs: () => nowMs, frames });
  clock.setDuration(100);
  return {
    clock,
    advance(seconds: number) {
      nowMs += seconds * 1000;
    },
    runFrame() {
      const pending = [...queued.entries()];
      queued.clear();
      for (const [, callback] of pending) callback();
    },
    pendingFrames: () => queued.size,
  };
}

describe("PlaybackClock (backend-driven, selfTimed = false)", () => {
  it("stays put while running until the first progress report arrives", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.advance(3);
    expect(h.clock.time()).toBe(0);
  });

  it("interpolates forward from the last progress report", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.clock.report(10);
    h.advance(0.1);
    expect(h.clock.time()).toBeCloseTo(10.1);
  });

  it("never runs more than MAX_EXTRAPOLATION_SECONDS past the last report", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.clock.report(10);
    h.advance(5);
    expect(h.clock.time()).toBeCloseTo(10 + MAX_EXTRAPOLATION_SECONDS);
  });

  it("freezes where it is when it stops running", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.clock.report(10);
    h.advance(0.1);
    h.clock.setRunning(false);
    h.advance(2);
    expect(h.clock.time()).toBeCloseTo(10.1);
  });

  it("after resuming, waits for the next report instead of jumping by the time spent paused", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.clock.report(10);
    h.clock.setRunning(false);
    h.advance(30);
    h.clock.setRunning(true);
    h.advance(1);
    expect(h.clock.time()).toBe(10);
    h.clock.report(10.02);
    expect(h.clock.time()).toBeCloseTo(10.02);
  });

  it("jump moves straight to the target and holds there until the backend confirms", () => {
    const h = harness(false);
    h.clock.setRunning(true);
    h.clock.report(10);
    h.clock.jump(50);
    h.advance(1);
    expect(h.clock.time()).toBe(50);
  });

  it("clamps to the song duration and to zero", () => {
    const h = harness(false);
    h.clock.jump(500);
    expect(h.clock.time()).toBe(100);
    h.clock.jump(-5);
    expect(h.clock.time()).toBe(0);
  });
});

describe("PlaybackClock (self-timed preview, selfTimed = true)", () => {
  it("advances on its own as soon as it runs, without any report", () => {
    const h = harness(true);
    h.clock.jump(5);
    h.clock.setRunning(true);
    h.advance(2);
    expect(h.clock.time()).toBeCloseTo(7);
  });

  it("is not limited by the extrapolation cap", () => {
    const h = harness(true);
    h.clock.setRunning(true);
    h.advance(30);
    expect(h.clock.time()).toBeCloseTo(30);
  });
});

describe("PlaybackClock subscriptions", () => {
  it("notifies once for each discrete change", () => {
    const h = harness(false);
    const listener = vi.fn();
    h.clock.subscribe(listener);
    h.clock.jump(4);
    h.clock.setRunning(true);
    expect(listener).toHaveBeenCalledTimes(2);
  });

  it("runs one frame loop while advancing and notifies on every frame", () => {
    const h = harness(true);
    const listener = vi.fn();
    h.clock.subscribe(listener);
    h.clock.setRunning(true);
    listener.mockClear();
    h.runFrame();
    h.runFrame();
    expect(listener).toHaveBeenCalledTimes(2);
    expect(h.pendingFrames()).toBe(1);
  });

  it("stops requesting frames once it stops advancing", () => {
    const h = harness(true);
    h.clock.subscribe(() => {});
    h.clock.setRunning(true);
    h.clock.setRunning(false);
    h.runFrame();
    expect(h.pendingFrames()).toBe(0);
  });

  it("does not notify for reports while it is already advancing, since the frame loop covers them", () => {
    const h = harness(false);
    const listener = vi.fn();
    h.clock.subscribe(listener);
    h.clock.setRunning(true);
    h.clock.report(1);
    listener.mockClear();
    h.clock.report(1.016);
    expect(listener).not.toHaveBeenCalled();
  });

  it("cancels its pending frame when the last listener unsubscribes", () => {
    const h = harness(true);
    const unsubscribe = h.clock.subscribe(() => {});
    h.clock.setRunning(true);
    expect(h.pendingFrames()).toBe(1);
    unsubscribe();
    expect(h.pendingFrames()).toBe(0);
  });
});
