import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LegacyThresholdsDialog } from "./LegacyThresholdsDialog";

describe("LegacyThresholdsDialog", () => {
  it("explains the flat lines and that curves are supported, and dismisses with OK", () => {
    const onDismiss = vi.fn();
    render(<LegacyThresholdsDialog onDismiss={onDismiss} />);
    expect(screen.getByText("Older save")).toBeTruthy();
    expect(screen.getByText(/loaded as flat lines/)).toBeTruthy();
    expect(screen.getByText(/Threshold curves are now supported/)).toBeTruthy();
    fireEvent.click(screen.getByText("OK"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
