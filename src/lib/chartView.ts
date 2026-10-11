export interface ChartView {
  start: number;
  end: number;
}

export const MIN_VIEW_SPAN = 5;
export const WHEEL_ZOOM_SENSITIVITY = 0.002;
const VIEW_TOLERANCE = 1e-9;

export function fullView(duration: number): ChartView {
  return { start: 0, end: duration };
}

export function isFullView(view: ChartView, duration: number): boolean {
  return view.start <= VIEW_TOLERANCE && view.end >= duration - VIEW_TOLERANCE;
}

function clampSpan(span: number, duration: number): number {
  const maxSpan = Math.max(duration, 0);
  const minSpan = Math.min(MIN_VIEW_SPAN, maxSpan);
  return Math.min(maxSpan, Math.max(minSpan, span));
}

export function clampView(view: ChartView, duration: number): ChartView {
  const maxSpan = Math.max(duration, 0);
  const span = clampSpan(view.end - view.start, duration);
  const start = Math.min(Math.max(view.start, 0), maxSpan - span);
  return { start, end: start + span };
}

export function zoomView(view: ChartView, anchorTime: number, scale: number, duration: number): ChartView {
  const span = view.end - view.start;
  if (span <= 0 || !Number.isFinite(scale) || scale <= 0) return view;
  const anchorRatio = (anchorTime - view.start) / span;
  const nextSpan = clampSpan(span * scale, duration);
  const start = anchorTime - anchorRatio * nextSpan;
  return clampView({ start, end: start + nextSpan }, duration);
}

export function panView(view: ChartView, deltaSeconds: number, duration: number): ChartView {
  return clampView({ start: view.start + deltaSeconds, end: view.end + deltaSeconds }, duration);
}

export function timeToX(time: number, view: ChartView, width: number): number {
  const span = view.end - view.start;
  return span > 0 ? ((time - view.start) / span) * width : 0;
}

export function xToTime(x: number, view: ChartView, width: number): number {
  return width > 0 ? view.start + (x / width) * (view.end - view.start) : view.start;
}
