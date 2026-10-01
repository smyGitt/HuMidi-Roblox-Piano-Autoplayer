import { describe, expect, it } from "vitest";
import type { BackendNote } from "./tauri";
import { getKeyRange } from "./keyRange";
import {
  activePitchesAt,
  buildNoteTimeline,
  firstIntervalIndexEndingAfter,
  forEachNoteInWindow,
  lowerBound,
  pedalActiveAt,
  upperBound,
  type VizNote,
} from "./noteTimeline";

function note(pitch: number, start: number, duration: number, hand = "left"): BackendNote {
  return { id: 0, pitch, velocity: 64, start_time: start, duration, hand, original_track_index: -1, channel: -1 };
}

function collect(timeline: ReturnType<typeof buildNoteTimeline>, from: number, to: number): VizNote[] {
  const seen: VizNote[] = [];
  forEachNoteInWindow(timeline, from, to, (n) => seen.push(n));
  return seen;
}

describe("buildNoteTimeline", () => {
  it("sorts notes by start time and converts duration to an end time", () => {
    const timeline = buildNoteTimeline([note(60, 2, 1), note(62, 0, 0.5)]);
    expect(timeline.notes.map((n) => n.start)).toEqual([0, 2]);
    expect(timeline.notes[1].end).toBe(3);
  });

  it("records the longest note duration for window lookups", () => {
    expect(buildNoteTimeline([note(60, 0, 0.5), note(62, 1, 4)]).maxDuration).toBe(4);
    expect(buildNoteTimeline([]).maxDuration).toBe(0);
  });

  it("maps left and right hands through and anything else to other", () => {
    const timeline = buildNoteTimeline([note(60, 0, 1, "left"), note(61, 1, 1, "right"), note(62, 2, 1, "unknown")]);
    expect(timeline.notes.map((n) => n.hand)).toEqual(["left", "right", "other"]);
  });

  it("folds pitches into the given key range and leaves them raw without one", () => {
    const compact = getKeyRange(false);
    expect(buildNoteTimeline([note(30, 0, 1)], compact).notes[0].pitch).toBe(42);
    expect(buildNoteTimeline([note(30, 0, 1)]).notes[0].pitch).toBe(30);
  });
});

describe("lowerBound / upperBound", () => {
  const values = [1, 2, 2, 3];
  const identity = (v: number) => v;

  it("lowerBound finds the first item not below the target", () => {
    expect(lowerBound(values, 2, identity)).toBe(1);
    expect(lowerBound(values, 0, identity)).toBe(0);
    expect(lowerBound(values, 9, identity)).toBe(4);
  });

  it("upperBound finds the first item above the target", () => {
    expect(upperBound(values, 2, identity)).toBe(3);
    expect(upperBound(values, 0, identity)).toBe(0);
    expect(upperBound(values, 3, identity)).toBe(4);
  });
});

describe("forEachNoteInWindow", () => {
  it("visits only notes overlapping the window", () => {
    const timeline = buildNoteTimeline([note(60, 0, 1), note(62, 5, 1), note(64, 10, 1)]);
    expect(collect(timeline, 4, 7).map((n) => n.pitch)).toEqual([62]);
  });

  it("includes a long note that started before the window and is still sounding", () => {
    const timeline = buildNoteTimeline([note(48, 0, 20), note(60, 5, 1), note(62, 15, 1)]);
    expect(collect(timeline, 14, 16).map((n) => n.pitch)).toEqual([48, 62]);
  });

  it("excludes a note that ends exactly where the window starts", () => {
    const timeline = buildNoteTimeline([note(60, 0, 2), note(62, 2, 1)]);
    expect(collect(timeline, 2, 3).map((n) => n.pitch)).toEqual([62]);
  });
});

describe("activePitchesAt", () => {
  const timeline = buildNoteTimeline([note(60, 0, 1), note(64, 0.5, 1), note(67, 3, 10)]);

  it("returns every pitch sounding at the given time", () => {
    expect([...activePitchesAt(timeline, 0.75)].sort()).toEqual([60, 64]);
  });

  it("treats a note as released at its end time", () => {
    expect([...activePitchesAt(timeline, 1)]).toEqual([64]);
  });

  it("finds a long note held from far earlier", () => {
    expect([...activePitchesAt(timeline, 12)]).toEqual([67]);
  });

  it("is empty before the first note and after the last one", () => {
    expect(activePitchesAt(timeline, -1).size).toBe(0);
    expect(activePitchesAt(timeline, 20).size).toBe(0);
  });
});

describe("pedalActiveAt / firstIntervalIndexEndingAfter", () => {
  const intervals: [number, number][] = [
    [0, 1],
    [2, 4],
    [6, 7],
  ];

  it("is down inside an interval and up between intervals", () => {
    expect(pedalActiveAt(intervals, 0.5)).toBe(true);
    expect(pedalActiveAt(intervals, 1.5)).toBe(false);
    expect(pedalActiveAt(intervals, 2)).toBe(true);
    expect(pedalActiveAt(intervals, 4)).toBe(false);
  });

  it("is up before the first interval and with no intervals at all", () => {
    expect(pedalActiveAt(intervals, -1)).toBe(false);
    expect(pedalActiveAt([], 1)).toBe(false);
  });

  it("finds the interval still held at the time, or the next one to start", () => {
    expect(firstIntervalIndexEndingAfter(intervals, 3)).toBe(1);
    expect(firstIntervalIndexEndingAfter(intervals, 5)).toBe(2);
    expect(firstIntervalIndexEndingAfter(intervals, -1)).toBe(0);
    expect(firstIntervalIndexEndingAfter(intervals, 9)).toBe(3);
  });
});
