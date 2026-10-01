import { describe, expect, it } from "vitest";
import { foldPitch, getKeyRange, isBlackKey } from "./keyRange";

describe("foldPitch", () => {
  const compact = getKeyRange(false);
  const full = getKeyRange(true);

  it("leaves a pitch already inside the range untouched", () => {
    expect(foldPitch(36, compact)).toBe(36);
    expect(foldPitch(60, compact)).toBe(60);
    expect(foldPitch(96, compact)).toBe(96);
  });

  it("moves a pitch below the range up by whole octaves until it fits, like KeyMapper::get_key_data", () => {
    expect(foldPitch(30, compact)).toBe(42);
    expect(foldPitch(21, compact)).toBe(45);
    expect(foldPitch(0, compact)).toBe(36);
  });

  it("moves a pitch above the range down by whole octaves until it fits", () => {
    expect(foldPitch(100, compact)).toBe(88);
    expect(foldPitch(108, compact)).toBe(96);
    expect(foldPitch(127, full)).toBe(103);
  });

  it("keeps the pitch class, so a folded black key stays a black key", () => {
    expect(isBlackKey(foldPitch(25, compact))).toBe(true);
    expect(isBlackKey(foldPitch(105, compact))).toBe(false);
  });
});

describe("getKeyRange", () => {
  it("returns the full 88-key range (A0-C8) when use88KeyLayout is true", () => {
    expect(getKeyRange(true)).toEqual({ minPitch: 21, maxPitch: 108 });
  });

  it("returns the compressed 61-key range (C2-C7) when use88KeyLayout is false", () => {
    expect(getKeyRange(false)).toEqual({ minPitch: 36, maxPitch: 96 });
  });

  it("matches the Rust KeyMapper::new pitch counts exactly (88 and 61 keys)", () => {
    const full = getKeyRange(true);
    expect(full.maxPitch - full.minPitch + 1).toBe(88);
    const compressed = getKeyRange(false);
    expect(compressed.maxPitch - compressed.minPitch + 1).toBe(61);
  });
});

describe("isBlackKey", () => {
  it("classifies the 5 black-key pitch classes correctly within an octave", () => {
    const blackClasses = [1, 3, 6, 8, 10];
    const whiteClasses = [0, 2, 4, 5, 7, 9, 11];
    for (const pc of blackClasses) expect(isBlackKey(60 + pc)).toBe(true);
    for (const pc of whiteClasses) expect(isBlackKey(60 + pc)).toBe(false);
  });

  it("is consistent across octaves (pitch-class only, not absolute pitch)", () => {
    expect(isBlackKey(61)).toBe(true);
    expect(isBlackKey(61 + 12)).toBe(true);
    expect(isBlackKey(61 - 12)).toBe(true);
  });

  it("handles pitch 0 and low pitches without going negative in the modulo", () => {
    expect(isBlackKey(0)).toBe(false);
    expect(isBlackKey(1)).toBe(true);
  });
});
