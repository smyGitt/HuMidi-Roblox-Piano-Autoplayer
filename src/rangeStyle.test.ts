import { describe, expect, it } from "vitest";
import { appCss, ruleBody } from "./test/appCss";

describe("slider (range input) style", () => {
  it("is drawn by the stylesheet, with no native outline, border or background", () => {
    const body = ruleBody(".range");
    expect(body).toMatch(/appearance:\s*none/);
    expect(body).toMatch(/outline:\s*none/);
    expect(body).toMatch(/border:\s*none/);
    expect(body).toMatch(/background:\s*transparent/);
  });

  it("has a track with no border, filled with the accent up to --range-fill", () => {
    for (const selector of [".range::-webkit-slider-runnable-track", ".range::-moz-range-track"]) {
      const body = ruleBody(selector);
      expect(body).toMatch(/border:\s*none/);
      expect(body).toMatch(/var\(--accent\)\s+var\(--range-fill/);
      expect(body).toMatch(/var\(--bg_input\)/);
    }
  });

  it("has a round accent thumb with no border", () => {
    for (const selector of [".range::-webkit-slider-thumb", ".range::-moz-range-thumb"]) {
      const body = ruleBody(selector);
      expect(body).toMatch(/background:\s*var\(--accent\)/);
      expect(body).toMatch(/border:\s*none/);
      expect(body).toMatch(/border-radius:\s*50%/);
    }
  });

  it("has no raw pixel values in the slider rules", () => {
    const rules = [
      ".range",
      ".range::-webkit-slider-runnable-track",
      ".range::-moz-range-track",
      ".range::-webkit-slider-thumb",
      ".range::-moz-range-thumb",
    ].map(ruleBody);
    for (const body of rules) expect(body).not.toMatch(/\d+px/);
  });
});

describe("fields inside cards", () => {
  it("have no visible outline, only their darker fill", () => {
    const body = ruleBody(".card .field");
    expect(body).toMatch(/border-color:\s*transparent/);
    expect(body).toMatch(/outline:\s*none/);
  });

  it("keep their input fill so they still read as fields", () => {
    expect(ruleBody(".field")).toMatch(/background:\s*var\(--bg_input\)/);
  });
});


describe("toggle switch row", () => {
  it("centers the track vertically against its label", () => {
    expect(ruleBody(".toggle-switch")).toMatch(/align-items:\s*center/);
  });
});

describe("toggle switch shape", () => {
  it("is a pill track with a round knob", () => {
    expect(ruleBody(".toggle-switch__track")).toMatch(/border-radius:\s*var\(--radius-pill\)/);
    expect(ruleBody(".toggle-switch__knob")).toMatch(/border-radius:\s*50%/);
  });

  it("gives the knob a shadow from a named token", () => {
    expect(ruleBody(".toggle-switch__knob")).toMatch(/box-shadow:\s*var\(--knob-shadow\)/);
    const root = appCss.slice(appCss.indexOf(":root {"), appCss.indexOf("}", appCss.indexOf(":root {")));
    expect(root).toMatch(/--knob-shadow:\s*[^;]+;/);
  });

  it("has a knob as tall as the track, so the circle fits the pill's end", () => {
    expect(ruleBody(".toggle-switch__track")).toMatch(/height:\s*var\(--space-2\)/);
    expect(ruleBody(".toggle-switch__knob")).toMatch(/height:\s*var\(--space-2\)/);
    expect(ruleBody(".toggle-switch__knob")).toMatch(/width:\s*var\(--space-2\)/);
  });
});

describe("info tip", () => {
  it("floats the message under the icon, above the cards, readable on its own panel", () => {
    expect(ruleBody(".info-tip")).toMatch(/position:\s*relative/);
    const text = ruleBody(".info-tip__text");
    expect(text).toMatch(/position:\s*absolute/);
    expect(text).toMatch(/z-index:\s*var\(--z-raised\)/);
    expect(text).toMatch(/background:\s*var\(--bg_primary\)/);
    expect(text).toMatch(/max-width:\s*var\(--space-30\)/);
    expect(text).toMatch(/white-space:\s*normal/);
  });
});

describe("control row label", () => {
  it("puts an info icon next to the label text", () => {
    const label = ruleBody(".control-row__label");
    expect(label).toMatch(/display:\s*flex/);
    expect(label).toMatch(/align-items:\s*center/);
    expect(label).toMatch(/gap:\s*var\(--space-1\)/);
  });
});

describe("info tip highlight", () => {
  it("is only the icon: no background box on hover, no padding around it", () => {
    expect(ruleBody(".info-tip .btn")).toMatch(/padding:\s*0/);
    expect(ruleBody(".info-tip .btn--icon:hover:not(:disabled)")).toMatch(/background:\s*transparent/);
  });

  it("lights the icon itself while the message is open", () => {
    expect(ruleBody('.info-tip .btn[aria-expanded="true"]')).toMatch(/color:\s*var\(--text_primary\)/);
  });
});

describe("card side column", () => {
  it("makes a card with a side column a row with no padding of its own, so the side column reaches the edges", () => {
    const card = ruleBody(".card--with-side");
    expect(card).toMatch(/flex-direction:\s*row/);
    expect(card).toMatch(/padding:\s*0/);
    expect(card).toMatch(/gap:\s*0/);
  });

  it("keeps the card's own padding and stacking on the main part", () => {
    const main = ruleBody(".card__main");
    expect(main).toMatch(/display:\s*flex/);
    expect(main).toMatch(/flex-direction:\s*column/);
    expect(main).toMatch(/flex:\s*1/);
    expect(main).toMatch(/min-width:\s*0/);
    expect(main).toMatch(/padding:\s*var\(--space-1\)\s+var\(--space-2\)/);
  });

  it("is a full-height column with a slightly darker background, the values stacked", () => {
    const side = ruleBody(".card__side");
    expect(side).toMatch(/display:\s*flex/);
    expect(side).toMatch(/flex-direction:\s*column/);
    expect(side).toMatch(/justify-content:\s*center/);
    expect(side).toMatch(/background:\s*var\(--bg_primary\)/);
    expect(side).toMatch(/padding:\s*var\(--space-1\)\s+var\(--space-2\)/);
  });

  it("lets the tempo slider take the whole width of the body, vertically centered in it", () => {
    const body = ruleBody(".tempo-card .card__body");
    expect(body).toMatch(/justify-content:\s*center/);
    expect(appCss).not.toMatch(/\.tempo-card \.slider-spinbox/);
  });

  it("has no tempo-specific side or footer rules any more", () => {
    expect(appCss).not.toMatch(/\.tempo-card__side/);
    expect(appCss).not.toMatch(/\.tempo-card__main/);
    expect(appCss).not.toMatch(/\.tempo-card \.card__footer/);
  });
});

describe("tempo BPM boxes", () => {
  it("make the read-only box line up with the input: same flex centering, normal line height", () => {
    const value = ruleBody(".tempo-card__value");
    expect(value).toMatch(/display:\s*flex/);
    expect(value).toMatch(/align-items:\s*center/);
    expect(value).toMatch(/line-height:\s*normal/);
  });
});

describe("humanize cards", () => {
  it("are as tall as their content, with no minimum height", () => {
    expect(appCss).not.toMatch(/humanize-card-min-height/);
    expect(appCss).not.toMatch(/\.playback-tab__humanize-page \.card/);
  });
});

describe("disabled sliders", () => {
  it("look dimmed and show the not-allowed cursor, like the disabled buttons", () => {
    const body = ruleBody(".range:disabled");
    expect(body).toMatch(/opacity:\s*var\(--opacity-disabled\)/);
    expect(body).toMatch(/cursor:\s*not-allowed/);
  });

  it("dim their number box too", () => {
    const body = ruleBody(".field:disabled");
    expect(body).toMatch(/opacity:\s*var\(--opacity-disabled\)/);
    expect(body).toMatch(/cursor:\s*not-allowed/);
  });
});

describe("humanize slider row layout", () => {
  it("is a two column grid: the switch inline with the title in the first row, centered against it", () => {
    const row = ruleBody(".hum-row");
    expect(row).toMatch(/display:\s*grid/);
    expect(row).toMatch(/grid-template-columns:\s*auto\s+1fr/);
    expect(row).toMatch(/align-items:\s*center/);
  });

  it("puts the slider under the title, in the second column, starting where the title starts", () => {
    const slider = ruleBody(".hum-row > .slider-spinbox");
    expect(slider).toMatch(/grid-column:\s*2/);
    expect(appCss).not.toMatch(/\.hum-row__stack/);
  });

  it("indents a row without a switch by the switch width plus the gap, so it lines up with the titles", () => {
    expect(ruleBody(".hum-row__indent")).toMatch(/margin-left:\s*var\(--toggle-indent\)/);
    const root = appCss.slice(appCss.indexOf(":root {"), appCss.indexOf("}", appCss.indexOf(":root {")));
    expect(root).toMatch(/--toggle-indent:\s*calc\(var\(--space-4\)\s*\+\s*var\(--space-1\)\)/);
    expect(ruleBody(".toggle-switch__track")).toMatch(/width:\s*var\(--space-4\)/);
    expect(ruleBody(".hum-row")).toMatch(/gap:\s*var\(--space-1\)/);
  });

  it("makes the title look clickable like the other switch labels", () => {
    expect(ruleBody(".hum-row__title")).toMatch(/cursor:\s*pointer/);
  });
});
