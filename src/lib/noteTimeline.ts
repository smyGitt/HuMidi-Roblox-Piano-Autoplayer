import type { BackendNote } from "./tauri";
import { foldPitch, type KeyRange } from "./keyRange";

export type VizHand = "left" | "right" | "other";

export const VIZ_HANDS: readonly VizHand[] = ["left", "right", "other"];

export interface VizNote {
  pitch: number;
  start: number;
  end: number;
  hand: VizHand;
}

export interface NoteTimeline {
  notes: readonly VizNote[];
  maxDuration: number;
}

export type TimeInterval = readonly [number, number];

function toHand(hand: string): VizHand {
  return hand === "left" || hand === "right" ? hand : "other";
}

export function buildNoteTimeline(notes: readonly BackendNote[], range?: KeyRange): NoteTimeline {
  const viz: VizNote[] = notes.map((n) => ({
    pitch: range ? foldPitch(n.pitch, range) : n.pitch,
    start: n.start_time,
    end: n.start_time + n.duration,
    hand: toHand(n.hand),
  }));
  viz.sort((a, b) => a.start - b.start);
  let maxDuration = 0;
  for (const n of viz) maxDuration = Math.max(maxDuration, n.end - n.start);
  return { notes: viz, maxDuration };
}

export function lowerBound<T>(items: readonly T[], target: number, key: (item: T) => number): number {
  let lo = 0;
  let hi = items.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (key(items[mid]) < target) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

export function upperBound<T>(items: readonly T[], target: number, key: (item: T) => number): number {
  let lo = 0;
  let hi = items.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (key(items[mid]) <= target) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

const noteStart = (note: VizNote) => note.start;
const intervalStart = (interval: TimeInterval) => interval[0];

export function forEachNoteInWindow(
  timeline: NoteTimeline,
  from: number,
  to: number,
  visit: (note: VizNote) => void,
): void {
  const { notes } = timeline;
  for (let i = lowerBound(notes, from - timeline.maxDuration, noteStart); i < notes.length; i++) {
    const note = notes[i];
    if (note.start > to) break;
    if (note.end > from) visit(note);
  }
}

export function activePitchesAt(timeline: NoteTimeline, time: number): Set<number> {
  const active = new Set<number>();
  const { notes } = timeline;
  for (let i = lowerBound(notes, time - timeline.maxDuration, noteStart); i < notes.length; i++) {
    const note = notes[i];
    if (note.start > time) break;
    if (note.end > time) active.add(note.pitch);
  }
  return active;
}

export function firstIntervalIndexEndingAfter(intervals: readonly TimeInterval[], time: number): number {
  const index = upperBound(intervals, time, intervalStart) - 1;
  return index >= 0 && intervals[index][1] > time ? index : index + 1;
}

export function pedalActiveAt(intervals: readonly TimeInterval[], time: number): boolean {
  const index = upperBound(intervals, time, intervalStart) - 1;
  return index >= 0 && time < intervals[index][1];
}
