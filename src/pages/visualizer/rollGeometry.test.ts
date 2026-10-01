import { describe, expect, it } from "vitest";
import { getKeyRange } from "../../lib/keyRange";
import { buildKeyLayout, keyGeometry } from "./keyLayout";
import {
  dragTime,
  fallingNoteRect,
  fallingWindow,
  fallingY,
  horizontalRowHeight,
  horizontalRowY,
  horizontalSpanRect,
  horizontalWindow,
  horizontalX,
  playheadX,
  type RollProjection,
} from "./rollGeometry";

const projection: RollProjection = { time: 10, viewport: { width: 800, height: 400 }, secondLength: 100 };

describe("falling notes", () => {
  it("puts the current time on the bottom edge, where the keyboard starts", () => {
    expect(fallingY(10, projection)).toBe(400);
  });

  it("places later notes higher up, one second per secondLength pixels", () => {
    expect(fallingY(11, projection)).toBe(300);
    expect(fallingY(14, projection)).toBe(0);
  });

  it("shows from now until the top edge", () => {
    expect(fallingWindow(projection)).toEqual([10, 14]);
  });

  it("draws a note in its key's column, from its start at the bottom to its end at the top", () => {
    const layout = buildKeyLayout(getKeyRange(false));
    const key = keyGeometry(layout, 60)!;
    const rect = fallingNoteRect(11, 12, key, projection, 0, 1);
    expect(rect.x).toBeCloseTo(key.left * 800);
    expect(rect.width).toBeCloseTo(key.width * 800);
    expect(rect.y).toBe(200);
    expect(rect.height).toBe(100);
  });

  it("insets the note by the gap so neighbouring and repeated notes stay separate", () => {
    const layout = buildKeyLayout(getKeyRange(false));
    const key = keyGeometry(layout, 60)!;
    const rect = fallingNoteRect(11, 12, key, projection, 1, 1);
    expect(rect.x).toBeCloseTo(key.left * 800 + 1);
    expect(rect.width).toBeCloseTo(key.width * 800 - 2);
    expect(rect.height).toBe(99);
  });

  it("never draws a note smaller than the minimum size", () => {
    const layout = buildKeyLayout(getKeyRange(false));
    const rect = fallingNoteRect(11, 11.001, keyGeometry(layout, 60)!, projection, 1, 1);
    expect(rect.height).toBe(1);
  });
});

describe("horizontal scrolling", () => {
  it("fixes the progress line at the configured fraction of the width", () => {
    expect(playheadX(projection, 0.25)).toBe(200);
    expect(horizontalX(10, projection, 0.25)).toBe(200);
  });

  it("places later notes to the right of the progress line", () => {
    expect(horizontalX(12, projection, 0.25)).toBe(400);
    expect(horizontalX(8, projection, 0.25)).toBe(0);
  });

  it("shows the stretch of song from the left edge to the right edge", () => {
    expect(horizontalWindow(projection, 0.25)).toEqual([8, 16]);
  });

  it("gives every pitch in the range one row plus one row for the pedal lane", () => {
    const compact = getKeyRange(false);
    expect(horizontalRowHeight({ width: 800, height: 620 }, compact)).toBe(10);
  });

  it("puts the highest pitch on the top row and the lowest just above the pedal lane", () => {
    const compact = getKeyRange(false);
    expect(horizontalRowY(96, compact, 10)).toBe(0);
    expect(horizontalRowY(36, compact, 10)).toBe(600);
  });

  it("spans a note from its start to its end along the time axis", () => {
    const rect = horizontalSpanRect(12, 13, 50, 10, projection, 0.25, 0, 1);
    expect(rect).toEqual({ x: 400, y: 50, width: 100, height: 10 });
  });
});

describe("dragTime", () => {
  it("dragging down in falling mode pulls the notes onto the keys and moves forward in time", () => {
    expect(dragTime(10, 100, 100, false, 60)).toBe(11);
  });

  it("dragging left in horizontal mode pulls the notes onto the line and moves forward in time", () => {
    expect(dragTime(10, -100, 100, true, 60)).toBe(11);
    expect(dragTime(10, 100, 100, true, 60)).toBe(9);
  });

  it("stays inside the song", () => {
    expect(dragTime(1, -500, 100, false, 60)).toBe(0);
    expect(dragTime(59, 500, 100, false, 60)).toBe(60);
  });
});
