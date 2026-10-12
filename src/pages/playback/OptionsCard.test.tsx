import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { OptionsCard } from "./OptionsCard";
import { DEFAULT_CONFIG } from "./types";

const SPACEBAR_LABEL = "Release Spacebar at Start";

function renderCard(overrides: Partial<typeof DEFAULT_CONFIG> = {}) {
  const onChange = vi.fn();
  render(<OptionsCard config={{ ...DEFAULT_CONFIG, ...overrides }} onChange={onChange} onReset={vi.fn()} />);
  return onChange;
}

describe("OptionsCard release spacebar toggle", () => {
  it("is off with the default config", () => {
    renderCard();
    expect(screen.getByRole("switch", { name: SPACEBAR_LABEL })).toHaveAttribute("aria-checked", "false");
  });

  it("shows on when the setting is on", () => {
    renderCard({ release_pedal_on_start: true });
    expect(screen.getByRole("switch", { name: SPACEBAR_LABEL })).toHaveAttribute("aria-checked", "true");
  });

  it("turns the setting on when clicked and changes nothing else", () => {
    const onChange = renderCard();
    fireEvent.click(screen.getByRole("switch", { name: SPACEBAR_LABEL }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith({ release_pedal_on_start: true });
  });
});
