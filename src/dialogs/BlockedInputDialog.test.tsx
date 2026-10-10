import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { BlockedInputDialog } from "./BlockedInputDialog";

describe("BlockedInputDialog", () => {
  it("shows the reason and dismisses with OK", () => {
    const onDismiss = vi.fn();
    render(<BlockedInputDialog message="Blocked unsafe input: event #4 has the unknown action 'shortcut'" onDismiss={onDismiss} />);
    expect(screen.getByText("Load blocked")).toBeTruthy();
    expect(screen.getByText(/event #4 has the unknown action 'shortcut'/)).toBeTruthy();
    fireEvent.click(screen.getByText("OK"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
