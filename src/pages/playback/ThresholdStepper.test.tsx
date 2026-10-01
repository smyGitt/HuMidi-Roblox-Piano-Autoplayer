import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { stepThreshold, ThresholdStepper } from "./ThresholdStepper";

const VALUE = 0.6;
const SMALL_STEP = 0.005;
const BIG_STEP = 0.05;

describe("stepThreshold", () => {
  it("adds and subtracts a step without floating point noise", () => {
    expect(stepThreshold(VALUE, SMALL_STEP)).toBe(0.605);
    expect(stepThreshold(VALUE, -SMALL_STEP)).toBe(0.595);
    expect(stepThreshold(0.1, 0.2)).toBe(0.3);
    expect(stepThreshold(VALUE, BIG_STEP)).toBe(0.65);
  });

  it("clamps to the 0 to 1 range", () => {
    expect(stepThreshold(0.98, BIG_STEP)).toBe(1);
    expect(stepThreshold(0.02, -BIG_STEP)).toBe(0);
  });
});

describe("ThresholdStepper", () => {
  it("shows decrease buttons before the field and increase buttons after it, biggest step outermost", () => {
    render(<ThresholdStepper label="On threshold" value={VALUE} onChange={vi.fn()} />);
    const controls = [...screen.getByRole("spinbutton").parentElement!.children].map((el) => el.textContent || "field");
    expect(controls).toEqual(["-0.05", "-0.005", "field", "+0.005", "+0.05"]);
  });

  it("each button changes the value by its own step", () => {
    const onChange = vi.fn();
    render(<ThresholdStepper label="On threshold" value={VALUE} onChange={onChange} />);
    fireEvent.click(screen.getByText("+0.005"));
    expect(onChange).toHaveBeenLastCalledWith(0.605);
    fireEvent.click(screen.getByText("+0.05"));
    expect(onChange).toHaveBeenLastCalledWith(0.65);
    fireEvent.click(screen.getByText("-0.005"));
    expect(onChange).toHaveBeenLastCalledWith(0.595);
    fireEvent.click(screen.getByText("-0.05"));
    expect(onChange).toHaveBeenLastCalledWith(0.55);
  });

  it("typing in the field passes the typed number through", () => {
    const onChange = vi.fn();
    render(<ThresholdStepper label="On threshold" value={VALUE} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText("On threshold"), { target: { value: "0.7" } });
    expect(onChange).toHaveBeenCalledWith(0.7);
  });
});
