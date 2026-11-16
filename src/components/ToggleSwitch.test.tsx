import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ToggleSwitch } from "./ToggleSwitch";

describe("ToggleSwitch", () => {
  it("is the track, then the label", () => {
    render(<ToggleSwitch checked={false} onChange={() => {}} label="Show Piano" />);
    const toggle = screen.getByRole("switch");
    expect([...toggle.children].map((child) => child.className.split(" ")[0])).toEqual([
      "toggle-switch__track",
      "label",
    ]);
  });

  it("reports the opposite state when clicked", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={false} label="A" onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("does not toggle when disabled", () => {
    const onChange = vi.fn();
    render(<ToggleSwitch checked={false} label="A" disabled onChange={onChange} />);
    fireEvent.click(screen.getByRole("switch"));
    expect(onChange).not.toHaveBeenCalled();
  });
});

describe("ToggleSwitch accessible name", () => {
  it("can be named without showing a label", () => {
    render(<ToggleSwitch checked={false} ariaLabel="Vary Timing" onChange={() => {}} />);
    const toggle = screen.getByRole("switch", { name: "Vary Timing" });
    expect(toggle.textContent).toBe("");
  });

  it("has no aria-label when it shows its own label", () => {
    render(<ToggleSwitch checked={false} label="Show Piano" onChange={() => {}} />);
    expect(screen.getByRole("switch").getAttribute("aria-label")).toBeNull();
  });
});
