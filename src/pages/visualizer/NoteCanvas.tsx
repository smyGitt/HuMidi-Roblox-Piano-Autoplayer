import { useCallback, useEffect, useRef, type PointerEvent } from "react";
import type { MeasureBoundary } from "../../lib/tauri";
import type { NoteTimeline, TimeInterval } from "../../lib/noteTimeline";
import type { PlaybackClock } from "../../state/playbackClock";
import { useTheme } from "../../theme/ThemeProvider";
import type { KeyLayout } from "./keyLayout";
import { dragTime, type Viewport } from "./rollGeometry";
import { drawFalling, drawHorizontal } from "./rollRenderer";
import { readVizStyle, type VizStyle } from "./vizStyle";

const PRIMARY_POINTER_BUTTON = 0;

interface NoteCanvasProps {
  clock: PlaybackClock;
  timeline: NoteTimeline;
  layout: KeyLayout;
  measureBoundaries: readonly MeasureBoundary[];
  pedalIntervals: readonly TimeInterval[];
  horizontal: boolean;
  duration: number;
  onDrag: (time: number | null) => void;
  onSeek: (time: number) => void;
}

interface DragState {
  pointerId: number;
  origin: number;
  startTime: number;
  time: number;
  moved: boolean;
}

export function NoteCanvas(props: NoteCanvasProps) {
  const { theme } = useTheme();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const propsRef = useRef(props);
  propsRef.current = props;
  const styleRef = useRef<VizStyle | null>(null);
  const viewportRef = useRef<Viewport>({ width: 0, height: 0 });
  const dragRef = useRef<DragState | null>(null);
  const pendingFrameRef = useRef<number | null>(null);

  const currentStyle = useCallback((): VizStyle | null => {
    if (!styleRef.current && canvasRef.current) styleRef.current = readVizStyle(canvasRef.current);
    return styleRef.current;
  }, []);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const style = currentStyle();
    if (!canvas || !style) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const current = propsRef.current;
    const ratio = window.devicePixelRatio;
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    const frame = {
      ctx,
      projection: {
        time: dragRef.current?.time ?? current.clock.time(),
        viewport: viewportRef.current,
        secondLength: style.secondLength,
      },
      style,
      timeline: current.timeline,
      layout: current.layout,
      measureBoundaries: current.measureBoundaries,
      pedalIntervals: current.pedalIntervals,
      showProgressLine: current.duration > 0,
    };
    if (current.horizontal) drawHorizontal(frame);
    else drawFalling(frame);
  }, [currentStyle]);

  const requestDraw = useCallback(() => {
    if (pendingFrameRef.current !== null) return;
    pendingFrameRef.current = window.requestAnimationFrame(() => {
      pendingFrameRef.current = null;
      draw();
    });
  }, [draw]);

  useEffect(
    () => () => {
      if (pendingFrameRef.current !== null) window.cancelAnimationFrame(pendingFrameRef.current);
    },
    [],
  );

  useEffect(() => {
    styleRef.current = null;
    requestDraw();
  }, [theme, requestDraw]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const resize = () => {
      const ratio = window.devicePixelRatio;
      viewportRef.current = { width: canvas.clientWidth, height: canvas.clientHeight };
      canvas.width = Math.round(canvas.clientWidth * ratio);
      canvas.height = Math.round(canvas.clientHeight * ratio);
      draw();
    };
    resize();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [draw]);

  useEffect(() => props.clock.subscribe(draw), [props.clock, draw]);

  useEffect(() => {
    requestDraw();
  }, [
    props.timeline,
    props.layout,
    props.measureBoundaries,
    props.pedalIntervals,
    props.horizontal,
    props.duration,
    requestDraw,
  ]);

  function pointerPosition(e: PointerEvent<HTMLCanvasElement>): number {
    return propsRef.current.horizontal ? e.clientX : e.clientY;
  }

  function handlePointerDown(e: PointerEvent<HTMLCanvasElement>) {
    if (e.button !== PRIMARY_POINTER_BUTTON || propsRef.current.duration <= 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    const time = propsRef.current.clock.time();
    dragRef.current = { pointerId: e.pointerId, origin: pointerPosition(e), startTime: time, time, moved: false };
    propsRef.current.onDrag(time);
  }

  function handlePointerMove(e: PointerEvent<HTMLCanvasElement>) {
    const drag = dragRef.current;
    const style = currentStyle();
    if (!drag || !style || drag.pointerId !== e.pointerId) return;
    const current = propsRef.current;
    drag.time = dragTime(
      drag.startTime,
      pointerPosition(e) - drag.origin,
      style.secondLength,
      current.horizontal,
      current.duration,
    );
    drag.moved = true;
    current.onDrag(drag.time);
    requestDraw();
  }

  function endDrag(e: PointerEvent<HTMLCanvasElement>, commit: boolean) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== e.pointerId) return;
    dragRef.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) e.currentTarget.releasePointerCapture(e.pointerId);
    propsRef.current.onDrag(null);
    if (commit && drag.moved) propsRef.current.onSeek(drag.time);
    requestDraw();
  }

  return (
    <canvas
      ref={canvasRef}
      className="note-canvas"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={(e) => endDrag(e, true)}
      onPointerCancel={(e) => endDrag(e, false)}
    />
  );
}
