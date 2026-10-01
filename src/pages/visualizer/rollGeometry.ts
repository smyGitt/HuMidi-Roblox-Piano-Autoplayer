import type { KeyRange } from "../../lib/keyRange";
import type { KeyGeometry } from "./keyLayout";

export interface Viewport {
  width: number;
  height: number;
}

export interface RollProjection {
  time: number;
  viewport: Viewport;
  secondLength: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const PEDAL_LANE_ROWS = 1;

export function fallingWindow({ time, viewport, secondLength }: RollProjection): [number, number] {
  return [time, time + viewport.height / secondLength];
}

export function fallingY(at: number, { time, viewport, secondLength }: RollProjection): number {
  return viewport.height - (at - time) * secondLength;
}

export function fallingNoteRect(
  start: number,
  end: number,
  key: KeyGeometry,
  projection: RollProjection,
  gap: number,
  minSize: number,
): Rect {
  const top = fallingY(end, projection);
  const bottom = fallingY(start, projection) - gap;
  const { width } = projection.viewport;
  return {
    x: key.left * width + gap,
    y: top,
    width: Math.max(key.width * width - gap - gap, minSize),
    height: Math.max(bottom - top, minSize),
  };
}

export function playheadX({ viewport }: RollProjection, playheadFraction: number): number {
  return viewport.width * playheadFraction;
}

export function horizontalWindow(projection: RollProjection, playheadFraction: number): [number, number] {
  const { time, viewport, secondLength } = projection;
  const head = playheadX(projection, playheadFraction);
  return [time - head / secondLength, time + (viewport.width - head) / secondLength];
}

export function horizontalX(at: number, projection: RollProjection, playheadFraction: number): number {
  return playheadX(projection, playheadFraction) + (at - projection.time) * projection.secondLength;
}

export function horizontalRowHeight(viewport: Viewport, range: KeyRange): number {
  return viewport.height / (range.maxPitch - range.minPitch + 1 + PEDAL_LANE_ROWS);
}

export function horizontalRowY(pitch: number, range: KeyRange, rowHeight: number): number {
  return (range.maxPitch - pitch) * rowHeight;
}

export function horizontalSpanRect(
  start: number,
  end: number,
  rowY: number,
  rowHeight: number,
  projection: RollProjection,
  playheadFraction: number,
  gap: number,
  minSize: number,
): Rect {
  const left = horizontalX(start, projection, playheadFraction);
  const right = horizontalX(end, projection, playheadFraction) - gap;
  return {
    x: left,
    y: rowY,
    width: Math.max(right - left, minSize),
    height: Math.max(rowHeight - gap, minSize),
  };
}

export function dragTime(
  startTime: number,
  deltaPx: number,
  secondLength: number,
  horizontal: boolean,
  duration: number,
): number {
  const deltaSeconds = (horizontal ? -deltaPx : deltaPx) / secondLength;
  return Math.min(Math.max(startTime + deltaSeconds, 0), duration);
}
