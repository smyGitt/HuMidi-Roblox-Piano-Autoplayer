import { describe, expect, it } from "vitest";
import { appCss, ruleBody } from "./test/appCss";

describe("debug page layout with the right panel", () => {
  it("has no page padding of its own, so the panel reaches the edges like on Playback", () => {
    expect(appCss).toMatch(/\.app-page-content:has\(> \.debug-tab\)[,\s][^{]*\{[^}]*padding:\s*0;/);
  });

  it("is the positioning parent of the panel", () => {
    expect(ruleBody(".debug-tab")).toMatch(/position:\s*relative/);
  });

  it("leaves room on the right for the collapsed panel handle, as TabPage does", () => {
    expect(ruleBody(".debug-tab__main")).toMatch(/margin-right:\s*calc\(var\(--space-5\) \+ var\(--border-width\)\)/);
  });

  it("stacks the panel cards and scrolls them", () => {
    const sidebar = ruleBody(".debug-tab__sidebar");
    expect(sidebar).toMatch(/overflow-y:\s*auto/);
    expect(sidebar).not.toMatch(/width:/);
  });
});

describe("card footer", () => {
  it("lays the footer buttons in a row", () => {
    const footer = ruleBody(".card__footer");
    expect(footer).toMatch(/display:\s*flex/);
    expect(footer).toMatch(/gap:\s*var\(--space-1\)/);
  });

  it("lets the log body take the height left over by the footer", () => {
    const body = ruleBody(".debug-tab__console-card .card__body");
    expect(body).toMatch(/flex:\s*1/);
    expect(body).toMatch(/min-height:\s*0/);
  });
});
