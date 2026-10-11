import { describe, expect, it } from "vitest";
import {
  addNode,
  curveValueAt,
  EMPTY_CURVES,
  flatCurves,
  hasCurves,
  isOrdered,
  MIN_NODE_GAP,
  moveNode,
  removeNode,
  thresholdsAt,
  type ThresholdCurves,
  type ThresholdNode,
} from "./thresholdCurve";

const DURATION = 100;

function node(time: number, value: number): ThresholdNode {
  return { time, value };
}

function curves(on: ThresholdNode[], off: ThresholdNode[]): ThresholdCurves {
  return { on, off };
}

const BASE = curves([node(0, 0.6), node(10, 0.6), node(20, 0.6)], [node(0, 0.4), node(10, 0.4), node(20, 0.4)]);

describe("curveValueAt", () => {
  it("has no value for an empty curve", () => {
    expect(curveValueAt([], 1)).toBeNull();
  });

  it("holds the end values outside the nodes", () => {
    const nodes = [node(2, 0.2), node(4, 0.6)];
    expect(curveValueAt(nodes, 0)).toBe(0.2);
    expect(curveValueAt(nodes, 9)).toBe(0.6);
  });

  it("interpolates linearly between nodes", () => {
    const nodes = [node(0, 0.2), node(4, 0.6)];
    expect(curveValueAt(nodes, 2)).toBeCloseTo(0.4);
    expect(curveValueAt(nodes, 1)).toBeCloseTo(0.3);
  });

  it("picks the right segment among many nodes", () => {
    const nodes = [node(0, 0), node(10, 1), node(20, 0), node(30, 1)];
    expect(curveValueAt(nodes, 15)).toBeCloseTo(0.5);
    expect(curveValueAt(nodes, 25)).toBeCloseTo(0.5);
    expect(curveValueAt(nodes, 20)).toBe(0);
  });
});

describe("thresholdsAt", () => {
  it("is null while a curve is empty", () => {
    expect(thresholdsAt(EMPTY_CURVES, 1)).toBeNull();
  });

  it("caps the off value at the on value", () => {
    expect(thresholdsAt(flatCurves(0.3, 0.5), 1)).toEqual({ on: 0.3, off: 0.3 });
  });
});

describe("flatCurves and hasCurves", () => {
  it("makes two single node curves", () => {
    const flat = flatCurves(0.57, 0.54);
    expect(flat.on).toEqual([node(0, 0.57)]);
    expect(flat.off).toEqual([node(0, 0.54)]);
    expect(hasCurves(flat)).toBe(true);
  });

  it("is not a curve pair unless both curves have nodes", () => {
    expect(hasCurves(EMPTY_CURVES)).toBe(false);
    expect(hasCurves({ on: [node(0, 0.5)], off: [] })).toBe(false);
  });
});

describe("isOrdered", () => {
  it("passes when the off curve stays at or below the on curve", () => {
    expect(isOrdered(BASE)).toBe(true);
  });

  it("fails when an off node pokes above the on line between on nodes", () => {
    const crossed = curves([node(0, 0.2), node(10, 0.2)], [node(0, 0.1), node(5, 0.5), node(10, 0.1)]);
    expect(isOrdered(crossed)).toBe(false);
  });

  it("fails when an on node dips below the off line", () => {
    const crossed = curves([node(0, 0.6), node(5, 0.2), node(10, 0.6)], [node(0, 0.4), node(10, 0.4)]);
    expect(isOrdered(crossed)).toBe(false);
  });

  it("treats touching lines as ordered", () => {
    expect(isOrdered(flatCurves(0.4, 0.4))).toBe(true);
  });
});

describe("addNode", () => {
  it("inserts a node on the line without changing its shape", () => {
    const sloped = curves([node(0, 0.2), node(10, 0.6)], [node(0, 0.1), node(10, 0.3)]);
    const next = addNode(sloped, "on", 5, DURATION);
    expect(next.on).toHaveLength(3);
    expect(next.on[1].time).toBe(5);
    expect(next.on[1].value).toBeCloseTo(0.4);
    expect(next.off).toBe(sloped.off);
  });

  it("keeps the nodes sorted by time", () => {
    const next = addNode(BASE, "off", 15, DURATION);
    expect(next.off.map((n) => n.time)).toEqual([0, 10, 15, 20]);
  });

  it("refuses a node too close to an existing one", () => {
    expect(addNode(BASE, "on", 10 + MIN_NODE_GAP / 2, DURATION)).toBe(BASE);
  });

  it("clamps the time into the song", () => {
    const next = addNode(BASE, "on", DURATION + 50, DURATION);
    expect(next.on[next.on.length - 1].time).toBe(DURATION);
  });

  it("does nothing on an empty curve", () => {
    expect(addNode(EMPTY_CURVES, "on", 5, DURATION)).toBe(EMPTY_CURVES);
  });
});

describe("removeNode", () => {
  it("removes the node", () => {
    const next = removeNode(BASE, "on", 1);
    expect(next.on.map((n) => n.time)).toEqual([0, 20]);
  });

  it("never removes the last node of a curve", () => {
    const single = flatCurves(0.6, 0.4);
    expect(removeNode(single, "on", 0)).toBe(single);
  });

  it("ignores an index that does not exist", () => {
    expect(removeNode(BASE, "on", 9)).toBe(BASE);
  });

  it("refuses a removal that would let the off line cross above the on line", () => {
    const peaked = curves(
      [node(0, 0.3), node(10, 0.9), node(20, 0.3)],
      [node(0, 0.2), node(10, 0.7), node(20, 0.2)],
    );
    expect(removeNode(peaked, "on", 1)).toBe(peaked);
  });
});

describe("moveNode", () => {
  it("moves a node in time and value", () => {
    const next = moveNode(BASE, "on", 1, 12, 0.8, DURATION);
    expect(next.on[1]).toEqual(node(12, 0.8));
  });

  it("keeps a node between its neighbours in time", () => {
    const next = moveNode(BASE, "on", 1, 50, 0.6, DURATION);
    expect(next.on[1].time).toBeCloseTo(20 - MIN_NODE_GAP);
    const before = moveNode(BASE, "on", 1, -5, 0.6, DURATION);
    expect(before.on[1].time).toBeCloseTo(MIN_NODE_GAP);
  });

  it("keeps the first node at or after zero and the last at or before the end", () => {
    expect(moveNode(BASE, "on", 0, -9, 0.6, DURATION).on[0].time).toBe(0);
    expect(moveNode(BASE, "on", 2, 500, 0.6, DURATION).on[2].time).toBe(DURATION);
  });

  it("clamps the value into the unit range", () => {
    expect(moveNode(BASE, "on", 1, 10, 3, DURATION).on[1].value).toBe(1);
    expect(moveNode(BASE, "off", 1, 10, -3, DURATION).off[1].value).toBe(0);
  });

  it("stops a press node at the lift line when dragged below it", () => {
    const next = moveNode(BASE, "on", 1, 10, 0.1, DURATION);
    expect(next.on[1].value).toBeCloseTo(0.4, 5);
    expect(isOrdered(next)).toBe(true);
  });

  it("stops a lift node at the press line when dragged above it", () => {
    const next = moveNode(BASE, "off", 1, 10, 0.95, DURATION);
    expect(next.off[1].value).toBeCloseTo(0.6, 5);
    expect(isOrdered(next)).toBe(true);
  });

  it("accounts for lift nodes that sit between the press nodes being moved", () => {
    const shape = curves([node(0, 0.6), node(10, 0.6)], [node(0, 0.4), node(5, 0.55), node(10, 0.4)]);
    const next = moveNode(shape, "on", 1, 10, 0.2, DURATION);
    expect(isOrdered(next)).toBe(true);
    expect(next.on[1].value).toBeCloseTo(0.5, 5);
  });

  it("leaves the other curve untouched", () => {
    const next = moveNode(BASE, "on", 1, 10, 0.9, DURATION);
    expect(next.off).toBe(BASE.off);
  });

  it("returns the same curves for an index that does not exist", () => {
    expect(moveNode(BASE, "on", 7, 10, 0.5, DURATION)).toBe(BASE);
  });

  it("returns the same curves when no value at the new time can keep the order", () => {
    const peaked = curves(
      [node(0, 0.2), node(10, 0.9), node(20, 0.2)],
      [node(0, 0.1), node(10, 0.85), node(20, 0.1)],
    );
    expect(isOrdered(peaked)).toBe(true);
    expect(moveNode(peaked, "on", 1, 19.95, 1, DURATION)).toBe(peaked);
  });
});
