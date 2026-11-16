import { describe, expect, it } from "vitest";
import { appCss, ruleBody } from "./test/appCss";

describe("text selection", () => {
  it("is off for the whole app", () => {
    const body = ruleBody("body");
    expect(body).toMatch(/user-select:\s*none/);
    expect(body).toMatch(/-webkit-user-select:\s*none/);
  });

  it("is on again only for the text fields and the log, which people select and copy", () => {
    const match = appCss.match(/\n([^{}\n]*(?:,\n[^{}\n]*)*)\{[^}]*user-select:\s*text;[^}]*\}/);
    expect(match).not.toBeNull();
    const selectors = (match as RegExpMatchArray)[1].split(",").map((selector) => selector.trim());
    expect(selectors).toEqual(["input", "textarea", ".debug-tab__console"]);
    expect((match as RegExpMatchArray)[0]).toMatch(/-webkit-user-select:\s*text/);
  });

  it("turns selection on nowhere else", () => {
    const turnedOn = appCss.match(/(?<!-webkit-)user-select:\s*(?:text|auto|all)/g) ?? [];
    expect(turnedOn).toHaveLength(1);
  });
});
