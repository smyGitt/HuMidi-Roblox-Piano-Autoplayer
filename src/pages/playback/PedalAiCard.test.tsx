import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flatCurves } from "../../lib/thresholdCurve";
import { ThemeProvider } from "../../theme/ThemeProvider";
import { PedalAiCard, type PedalAiStats } from "./PedalAiCard";

function baseProps(overrides: Partial<Parameters<typeof PedalAiCard>[0]> = {}) {
  return {
    hasThresholds: true,
    generateEnabled: true,
    curves: flatCurves(0.6, 0.4),
    stats: null as PedalAiStats | null,
    onGenerate: vi.fn(),
    onCurvesChange: vi.fn(),
    onReset: vi.fn(),
    ...overrides,
  };
}

describe("PedalAiCard", () => {
  it("shows the Generate button and hides thresholds/reset before any generation", () => {
    render(<PedalAiCard {...baseProps({ hasThresholds: false })} />);
    expect(screen.getByText("Generate pedal events")).toBeInTheDocument();
    expect(screen.queryByTitle("Reset")).not.toBeInTheDocument();
  });

  it("shows no diagnostics when stats are within normal range", () => {
    const stats: PedalAiStats = { avgDur: 0.5, minDur: 0.3, maxDur: 1.0, pressesPerMin: 20 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.queryByText(/chattering/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/holds too long/i)).not.toBeInTheDocument();
  });

  it("detects chattering (high presses/min) and offers only the Regenerate row, no exact-number suggestions", () => {
    const onGenerate = vi.fn();
    const stats: PedalAiStats = { avgDur: 0.5, minDur: 0.3, maxDur: 1.0, pressesPerMin: 50 };
    render(<PedalAiCard {...baseProps({ stats, onGenerate })} />);
    expect(screen.getByText(/Pedal chattering detected/)).toBeInTheDocument();
    expect(screen.queryByText(/Raise On Threshold/)).not.toBeInTheDocument();
    expect(screen.queryByText(/Lower Off Threshold/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByText(/Regenerate/));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("detects chattering via short average hold even at a low press rate", () => {
    const stats: PedalAiStats = { avgDur: 0.1, minDur: 0.05, maxDur: 0.2, pressesPerMin: 10 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.getByText(/Pedal chattering detected/)).toBeInTheDocument();
  });

  it("detects sparse/long holds and offers only the Regenerate row", () => {
    const onGenerate = vi.fn();
    const stats: PedalAiStats = { avgDur: 2, minDur: 1, maxDur: 20, pressesPerMin: 10 };
    render(<PedalAiCard {...baseProps({ stats, onGenerate })} />);
    expect(screen.getByText(/Pedal holds too long or too sparse/)).toBeInTheDocument();
    expect(screen.queryByText(/Raise Off Threshold/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByText(/Regenerate/));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("detects sparse via a very low press rate even with an otherwise unremarkable max duration", () => {
    const stats: PedalAiStats = { avgDur: 1, minDur: 0.5, maxDur: 2, pressesPerMin: 1 };
    render(<PedalAiCard {...baseProps({ stats })} />);
    expect(screen.getByText(/Pedal holds too long or too sparse/)).toBeInTheDocument();
  });

  it("always shows the Regenerate button once thresholds exist, with or without a warning", () => {
    const onGenerate = vi.fn();
    const stats: PedalAiStats = { avgDur: 0.5, minDur: 0.3, maxDur: 1.0, pressesPerMin: 20 };
    render(<PedalAiCard {...baseProps({ stats, onGenerate })} />);
    expect(screen.queryByText(/chattering/i)).not.toBeInTheDocument();
    fireEvent.click(screen.getByText("Regenerate"));
    expect(onGenerate).toHaveBeenCalledTimes(1);
  });

  it("does not show the Regenerate button before the first generation", () => {
    render(<PedalAiCard {...baseProps({ hasThresholds: false })} />);
    expect(screen.queryByText("Regenerate")).not.toBeInTheDocument();
  });

  it("shows the threshold chart only when a confidence curve is available", () => {
    const confidence = { values: [0, 0.5, 0.8], fps: 50 };
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
    const { container, rerender } = render(<PedalAiCard {...baseProps({ confidence })} />, { wrapper: ThemeProvider });
    expect(container.querySelector(".pedal-ai-card__chart")).not.toBeNull();
    expect(screen.getByText("On threshold", { selector: ".pedal-ai-card__legend-item" })).toBeInTheDocument();
    expect(screen.getByText("Off threshold", { selector: ".pedal-ai-card__legend-item" })).toBeInTheDocument();
    rerender(<PedalAiCard {...baseProps({ confidence: null })} />);
    expect(container.querySelector(".pedal-ai-card__chart")).toBeNull();
    expect(container.querySelector(".pedal-ai-card__legend")).toBeNull();
    vi.restoreAllMocks();
  });

  it("has no threshold number fields or step buttons any more", () => {
    render(<PedalAiCard {...baseProps()} />);
    expect(screen.queryByLabelText(/On threshold/)).not.toBeInTheDocument();
    expect(screen.queryByTitle("On threshold +0.005")).not.toBeInTheDocument();
  });

  it("Reset button calls onReset when thresholds exist", () => {
    const onReset = vi.fn();
    render(<PedalAiCard {...baseProps({ onReset })} />);
    fireEvent.click(screen.getByTitle("Reset"));
    expect(onReset).toHaveBeenCalledTimes(1);
  });
});

const EDIT_WIDTH = 200;
const EDIT_HEIGHT = 100;
const EDIT_HIT_RADIUS = 16;
const EDIT_FPS = 10;
const EDIT_FRAMES = 100;
const EDIT_ON_VALUE = 0.6;

describe("PedalAiCard curve editor", () => {
  const confidence = { values: [0, 0.5, 0.8], fps: 50 };

  beforeEach(() => {
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("offers the Edit curves button only when there is a chart to edit", () => {
    const { rerender } = render(<PedalAiCard {...baseProps({ confidence })} />, { wrapper: ThemeProvider });
    expect(screen.getByTitle("Edit curves")).toBeInTheDocument();
    rerender(<PedalAiCard {...baseProps({ confidence: null })} />);
    expect(screen.queryByTitle("Edit curves")).not.toBeInTheDocument();
    rerender(<PedalAiCard {...baseProps({ hasThresholds: false, confidence })} />);
    expect(screen.queryByTitle("Edit curves")).not.toBeInTheDocument();
  });

  it("opens the editor window from that button and closes it again", () => {
    render(<PedalAiCard {...baseProps({ confidence })} />, { wrapper: ThemeProvider });
    expect(screen.queryByText("Edit threshold curves")).not.toBeInTheDocument();
    fireEvent.click(screen.getByTitle("Edit curves"));
    expect(screen.getByText("Edit threshold curves")).toBeInTheDocument();
    fireEvent.click(screen.getByText("×"));
    expect(screen.queryByText("Edit threshold curves")).not.toBeInTheDocument();
  });

  it("closes the editor when the chart goes away and does not bring it back with the next chart", () => {
    const { rerender } = render(<PedalAiCard {...baseProps({ confidence })} />, { wrapper: ThemeProvider });
    fireEvent.click(screen.getByTitle("Edit curves"));
    rerender(<PedalAiCard {...baseProps({ confidence: null })} />);
    expect(screen.queryByText("Edit threshold curves")).not.toBeInTheDocument();
    rerender(<PedalAiCard {...baseProps({ confidence })} />);
    expect(screen.queryByText("Edit threshold curves")).not.toBeInTheDocument();
  });

  it("keeps the chart in the card as a plain picture that leaves the wheel to the page", () => {
    const { container } = render(<PedalAiCard {...baseProps({ confidence })} />, { wrapper: ThemeProvider });
    const event = new WheelEvent("wheel", { deltaY: -1000, cancelable: true, bubbles: true });
    act(() => {
      (container.querySelector(".pedal-ai-card__chart") as HTMLCanvasElement).dispatchEvent(event);
    });
    expect(event.defaultPrevented).toBe(false);
  });

  it("passes curve edits made in the window on to onCurvesChange", () => {
    const onCurvesChange = vi.fn();
    const longConfidence = { values: new Array(EDIT_FRAMES).fill(0.1), fps: EDIT_FPS };
    vi.spyOn(HTMLCanvasElement.prototype, "clientWidth", "get").mockReturnValue(EDIT_WIDTH);
    vi.spyOn(HTMLCanvasElement.prototype, "clientHeight", "get").mockReturnValue(EDIT_HEIGHT);
    document.documentElement.style.setProperty("--pedal-node-hit-radius", `${EDIT_HIT_RADIUS}px`);
    render(<PedalAiCard {...baseProps({ confidence: longConfidence, onCurvesChange })} />, { wrapper: ThemeProvider });
    fireEvent.click(screen.getByTitle("Edit curves"));
    const canvases = document.querySelectorAll(".pedal-ai-card__chart");
    fireEvent.doubleClick(canvases[canvases.length - 1], {
      clientX: EDIT_WIDTH / 2,
      clientY: EDIT_HEIGHT - EDIT_ON_VALUE * EDIT_HEIGHT,
    });
    expect(onCurvesChange).toHaveBeenCalledTimes(1);
  });
});
