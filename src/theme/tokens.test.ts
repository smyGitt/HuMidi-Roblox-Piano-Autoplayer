import { describe, expect, it } from "vitest";
import { clampImageZoom, THEMES } from "./tokens";

const HEX = /^#[0-9a-f]{6}$/i;

describe("built-in themes", () => {
  it("have unique names", () => {
    const names = THEMES.map((theme) => theme.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("give every token a six digit hex colour, in the same set of tokens as Midnight", () => {
    const keys = Object.keys(THEMES[0].tokens).sort();
    for (const theme of THEMES) {
      expect(Object.keys(theme.tokens).sort()).toEqual(keys);
      for (const value of Object.values(theme.tokens)) expect(value).toMatch(HEX);
    }
  });
});

describe("Hatsune Miku theme", () => {
  const miku = () => THEMES.find((theme) => theme.name === "Hatsune Miku")!.tokens;

  it("uses the coolors palette: cyan accent, white text, grey muted text, black background", () => {
    const tokens = miku();
    expect(tokens.accent).toBe("#00d0ea");
    expect(tokens.pedal_color).toBe("#009db2");
    expect(tokens.text_primary).toBe("#ffffff");
    expect(tokens.text_muted).toBe("#92898b");
    expect(tokens.bg_primary).toBe("#000000");
  });

  it("uses the palette red for stop and the progress line and the palette yellow for the loaded marker", () => {
    const tokens = miku();
    expect(tokens.accent_stop).toBe("#ff0003");
    expect(tokens.viz_progress_line).toBe("#ff0003");
    expect(tokens.accent_loaded).toBe("#ffd800");
  });

  it("keeps the surfaces dark greys tinted from the palette grey", () => {
    const tokens = miku();
    expect(tokens.bg_surface).toBe("#1f1d1e");
    expect(tokens.bg_input).toBe("#121112");
    expect(tokens.border).toBe("#3a3637");
  });
});

describe("Kasane Teto theme", () => {
  const teto = () => THEMES.find((theme) => theme.name === "Kasane Teto");

  it("is a built-in theme", () => {
    expect(teto()).toBeDefined();
  });

  it("uses her red #d4203c only as the accent, on neutral dark grey with white text", () => {
    const tokens = teto()!.tokens;
    expect(tokens.accent).toBe("#d4203c");
    expect(tokens.bg_primary).toBe("#111111");
    expect(tokens.bg_surface).toBe("#2a2a2a");
    expect(tokens.bg_input).toBe("#1a1a1a");
    expect(tokens.text_primary).toBe("#f0f0f0");
  });

  it("keeps the stop colour apart from the red accent", () => {
    const tokens = teto()!.tokens;
    expect(tokens.accent_stop).not.toBe(tokens.accent);
    expect(tokens.accent_stop).not.toBe(tokens.accent_play);
  });
});

describe("clampImageZoom", () => {
  it("rounds to one decimal, never goes below 0.1 and has no upper limit", () => {
    expect(clampImageZoom(6.74)).toBe(6.7);
    expect(clampImageZoom(6.76)).toBe(6.8);
    expect(clampImageZoom(0)).toBe(0.1);
    expect(clampImageZoom(-3)).toBe(0.1);
    expect(clampImageZoom(0.04)).toBe(0.1);
    expect(clampImageZoom(1234.56)).toBe(1234.6);
    expect(clampImageZoom(1)).toBe(1);
  });
});
