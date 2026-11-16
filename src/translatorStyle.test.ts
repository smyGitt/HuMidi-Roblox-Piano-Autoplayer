import { describe, expect, it } from "vitest";
import { ruleBody } from "./test/appCss";

describe("translator page layout", () => {
  it("fills the whole frame, so the text box can fill it", () => {
    const workspace = ruleBody(".translator-tab__workspace");
    expect(workspace).toMatch(/display:\s*flex/);
    expect(workspace).toMatch(/flex-direction:\s*column/);
    expect(workspace).toMatch(/height:\s*100%/);
    expect(workspace).toMatch(/min-height:\s*0/);
  });

  it("makes the text box take all the remaining height and not resize", () => {
    const textarea = ruleBody(".translator-tab__textarea");
    expect(textarea).toMatch(/flex:\s*1/);
    expect(textarea).toMatch(/min-height:\s*0/);
    expect(textarea).toMatch(/resize:\s*none/);
  });

  it("keeps the buttons at their own height at the bottom of the frame", () => {
    expect(ruleBody(".translator-tab__action-bar")).toMatch(/flex:\s*none/);
    expect(ruleBody(".translator-tab__format")).toMatch(/flex:\s*none/);
  });
});

describe("translator bar right side", () => {
  it("pushes the marked button, and so the buttons after it, to the right edge of the bar", () => {
    expect(ruleBody(".translator-tab__end")).toMatch(/margin-left:\s*auto/);
  });
});

describe("translator text box color", () => {
  it("uses the surface color instead of the bright input color", () => {
    const textarea = ruleBody(".translator-tab__textarea");
    expect(textarea).toMatch(/background:\s*var\(--bg_surface\)/);
    expect(textarea).not.toMatch(/--bg_input/);
  });
});
