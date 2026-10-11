export interface ThresholdNode {
  time: number;
  value: number;
}

export interface ThresholdCurves {
  on: ThresholdNode[];
  off: ThresholdNode[];
}

export type CurveKind = keyof ThresholdCurves;

export const MIN_THRESHOLD_VALUE = 0;
export const MAX_THRESHOLD_VALUE = 1;
export const MIN_NODE_GAP = 0.05;
const ORDER_TOLERANCE = 1e-9;
const BISECTION_STEPS = 24;
const HALF = 2;

export const EMPTY_CURVES: ThresholdCurves = { on: [], off: [] };

export function hasCurves(curves: ThresholdCurves): boolean {
  return curves.on.length > 0 && curves.off.length > 0;
}

export function flatCurves(on: number, off: number): ThresholdCurves {
  return { on: [{ time: 0, value: on }], off: [{ time: 0, value: off }] };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function curveValueAt(nodes: readonly ThresholdNode[], time: number): number | null {
  if (nodes.length === 0) return null;
  const first = nodes[0];
  const last = nodes[nodes.length - 1];
  if (time <= first.time) return first.value;
  if (time >= last.time) return last.value;
  let low = 0;
  let high = nodes.length - 1;
  while (high - low > 1) {
    const mid = Math.floor((low + high) / HALF);
    if (nodes[mid].time <= time) low = mid;
    else high = mid;
  }
  const a = nodes[low];
  const b = nodes[high];
  const span = b.time - a.time;
  if (span <= 0) return b.value;
  return a.value + ((b.value - a.value) * (time - a.time)) / span;
}

export function thresholdsAt(curves: ThresholdCurves, time: number): { on: number; off: number } | null {
  const on = curveValueAt(curves.on, time);
  const off = curveValueAt(curves.off, time);
  if (on === null || off === null) return null;
  return { on, off: Math.min(off, on) };
}

export function isOrdered(curves: ThresholdCurves): boolean {
  if (!hasCurves(curves)) return true;
  return [...curves.on, ...curves.off].every((node) => {
    const on = curveValueAt(curves.on, node.time) as number;
    const off = curveValueAt(curves.off, node.time) as number;
    return on >= off - ORDER_TOLERANCE;
  });
}

function withNodes(curves: ThresholdCurves, kind: CurveKind, nodes: ThresholdNode[]): ThresholdCurves {
  return { ...curves, [kind]: nodes };
}

export function addNode(curves: ThresholdCurves, kind: CurveKind, time: number, maxTime: number): ThresholdCurves {
  const nodes = curves[kind];
  const at = clamp(time, 0, maxTime);
  if (nodes.length === 0) return curves;
  if (nodes.some((node) => Math.abs(node.time - at) < MIN_NODE_GAP)) return curves;
  const value = curveValueAt(nodes, at) as number;
  const next = [...nodes, { time: at, value }].sort((a, b) => a.time - b.time);
  return withNodes(curves, kind, next);
}

export function removeNode(curves: ThresholdCurves, kind: CurveKind, index: number): ThresholdCurves {
  const nodes = curves[kind];
  if (nodes.length <= 1 || index < 0 || index >= nodes.length) return curves;
  const next = withNodes(
    curves,
    kind,
    nodes.filter((_, i) => i !== index),
  );
  return isOrdered(next) ? next : curves;
}

export function moveNode(
  curves: ThresholdCurves,
  kind: CurveKind,
  index: number,
  time: number,
  value: number,
  maxTime: number,
): ThresholdCurves {
  const nodes = curves[kind];
  if (index < 0 || index >= nodes.length) return curves;
  const lower = index > 0 ? nodes[index - 1].time + MIN_NODE_GAP : 0;
  const upper = index < nodes.length - 1 ? nodes[index + 1].time - MIN_NODE_GAP : maxTime;
  const at = clamp(time, lower, Math.max(lower, upper));
  const place = (v: number) =>
    withNodes(
      curves,
      kind,
      nodes.map((node, i) => (i === index ? { time: at, value: v } : node)),
    );

  const requested = clamp(value, MIN_THRESHOLD_VALUE, MAX_THRESHOLD_VALUE);
  if (isOrdered(place(requested))) return place(requested);

  const safe = kind === "on" ? MAX_THRESHOLD_VALUE : MIN_THRESHOLD_VALUE;
  if (!isOrdered(place(safe))) return curves;
  let feasible = safe;
  let infeasible = requested;
  for (let step = 0; step < BISECTION_STEPS; step++) {
    const mid = (feasible + infeasible) / HALF;
    if (isOrdered(place(mid))) feasible = mid;
    else infeasible = mid;
  }
  return place(feasible);
}
