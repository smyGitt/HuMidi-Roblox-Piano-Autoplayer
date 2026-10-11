import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { ArrowsInIcon } from "@phosphor-icons/react";
import { Button } from "../../components/Button";
import {
  fullView,
  isFullView,
  panView,
  timeToX,
  WHEEL_ZOOM_SENSITIVITY,
  xToTime,
  zoomView,
  type ChartView,
} from "../../lib/chartView";
import {
  confidenceDuration,
  peakPerColumn,
  pedalIntervalsFromConfidence,
  type PedalConfidence,
} from "../../lib/pedalPreview";
import {
  addNode,
  curveValueAt,
  moveNode,
  removeNode,
  type CurveKind,
  type ThresholdCurves,
  type ThresholdNode,
} from "../../lib/thresholdCurve";
import { useTheme } from "../../theme/ThemeProvider";
import { readThresholdChartStyle } from "./thresholdChartStyle";

const MIN_CONFIDENCE = 0;
const MAX_CONFIDENCE = 1;
const PRIMARY_BUTTON = 0;
const HALF = 2;
const TIME_DECIMALS = 2;
const VALUE_DECIMALS = 3;
const CURVE_KINDS: readonly CurveKind[] = ["on", "off"];

type Drag = { type: "node"; kind: CurveKind; index: number } | { type: "pan"; lastX: number };
type NodeHit = { kind: CurveKind; index: number };

interface Readout {
  kind: CurveKind;
  time: number;
  value: number;
  x: number;
  y: number;
  flipX: boolean;
  flipY: boolean;
}

interface ThresholdChartProps {
  confidence: PedalConfidence;
  curves: ThresholdCurves;
  onCurvesChange?: (curves: ThresholdCurves) => void;
  showValues?: boolean;
}

function clampConfidence(value: number): number {
  return Math.min(MAX_CONFIDENCE, Math.max(MIN_CONFIDENCE, value));
}

function readoutFor(kind: CurveKind, node: ThresholdNode, view: ChartView, width: number, height: number): Readout {
  const x = timeToX(node.time, view, width);
  const y = height - clampConfidence(node.value) * height;
  return { kind, time: node.time, value: node.value, x, y, flipX: x > width / HALF, flipY: y < height / HALF };
}

export function ThresholdChart({ confidence, curves, onCurvesChange, showValues = false }: ThresholdChartProps) {
  const { theme } = useTheme();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const dragRef = useRef<Drag | null>(null);
  const dragCleanupRef = useRef<(() => void) | null>(null);
  const duration = confidenceDuration(confidence);
  const [view, setView] = useState<ChartView>(() => fullView(duration));
  const [readout, setReadout] = useState<Readout | null>(null);
  const viewRef = useRef(view);
  viewRef.current = view;
  const curvesRef = useRef(curves);
  curvesRef.current = curves;
  const onChangeRef = useRef(onCurvesChange);
  onChangeRef.current = onCurvesChange;
  const durationRef = useRef(duration);
  durationRef.current = duration;
  const editable = onCurvesChange !== undefined;

  useEffect(() => {
    setView((current) => (current.start === 0 && current.end === duration ? current : fullView(duration)));
  }, [duration]);

  useEffect(() => () => dragCleanupRef.current?.(), []);

  const intervals = useMemo(
    () => pedalIntervalsFromConfidence(confidence, curves, duration),
    [confidence, curves, duration],
  );

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx || duration <= 0) return;
    const ratio = window.devicePixelRatio;
    const width = canvas.clientWidth;
    const height = canvas.clientHeight;
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    ctx.clearRect(0, 0, width, height);
    const style = readThresholdChartStyle(canvas);
    const xOf = (time: number) => timeToX(time, view, width);
    const yOf = (value: number) => height - clampConfidence(value) * height;

    ctx.globalAlpha = style.shadeOpacity;
    ctx.fillStyle = style.shadeColor;
    for (const [start, end] of intervals) {
      if (end < view.start || start > view.end) continue;
      const left = xOf(Math.max(start, view.start));
      const right = xOf(Math.min(end, view.end));
      ctx.fillRect(left, 0, Math.max(style.lineWidth, right - left), height);
    }
    ctx.globalAlpha = 1;

    const firstFrame = Math.max(0, Math.floor(view.start * confidence.fps));
    const lastFrame = Math.min(confidence.values.length, Math.ceil(view.end * confidence.fps) + 1);
    const peaks = peakPerColumn(confidence.values.slice(firstFrame, lastFrame), Math.max(1, Math.floor(width)));
    ctx.strokeStyle = style.curveColor;
    ctx.lineWidth = style.lineWidth;
    ctx.beginPath();
    peaks.forEach((peak, column) => {
      if (column === 0) ctx.moveTo(column, yOf(peak));
      else ctx.lineTo(column, yOf(peak));
    });
    ctx.stroke();

    for (const kind of CURVE_KINDS) {
      const nodes = curves[kind];
      if (nodes.length === 0) continue;
      const color = kind === "on" ? style.onColor : style.offColor;
      ctx.strokeStyle = color;
      ctx.lineWidth = style.lineWidth;
      ctx.beginPath();
      ctx.moveTo(0, yOf(curveValueAt(nodes, view.start) as number));
      for (const node of nodes) {
        if (node.time > view.start && node.time < view.end) ctx.lineTo(xOf(node.time), yOf(node.value));
      }
      ctx.lineTo(width, yOf(curveValueAt(nodes, view.end) as number));
      ctx.stroke();

      if (!editable) continue;
      ctx.fillStyle = color;
      for (const node of nodes) {
        if (node.time < view.start || node.time > view.end) continue;
        ctx.fillRect(
          xOf(node.time) - style.nodeSize / HALF,
          yOf(node.value) - style.nodeSize / HALF,
          style.nodeSize,
          style.nodeSize,
        );
      }
    }
  }, [confidence, curves, duration, editable, intervals, view]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const resize = () => {
      const ratio = window.devicePixelRatio;
      canvas.width = Math.round(canvas.clientWidth * ratio);
      canvas.height = Math.round(canvas.clientHeight * ratio);
      draw();
    };
    resize();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [draw, theme]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !editable) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const x = event.clientX - canvas.getBoundingClientRect().left;
      const width = canvas.clientWidth;
      const scale = Math.exp(event.deltaY * WHEEL_ZOOM_SENSITIVITY);
      setView((current) => zoomView(current, xToTime(x, current, width), scale, durationRef.current));
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => canvas.removeEventListener("wheel", onWheel);
  }, [editable]);

  const pointerOf = useCallback((clientX: number, clientY: number) => {
    const canvas = canvasRef.current as HTMLCanvasElement;
    const rect = canvas.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top, width: canvas.clientWidth, height: canvas.clientHeight };
  }, []);

  const hitNode = useCallback(
    (x: number, y: number, width: number, height: number): NodeHit | null => {
      const canvas = canvasRef.current as HTMLCanvasElement;
      const { hitRadius } = readThresholdChartStyle(canvas);
      let best: (NodeHit & { distance: number }) | null = null;
      for (const kind of CURVE_KINDS) {
        curvesRef.current[kind].forEach((node, index) => {
          const dx = timeToX(node.time, viewRef.current, width) - x;
          const dy = height - clampConfidence(node.value) * height - y;
          const distance = Math.hypot(dx, dy);
          if (distance <= hitRadius && (best === null || distance < best.distance)) {
            best = { kind, index, distance };
          }
        });
      }
      return best;
    },
    [],
  );

  const setCursor = useCallback((value: string) => {
    const canvas = canvasRef.current;
    if (canvas) canvas.dataset.cursor = value;
  }, []);

  const applyDrag = useCallback(
    (clientX: number, clientY: number) => {
      const drag = dragRef.current;
      if (!drag) return;
      const { x, y, width, height } = pointerOf(clientX, clientY);
      if (drag.type === "node") {
        const time = xToTime(x, viewRef.current, width);
        const value = MAX_CONFIDENCE - y / height;
        const next = moveNode(curvesRef.current, drag.kind, drag.index, time, value, durationRef.current);
        if (next !== curvesRef.current) {
          onChangeRef.current?.(next);
          setReadout(readoutFor(drag.kind, next[drag.kind][drag.index], viewRef.current, width, height));
        }
        return;
      }
      const span = viewRef.current.end - viewRef.current.start;
      const deltaSeconds = -((x - drag.lastX) / width) * span;
      dragRef.current = { type: "pan", lastX: x };
      setView((current) => panView(current, deltaSeconds, durationRef.current));
    },
    [pointerOf],
  );

  const handleMouseDown = (event: ReactMouseEvent<HTMLCanvasElement>) => {
    if (event.button !== PRIMARY_BUTTON) return;
    const { x, y, width, height } = pointerOf(event.clientX, event.clientY);
    const hit = onChangeRef.current ? hitNode(x, y, width, height) : null;
    dragRef.current = hit ? { type: "node", kind: hit.kind, index: hit.index } : { type: "pan", lastX: x };
    setCursor(hit ? "node" : "grabbing");
    if (hit) {
      const node = curvesRef.current[hit.kind][hit.index];
      setReadout(readoutFor(hit.kind, node, viewRef.current, width, height));
    }
    const onMove = (moveEvent: MouseEvent) => applyDrag(moveEvent.clientX, moveEvent.clientY);
    const onUp = () => {
      dragRef.current = null;
      dragCleanupRef.current = null;
      setCursor("");
      setReadout(null);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
    dragCleanupRef.current = onUp;
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  const handleMouseMove = (event: ReactMouseEvent<HTMLCanvasElement>) => {
    if (dragRef.current) return;
    const { x, y, width, height } = pointerOf(event.clientX, event.clientY);
    const overNode = onChangeRef.current ? hitNode(x, y, width, height) !== null : false;
    setCursor(overNode ? "node" : isFullView(viewRef.current, durationRef.current) ? "" : "grab");
  };

  const handleContextMenu = (event: ReactMouseEvent<HTMLCanvasElement>) => {
    if (!onChangeRef.current) return;
    event.preventDefault();
    const { x, y, width, height } = pointerOf(event.clientX, event.clientY);
    const hit = hitNode(x, y, width, height);
    if (!hit) return;
    const next = removeNode(curvesRef.current, hit.kind, hit.index);
    if (next !== curvesRef.current) onChangeRef.current(next);
  };

  const handleDoubleClick = (event: ReactMouseEvent<HTMLCanvasElement>) => {
    if (!onChangeRef.current) return;
    const { x, y, width, height } = pointerOf(event.clientX, event.clientY);
    if (hitNode(x, y, width, height)) return;
    const { hitRadius } = readThresholdChartStyle(event.currentTarget);
    const time = xToTime(x, viewRef.current, width);
    let best: { kind: CurveKind; distance: number } | null = null;
    for (const kind of CURVE_KINDS) {
      const value = curveValueAt(curvesRef.current[kind], time);
      if (value === null) continue;
      const distance = Math.abs(height - clampConfidence(value) * height - y);
      if (distance <= hitRadius && (best === null || distance < best.distance)) best = { kind, distance };
    }
    if (!best) return;
    const next = addNode(curvesRef.current, best.kind, time, durationRef.current);
    if (next !== curvesRef.current) onChangeRef.current(next);
  };

  return (
    <div className="pedal-ai-card__chart-frame">
      <canvas
        ref={canvasRef}
        className="pedal-ai-card__chart"
        onMouseDown={editable ? handleMouseDown : undefined}
        onMouseMove={editable ? handleMouseMove : undefined}
        onContextMenu={editable ? handleContextMenu : undefined}
        onDoubleClick={editable ? handleDoubleClick : undefined}
      />
      {showValues && readout && (
        <div
          className={`pedal-ai-card__chart-readout pedal-ai-card__chart-readout--${readout.kind}${
            readout.flipX ? " pedal-ai-card__chart-readout--left" : ""
          }${readout.flipY ? " pedal-ai-card__chart-readout--below" : ""}`}
          style={{ left: readout.x, top: readout.y }}
        >
          <span>{readout.time.toFixed(TIME_DECIMALS)} s</span>
          <span>{readout.value.toFixed(VALUE_DECIMALS)}</span>
        </div>
      )}
      {!isFullView(view, duration) && (
        <Button
          variant="icon"
          className="pedal-ai-card__chart-reset"
          title="Reset zoom"
          onClick={() => setView(fullView(duration))}
        >
          <ArrowsInIcon weight="duotone" />
        </Button>
      )}
    </div>
  );
}
