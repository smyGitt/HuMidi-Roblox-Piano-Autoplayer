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

  it("glows on hover: a round gradient from the text color fading to transparent, no solid fill", () => {
    const hover = ruleBody(".transport-bar__controls .btn--icon:hover:not(:disabled)");
    expect(hover).toMatch(
      /background:\s*radial-gradient\(circle,\s*color-mix\(in srgb,\s*var\(--text_primary\)\s+var\(--glow-strength\),\s*transparent\),\s*transparent\s+var\(--glow-reach\)\)/,
    );
    expect(hover).not.toMatch(/\d+px/);
  });

  it("has the glow strength and reach as named tokens in :root", () => {
    const root = css.slice(css.indexOf(":root {"), css.indexOf("}", css.indexOf(":root {")));
    expect(root).toMatch(/--glow-strength:\s*\d+%/);
    expect(root).toMatch(/--glow-reach:\s*\d+%/);
  });

  it("stacks the saving dialog spinner above its centered, muted note", () => {
    const dialog = ruleBody(".saving-dialog");
    expect(dialog).toMatch(/flex-direction:\s*column/);
    expect(dialog).toMatch(/gap:\s*var\(--space-2\)/);
    const note = ruleBody(".saving-dialog__note");
    expect(note).toMatch(/text-align:\s*center/);
    expect(note).toMatch(/color:\s*var\(--text_muted\)/);
  });

  it("draws the control icons at 32px", () => {
    expect(ruleBody(".transport-bar__controls .btn")).toMatch(/--icon-size:\s*var\(--space-4\)/);
  });
});

describe("transport bar image", () => {
  it("is a background layer: out of the layout, behind the controls, never catching the mouse", () => {
    const image = ruleBody(".transport-bar__image");
    expect(image).toMatch(/position:\s*absolute/);
    expect(image).toMatch(/z-index:\s*var\(--z-behind\)/);
    expect(image).toMatch(/pointer-events:\s*none/);
  });

  it("is as tall as the base height times its zoom, centered vertically then moved by its x and y offsets, towards the right", () => {
    const image = ruleBody(".transport-bar__image");
    expect(image).toMatch(/height:\s*calc\(var\(--space-5\)\s*\*\s*var\(--image-zoom\)\)/);
    expect(image).toMatch(/top:\s*50%/);
    expect(image).toMatch(/transform:\s*translate\(var\(--image-x\),\s*calc\(-50%\s*\+\s*var\(--image-y\)\)\)/);
    expect(image).toMatch(/right:\s*var\(--space-6\)/);
    expect(image).toMatch(/object-fit:\s*contain/);
    expect(image).toMatch(/max-width:\s*100%/);
  });

  it("is clipped by the bar, which makes its own stacking layer, so it never grows the bar or covers the page", () => {
    const bar = ruleBody(".transport-bar");
    expect(bar).toMatch(/position:\s*relative/);
    expect(bar).toMatch(/z-index:\s*0/);
    expect(bar).toMatch(/overflow:\s*hidden/);
  });

  it("has a named layer token below the page content", () => {
    const root = css.slice(css.indexOf(":root {"), css.indexOf("}", css.indexOf(":root {")));
    expect(root).toMatch(/--z-behind:\s*-1/);
  });
});
