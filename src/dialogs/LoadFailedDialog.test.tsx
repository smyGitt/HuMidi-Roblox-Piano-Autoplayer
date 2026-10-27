import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LoadFailedDialog } from "./LoadFailedDialog";

const REASON = "The MIDI file for this save was not found: /music/song.mid. Move it back to that location, or use Load.";

describe("LoadFailedDialog", () => {
  it("shows the reason it was given, unchanged", () => {
    render(<LoadFailedDialog message={REASON} onDismiss={vi.fn()} />);
    expect(screen.getByText("Cannot open for editing")).toBeTruthy();
    expect(screen.getByText(REASON)).toBeTruthy();
  });

  it("dismisses with OK", () => {
    const onDismiss = vi.fn();
    render(<LoadFailedDialog message={REASON} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByText("OK"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("dismisses with the close button too", () => {
    const onDismiss = vi.fn();
    render(<LoadFailedDialog message={REASON} onDismiss={onDismiss} />);
    fireEvent.click(screen.getByRole("button", { name: "×" }));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
