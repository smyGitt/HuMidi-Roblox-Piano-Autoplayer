import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ThemeProvider } from "../../theme/ThemeProvider";
import type { PedalConfidence } from "../../lib/pedalPreview";
import { flatCurves, type ThresholdCurves } from "../../lib/thresholdCurve";
import { ThresholdChart } from "./ThresholdChart";

const WIDTH = 200;
const HEIGHT = 100;
const LINE_WIDTH = 2;
const SHADE_OPACITY = 0.25;
const NODE_SIZE = 8;
const HIT_RADIUS = 16;
const FPS = 10;
const DURATION = 10;
const VALUES = [0, 0.2, 0.9, 0.9, 0.1, 0.1, 0.9, 0.9, 0.1, 0.1];
const CONFIDENCE: PedalConfidence = { values: [...VALUES, ...new Array(90).fill(0.1)], fps: FPS };
const ON_INDEX = 1;
const OFF_INDEX = 2;

interface FillRectCall {
  alpha: number;
  rect: number[];
}

interface StrokeCall {
  points: number[][];
}

let fillRects: FillRectCall[];
let strokes: StrokeCall[];

function fakeContext() {
  let current: number[][] = [];
  const ctx = {
    fillStyle: "",
    strokeStyle: "",
    lineWidth: 0,
    globalAlpha: 1,
    setTransform: vi.fn(),
    clearRect: vi.fn(),
    beginPath: () => {
      current = [];
    },
    moveTo: (x: number, y: number) => current.push([x, y]),
    lineTo: (x: number, y: number) => current.push([x, y]),
    stroke: () => strokes.push({ points: current }),
    fillRect: (...rect: number[]) => fillRects.push({ alpha: ctx.globalAlpha, rect }),
  };
  return ctx;
}

beforeEach(() => {
  fillRects = [];
  strokes = [];
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => fakeContext() as unknown as CanvasRenderingContext2D,
  );
  vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(WIDTH);
  vi.spyOn(HTMLCanvasElement.prototype, "clientHeight", "get").mockReturnValue(HEIGHT);
  document.documentElement.style.setProperty("--border-width", `${LINE_WIDTH}px`);
  document.documentElement.style.setProperty("--pedal-chart-shade-opacity", String(SHADE_OPACITY));
  document.documentElement.style.setProperty("--pedal-node-size", `${NODE_SIZE}px`);
  document.documentElement.style.setProperty("--pedal-node-hit-radius", `${HIT_RADIUS}px`);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function xOf(time: number): number {
  return (time / DURATION) * WIDTH;
}

function yOf(value: number): number {
  return HEIGHT - value * HEIGHT;
}

function renderChart(
  curves: ThresholdCurves,
  onCurvesChange?: (curves: ThresholdCurves) => void,
  showValues = false,
) {
  const ui = (c: ThresholdCurves) => (
    <ThemeProvider>
      <ThresholdChart confidence={CONFIDENCE} curves={c} onCurvesChange={onCurvesChange} showValues={showValues} />
    </ThemeProvider>
  );
  const view = render(ui(curves));
  return { ...view, update: (c: ThresholdCurves) => view.rerender(ui(c)) };
}

function canvasOf(container: HTMLElement): HTMLCanvasElement {
  return container.querySelector("canvas") as HTMLCanvasElement;
}

function nodeCentres(): number[][] {
  return fillRects
    .filter((call) => call.alpha === 1 && call.rect[2] === NODE_SIZE && call.rect[3] === NODE_SIZE)
    .map((call) => [call.rect[0] + NODE_SIZE / 2, call.rect[1] + NODE_SIZE / 2]);
}

function lineStroke(index: number): number[][] {
  return strokes[index].points;
}

const SLOPED: ThresholdCurves = {
  on: [
    { time: 0, value: 0.6 },
    { time: 5, value: 0.8 },
    { time: 10, value: 0.6 },
  ],
  off: [
    { time: 0, value: 0.3 },
    { time: 5, value: 0.4 },
    { time: 10, value: 0.3 },
  ],
};

describe("ThresholdChart drawing", () => {
  it("draws a flat On and Off line at the heights of their values", () => {
    renderChart(flatCurves(0.6, 0.3));
    expect(lineStroke(ON_INDEX)).toEqual([[0, yOf(0.6)], [WIDTH, yOf(0.6)]]);
    expect(lineStroke(OFF_INDEX)).toEqual([[0, yOf(0.3)], [WIDTH, yOf(0.3)]]);
  });

  it("bends the lines through their nodes", () => {
    renderChart(SLOPED);
    expect(lineStroke(ON_INDEX)).toEqual([
      [0, yOf(0.6)],
      [xOf(5), yOf(0.8)],
      [WIDTH, yOf(0.6)],
    ]);
  });

  it("draws a square handle on every node of both lines", () => {
    renderChart(SLOPED, vi.fn());
    const centres = nodeCentres();
    expect(centres).toHaveLength(SLOPED.on.length + SLOPED.off.length);
    expect(centres).toContainEqual([xOf(5), yOf(0.8)]);
    expect(centres).toContainEqual([xOf(5), yOf(0.4)]);
    const handles = fillRects.filter((call) => call.alpha === 1 && call.rect[2] === NODE_SIZE);
    expect(handles.every((call) => call.rect[2] === call.rect[3])).toBe(true);
  });

  it("shades the stretches where the pedal would be down for the current curves", () => {
    renderChart(flatCurves(0.6, 0.3));
    const shaded = fillRects
      .filter((call) => call.alpha === SHADE_OPACITY)
      .map((call) => call.rect.map((value) => Math.round(value)));
    const secondWidth = WIDTH / DURATION;
    const pedalStart = (seconds: number) => Math.round(seconds * secondWidth);
    expect(shaded).toEqual([
      [pedalStart(0.2), 0, pedalStart(0.2), HEIGHT],
      [pedalStart(0.6), 0, pedalStart(0.2), HEIGHT],
    ]);
  });

  it("redraws with the new lines and shading as soon as the curves change", () => {
    const { update } = renderChart(flatCurves(0.6, 0.3));
    strokes = [];
    fillRects = [];
    update(flatCurves(0.95, 0.3));
    expect(lineStroke(ON_INDEX)).toEqual([[0, yOf(0.95)], [WIDTH, yOf(0.95)]]);
    expect(fillRects.filter((call) => call.alpha === SHADE_OPACITY)).toEqual([]);
  });
});

describe("ThresholdChart editing", () => {
  it("dragging a node reports the curves with that node moved", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    const canvas = canvasOf(container);
    fireEvent.mouseDown(canvas, { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    fireEvent.mouseMove(window, { clientX: xOf(6), clientY: yOf(0.9) });
    fireEvent.mouseUp(window);
    const next = onChange.mock.calls[onChange.mock.calls.length - 1][0] as ThresholdCurves;
    expect(next.on[1].time).toBeCloseTo(6);
    expect(next.on[1].value).toBeCloseTo(0.9);
    expect(next.off).toEqual(SLOPED.off);
  });

  it("a dragged press node stops at the lift line instead of crossing it", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.mouseDown(canvasOf(container), { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    fireEvent.mouseMove(window, { clientX: xOf(5), clientY: yOf(0.05) });
    fireEvent.mouseUp(window);
    const next = onChange.mock.calls[onChange.mock.calls.length - 1][0] as ThresholdCurves;
    expect(next.on[1].value).toBeCloseTo(0.4, 4);
  });

  it("stops following the pointer after the mouse is released", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.mouseDown(canvasOf(container), { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    fireEvent.mouseUp(window);
    onChange.mockClear();
    fireEvent.mouseMove(window, { clientX: xOf(6), clientY: yOf(0.9) });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("right clicking a node removes it", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.contextMenu(canvasOf(container), { clientX: xOf(5), clientY: yOf(0.8) });
    const next = onChange.mock.calls[0][0] as ThresholdCurves;
    expect(next.on.map((node) => node.time)).toEqual([0, 10]);
  });

  it("right clicking empty space does nothing", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.contextMenu(canvasOf(container), { clientX: xOf(2), clientY: yOf(0.1) });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("double clicking a line adds a node on it without changing its shape", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.doubleClick(canvasOf(container), { clientX: xOf(2.5), clientY: yOf(0.7) });
    const next = onChange.mock.calls[0][0] as ThresholdCurves;
    expect(next.on.map((node) => node.time)).toEqual([0, 2.5, 5, 10]);
    expect(next.on[1].value).toBeCloseTo(0.7);
  });

  it("double clicking away from both lines adds nothing", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.doubleClick(canvasOf(container), { clientX: xOf(2.5), clientY: yOf(0.05) });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("double clicking an existing node does not add another", () => {
    const onChange = vi.fn();
    const { container } = renderChart(SLOPED, onChange);
    fireEvent.doubleClick(canvasOf(container), { clientX: xOf(5), clientY: yOf(0.8) });
    expect(onChange).not.toHaveBeenCalled();
  });

});

describe("ThresholdChart without a change handler", () => {
  it("draws only the lines and the shading, with no node handles", () => {
    renderChart(SLOPED);
    expect(nodeCentres()).toEqual([]);
    expect(lineStroke(ON_INDEX)).toEqual([
      [0, yOf(0.6)],
      [xOf(5), yOf(0.8)],
      [WIDTH, yOf(0.6)],
    ]);
    expect(fillRects.filter((call) => call.alpha === SHADE_OPACITY).length).toBeGreaterThan(0);
  });

  it("is a plain picture: the wheel is left to scroll the page and nothing zooms", () => {
    const { container } = renderChart(SLOPED);
    const event = new WheelEvent("wheel", { deltaY: -1000, clientX: WIDTH / 2, cancelable: true, bubbles: true });
    act(() => {
      canvasOf(container).dispatchEvent(event);
    });
    expect(event.defaultPrevented).toBe(false);
    expect(screen.queryByTitle("Reset zoom")).not.toBeInTheDocument();
  });

  it("does not react to the mouse over or on a node", () => {
    const { container } = renderChart(SLOPED, undefined, true);
    const canvas = canvasOf(container);
    fireEvent.mouseMove(canvas, { clientX: xOf(5), clientY: yOf(0.8) });
    fireEvent.mouseDown(canvas, { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    expect(canvas.dataset.cursor ?? "").toBe("");
    expect(container.querySelector(".pedal-ai-card__chart-readout")).toBeNull();
    fireEvent.mouseUp(window);
  });
});

describe("ThresholdChart value readout", () => {
  function pressNode(container: HTMLElement, time: number, value: number) {
    fireEvent.mouseDown(canvasOf(container), { clientX: xOf(time), clientY: yOf(value), button: 0 });
  }

  function readoutOf(container: HTMLElement): HTMLElement | null {
    return container.querySelector(".pedal-ai-card__chart-readout");
  }

  it("shows nothing while a node is held when the numbers are off", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    pressNode(container, 5, 0.8);
    expect(readoutOf(container)).toBeNull();
    fireEvent.mouseUp(window);
  });

  it("shows the exact time and value of a held node when the numbers are on", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 5, 0.8);
    expect(screen.getByText("5.00 s")).toBeInTheDocument();
    expect(screen.getByText("0.800")).toBeInTheDocument();
    fireEvent.mouseUp(window);
  });

  it("follows the node to its new time and value while it is dragged", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 5, 0.8);
    fireEvent.mouseMove(window, { clientX: xOf(6), clientY: yOf(0.9) });
    expect(screen.getByText("6.00 s")).toBeInTheDocument();
    expect(screen.getByText("0.900")).toBeInTheDocument();
    fireEvent.mouseUp(window);
  });

  it("shows the value the node actually stopped at when the other line blocks it", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 5, 0.8);
    fireEvent.mouseMove(window, { clientX: xOf(5), clientY: yOf(0.05) });
    expect(screen.getByText("0.400")).toBeInTheDocument();
    fireEvent.mouseUp(window);
  });

  it("disappears when the mouse is released", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 5, 0.8);
    fireEvent.mouseUp(window);
    expect(readoutOf(container)).toBeNull();
  });

  it("is not shown when the background is dragged instead of a node", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 2, 0.05);
    expect(readoutOf(container)).toBeNull();
    fireEvent.mouseUp(window);
  });

  it("is coloured by the line of the held node", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 5, 0.8);
    expect(readoutOf(container)).toHaveClass("pedal-ai-card__chart-readout--on");
    fireEvent.mouseUp(window);
    pressNode(container, 5, 0.4);
    expect(readoutOf(container)).toHaveClass("pedal-ai-card__chart-readout--off");
    fireEvent.mouseUp(window);
  });

  it("sits to the left of nodes in the right half and below nodes in the upper half", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 10, 0.6);
    expect(readoutOf(container)).toHaveClass("pedal-ai-card__chart-readout--left");
    expect(readoutOf(container)).toHaveClass("pedal-ai-card__chart-readout--below");
    fireEvent.mouseUp(window);
  });

  it("sits to the right of nodes in the left half and above nodes in the lower half", () => {
    const { container } = renderChart(SLOPED, vi.fn(), true);
    pressNode(container, 0, 0.3);
    expect(readoutOf(container)).not.toHaveClass("pedal-ai-card__chart-readout--left");
    expect(readoutOf(container)).not.toHaveClass("pedal-ai-card__chart-readout--below");
    fireEvent.mouseUp(window);
  });
});

describe("ThresholdChart zoom", () => {
  function wheel(canvas: HTMLCanvasElement, deltaY: number, clientX: number) {
    act(() => {
      canvas.dispatchEvent(new WheelEvent("wheel", { deltaY, clientX, cancelable: true, bubbles: true }));
    });
  }

  it("shows no reset button until the chart is zoomed", () => {
    renderChart(SLOPED, vi.fn());
    expect(screen.queryByTitle("Reset zoom")).not.toBeInTheDocument();
  });

  it("scrolling the wheel up zooms in around the pointer and offers a reset", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    fillRects = [];
    wheel(canvasOf(container), -1000, WIDTH / 2);
    expect(screen.getByTitle("Reset zoom")).toBeInTheDocument();
    expect(nodeCentres().length).toBeLessThan(SLOPED.on.length + SLOPED.off.length + 1);
    const centreNodes = nodeCentres().filter(([x]) => Math.abs(x - WIDTH / 2) < 1);
    expect(centreNodes).toHaveLength(2);
  });

  it("prevents the page from scrolling while zooming", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    const event = new WheelEvent("wheel", { deltaY: -100, clientX: 0, cancelable: true, bubbles: true });
    act(() => {
      canvasOf(container).dispatchEvent(event);
    });
    expect(event.defaultPrevented).toBe(true);
  });

  it("scrolling the wheel down zooms back out", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    wheel(canvasOf(container), -1000, WIDTH / 2);
    wheel(canvasOf(container), 5000, WIDTH / 2);
    expect(screen.queryByTitle("Reset zoom")).not.toBeInTheDocument();
  });

  it("the reset button returns to the whole song", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    wheel(canvasOf(container), -1000, WIDTH / 2);
    fillRects = [];
    fireEvent.click(screen.getByTitle("Reset zoom"));
    expect(screen.queryByTitle("Reset zoom")).not.toBeInTheDocument();
    expect(nodeCentres()).toContainEqual([xOf(0), yOf(0.6)]);
    expect(nodeCentres()).toContainEqual([xOf(10), yOf(0.6)]);
  });

  it("dragging the background while zoomed moves the view", () => {
    const { container } = renderChart(SLOPED, vi.fn());
    const canvas = canvasOf(container);
    wheel(canvas, -1000, WIDTH / 2);
    fireEvent.mouseDown(canvas, { clientX: xOf(1), clientY: yOf(0.02), button: 0 });
    fillRects = [];
    fireEvent.mouseMove(window, { clientX: xOf(1) + 20, clientY: yOf(0.02) });
    fireEvent.mouseUp(window);
    const centreAfter = nodeCentres().filter(([x]) => Math.abs(x - (WIDTH / 2 + 20)) < 1);
    expect(centreAfter).toHaveLength(2);
  });
});
