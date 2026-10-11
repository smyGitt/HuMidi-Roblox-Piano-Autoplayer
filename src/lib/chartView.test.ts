import { describe, expect, it } from "vitest";
import {
  clampView,
  fullView,
  isFullView,
  MIN_VIEW_SPAN,
  panView,
  timeToX,
  xToTime,
  zoomView,
} from "./chartView";

const DURATION = 200;
const WIDTH = 400;

describe("fullView and isFullView", () => {
  it("covers the whole song", () => {
    expect(fullView(DURATION)).toEqual({ start: 0, end: DURATION });
    expect(isFullView(fullView(DURATION), DURATION)).toBe(true);
  });

  it("is false once the view is narrower", () => {
    expect(isFullView({ start: 0, end: 100 }, DURATION)).toBe(false);
    expect(isFullView({ start: 10, end: DURATION }, DURATION)).toBe(false);
  });
});

describe("zoomView", () => {
  it("zooms in around the anchor and keeps it under the cursor", () => {
    const next = zoomView(fullView(DURATION), 100, 0.5, DURATION);
    expect(next.end - next.start).toBeCloseTo(100);
    expect(next.start).toBeCloseTo(50);
    expect(next.end).toBeCloseTo(150);
  });

  it("keeps an off center anchor at the same relative position", () => {
    const view = { start: 0, end: 100 };
    const next = zoomView(view, 25, 0.5, DURATION);
    expect((25 - next.start) / (next.end - next.start)).toBeCloseTo(0.25);
  });

  it("zooms out and stops at the whole song", () => {
    const next = zoomView({ start: 50, end: 150 }, 100, 10, DURATION);
    expect(next).toEqual(fullView(DURATION));
  });

  it("stops zooming in at the minimum span", () => {
    const next = zoomView({ start: 0, end: 20 }, 10, 0.01, DURATION);
    expect(next.end - next.start).toBeCloseTo(MIN_VIEW_SPAN);
    expect((10 - next.start) / (next.end - next.start)).toBeCloseTo(0.5);
  });

  it("never leaves the song when zooming near an edge", () => {
    const next = zoomView(fullView(DURATION), 0, 0.5, DURATION);
    expect(next.start).toBe(0);
    const far = zoomView(fullView(DURATION), DURATION, 0.5, DURATION);
    expect(far.end).toBeCloseTo(DURATION);
  });

  it("ignores a scale that is not a positive number", () => {
    const view = { start: 10, end: 60 };
    expect(zoomView(view, 30, 0, DURATION)).toBe(view);
    expect(zoomView(view, 30, Number.NaN, DURATION)).toBe(view);
  });
});

describe("panView", () => {
  it("shifts the view without changing its width", () => {
    const next = panView({ start: 50, end: 100 }, 20, DURATION);
    expect(next).toEqual({ start: 70, end: 120 });
  });

  it("stops at both ends of the song", () => {
    expect(panView({ start: 5, end: 55 }, -50, DURATION)).toEqual({ start: 0, end: 50 });
    expect(panView({ start: 150, end: 200 }, 50, DURATION)).toEqual({ start: 150, end: 200 });
  });
});

describe("clampView", () => {
  it("keeps a song shorter than the minimum span fully visible", () => {
    expect(clampView({ start: 0, end: 2 }, 3)).toEqual({ start: 0, end: 3 });
  });
});

describe("timeToX and xToTime", () => {
  it("map the view onto the width and back", () => {
    const view = { start: 50, end: 150 };
    expect(timeToX(100, view, WIDTH)).toBe(WIDTH / 2);
    expect(xToTime(WIDTH / 2, view, WIDTH)).toBe(100);
    expect(timeToX(50, view, WIDTH)).toBe(0);
    expect(xToTime(WIDTH, view, WIDTH)).toBe(150);
  });

  it("handle a zero sized view or width without dividing by zero", () => {
    expect(timeToX(5, { start: 5, end: 5 }, WIDTH)).toBe(0);
    expect(xToTime(10, { start: 5, end: 9 }, 0)).toBe(5);
  });
});
