import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { RangeInput, rangeFillPercent } from "./Field";

describe("rangeFillPercent", () => {
  it("is the position of the value between min and max", () => {
    expect(rangeFillPercent(25, 0, 100)).toBe(25);
    expect(rangeFillPercent(60, 20, 100)).toBe(50);
    expect(rangeFillPercent(0, 0, 10000)).toBe(0);
    expect(rangeFillPercent(5000, 0, 10000)).toBe(50);
  });

  it("stays between 0 and 100 when the value is outside the range", () => {
    expect(rangeFillPercent(-5, 0, 100)).toBe(0);
    expect(rangeFillPercent(150, 0, 100)).toBe(100);
  });

  it("is 0 when the range has no width or the numbers are missing", () => {
    expect(rangeFillPercent(5, 5, 5)).toBe(0);
    expect(rangeFillPercent(Number.NaN, 0, 100)).toBe(0);
  });
});

describe("RangeInput", () => {
  function input(props: React.ComponentProps<typeof RangeInput>) {
    const { container } = render(<RangeInput {...props} />);
    return container.querySelector("input") as HTMLInputElement;
  }

  it("exposes the filled share to the stylesheet as --range-fill", () => {
    const element = input({ min: 20, max: 100, value: 60, onChange: () => {} });
    expect(element.style.getPropertyValue("--range-fill")).toBe("50%");
  });

  it("follows the value as it changes", () => {
    const { container, rerender } = render(<RangeInput min={0} max={10} value={2} onChange={() => {}} />);
    rerender(<RangeInput min={0} max={10} value={8} onChange={() => {}} />);
    expect((container.querySelector("input") as HTMLInputElement).style.getPropertyValue("--range-fill")).toBe("80%");
  });

  it("keeps the caller's own style and class", () => {
    const element = input({ min: 0, max: 100, value: 10, className: "x", style: { width: "10px" }, onChange: () => {} });
    expect(element.className).toBe("range x");
    expect(element.style.width).toBe("10px");
  });

  it("defaults min to 0 and max to 100 like the browser does", () => {
    const element = input({ value: 30, onChange: () => {} });
    expect(element.style.getPropertyValue("--range-fill")).toBe("30%");
  });
});
