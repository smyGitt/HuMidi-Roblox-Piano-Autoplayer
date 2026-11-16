import { describe, expect, it } from "vitest";

const ICON_TAG = /<([A-Z]\w*Icon)\b([^>]*?)\/?>/gs;
const FILL_WEIGHT = /weight="fill"/;
const BOLD_WEIGHT = /weight="bold"/;
const BOLD_ICONS = ["XIcon"];
const TEST_FILE = /\.test\.tsx$/;

const sources = import.meta.glob<string>("/src/**/*.tsx", { query: "?raw", import: "default", eager: true });

describe("icon weights", () => {
  it("every Phosphor icon in the app is drawn in the fill weight", () => {
    const wrong: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      if (TEST_FILE.test(file)) continue;
      for (const match of text.matchAll(ICON_TAG)) {
        const [, name, attributes] = match;
        if (BOLD_ICONS.includes(name) && BOLD_WEIGHT.test(attributes)) continue;
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

describe("the bold exception", () => {
  it("is only the X used for not supported platforms, and it must be bold", () => {
    const xIcons: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      if (TEST_FILE.test(file)) continue;
      for (const match of text.matchAll(ICON_TAG)) {
        if (match[1] === "XIcon") xIcons.push(`${file}: ${BOLD_WEIGHT.test(match[2]) ? "bold" : "not bold"}`);
      }
    }
    expect(xIcons.length).toBeGreaterThan(0);
    expect(xIcons.every((entry) => entry.endsWith(": bold"))).toBe(true);
  });
});
