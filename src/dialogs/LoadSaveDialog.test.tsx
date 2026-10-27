import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LoadSaveDialog, type SaveEntry } from "./LoadSaveDialog";

const FIRST: SaveEntry = {
  filepath: "/saves/first.json",
  sourceMidiName: "song.mid",
  saveName: "first",
  createdAt: "2026-10-01",
  tempo: 100,
  pedalStyle: "ai",
  use88Key: false,
  humanization: [],
};

const SECOND: SaveEntry = { ...FIRST, filepath: "/saves/second.json", saveName: "second" };

function renderDialog(initialSelected: string | null = null) {
  const handlers = {
    onCancel: vi.fn(),
    onLoad: vi.fn(),
    onLoadAndEdit: vi.fn(),
    onRename: vi.fn(),
    onDelete: vi.fn(),
  };
  render(<LoadSaveDialog saves={[FIRST, SECOND]} initialSelected={initialSelected} {...handlers} />);
  return handlers;
}

function button(name: string): HTMLButtonElement {
  return screen.getByRole("button", { name }) as HTMLButtonElement;
}

describe("LoadSaveDialog footer", () => {
  it("lists Rename, Delete, Cancel, Load and Edit, Load in that order", () => {
    renderDialog();
    const labels = screen.getAllByRole("button").map((b) => b.textContent);
    const footer = labels.slice(-5);
    expect(footer).toEqual(["Rename", "Delete", "Cancel", "Load and Edit", "Load"]);
  });

  it("keeps Load and Load and Edit disabled until a save is picked", () => {
    renderDialog();
    expect(button("Load").disabled).toBe(true);
    expect(button("Load and Edit").disabled).toBe(true);
    fireEvent.click(screen.getByText("first"));
    expect(button("Load").disabled).toBe(false);
    expect(button("Load and Edit").disabled).toBe(false);
  });

  it("Load sends the picked save to onLoad only", () => {
    const handlers = renderDialog();
    fireEvent.click(screen.getByText("second"));
    fireEvent.click(button("Load"));
    expect(handlers.onLoad).toHaveBeenCalledTimes(1);
    expect(handlers.onLoad).toHaveBeenCalledWith(SECOND);
    expect(handlers.onLoadAndEdit).not.toHaveBeenCalled();
  });

  it("Load and Edit sends the picked save to onLoadAndEdit only", () => {
    const handlers = renderDialog();
    fireEvent.click(screen.getByText("first"));
    fireEvent.click(button("Load and Edit"));
    expect(handlers.onLoadAndEdit).toHaveBeenCalledTimes(1);
    expect(handlers.onLoadAndEdit).toHaveBeenCalledWith(FIRST);
    expect(handlers.onLoad).not.toHaveBeenCalled();
  });

  it("both buttons act on a save that was preselected when the dialog opened", () => {
    const handlers = renderDialog(SECOND.filepath);
    fireEvent.click(button("Load and Edit"));
    fireEvent.click(button("Load"));
    expect(handlers.onLoadAndEdit).toHaveBeenCalledWith(SECOND);
    expect(handlers.onLoad).toHaveBeenCalledWith(SECOND);
  });

  it("does nothing when a disabled Load and Edit is clicked", () => {
    const handlers = renderDialog();
    fireEvent.click(button("Load and Edit"));
    expect(handlers.onLoadAndEdit).not.toHaveBeenCalled();
  });

  it("Cancel closes without loading anything", () => {
    const handlers = renderDialog(FIRST.filepath);
    fireEvent.click(button("Cancel"));
    expect(handlers.onCancel).toHaveBeenCalledTimes(1);
    expect(handlers.onLoad).not.toHaveBeenCalled();
    expect(handlers.onLoadAndEdit).not.toHaveBeenCalled();
  });
});
