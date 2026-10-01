import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "../../theme/ThemeProvider";
import type { PedalConfidence } from "../../lib/pedalPreview";
import { ThresholdChart } from "./ThresholdChart";

const WIDTH = 200;
const HEIGHT = 100;
const LINE_WIDTH = 2;
const SHADE_OPACITY = 0.25;
const FPS = 10;
const CONFIDENCE: PedalConfidence = { values: [0, 0.2, 0.9, 0.9, 0.1, 0.1, 0.9, 0.9, 0.1, 0.1], fps: FPS };

interface FillRectCall {
  style: string;
  alpha: number;
  rect: number[];
}

let calls: FillRectCall[];

function fakeContext() {
  const ctx = {
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 0,
    globalAlpha: 1,
    setTransform: vi.fn(),
    clearRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    fillRect: (...rect: number[]) => calls.push({ style: ctx.fillStyle, alpha: ctx.globalAlpha, rect }),
  };
  return ctx;
}

beforeEach(() => {
  calls = [];
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => fakeContext() as unknown as CanvasRenderingContext2D,
  );
  vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(WIDTH);
  vi.spyOn(HTMLCanvasElement.prototype, "clientHeight", "get").mockReturnValue(HEIGHT);
  document.documentElement.style.setProperty("--border-width", `${LINE_WIDTH}px`);
  document.documentElement.style.setProperty("--pedal-chart-shade-opacity", String(SHADE_OPACITY));
});

afterEach(() => {
  vi.restoreAllMocks();
});

function renderChart(thresholdOn: number, thresholdOff: number) {
  const ui = (on: number, off: number) => (
    <ThemeProvider>
      <ThresholdChart confidence={CONFIDENCE} thresholdOn={on} thresholdOff={off} />
    </ThemeProvider>
  );
  const view = render(ui(thresholdOn, thresholdOff));
  return { ...view, update: (on: number, off: number) => view.rerender(ui(on, off)) };
}

function lineRects() {
  return calls.filter((call) => call.rect[2] === WIDTH && call.rect[3] === LINE_WIDTH).map((call) => call.rect);
}

describe("ThresholdChart", () => {
  it("draws the On and Off lines at the heights of their thresholds", () => {
    renderChart(0.6, 0.3);
    const lines = lineRects();
    expect(lines).toContainEqual([0, HEIGHT - 0.6 * HEIGHT - LINE_WIDTH / 2, WIDTH, LINE_WIDTH]);
    expect(lines).toContainEqual([0, HEIGHT - 0.3 * HEIGHT - LINE_WIDTH / 2, WIDTH, LINE_WIDTH]);
  });

  it("shades the stretches where the pedal would be down for the current thresholds", () => {
    renderChart(0.6, 0.3);
    const shaded = calls
      .filter((call) => call.alpha === SHADE_OPACITY)
      .map((call) => call.rect.map((value) => Math.round(value)));
    const secondWidth = WIDTH / (CONFIDENCE.values.length / FPS);
    const pedalStart = (seconds: number) => Math.round(seconds * secondWidth);
    expect(shaded).toEqual([
      [pedalStart(0.2), 0, pedalStart(0.2), HEIGHT],
      [pedalStart(0.6), 0, pedalStart(0.2), HEIGHT],
    ]);
  });

  it("redraws with the new lines and shading as soon as a threshold changes", () => {
    const { update } = renderChart(0.6, 0.3);
    calls = [];
    update(0.95, 0.3);
    expect(lineRects()).toContainEqual([0, HEIGHT - 0.95 * HEIGHT - LINE_WIDTH / 2, WIDTH, LINE_WIDTH]);
    expect(calls.filter((call) => call.alpha === SHADE_OPACITY)).toEqual([]);
  });
});
