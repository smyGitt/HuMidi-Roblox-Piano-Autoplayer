import { thresholdsAt, type ThresholdCurves } from "./thresholdCurve";

export interface PedalConfidence {
  values: readonly number[];
  fps: number;
}

export type PedalInterval = [number, number];

export function confidenceDuration(confidence: PedalConfidence): number {
  return confidence.values.length / confidence.fps;
}

export function pedalIntervalsFromConfidence(
  confidence: PedalConfidence,
  curves: ThresholdCurves,
  endTime: number,
): PedalInterval[] {
  const intervals: PedalInterval[] = [];
  let downAt: number | null = null;
  for (let index = 1; index < confidence.values.length; index++) {
    const value = confidence.values[index];
    if (value === 0) continue;
    const time = index / confidence.fps;
    const thresholds = thresholdsAt(curves, time);
    if (thresholds === null) continue;
    if (downAt === null && value > thresholds.on) {
      downAt = time;
    } else if (downAt !== null && value <= thresholds.off) {
      intervals.push([downAt, time]);
      downAt = null;
    }
  }
  if (downAt !== null) intervals.push([downAt, endTime]);
  return intervals;
}

export function peakPerColumn(values: readonly number[], columns: number): number[] {
  if (columns <= 0 || values.length === 0) return [];
  const peaks = new Array<number>(columns).fill(0);
  for (let column = 0; column < columns; column++) {
    const from = Math.floor((column * values.length) / columns);
    const to = Math.max(from + 1, Math.floor(((column + 1) * values.length) / columns));
    let peak = 0;
    for (let index = from; index < to && index < values.length; index++) {
      if (values[index] > peak) peak = values[index];
    }
    peaks[column] = peak;
  }
  return peaks;
}
