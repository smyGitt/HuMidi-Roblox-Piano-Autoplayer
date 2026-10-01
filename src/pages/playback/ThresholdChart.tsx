import { useCallback, useEffect, useMemo, useRef } from "react";
import {
  confidenceDuration,
  peakPerColumn,
  pedalIntervalsFromConfidence,
  type PedalConfidence,
} from "../../lib/pedalPreview";
import { useTheme } from "../../theme/ThemeProvider";
import { readThresholdChartStyle } from "./thresholdChartStyle";

const MIN_CONFIDENCE = 0;
const MAX_CONFIDENCE = 1;

interface ThresholdChartProps {
  confidence: PedalConfidence;
  thresholdOn: number;
  thresholdOff: number;
}

function clampConfidence(value: number): number {
  return Math.min(MAX_CONFIDENCE, Math.max(MIN_CONFIDENCE, value));
}

export function ThresholdChart({ confidence, thresholdOn, thresholdOff }: ThresholdChartProps) {
  const { theme } = useTheme();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const duration = confidenceDuration(confidence);
  const intervals = useMemo(
    () => pedalIntervalsFromConfidence(confidence, thresholdOn, thresholdOff, duration),
    [confidence, thresholdOn, thresholdOff, duration],
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
    const yOf = (value: number) => height - clampConfidence(value) * height;

    ctx.globalAlpha = style.shadeOpacity;
    ctx.fillStyle = style.shadeColor;
    for (const [start, end] of intervals) {
      const left = (start / duration) * width;
      ctx.fillRect(left, 0, Math.max(style.lineWidth, ((end - start) / duration) * width), height);
    }
    ctx.globalAlpha = 1;

    const peaks = peakPerColumn(confidence.values, Math.max(1, Math.floor(width)));
    ctx.strokeStyle = style.curveColor;
    ctx.lineWidth = style.lineWidth;
    ctx.beginPath();
    peaks.forEach((peak, column) => {
      if (column === 0) ctx.moveTo(column, yOf(peak));
      else ctx.lineTo(column, yOf(peak));
    });
    ctx.stroke();

    ctx.fillStyle = style.onColor;
    ctx.fillRect(0, yOf(thresholdOn) - style.lineWidth / 2, width, style.lineWidth);
    ctx.fillStyle = style.offColor;
    ctx.fillRect(0, yOf(thresholdOff) - style.lineWidth / 2, width, style.lineWidth);
  }, [confidence, duration, intervals, thresholdOn, thresholdOff]);

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

  return <canvas ref={canvasRef} className="pedal-ai-card__chart" />;
}
