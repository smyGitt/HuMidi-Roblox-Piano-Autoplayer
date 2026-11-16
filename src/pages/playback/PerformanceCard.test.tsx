import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DEFAULT_CONFIG } from "./types";
import { PerformanceCard } from "./PerformanceCard";

const NOTE = "PedalAI or Adaptive is recommended, or pick None or no pedal event generation";

function renderCard() {
  return render(
    <PerformanceCard config={DEFAULT_CONFIG} onChange={vi.fn()} onReset={vi.fn()} midiPedalAvailable={false} />,
  );
}

describe("PerformanceCard pedal style note", () => {
  it("is hidden behind an info icon in the Pedal Style row", () => {
    renderCard();
    expect(screen.queryByText(NOTE)).toBeNull();
    const row = screen.getByText("Pedal Style").closest(".control-row") as HTMLElement;
    expect(row.querySelector(".info-tip")).not.toBeNull();
    expect(document.querySelector(".performance-card__pedal-note")).toBeNull();
  });

  it("shows the recommendation when the info icon is hovered", () => {
    renderCard();
    const tip = document.querySelector(".info-tip") as HTMLElement;
    fireEvent.mouseEnter(tip);
    expect(screen.getByRole("tooltip").textContent).toBe(NOTE);
  });

  it("shows the recommendation when the info icon is clicked", () => {
    renderCard();
    fireEvent.click(screen.getByRole("button", { name: "More information" }));
    expect(screen.getByText(NOTE)).toBeTruthy();
  });

  it("no longer shows the old wording", () => {
    renderCard();
    fireEvent.click(screen.getByRole("button", { name: "More information" }));
    expect(screen.queryByText(/Prefer PedalAI/)).toBeNull();
  });
});
