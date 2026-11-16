import { fireEvent, render, screen } from "@testing-library/react";
import { InfoIcon } from "@phosphor-icons/react";
import { describe, expect, it } from "vitest";
import { InfoTip } from "./InfoTip";

const TEXT = "Some helpful message";

function pathsOf(html: string): string[] {
  return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
}

function button(): HTMLElement {
  return screen.getByRole("button", { name: "More information" });
}

describe("InfoTip", () => {
  it("shows only the filled info icon at first, with the message hidden", () => {
    render(<InfoTip text={TEXT} />);
    const expected = render(<InfoIcon weight="fill" />);
    expect(pathsOf(button().innerHTML)).toEqual(pathsOf(expected.container.innerHTML));
    expect(screen.queryByText(TEXT)).toBeNull();
    expect(button().getAttribute("aria-expanded")).toBe("false");
  });

  it("shows the message while the mouse is over it and hides it when the mouse leaves", () => {
    const { container } = render(<InfoTip text={TEXT} />);
    const tip = container.firstElementChild as HTMLElement;
    fireEvent.mouseEnter(tip);
    expect(screen.getByRole("tooltip").textContent).toBe(TEXT);
    fireEvent.mouseLeave(tip);
    expect(screen.queryByText(TEXT)).toBeNull();
  });

  it("keeps the message open after a click, even once the mouse leaves", () => {
    const { container } = render(<InfoTip text={TEXT} />);
    const tip = container.firstElementChild as HTMLElement;
    fireEvent.mouseEnter(tip);
    fireEvent.click(button());
    fireEvent.mouseLeave(tip);
    expect(screen.getByText(TEXT)).toBeTruthy();
    expect(button().getAttribute("aria-expanded")).toBe("true");
  });

  it("closes the pinned message with a second click", () => {
    const { container } = render(<InfoTip text={TEXT} />);
    fireEvent.click(button());
    expect(screen.getByText(TEXT)).toBeTruthy();
    fireEvent.click(button());
    fireEvent.mouseLeave(container.firstElementChild as HTMLElement);
    expect(screen.queryByText(TEXT)).toBeNull();
  });

  it("opens on a click without any hover, as on a touch screen", () => {
    render(<InfoTip text={TEXT} />);
    fireEvent.click(button());
    expect(screen.getByText(TEXT)).toBeTruthy();
  });

  it("closes the pinned message on a click elsewhere", () => {
    render(
      <div>
        <InfoTip text={TEXT} />
        <span>elsewhere</span>
      </div>,
    );
    fireEvent.click(button());
    fireEvent.mouseDown(screen.getByText("elsewhere"));
    expect(screen.queryByText(TEXT)).toBeNull();
  });

  it("closes the pinned message with Escape", () => {
    render(<InfoTip text={TEXT} />);
    fireEvent.click(button());
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByText(TEXT)).toBeNull();
  });

  it("stays open on a click inside the tip itself", () => {
    render(<InfoTip text={TEXT} />);
    fireEvent.click(button());
    fireEvent.mouseDown(screen.getByText(TEXT));
    expect(screen.getByText(TEXT)).toBeTruthy();
  });
});
