import type { BackendNote, MeasureBoundary, TempoEvent } from "../lib/tauri";

const LEFT_PATTERN = [48, 52, 55, 48, 53, 57, 48, 55, 59];
const RIGHT_PATTERN = [72, 74, 76, 77, 76, 74, 72, 71, 69, 71, 72, 74];
const LEFT_REPEATS = 6;
const RIGHT_REPEATS = 4;
const LEFT_STEP_SECONDS = 0.5;
const LEFT_NOTE_SECONDS = 0.45;
const RIGHT_STEP_SECONDS = 0.375;
const RIGHT_NOTE_SECONDS = 0.3;
const TAIL_SECONDS = 1;
const PEDAL_PERIOD_SECONDS = 3;
const PEDAL_HOLD_SECONDS = 1.8;
const MEASURE_SECONDS = 2;
const TEMPO_US_PER_BEAT = 500_000;
const VELOCITY = 64;
const NO_TRACK = -1;
const NO_CHANNEL = -1;

export interface PreviewSong {
  notes: BackendNote[];
  totalDuration: number;
  pedalIntervals: [number, number][];
  tempoEvents: TempoEvent[];
  measureBoundaries: MeasureBoundary[];
}

function buildHand(pattern: number[], repeats: number, step: number, length: number, hand: string, firstId: number) {
  const notes: BackendNote[] = [];
  let start = 0;
  for (let repeat = 0; repeat < repeats; repeat++) {
    for (const pitch of pattern) {
      notes.push({
        id: firstId + notes.length,
        pitch,
        velocity: VELOCITY,
        start_time: start,
        duration: length,
        hand,
        original_track_index: NO_TRACK,
        channel: NO_CHANNEL,
      });
      start += step;
    }
  }
  return notes;
}

function buildPreviewSong(): PreviewSong {
  const left = buildHand(LEFT_PATTERN, LEFT_REPEATS, LEFT_STEP_SECONDS, LEFT_NOTE_SECONDS, "left", 0);
  const right = buildHand(RIGHT_PATTERN, RIGHT_REPEATS, RIGHT_STEP_SECONDS, RIGHT_NOTE_SECONDS, "right", left.length);
  const notes = [...left, ...right].sort((a, b) => a.start_time - b.start_time);
  const totalDuration = Math.max(...notes.map((n) => n.start_time + n.duration)) + TAIL_SECONDS;

  const pedalIntervals: [number, number][] = [];
  for (let start = 0; start < totalDuration; start += PEDAL_PERIOD_SECONDS) {
    pedalIntervals.push([start, Math.min(start + PEDAL_HOLD_SECONDS, totalDuration)]);
  }

  const measureBoundaries: MeasureBoundary[] = [];
  for (let start = 0; start < totalDuration; start += MEASURE_SECONDS) {
    measureBoundaries.push([start, Math.min(start + MEASURE_SECONDS, totalDuration)]);
  }

  return { notes, totalDuration, pedalIntervals, tempoEvents: [[0, TEMPO_US_PER_BEAT]], measureBoundaries };
}

export const PREVIEW_SONG = buildPreviewSong();
