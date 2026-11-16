import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HumanizeMasterRow } from "./HumanizeMasterRow";
import { HumRow } from "./HumRow";

function rowOf(label: string): HTMLElement {
  return screen.getByText(label).closest(".control-row") as HTMLElement;
}

function hoverTip(row: HTMLElement): string | null {
  const tip = row.querySelector(".info-tip") as HTMLElement;
  fireEvent.mouseEnter(tip);
  return screen.getByRole("tooltip").textContent;
}

describe("HumanizeMasterRow", () => {
  function renderRow() {
    return render(
      <HumanizeMasterRow
        humanizeAll={false}
        simulateHands={false}
        chordRoll={false}
        onHumanizeAllChange={vi.fn()}
        onSimulateHandsChange={vi.fn()}
        onChordRollChange={vi.fn()}
        onReset={vi.fn()}
      />,
    );
  }

  it("puts each explanation behind an info icon in the same row as its switch", () => {
    renderRow();
    const expected: [string, string][] = [
      ["Humanize all", "turn all the human touches on or off at once"],
      ["Simulate Hands", "each hand gets its own slightly different timing"],
      ["Chord Roll", "notes in a chord start just a hair apart, like a real hand"],
    ];
    for (const [label, description] of expected) {
      expect(hoverTip(rowOf(label))).toBe(description);
      fireEvent.mouseLeave(rowOf(label).querySelector(".info-tip") as HTMLElement);
    }
  });

  it("shows no explanation text until the icon is used", () => {
    renderRow();
    expect(screen.queryByText(/turn all the human touches/)).toBeNull();
    expect(document.querySelector(".check-pair__desc")).toBeNull();
    expect(document.querySelector(".toggle-switch__description")).toBeNull();
  });

  it("has the icon right after the switch, not inside it", () => {
    renderRow();
    const row = rowOf("Humanize all");
    const toggle = row.querySelector("[role='switch']") as HTMLElement;
    expect(toggle.querySelector(".info-tip")).toBeNull();
    expect(toggle.nextElementSibling?.className).toBe("info-tip");
  });
});

describe("HumRow", () => {
  function renderRow(desc: string) {
    return render(
      <HumRow
        name="Vary Timing"
        desc={desc}
        checked
        onCheckedChange={vi.fn()}
        min={0}
        max={1}
        value={0.5}
        suffix=" s"
        onValueChange={vi.fn()}
      />,
    );
  }

  it("puts its explanation behind an info icon right after the title", () => {
    renderRow("notes start a little early or late");
    const title = screen.getByText("Vary Timing").closest(".hum-row__title") as HTMLElement;
    expect(title.nextElementSibling?.className).toBe("info-tip");
    expect(hoverTip(rowOf("Vary Timing"))).toBe("notes start a little early or late");
    expect(document.querySelector(".hum-row__desc")).toBeNull();
  });

  it("has the switch and the title in the same row, with the slider under the title", () => {
    renderRow("x");
    const row = document.querySelector(".hum-row") as HTMLElement;
    expect([...row.children].map((child) => child.getAttribute("role") ?? child.className.split(" ")[0])).toEqual([
      "switch",
      "control-row",
      "slider-spinbox",
    ]);
    expect(row.querySelector(".hum-row__stack")).toBeNull();
    expect(row.children[1].contains(screen.getByText("Vary Timing"))).toBe(true);
  });

  it("keeps the title out of the switch, which is named for screen readers instead", () => {
    renderRow("x");
    const toggle = screen.getByRole("switch", { name: "Vary Timing" });
    expect(toggle.textContent).toBe("");
    expect(toggle.contains(screen.getByText("Vary Timing"))).toBe(false);
  });

  it("still switches when the title is clicked", () => {
    const onCheckedChange = vi.fn();
    render(
      <HumRow
        name="Vary Timing"
        desc=""
        checked={false}
        onCheckedChange={onCheckedChange}
        min={0}
        max={1}
        value={0.5}
        suffix=" s"
        onValueChange={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByText("Vary Timing"));
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });

  it("has no icon when there is no explanation", () => {
    renderRow("");
    expect(document.querySelector(".info-tip")).toBeNull();
  });
});

describe("HumRow slider follows its switch", () => {
  function renderWith(checked: boolean) {
    return render(
      <HumRow
        name="Vary Timing"
        desc="x"
        checked={checked}
        onCheckedChange={vi.fn()}
        min={0}
        max={1}
        value={0.5}
        suffix=" s"
        onValueChange={vi.fn()}
      />,
    );
  }

  it("disables both the slider and its number box while the switch is off", () => {
    renderWith(false);
    expect((document.querySelector("input[type='range']") as HTMLInputElement).disabled).toBe(true);
    expect((document.querySelector("input[type='number']") as HTMLInputElement).disabled).toBe(true);
  });

  it("enables them while the switch is on", () => {
    renderWith(true);
    expect((document.querySelector("input[type='range']") as HTMLInputElement).disabled).toBe(false);
    expect((document.querySelector("input[type='number']") as HTMLInputElement).disabled).toBe(false);
  });
});
