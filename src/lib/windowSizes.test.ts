import { describe, expect, it } from "vitest";
import { FULL_SIZE_KEY, MINI_SIZE_KEY, readSavedSize, sameSize } from "./windowSizes";

const WIDTH = 800;
const HEIGHT = 600;
const SUBPIXEL = 0.4;

describe("readSavedSize", () => {
  it("reads a saved width and height", () => {
    expect(readSavedSize({ [FULL_SIZE_KEY]: { width: WIDTH, height: HEIGHT } }, FULL_SIZE_KEY)).toEqual({
      width: WIDTH,
      height: HEIGHT,
    });
  });

  it("keeps the two modes under separate keys", () => {
    const config = { [FULL_SIZE_KEY]: { width: WIDTH, height: HEIGHT }, [MINI_SIZE_KEY]: { width: HEIGHT, height: WIDTH } };
    expect(readSavedSize(config, MINI_SIZE_KEY)).toEqual({ width: HEIGHT, height: WIDTH });
  });

  it("returns null for a missing, malformed, zero or negative size", () => {
    expect(readSavedSize({}, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: "wide" }, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: null }, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: { width: WIDTH } }, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: { width: 0, height: HEIGHT } }, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: { width: -WIDTH, height: HEIGHT } }, FULL_SIZE_KEY)).toBeNull();
    expect(readSavedSize({ [FULL_SIZE_KEY]: { width: "800", height: HEIGHT } }, FULL_SIZE_KEY)).toBeNull();
  });
});

describe("sameSize", () => {
  it("compares whole pixels", () => {
    expect(sameSize({ width: WIDTH, height: HEIGHT }, { width: WIDTH + SUBPIXEL, height: HEIGHT })).toBe(true);
    expect(sameSize({ width: WIDTH, height: HEIGHT }, { width: WIDTH + 1, height: HEIGHT })).toBe(false);
  });

  it("is false when either side is missing", () => {
    expect(sameSize(null, { width: WIDTH, height: HEIGHT })).toBe(false);
    expect(sameSize({ width: WIDTH, height: HEIGHT }, null)).toBe(false);
  });
});
