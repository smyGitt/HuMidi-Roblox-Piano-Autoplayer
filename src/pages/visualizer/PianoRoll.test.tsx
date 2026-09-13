import { render } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PianoRoll } from "./PianoRoll";
import type { VizNote } from "./demoNotes";

const NOTES: VizNote[] = [
  { pitch: 60, start: 0, duration: 1, hand: "left" },
  { pitch: 64, start: 1, duration: 1, hand: "right" },
  { pitch: 67, start: 2, duration: 1, hand: "unknown" },
];

function renderRoll(overrides: Partial<Parameters<typeof PianoRoll>[0]> = {}) {
  const onScrub = vi.fn();
  const onSeek = vi.fn();
  const props = {
    notes: NOTES,
    totalDuration: 10,
    currentTime: 0,
    measureBoundaries: [[0, 2], [2, 4], [4, 6]] as [number, number][],
    pedalIntervals: [[0, 1]] as [number, number][],
    minPitch: 21,
    maxPitch: 108,
    showPedal: true,
    onScrub,
    onSeek,
    ...overrides,
  };
  const utils = render(<PianoRoll {...props} />);
  const svg = utils.container.querySelector("svg.piano-roll__svg") as SVGSVGElement;
  vi.spyOn(svg, "getBoundingClientRect").mockReturnValue({
    left: 0,
    right: 800,
    top: 0,
    bottom: 260,
    width: 800,
    height: 260,
    x: 0,
    y: 0,
    toJSON() {},
  });
  return { ...utils, svg, onScrub, onSeek };
}

beforeEach(() => {
  if (!SVGElement.prototype.setPointerCapture) {
    SVGElement.prototype.setPointerCapture = vi.fn();
  }
  if (!SVGElement.prototype.releasePointerCapture) {
    SVGElement.prototype.releasePointerCapture = vi.fn();
  }
});

function pointerEvent(type: string, clientX: number, pointerId = 1) {
  return new PointerEvent(type, { clientX, pointerId, button: 0, bubbles: true });
}

describe("PianoRoll", () => {
  it("draws one measure line per boundary, positioned at each boundary's start time", () => {
    const { container } = renderRoll({ measureBoundaries: [[0, 2], [2, 4], [4, 6]] });
    const lines = container.querySelectorAll("line.piano-roll__measure-line");
    expect(lines.length).toBe(3);
    expect(lines[0].getAttribute("x1")).toBe("0");
    expect(lines[1].getAttribute("x1")).toBe(String((2 / 10) * 800));
    expect(lines[2].getAttribute("x1")).toBe(String((4 / 10) * 800));
  });

  it("draws no measure lines when measureBoundaries is empty", () => {
    const { container } = renderRoll({ measureBoundaries: [] });
    expect(container.querySelectorAll("line.piano-roll__measure-line").length).toBe(0);
  });

  it("draws one note rect per note, classed by hand", () => {
    const { container } = renderRoll();
    const rects = container.querySelectorAll("rect.piano-roll__note");
    expect(rects.length).toBe(3);
    expect(rects[0].classList.contains("piano-roll__note--left")).toBe(true);
    expect(rects[1].classList.contains("piano-roll__note--right")).toBe(true);
    expect(rects[2].classList.contains("piano-roll__note--unknown")).toBe(true);
  });

  it("draws pedal rects only when showPedal is true", () => {
    const shown = renderRoll({ showPedal: true, pedalIntervals: [[0, 1], [3, 4]] });
    expect(shown.container.querySelectorAll("rect.piano-roll__pedal").length).toBe(2);

    const hidden = renderRoll({ showPedal: false, pedalIntervals: [[0, 1], [3, 4]] });
    expect(hidden.container.querySelectorAll("rect.piano-roll__pedal").length).toBe(0);
  });

  it("pointer down calls onScrub once with the time under the cursor", () => {
    const { svg, onScrub, onSeek } = renderRoll({ totalDuration: 10 });
    svg.dispatchEvent(pointerEvent("pointerdown", 400));
    expect(onScrub).toHaveBeenCalledTimes(1);
    expect(onScrub).toHaveBeenCalledWith(5);
    expect(onSeek).not.toHaveBeenCalled();
  });

  it("pointer move while dragging calls onScrub again with the updated time", () => {
    const { svg, onScrub } = renderRoll({ totalDuration: 10 });
    svg.dispatchEvent(pointerEvent("pointerdown", 0));
    svg.dispatchEvent(pointerEvent("pointermove", 800));
    expect(onScrub).toHaveBeenCalledTimes(2);
    expect(onScrub).toHaveBeenNthCalledWith(1, 0);
    expect(onScrub).toHaveBeenNthCalledWith(2, 10);
  });

  it("pointer move before any pointer down does not call onScrub (no drag in progress)", () => {
    const { svg, onScrub } = renderRoll();
    svg.dispatchEvent(pointerEvent("pointermove", 400));
    expect(onScrub).not.toHaveBeenCalled();
  });

  it("pointer up calls onSeek exactly once with the release time, ending the drag", () => {
    const { svg, onScrub, onSeek } = renderRoll({ totalDuration: 10 });
    svg.dispatchEvent(pointerEvent("pointerdown", 0));
    svg.dispatchEvent(pointerEvent("pointermove", 400));
    svg.dispatchEvent(pointerEvent("pointerup", 800));
    expect(onSeek).toHaveBeenCalledTimes(1);
    expect(onSeek).toHaveBeenCalledWith(10);

    onScrub.mockClear();
    svg.dispatchEvent(pointerEvent("pointermove", 0));
    expect(onScrub).not.toHaveBeenCalled();
  });

  it("a plain click (down then up, no move) previews once and seeks once to the same time", () => {
    const { svg, onScrub, onSeek } = renderRoll({ totalDuration: 10 });
    svg.dispatchEvent(pointerEvent("pointerdown", 400));
    svg.dispatchEvent(pointerEvent("pointerup", 400));
    expect(onScrub).toHaveBeenCalledTimes(1);
    expect(onScrub).toHaveBeenCalledWith(5);
    expect(onSeek).toHaveBeenCalledTimes(1);
    expect(onSeek).toHaveBeenCalledWith(5);
  });

  it("clamps time to [0, totalDuration] for out-of-bounds pointer positions", () => {
    const { svg, onScrub } = renderRoll({ totalDuration: 10 });
    svg.dispatchEvent(pointerEvent("pointerdown", -200));
    expect(onScrub).toHaveBeenCalledWith(0);
    svg.dispatchEvent(pointerEvent("pointermove", 5000));
    expect(onScrub).toHaveBeenLastCalledWith(10);
  });

  it("a non-primary button pointer down does not start a drag", () => {
    const { svg, onScrub } = renderRoll();
    const rightClick = new PointerEvent("pointerdown", { clientX: 400, pointerId: 1, button: 2, bubbles: true });
    svg.dispatchEvent(rightClick);
    expect(onScrub).not.toHaveBeenCalled();
  });
});
