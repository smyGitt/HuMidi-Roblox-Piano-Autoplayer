export interface KeyRange {
  minPitch: number;
  maxPitch: number;
}

export const OCTAVE = 12;

const FULL_RANGE: KeyRange = { minPitch: 21, maxPitch: 108 };
const COMPACT_RANGE: KeyRange = { minPitch: 36, maxPitch: 96 };

export function getKeyRange(use88KeyLayout: boolean): KeyRange {
  return use88KeyLayout ? FULL_RANGE : COMPACT_RANGE;
}

export function foldPitch(pitch: number, range: KeyRange): number {
  let folded = pitch;
  while (folded < range.minPitch) folded += OCTAVE;
  while (folded > range.maxPitch) folded -= OCTAVE;
  return folded;
}

const BLACK_KEY_PITCH_CLASSES = new Set([1, 3, 6, 8, 10]);

export function isBlackKey(pitch: number): boolean {
  return BLACK_KEY_PITCH_CLASSES.has(((pitch % OCTAVE) + OCTAVE) % OCTAVE);
}
