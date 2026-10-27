import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PlaybackFailedDialog } from "./PlaybackFailedDialog";

const REASON =
  "HuMidi cannot send key presses because it has no access to /dev/uinput. Run these two commands in a terminal, then press Play again: sudo modprobe uinput && sudo chmod 0666 /dev/uinput. Details: no display";

describe("PlaybackFailedDialog", () => {
  it("shows the reason it was given, unchanged", () => {
    render(<PlaybackFailedDialog message={REASON} onDismiss={vi.fn()} />);
    expect(screen.getByText("Cannot start playback")).toBeTruthy();
    expect(screen.getByText(REASON)).toBeTruthy();
  });

  it("dismisses with OK", () => {
    const onDismiss = vi.fn();
    render(<PlaybackFailedDialog message={REASON} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByText("OK"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("dismisses with the close button too", () => {
    const onDismiss = vi.fn();
    render(<PlaybackFailedDialog message={REASON} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByRole("button", { name: "×" }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
