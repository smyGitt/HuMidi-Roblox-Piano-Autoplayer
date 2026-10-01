import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { getKeyRange } from "../../lib/keyRange";
import { buildKeyLayout } from "./keyLayout";
import { PianoKeyboard } from "./PianoKeyboard";

function renderKeyboard(use88: boolean, pressed: number[] = [], pedalDown = false, showPedal = true) {
  return render(
    <PianoKeyboard
      layout={buildKeyLayout(getKeyRange(use88))}
      pressedPitches={new Set(pressed)}
      pedalDown={pedalDown}
      showPedal={showPedal}
    />,
  );
}

describe("PianoKeyboard", () => {
  it("renders 88 keys with 52 white for the 88-key layout", () => {
    const { container } = renderKeyboard(true);
    expect(container.querySelectorAll(".piano-keyboard__key")).toHaveLength(88);
    expect(container.querySelectorAll(".piano-keyboard__key--white")).toHaveLength(52);
  });

  it("renders 61 keys with 36 white for the 61-key layout", () => {
    const { container } = renderKeyboard(false);
    expect(container.querySelectorAll(".piano-keyboard__key")).toHaveLength(61);
    expect(container.querySelectorAll(".piano-keyboard__key--white")).toHaveLength(36);
  });

  it("positions keys from the shared layout so they line up with the note columns", () => {
    const { container } = renderKeyboard(false);
    const first = container.querySelector(".piano-keyboard__key") as HTMLElement;
    expect(first.style.left).toBe("0%");
    expect(parseFloat(first.style.width)).toBeCloseTo(100 / 36);
  });

  it("marks only the pressed keys", () => {
    const { container } = renderKeyboard(false, [60, 61]);
    expect(container.querySelectorAll(".piano-keyboard__key--pressed")).toHaveLength(2);
  });

  it("shows the pedal strip only when enabled and fills it while the pedal is down", () => {
    expect(renderKeyboard(false, [], true, false).container.querySelector(".piano-keyboard__pedal")).toBeNull();
    const { container } = renderKeyboard(false, [], true, true);
    expect(container.querySelector(".piano-keyboard__pedal-fill--down")).not.toBeNull();
  });
});
