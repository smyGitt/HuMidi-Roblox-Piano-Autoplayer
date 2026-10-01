import { isBlackKey, type KeyRange } from "../../lib/keyRange";

export const BLACK_KEY_WIDTH_RATIO = 0.62;

export interface KeyGeometry {
  pitch: number;
  black: boolean;
  left: number;
  width: number;
}

export interface KeyLayout {
  range: KeyRange;
  keys: readonly KeyGeometry[];
}

export function buildKeyLayout(range: KeyRange): KeyLayout {
  let whiteCount = 0;
  for (let pitch = range.minPitch; pitch <= range.maxPitch; pitch++) {
    if (!isBlackKey(pitch)) whiteCount++;
  }
  const whiteWidth = 1 / whiteCount;
  const blackWidth = whiteWidth * BLACK_KEY_WIDTH_RATIO;
  const keys: KeyGeometry[] = [];
  let whiteIndex = 0;
  for (let pitch = range.minPitch; pitch <= range.maxPitch; pitch++) {
    if (isBlackKey(pitch)) {
      keys.push({ pitch, black: true, left: whiteIndex * whiteWidth - blackWidth / 2, width: blackWidth });
    } else {
      keys.push({ pitch, black: false, left: whiteIndex * whiteWidth, width: whiteWidth });
      whiteIndex++;
    }
  }
  return { range, keys };
}

export function keyGeometry(layout: KeyLayout, pitch: number): KeyGeometry | undefined {
  return layout.keys[pitch - layout.range.minPitch];
}
