import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { FileHeader } from "./FileHeader";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  checkSaveName: vi.fn<(name: string) => Promise<string>>(),
}));
vi.mock("../lib/tauri", () => tauriMocks);

const engine = vi.hoisted(() => ({
  fileName: "song.mid",
  parts: [{ name: "a", meta: "b" }],
  hasCompiledNotes: true,
  isSaving: false,
  save: vi.fn<(name?: string) => Promise<{ path: string } | { error: string }>>(),
  openFileBrowser: vi.fn(),
  loadFile: vi.fn(),
  clearSong: vi.fn(),
}));
vi.mock("../state/PlaybackEngineContext", () => ({ usePlaybackEngine: () => engine }));

function nameInput(): HTMLInputElement {
  return screen.getByRole("textbox") as HTMLInputElement;
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.checkSaveName.mockReset().mockImplementation((name) => Promise.resolve(name.trim()));
  engine.isSaving = false;
  engine.save.mockReset().mockResolvedValue({ path: "/saves/x.json" });
});

describe("FileHeader save flow", () => {
  it("Save opens a name dialog prefilled with the file name plus a timestamp suffix", () => {
    render(<FileHeader />);
    expect(screen.queryByText("Save playback", { selector: ".modal__title" })).toBeNull();
    fireEvent.click(screen.getByLabelText("Save playback"));
    expect(screen.getByText("Save playback", { selector: ".modal__title" })).toBeTruthy();
    expect(nameInput().value).toMatch(/^song_\d{8}_\d{6}$/);
    expect(document.querySelector(".modal__title-row button")).toBeNull();
  });

  it("checks the name, closes the dialog and saves with exactly the typed name", async () => {
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.change(nameInput(), { target: { value: " My Song " } });
    fireEvent.click(screen.getByText("Save", { selector: ".modal__footer .btn" }));
    await waitFor(() => expect(engine.save).toHaveBeenCalledWith("My Song"));
    expect(tauriMocks.checkSaveName).toHaveBeenCalledWith(" My Song ");
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("Enter in the name field saves", async () => {
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.change(nameInput(), { target: { value: "quick" } });
    fireEvent.keyDown(nameInput(), { key: "Enter" });
    await waitFor(() => expect(engine.save).toHaveBeenCalledWith("quick"));
  });

  it("a name that fails the check keeps the dialog open with the error and does not save", async () => {
    tauriMocks.checkSaveName.mockRejectedValue('A save named "taken" already exists.');
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.change(nameInput(), { target: { value: "taken" } });
    fireEvent.click(screen.getByText("Save", { selector: ".modal__footer .btn" }));
    await waitFor(() => expect(screen.getByText('A save named "taken" already exists.')).toBeTruthy());
    expect(engine.save).not.toHaveBeenCalled();
    expect(nameInput().value).toBe("taken");
  });

  it("a backend save failure reopens the dialog with the typed name and the error", async () => {
    engine.save.mockResolvedValue({ error: "disk full" });
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.change(nameInput(), { target: { value: "keepme" } });
    fireEvent.click(screen.getByText("Save", { selector: ".modal__footer .btn" }));
    await waitFor(() => expect(screen.getByText("disk full")).toBeTruthy());
    expect(nameInput().value).toBe("keepme");
  });

  it("Cancel closes the dialog without checking or saving", () => {
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.click(screen.getByText("Cancel"));
    expect(screen.queryByRole("textbox")).toBeNull();
    expect(tauriMocks.checkSaveName).not.toHaveBeenCalled();
    expect(engine.save).not.toHaveBeenCalled();
  });

  it("shows the Saving... modal with the spinner only while a save is in progress", () => {
    engine.isSaving = true;
    const { unmount } = render(<FileHeader />);
    expect(screen.getByText("Saving...", { selector: ".modal__title" })).toBeTruthy();
    expect(document.querySelector(".saving-dialog .spinner")).not.toBeNull();
    expect(document.querySelector(".modal__title-row button")).toBeNull();
    unmount();

    engine.isSaving = false;
    render(<FileHeader />);
    expect(screen.queryByText("Saving...", { selector: ".modal__title" })).toBeNull();
  });

  it("outside the desktop app the backend name check is skipped", async () => {
    tauriMocks.isTauri.mockReturnValue(false);
    render(<FileHeader />);
    fireEvent.click(screen.getByLabelText("Save playback"));
    fireEvent.change(nameInput(), { target: { value: "preview" } });
    fireEvent.click(screen.getByText("Save", { selector: ".modal__footer .btn" }));
    await waitFor(() => expect(engine.save).toHaveBeenCalledWith("preview"));
    expect(tauriMocks.checkSaveName).not.toHaveBeenCalled();
  });
});
