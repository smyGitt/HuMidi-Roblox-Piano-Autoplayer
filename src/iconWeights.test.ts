import { describe, expect, it } from "vitest";

const ICON_TAG = /<([A-Z]\w*Icon)\b([^>]*?)\/?>/gs;
const FILL_WEIGHT = /weight="fill"/;
const TEST_FILE = /\.test\.tsx$/;

const sources = import.meta.glob<string>("/src/**/*.tsx", { query: "?raw", import: "default", eager: true });

describe("icon weights", () => {
  it("every Phosphor icon in the app is drawn in the fill weight", () => {
    const wrong: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      if (TEST_FILE.test(file)) continue;
      for (const match of text.matchAll(ICON_TAG)) {
        const [, name, attributes] = match;
        if (!FILL_WEIGHT.test(attributes)) wrong.push(`${file}: <${name} ${attributes.trim()} />`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("actually scans the component files", () => {
    const scanned = Object.keys(sources).filter((file) => !TEST_FILE.test(file));
    expect(scanned.length).toBeGreaterThan(20);
    expect(scanned.some((file) => file.endsWith("/components/TransportBar.tsx"))).toBe(true);
  });
});
