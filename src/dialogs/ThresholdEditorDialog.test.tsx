import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { PedalConfidence } from "../lib/pedalPreview";
import type { ThresholdCurves } from "../lib/thresholdCurve";
import { ThemeProvider } from "../theme/ThemeProvider";
import { ThresholdEditorDialog } from "./ThresholdEditorDialog";

const WIDTH = 200;
const HEIGHT = 100;
const HIT_RADIUS = 16;
const FPS = 10;
const DURATION = 10;
const CONFIDENCE: PedalConfidence = { values: new Array(FPS * DURATION).fill(0.1), fps: FPS };
const CURVES: ThresholdCurves = {
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

beforeEach(() => {
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(WIDTH);
  vi.spyOn(HTMLCanvasElement.prototype, "clientHeight", "get").mockReturnValue(HEIGHT);
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

function renderDialog(overrides: { onCurvesChange?: (curves: ThresholdCurves) => void; onClose?: () => void } = {}) {
  const view = render(
    <ThemeProvider>
      <ThresholdEditorDialog
        confidence={CONFIDENCE}
        curves={CURVES}
        onCurvesChange={overrides.onCurvesChange ?? vi.fn()}
        onClose={overrides.onClose ?? vi.fn()}
      />
    </ThemeProvider>,
  );
  return { ...view, canvas: view.container.querySelector("canvas") as HTMLCanvasElement };
}

describe("ThresholdEditorDialog", () => {
  it("is titled and starts with the exact numbers switched off", () => {
    renderDialog();
    expect(screen.getByText("Edit threshold curves")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Show exact numbers" })).toHaveAttribute("aria-checked", "false");
  });

  it("closes from the close button", () => {
    const onClose = vi.fn();
    renderDialog({ onClose });
    fireEvent.click(screen.getByText("×"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("lets nodes be added by double clicking a line", () => {
    const onCurvesChange = vi.fn();
    const { canvas } = renderDialog({ onCurvesChange });
    fireEvent.doubleClick(canvas, { clientX: xOf(2.5), clientY: yOf(0.7) });
    const next = onCurvesChange.mock.calls[0][0] as ThresholdCurves;
    expect(next.on.map((node) => node.time)).toEqual([0, 2.5, 5, 10]);
  });

  it("lets nodes be dragged and reports the moved curves", () => {
    const onCurvesChange = vi.fn();
    const { canvas } = renderDialog({ onCurvesChange });
    fireEvent.mouseDown(canvas, { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    fireEvent.mouseMove(window, { clientX: xOf(6), clientY: yOf(0.9) });
    fireEvent.mouseUp(window);
    const next = onCurvesChange.mock.calls[onCurvesChange.mock.calls.length - 1][0] as ThresholdCurves;
    expect(next.on[1].time).toBeCloseTo(6);
  });

  it("shows no numbers while dragging until the switch is turned on", () => {
    const { canvas, container } = renderDialog();
    fireEvent.mouseDown(canvas, { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    expect(container.querySelector(".pedal-ai-card__chart-readout")).toBeNull();
    fireEvent.mouseUp(window);
    fireEvent.click(screen.getByRole("switch", { name: "Show exact numbers" }));
    fireEvent.mouseDown(canvas, { clientX: xOf(5), clientY: yOf(0.8), button: 0 });
    expect(screen.getByText("5.00 s")).toBeInTheDocument();
    expect(screen.getByText("0.800")).toBeInTheDocument();
    fireEvent.mouseUp(window);
    expect(container.querySelector(".pedal-ai-card__chart-readout")).toBeNull();
  });
});
