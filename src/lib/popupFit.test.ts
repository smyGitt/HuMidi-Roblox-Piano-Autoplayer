import { describe, expect, it } from "vitest";
import { EDGE_MARGIN, fitToViewport } from "./popupFit";

const VIEWPORT = { width: 1000, height: 700 };

function rect(left: number, top: number, width: number, height: number) {
  return { left, top, right: left + width, bottom: top + height };
}

describe("fitToViewport", () => {
  it("leaves a popup that fits where it is", () => {
    expect(fitToViewport(rect(100, 100, 200, 50), VIEWPORT)).toEqual({ dx: 0, above: false });
  });

  it("shifts a popup that runs past the right edge back inside, keeping the margin", () => {
    expect(fitToViewport(rect(850, 100, 200, 50), VIEWPORT)).toEqual({ dx: -(1050 - (1000 - EDGE_MARGIN)), above: false });
  });

  it("shifts a popup that runs past the left edge back inside, keeping the margin", () => {
    expect(fitToViewport(rect(-40, 100, 200, 50), VIEWPORT)).toEqual({ dx: 40 + EDGE_MARGIN, above: false });
  });

  it("treats a popup touching the edge as too close and keeps the margin", () => {
    expect(fitToViewport(rect(800, 100, 200, 50), VIEWPORT).dx).toBe(-EDGE_MARGIN);
  });

  it("never pushes a popup wider than the window off the left edge", () => {
    expect(fitToViewport(rect(-50, 100, 1100, 50), VIEWPORT).dx).toBe(50 + EDGE_MARGIN);
  });

  it("puts a popup above its anchor when it would run past the bottom", () => {
    expect(fitToViewport(rect(100, 660, 200, 50), VIEWPORT)).toEqual({ dx: 0, above: true });
  });

  it("combines a horizontal shift with flipping above", () => {
    expect(fitToViewport(rect(900, 680, 200, 50), VIEWPORT)).toEqual({ dx: -(1100 - (1000 - EDGE_MARGIN)), above: true });
  });
});
