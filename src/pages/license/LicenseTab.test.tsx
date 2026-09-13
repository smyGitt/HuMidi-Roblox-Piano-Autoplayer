import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LicenseTab } from "./LicenseTab";

describe("LicenseTab", () => {
  it("shows the full MIT license text for HuMidi by default, not a paraphrase", () => {
    render(<LicenseTab />);
    expect(screen.getByText(/MIT License/)).toBeInTheDocument();
    expect(screen.getByText(/Copyright \(c\) 2026 smyGitt/)).toBeInTheDocument();
    expect(screen.getByText(/THE SOFTWARE IS PROVIDED "AS IS"/)).toBeInTheDocument();
  });

  it("switches to the PedalAI Dataset tab and shows the real dataset citations", () => {
    render(<LicenseTab />);
    fireEvent.click(screen.getByText("PedalAI Dataset"));
    expect(screen.getByText(/POP909/)).toBeInTheDocument();
    expect(screen.getByText(/GiantMIDI-Piano/)).toBeInTheDocument();
    expect(screen.getByText(/CC BY 4.0/)).toBeInTheDocument();
  });

  it("Third-Party Libraries lists the actual Rust/npm stack, not the old Python one", () => {
    render(<LicenseTab />);
    fireEvent.click(screen.getByText("Third-Party Libraries"));
    expect(screen.getByText(/tauri/)).toBeInTheDocument();
    expect(screen.getByText(/react-dom/)).toBeInTheDocument();
    expect(screen.getByText(/midly/)).toBeInTheDocument();
    expect(screen.queryByText(/PySide6/)).not.toBeInTheDocument();
    expect(screen.queryByText(/PyInstaller/)).not.toBeInTheDocument();
  });

  it("shows the full MIT license text for Phosphor Icons", () => {
    render(<LicenseTab />);
    fireEvent.click(screen.getByText("Phosphor Icons"));
    expect(screen.getByText(/Copyright \(c\) 2020 Phosphor Icons/)).toBeInTheDocument();
  });
});
