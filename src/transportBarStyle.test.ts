import { describe, expect, it } from "vitest";
import { appCss as css, ruleBody } from "./test/appCss";

describe("transport bar media controls", () => {
  const controls = () => ruleBody(".transport-bar__controls");

  it("sit in a pill with a darker background than the bar and no outline", () => {
    expect(controls()).toMatch(/border-radius:\s*var\(--radius-pill\)/);
    expect(controls()).toMatch(/background:\s*var\(--bg_primary\)/);
    expect(ruleBody(".transport-bar")).toMatch(/background:\s*var\(--bg_surface\)/);
    expect(controls()).not.toMatch(/border:/);
  });

  it("has only a little horizontal padding, since each round button carries its own padding", () => {
    expect(controls()).toMatch(/padding:\s*0\s+var\(--space-1\)/);
  });

  it("makes each media button a circle with room around its icon, so the highlight is round", () => {
    const button = ruleBody(".transport-bar__controls .btn");
    expect(button).toMatch(/border-radius:\s*50%/);
    expect(button).toMatch(/padding:\s*var\(--space-1\)/);
  });

  it("has a named pill radius token in :root", () => {
    const root = css.slice(css.indexOf(":root {"), css.indexOf("}", css.indexOf(":root {")));
    expect(root).toMatch(/--radius-pill:/);
  });

  it("keeps the pill centered in the bar", () => {
    expect(ruleBody(".transport-bar__btn-row")).toMatch(/grid-template-columns:\s*1fr auto 1fr/);
  });

  it("draws the skip icons smaller, at 24px, with the row centering them vertically", () => {
    expect(ruleBody(".transport-bar__controls .transport-bar__skip")).toMatch(/--icon-size:\s*var\(--space-3\)/);
    expect(controls()).toMatch(/align-items:\s*center/);
  });

  it("draws the control icons at 32px", () => {
    expect(ruleBody(".transport-bar__controls .btn")).toMatch(/--icon-size:\s*var\(--space-4\)/);
  });
});
