import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { DraftNumberInput } from "./DraftNumberInput";

function box(): HTMLInputElement {
  return screen.getByLabelText("Amount") as HTMLInputElement;
}

function Harness({ initial, clamp = (n: number) => n, onCommit }: { initial: number; clamp?: (n: number) => number; onCommit: (n: number) => void }) {
  const [value, setValue] = useState(initial);
  return (
    <DraftNumberInput
      aria-label="Amount"
      value={value}
      onCommit={(n) => {
        const next = clamp(n);
        setValue(next);
        onCommit(next);
      }}
    />
  );
}

function type(text: string) {
  fireEvent.change(box(), { target: { value: text } });
}

describe("DraftNumberInput", () => {
  it("shows the value it is given, as a number box with the props passed through", () => {
    render(<DraftNumberInput aria-label="Amount" value={2.5} min={0.1} step={0.1} disabled onCommit={vi.fn()} />);
    expect(box().type).toBe("number");
    expect(box().value).toBe("2.5");
    expect(box().min).toBe("0.1");
    expect(box().step).toBe("0.1");
    expect(box().disabled).toBe(true);
  });

  it("lets the box be emptied, without committing anything and without putting a 0 in it", () => {
    const onCommit = vi.fn();
    render(<Harness initial={100} onCommit={onCommit} />);
    fireEvent.focus(box());
    type("");
    expect(box().value).toBe("");
    expect(onCommit).not.toHaveBeenCalled();
  });

  it("types a number after emptying the box with no leading zero: 100 stays 100", () => {
    const onCommit = vi.fn();
    render(<Harness initial={100} onCommit={onCommit} />);
    fireEvent.focus(box());
    type("");
    type("1");
    type("10");
    type("100");
    expect(box().value).toBe("100");
    expect(onCommit.mock.calls.map(([n]) => n)).toEqual([1, 10, 100]);
  });

  it("lets a negative sign be typed first: the half typed minus commits nothing, then -5 commits -5", () => {
    const onCommit = vi.fn();
    render(<Harness initial={100} onCommit={onCommit} />);
    fireEvent.focus(box());
    type("");
    type("-5");
    expect(box().value).toBe("-5");
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(-5);
  });

  it("keeps what is being typed while focused, even when the committed value comes back different", () => {
    render(<Harness initial={2} clamp={(n) => Math.max(0.1, n)} onCommit={vi.fn()} />);
    fireEvent.focus(box());
    type("0");
    expect(box().value).toBe("0");
    type("0.");
    type("0.5");
    expect(box().value).toBe("0.5");
  });

  it("shows the committed value when it loses focus, replacing an empty box or an out of range entry", () => {
    render(<Harness initial={2} clamp={(n) => Math.max(0.1, n)} onCommit={vi.fn()} />);
    fireEvent.focus(box());
    type("0");
    fireEvent.blur(box());
    expect(box().value).toBe("0.1");
    fireEvent.focus(box());
    type("");
    fireEvent.blur(box());
    expect(box().value).toBe("0.1");
  });

  it("follows a change of the value from outside when it is not being edited", () => {
    const { rerender } = render(<DraftNumberInput aria-label="Amount" value={3} onCommit={vi.fn()} />);
    rerender(<DraftNumberInput aria-label="Amount" value={9} onCommit={vi.fn()} />);
    expect(box().value).toBe("9");
  });
});
