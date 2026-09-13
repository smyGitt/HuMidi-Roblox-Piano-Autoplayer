export interface KeyRange {
  minPitch: number;
  maxPitch: number;
}

export function getKeyRange(use88KeyLayout: boolean): KeyRange {
  return use88KeyLayout ? { minPitch: 21, maxPitch: 108 } : { minPitch: 36, maxPitch: 96 };
}

const BLACK_KEY_PITCH_CLASSES = new Set([1, 3, 6, 8, 10]);

export function isBlackKey(pitch: number): boolean {
  return BLACK_KEY_PITCH_CLASSES.has(((pitch % 12) + 12) % 12);
}
