import { CopySimpleIcon, FloppyDiskIcon } from "@phosphor-icons/react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { TranslatorTab } from "./TranslatorTab";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => false),
  notesToSheet: vi.fn<() => Promise<string>>(),
  writeTextFile: vi.fn<(path: string, contents: string) => Promise<void>>(),
}));
const dialogMocks = vi.hoisted(() => ({ save: vi.fn<(options: unknown) => Promise<string | null>>() }));
const engineMock = vi.hoisted(() => ({
  fileName: "",
  finalNotes: [] as unknown[],
  tempoEvents: [],
  timeSignatures: [],
  playTranslatedSheet: () => Promise.resolve(),
}));

vi.mock("../../lib/tauri", () => ({
  isTauri: tauriMocks.isTauri,
  loadAppConfig: vi.fn(() => Promise.resolve({})),
  saveAppConfig: vi.fn(() => Promise.resolve()),
  translateSheetToNotes: vi.fn(),
  notesToSheet: tauriMocks.notesToSheet,
  writeTextFile: tauriMocks.writeTextFile,
}));

vi.mock("@tauri-apps/plugin-dialog", () => dialogMocks);

vi.mock("../../state/PlaybackConfigContext", () => ({
  usePlaybackConfig: () => ({ config: { use_88_key_layout: false } }),
}));

vi.mock("../../state/PlaybackEngineContext", () => ({
  usePlaybackEngine: () => engineMock,
}));

vi.mock("../../state/LogContext", () => ({
  useLog: () => ({ appendLog: vi.fn() }),
  useOptionalLog: () => null,
}));

const SHEET = "t y u\n[tu] p";

function header(): HTMLElement {
  return document.querySelector(".tab-page__header") as HTMLElement;
}

function workspace(): HTMLElement {
  return document.querySelector(".translator-tab__workspace") as HTMLElement;
}

function openExport() {
  fireEvent.click(screen.getByText("Export"));
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(false);
  tauriMocks.notesToSheet.mockReset().mockResolvedValue(SHEET);
  tauriMocks.writeTextFile.mockReset().mockResolvedValue(undefined);
  dialogMocks.save.mockReset().mockResolvedValue("/home/me/song.txt");
  engineMock.fileName = "";
  engineMock.finalNotes = [];
  window.localStorage.clear();
});

describe("TranslatorTab format dropdown", () => {
  it("is not in the header next to the Import and Export tabs", () => {
    render(<TranslatorTab />);
    expect(header().querySelector("select")).toBeNull();
  });

  it("is in the bottom button bar of the Import page", () => {
    render(<TranslatorTab />);
    const select = workspace().querySelector("select") as HTMLSelectElement;
    expect(select).not.toBeNull();
    expect(select.value).toBe("Virtual Piano");
    expect(workspace().lastElementChild?.contains(select)).toBe(true);
    expect(workspace().lastElementChild?.className).toBe("translator-tab__action-bar");
    expect(workspace().lastElementChild?.firstElementChild?.contains(select)).toBe(true);
  });

  it("is in the bottom button bar of the Export page too", () => {
    render(<TranslatorTab />);
    openExport();
    const select = workspace().querySelector("select") as HTMLSelectElement;
    expect(select).not.toBeNull();
    expect(select.value).toBe("Virtual Piano");
    expect(workspace().lastElementChild?.contains(select)).toBe(true);
    expect(workspace().lastElementChild?.className).toBe("translator-tab__action-bar");
    expect(workspace().lastElementChild?.firstElementChild?.contains(select)).toBe(true);
    expect(header().querySelector("select")).toBeNull();
  });

  it("has exactly one dropdown on each page", () => {
    render(<TranslatorTab />);
    expect(document.querySelectorAll("select")).toHaveLength(1);
    openExport();
    expect(document.querySelectorAll("select")).toHaveLength(1);
  });
});

describe("TranslatorTab preview", () => {
  it("has no Preview card on the Import page", () => {
    render(<TranslatorTab />);
    expect(screen.queryByText("Preview")).toBeNull();
    expect(screen.queryByText("Not built yet.")).toBeNull();
  });

  it("has no Source card on the Import page, just the sheet text box", () => {
    render(<TranslatorTab />);
    expect(document.querySelectorAll(".card")).toHaveLength(0);
    expect(screen.queryByText("Source")).toBeNull();
    expect(workspace().querySelector("textarea")?.getAttribute("placeholder")).toBe("Paste a sheet here...");
    expect(workspace().querySelector(".playback-tab__two-col")).toBeNull();
  });

  it("has no Source card on the Export page", () => {
    render(<TranslatorTab />);
    openExport();
    expect(screen.queryByText("Source")).toBeNull();
    expect(workspace().querySelector(".playback-tab__two-col")).toBeNull();
  });
});

describe("TranslatorTab full-screen text box", () => {
  it("has the sheet text box and then the buttons, nothing else, on the Import page", () => {
    render(<TranslatorTab />);
    const children = [...workspace().children].map((child) => child.className.split(" ")[0] || child.tagName);
    expect(children).toEqual(["field", "translator-tab__action-bar"]);
    expect(workspace().lastElementChild?.className).toBe("translator-tab__action-bar");
    expect(workspace().lastElementChild?.textContent).toContain("Play Sheet");
  });

  it("has a read-only output text box and then the buttons on the Export page, no cards", () => {
    render(<TranslatorTab />);
    openExport();
    expect(document.querySelectorAll(".card")).toHaveLength(0);
    const children = [...workspace().children].map((child) => child.className.split(" ")[0] || child.tagName);
    expect(children).toEqual(["field", "translator-tab__action-bar"]);
    expect((workspace().querySelector("textarea") as HTMLTextAreaElement).readOnly).toBe(true);
    const bar = workspace().lastElementChild as HTMLElement;
    expect(bar.textContent).toContain("Generate Sheet");
    expect(bar.querySelector("button[aria-label='Copy']")).not.toBeNull();
  });

  it("shows the export status in the button bar, not above the text box", async () => {
    render(<TranslatorTab />);
    openExport();
    fireEvent.click(screen.getByText("Generate Sheet"));
    const status = await screen.findByText("Not available outside the desktop app.");
    expect(status.closest(".translator-tab__action-bar")).not.toBeNull();
  });
});

describe("TranslatorTab Play Sheet position", () => {
  it("marks Play Sheet as the right-side button of the bar, last in the bar", () => {
    render(<TranslatorTab />);
    const play = screen.getByText("Play Sheet");
    expect(play.className).toContain("translator-tab__end");
    expect(workspace().lastElementChild?.lastElementChild).toBe(play);
  });
});

describe("TranslatorTab export without a song", () => {
  it("asks the user to load a MIDI file first", async () => {
    tauriMocks.isTauri.mockReturnValue(true);
    render(<TranslatorTab />);
    openExport();
    fireEvent.click(screen.getByText("Generate Sheet"));
    expect(await screen.findByText("Please load a MIDI file first")).toBeTruthy();
    expect(screen.queryByText("Load and compile a MIDI file on the Playback tab first.")).toBeNull();
  });
});

describe("TranslatorTab copy button", () => {
  function pathsOf(html: string): string[] {
    return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
  }

  it("is an icon-only button with the filled copy icon", () => {
    render(<TranslatorTab />);
    openExport();
    const copy = screen.getByLabelText("Copy") as HTMLElement;
    expect(copy.textContent).toBe("");
    const expected = render(<CopySimpleIcon weight="fill" />);
    expect(pathsOf(copy.innerHTML)).toEqual(pathsOf(expected.container.innerHTML));
    expect(screen.queryByText("Copy")).toBeNull();
  });
});

describe("TranslatorTab save button", () => {
  async function generateSheet() {
    tauriMocks.isTauri.mockReturnValue(true);
    engineMock.fileName = "song.mid";
    engineMock.finalNotes = [{}];
    render(<TranslatorTab />);
    openExport();
    fireEvent.click(screen.getByText("Generate Sheet"));
    await waitFor(() => expect((document.querySelector("textarea") as HTMLTextAreaElement).value).toBe(SHEET));
  }

  function pathsOf(html: string): string[] {
    return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
  }

  it("is an icon-only button with the filled floppy disk icon, after Copy in the button bar", () => {
    render(<TranslatorTab />);
    openExport();
    const save = screen.getByLabelText("Save sheet") as HTMLElement;
    expect(save.textContent).toBe("");
    const expected = render(<FloppyDiskIcon weight="fill" />);
    expect(pathsOf(save.innerHTML)).toEqual(pathsOf(expected.container.innerHTML));
    const buttons = [...(workspace().lastElementChild as HTMLElement).querySelectorAll("button")];
    expect(buttons.indexOf(save as HTMLButtonElement)).toBe(buttons.indexOf(screen.getByLabelText("Copy") as HTMLButtonElement) + 1);
  });

  it("is disabled until a sheet has been generated", async () => {
    render(<TranslatorTab />);
    openExport();
    expect((screen.getByLabelText("Save sheet") as HTMLButtonElement).disabled).toBe(true);
  });

  it("asks where to save, suggesting the song name with .txt, and writes the sheet there", async () => {
    await generateSheet();
    expect((screen.getByLabelText("Save sheet") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByLabelText("Save sheet"));
    await waitFor(() => expect(tauriMocks.writeTextFile).toHaveBeenCalledWith("/home/me/song.txt", SHEET));
    expect(dialogMocks.save).toHaveBeenCalledWith({
      defaultPath: "song.txt",
      filters: [{ name: "Text", extensions: ["txt"] }],
    });
    expect(await screen.findByText("Saved to /home/me/song.txt")).toBeTruthy();
  });

  it("suggests sheet.txt when the song has no name", async () => {
    await generateSheet();
    engineMock.fileName = "";
    fireEvent.click(screen.getByLabelText("Save sheet"));
    await waitFor(() => expect(dialogMocks.save).toHaveBeenCalled());
    expect(dialogMocks.save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: "sheet.txt" }));
  });

  it("only strips a real .mid or .midi ending from the song name", async () => {
    await generateSheet();
    for (const [name, expected] of [
      ["tune.MIDI", "tune.txt"],
      ["tune.midi", "tune.txt"],
      ["amid", "amid.txt"],
      ["notes.mid.bak", "notes.mid.bak.txt"],
    ]) {
      engineMock.fileName = name;
      dialogMocks.save.mockClear();
      fireEvent.click(screen.getByLabelText("Save sheet"));
      await waitFor(() => expect(dialogMocks.save).toHaveBeenCalled());
      expect(dialogMocks.save).toHaveBeenCalledWith(expect.objectContaining({ defaultPath: expected }));
    }
  });

  it("writes nothing when the user cancels the dialog", async () => {
    await generateSheet();
    dialogMocks.save.mockResolvedValue(null);
    fireEvent.click(screen.getByLabelText("Save sheet"));
    await waitFor(() => expect(dialogMocks.save).toHaveBeenCalled());
    expect(tauriMocks.writeTextFile).not.toHaveBeenCalled();
    expect(screen.queryByText(/^Saved to/)).toBeNull();
  });

  it("shows the error when the file cannot be written", async () => {
    await generateSheet();
    tauriMocks.writeTextFile.mockRejectedValue("Permission denied (os error 13)");
    fireEvent.click(screen.getByLabelText("Save sheet"));
    expect(await screen.findByText("Failed to save: Permission denied (os error 13)")).toBeTruthy();
  });
});
