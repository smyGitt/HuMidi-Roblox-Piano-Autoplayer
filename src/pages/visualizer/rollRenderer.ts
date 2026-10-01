import type { MeasureBoundary } from "../../lib/tauri";
import {
  VIZ_HANDS,
  firstIntervalIndexEndingAfter,
  forEachNoteInWindow,
  lowerBound,
  type NoteTimeline,
  type TimeInterval,
} from "../../lib/noteTimeline";
import { keyGeometry, type KeyLayout } from "./keyLayout";
import {
  fallingNoteRect,
  fallingWindow,
  fallingY,
  horizontalRowHeight,
  horizontalRowY,
  horizontalSpanRect,
  horizontalWindow,
  horizontalX,
  playheadX,
  type Rect,
  type RollProjection,
} from "./rollGeometry";
import type { VizStyle } from "./vizStyle";

export interface RollFrame {
  ctx: CanvasRenderingContext2D;
  projection: RollProjection;
  style: VizStyle;
  timeline: NoteTimeline;
  layout: KeyLayout;
  measureBoundaries: readonly MeasureBoundary[];
  pedalIntervals: readonly TimeInterval[];
  showProgressLine: boolean;
}

const KEY_PASSES = [false, true] as const;
const boundaryStart = (boundary: MeasureBoundary) => boundary[0];

function addRect(ctx: CanvasRenderingContext2D, rect: Rect): void {
  ctx.rect(rect.x, rect.y, rect.width, rect.height);
}

function forEachBoundaryInWindow(
  boundaries: readonly MeasureBoundary[],
  from: number,
  to: number,
  visit: (start: number) => void,
): void {
  for (let i = lowerBound(boundaries, from, boundaryStart); i < boundaries.length; i++) {
    const start = boundaries[i][0];
    if (start > to) break;
    visit(start);
  }
}

export function drawFalling(frame: RollFrame): void {
  const { ctx, projection, style, timeline, layout } = frame;
  const { width, height } = projection.viewport;
  const [from, to] = fallingWindow(projection);
  ctx.clearRect(0, 0, width, height);

  ctx.fillStyle = style.measureLineColor;
  forEachBoundaryInWindow(frame.measureBoundaries, from, to, (start) => {
    ctx.fillRect(0, fallingY(start, projection) - style.lineWidth, width, style.lineWidth);
  });

  for (const black of KEY_PASSES) {
    for (const hand of VIZ_HANDS) {
      ctx.fillStyle = style.noteColors[hand];
      ctx.beginPath();
      forEachNoteInWindow(timeline, from, to, (note) => {
        if (note.hand !== hand) return;
        const key = keyGeometry(layout, note.pitch);
        if (!key || key.black !== black) return;
        addRect(ctx, fallingNoteRect(note.start, note.end, key, projection, style.noteGap, style.lineWidth));
      });
      ctx.fill();
    }
  }

  if (frame.showProgressLine) {
    ctx.fillStyle = style.progressLineColor;
    ctx.fillRect(0, height - style.lineWidth, width, style.lineWidth);
  }
}

export function drawHorizontal(frame: RollFrame): void {
  const { ctx, projection, style, timeline, layout } = frame;
  const { width, height } = projection.viewport;
  const fraction = style.playheadFraction;
  const [from, to] = horizontalWindow(projection, fraction);
  const rowHeight = horizontalRowHeight(projection.viewport, layout.range);
  ctx.clearRect(0, 0, width, height);

  ctx.fillStyle = style.measureLineColor;
  forEachBoundaryInWindow(frame.measureBoundaries, from, to, (start) => {
    ctx.fillRect(horizontalX(start, projection, fraction), 0, style.lineWidth, height);
  });

  for (const hand of VIZ_HANDS) {
    ctx.fillStyle = style.noteColors[hand];
    ctx.beginPath();
    forEachNoteInWindow(timeline, from, to, (note) => {
      if (note.hand !== hand) return;
      const rowY = horizontalRowY(note.pitch, layout.range, rowHeight);
      addRect(
        ctx,
        horizontalSpanRect(note.start, note.end, rowY, rowHeight, projection, fraction, style.noteGap, style.lineWidth),
      );
    });
    ctx.fill();
  }

  const intervals = frame.pedalIntervals;
  const laneY = height - rowHeight;
  ctx.fillStyle = style.pedalColor;
  ctx.beginPath();
  for (let i = firstIntervalIndexEndingAfter(intervals, from); i < intervals.length; i++) {
    const [start, end] = intervals[i];
    if (start > to) break;
    addRect(ctx, horizontalSpanRect(start, end, laneY, rowHeight, projection, fraction, style.noteGap, style.lineWidth));
  }
  ctx.fill();

  if (frame.showProgressLine) {
    ctx.fillStyle = style.progressLineColor;
    ctx.fillRect(playheadX(projection, fraction), 0, style.lineWidth, height);
  }
}
