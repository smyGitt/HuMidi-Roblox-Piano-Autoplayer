import { describe, expect, it } from "vitest";
import { confidenceDuration, pedalIntervalsFromConfidence, peakPerColumn, type PedalConfidence } from "./pedalPreview";

const FPS = 10;
const ON = 0.6;
const OFF = 0.3;

function curve(values: number[]): PedalConfidence {
  return { values, fps: FPS };
}

describe("pedalIntervalsFromConfidence", () => {
  it("presses above the On threshold and lifts at or below the Off threshold", () => {
    const values = [0, 0.2, 0.7, 0.8, 0.5, 0.4, 0.3, 0.2, 0.9, 0.5];
    expect(pedalIntervalsFromConfidence(curve(values), ON, OFF, 1)).toEqual([
      [0.2, 0.6],
      [0.8, 1],
    ]);
  });

  it("holds between the thresholds (hysteresis): values between Off and On neither press nor lift", () => {
    const values = [0, 0.5, 0.5, 0.7, 0.5, 0.5, 0.5, 0.2];
    expect(pedalIntervalsFromConfidence(curve(values), ON, OFF, 1)).toEqual([[0.3, 0.7]]);
  });

  it("ignores silent frames (exactly 0) instead of treating them as a lift", () => {
    const values = [0, 0.7, 0, 0, 0.5, 0.2];
    expect(pedalIntervalsFromConfidence(curve(values), ON, OFF, 1)).toEqual([[0.1, 0.5]]);
  });

  it("never presses on the very first frame", () => {
    expect(pedalIntervalsFromConfidence(curve([0.9, 0.5, 0.2]), ON, OFF, 1)).toEqual([]);
  });

  it("closes a pedal still down at the end at the given end time", () => {
    expect(pedalIntervalsFromConfidence(curve([0, 0.7, 0.8]), ON, OFF, 5)).toEqual([[0.1, 5]]);
  });

  it("a lower On threshold presses more often and a higher Off threshold lifts sooner", () => {
    const values = [0, 0.4, 0.5, 0.45, 0.5, 0.35, 0.2];
    expect(pedalIntervalsFromConfidence(curve(values), 0.45, 0.3, 1)).toEqual([[0.2, 0.6]]);
    expect(pedalIntervalsFromConfidence(curve(values), 0.35, 0.3, 1)).toEqual([[0.1, 0.6]]);
    expect(pedalIntervalsFromConfidence(curve(values), 0.35, 0.45, 1)).toEqual([[0.1, 0.3], [0.4, 0.5]]);
  });

  it("returns nothing for an empty curve", () => {
    expect(pedalIntervalsFromConfidence(curve([]), ON, OFF, 1)).toEqual([]);
  });
});

describe("confidenceDuration", () => {
  it("is the number of frames divided by the frame rate", () => {
    expect(confidenceDuration(curve([0, 0, 0, 0, 0]))).toBe(0.5);
  });
});

describe("peakPerColumn", () => {
  it("keeps the highest value of each column so narrow peaks stay visible", () => {
    expect(peakPerColumn([0.1, 0.9, 0.2, 0.3, 0.8, 0.1], 3)).toEqual([0.9, 0.3, 0.8]);
  });

  it("repeats values when there are fewer frames than columns", () => {
    expect(peakPerColumn([0.5, 0.7], 4)).toEqual([0.5, 0.5, 0.7, 0.7]);
  });

  it("returns nothing for no columns or no values", () => {
    expect(peakPerColumn([0.5], 0)).toEqual([]);
    expect(peakPerColumn([], 3)).toEqual([]);
  });
});
