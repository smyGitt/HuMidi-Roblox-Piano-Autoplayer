import { describe, expect, it } from "vitest";
import { getKeyRange } from "../../lib/keyRange";
import { BLACK_KEY_WIDTH_RATIO, buildKeyLayout, keyGeometry } from "./keyLayout";

describe("buildKeyLayout", () => {
  it("has one entry per pitch, 88 for the full layout and 61 for the compact one", () => {
    expect(buildKeyLayout(getKeyRange(true)).keys).toHaveLength(88);
    expect(buildKeyLayout(getKeyRange(false)).keys).toHaveLength(61);
  });

  it("splits the full width evenly across the white keys, 52 on an 88-key piano and 36 on 61 keys", () => {
    const full = buildKeyLayout(getKeyRange(true)).keys.filter((k) => !k.black);
    expect(full).toHaveLength(52);
    expect(full[0].left).toBe(0);
    expect(full[full.length - 1].left + full[full.length - 1].width).toBeCloseTo(1);
    const compact = buildKeyLayout(getKeyRange(false)).keys.filter((k) => !k.black);
    expect(compact).toHaveLength(36);
  });

  it("places white keys edge to edge with no gaps or overlaps", () => {
    const whites = buildKeyLayout(getKeyRange(true)).keys.filter((k) => !k.black);
    for (let i = 1; i < whites.length; i++) {
      expect(whites[i].left).toBeCloseTo(whites[i - 1].left + whites[i - 1].width);
    }
  });

  it("centres each black key on the boundary between its neighbouring white keys", () => {
    const layout = buildKeyLayout(getKeyRange(false));
    const cSharp = keyGeometry(layout, 37)!;
    const d = keyGeometry(layout, 38)!;
    expect(cSharp.black).toBe(true);
    expect(cSharp.left + cSharp.width / 2).toBeCloseTo(d.left);
    expect(cSharp.width).toBeCloseTo(d.width * BLACK_KEY_WIDTH_RATIO);
  });

  it("keyGeometry finds a key by pitch and returns undefined outside the range", () => {
    const layout = buildKeyLayout(getKeyRange(false));
    expect(keyGeometry(layout, 60)?.pitch).toBe(60);
    expect(keyGeometry(layout, 35)).toBeUndefined();
    expect(keyGeometry(layout, 97)).toBeUndefined();
  });
});
