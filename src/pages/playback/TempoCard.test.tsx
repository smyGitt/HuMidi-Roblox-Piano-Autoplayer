import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TempoCard } from "./TempoCard";

function renderCard(props: Partial<Parameters<typeof TempoCard>[0]> = {}) {
  const onTempoChange = vi.fn();
  render(<TempoCard tempo={100} originalBpm={120} onTempoChange={onTempoChange} {...props} />);
  return onTempoChange;
}

function bpmInput(): HTMLInputElement {
  return screen.getByLabelText("Resulting BPM") as HTMLInputElement;
}

describe("TempoCard slider", () => {
  it("runs from 10% up to 500%", () => {
    renderCard();
    const slider = document.querySelector("input[type='range']") as HTMLInputElement;
    expect(slider.min).toBe("10");
    expect(slider.max).toBe("500");
    const spinbox = document.querySelector(".slider-spinbox__spinbox") as HTMLInputElement;
    expect(spinbox.max).toBe("500");
  });

  it("clamps a typed tempo above 500% to 500%", () => {
    const onTempoChange = renderCard();
    const spinbox = document.querySelector(".slider-spinbox__spinbox") as HTMLInputElement;
    fireEvent.change(spinbox, { target: { value: "900" } });
    expect(onTempoChange).toHaveBeenCalledWith(500);
  });
});

describe("TempoCard resulting BPM", () => {
  it("turns a typed BPM into a tempo percent", () => {
    const onTempoChange = renderCard();
    fireEvent.change(bpmInput(), { target: { value: "240" } });
    expect(onTempoChange).toHaveBeenCalledWith(200);
  });

  it("caps the tempo at 500% however high the typed BPM is", () => {
    const onTempoChange = renderCard();
    fireEvent.change(bpmInput(), { target: { value: "1000" } });
    expect(onTempoChange).toHaveBeenCalledWith(500);
  });

  it("keeps the tempo at least 10%", () => {
    const onTempoChange = renderCard();
    fireEvent.change(bpmInput(), { target: { value: "1" } });
    expect(onTempoChange).toHaveBeenCalledWith(10);
  });

  it("ignores zero, negative and when no song is loaded", () => {
    const onTempoChange = renderCard();
    fireEvent.change(bpmInput(), { target: { value: "0" } });
    fireEvent.change(bpmInput(), { target: { value: "-5" } });
    expect(onTempoChange).not.toHaveBeenCalled();
  });

  it("is disabled and shows dashes without a song", () => {
    renderCard({ originalBpm: 0 });
    expect(bpmInput().disabled).toBe(true);
    expect(screen.getByText("--")).toBeTruthy();
  });

  it("shows the resulting BPM of the current tempo", () => {
    renderCard({ tempo: 150 });
    expect(bpmInput().value).toBe("180");
  });
});

describe("TempoCard side column", () => {
  it("is the card's full-height side column, with the previous BPM on top and the resulting BPM below it", () => {
    renderCard();
    const card = document.querySelector(".tempo-card") as HTMLElement;
    const side = card.querySelector(":scope > .card__side") as HTMLElement;
    expect(side).not.toBeNull();
    const stats = [...side.children] as HTMLElement[];
    expect(stats).toHaveLength(2);
    expect(stats[0].textContent).toContain("Previous BPM");
    expect(stats[0].textContent).toContain("120");
    expect(stats[1].textContent).toContain("Resulting BPM");
    expect(stats[1].contains(bpmInput())).toBe(true);
  });

  it("leaves only the slider in the body", () => {
    renderCard();
    const body = document.querySelector(".tempo-card .card__body") as HTMLElement;
    expect(body.querySelector("input[type='range']")).not.toBeNull();
    expect(body.contains(bpmInput())).toBe(false);
    expect(document.querySelector(".tempo-card__main")).toBeNull();
  });

  it("shows dashes for the previous BPM without a song", () => {
    renderCard({ originalBpm: 0 });
    const side = document.querySelector(".tempo-card .card__side") as HTMLElement;
    expect(side.children[0].textContent).toContain("--");
  });

  it("has no footer", () => {
    renderCard();
    expect(document.querySelector(".tempo-card .card__footer")).toBeNull();
  });
});

describe("TempoCard BPM display looks", () => {
  function box(index: number): HTMLElement {
    const stat = document.querySelectorAll(".tempo-card__stat")[index] as HTMLElement;
    return stat.querySelector(".field") as HTMLElement;
  }

  it("shows the previous and the resulting BPM in the same kind of box", () => {
    renderCard();
    const previous = box(0);
    const resulting = box(1);
    expect(previous).not.toBeNull();
    expect(resulting).not.toBeNull();
    expect(previous.className).toBe(resulting.className);
    expect(previous.className).toContain("field--number");
  });

  it("writes both numbers the same way, one decimal at most and no unit", () => {
    renderCard({ originalBpm: 98.54, tempo: 100 });
    expect(box(0).textContent).toBe("98.5");
    expect((box(1) as HTMLInputElement).value).toBe("98.5");
  });

  it("writes a whole BPM without a trailing .0 in both", () => {
    renderCard({ originalBpm: 120, tempo: 100 });
    expect(box(0).textContent).toBe("120");
    expect((box(1) as HTMLInputElement).value).toBe("120");
  });

  it("keeps the previous BPM a plain, non-editable box", () => {
    renderCard();
    expect(box(0).tagName).toBe("SPAN");
    expect(box(0).getAttribute("tabindex")).toBeNull();
  });
});
