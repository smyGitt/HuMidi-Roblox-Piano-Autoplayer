import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppSettingsProvider } from "../../state/AppSettingsContext";
import { PlaybackConfigProvider } from "../../state/PlaybackConfigContext";
import { LogProvider } from "../../state/LogContext";
import { PlaybackTab } from "./PlaybackTab";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  onEvent: vi.fn(() => Promise.resolve(() => {})),
  loadAppConfig: vi.fn<() => Promise<Record<string, unknown>>>(() => Promise.resolve({})),
  saveAppConfig: vi.fn(() => Promise.resolve()),
  getSaveDir: vi.fn<() => Promise<string>>(() => Promise.resolve("C:\\saves")),
  listSaves: vi.fn<(dir: string) => Promise<unknown[]>>(() => Promise.resolve([])),
  renameSave: vi.fn(),
  deleteSave: vi.fn(),
}));
vi.mock("../../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../../lib/tauri")>("../../lib/tauri");
  return { ...actual, ...tauriMocks };
});

const engine = vi.hoisted(() => ({
  fileName: "song.mid",
  parts: [] as unknown[],
  tracks: [] as unknown[],
  pedalIntervals: [] as unknown[],
  saveCount: 0,
  originalBpm: 120,
  hasCompiledPedal: false,
  hasCompiledNotes: false,
  isGeneratingPedal: false,
  aiStats: null,
  defaultAiThresholds: null,
  generatePedal: vi.fn(),
  resumeSave: vi.fn(),
  loadFile: vi.fn(),
  openFileBrowser: vi.fn(),
}));
vi.mock("../../state/PlaybackEngineContext", () => ({ usePlaybackEngine: () => engine }));

function saveSummary(name: string, songName: string, tempo = 100) {
  return {
    path: `C:\\saves\\${name}.json`,
    filename: `${name}.json`,
    song_name: songName,
    created: "2026-09-24T10:00:00",
    last_accessed: "2026-09-24T10:00:00",
    tempo,
    pedal_style: "ai",
    use_88_key_layout: false,
    humanization: [],
  };
}

function tree() {
  return (
    <AppSettingsProvider>
      <PlaybackConfigProvider>
        <LogProvider>
          <PlaybackTab onEditTrackSelection={() => {}} />
        </LogProvider>
      </PlaybackConfigProvider>
    </AppSettingsProvider>
  );
}

beforeEach(() => {
  engine.saveCount = 0;
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.loadAppConfig.mockReset().mockResolvedValue({});
  tauriMocks.getSaveDir.mockReset().mockResolvedValue("C:\\saves");
  tauriMocks.listSaves.mockReset().mockResolvedValue([]);
});

describe("PlaybackTab saved songs list", () => {
  it("loads the saves once on mount and does not refresh again until a save finishes", async () => {
    render(tree());
    await waitFor(() => expect(tauriMocks.listSaves).toHaveBeenCalledTimes(1));
  });

  it("refreshes the saves list after a save completes", async () => {
    const { rerender } = render(tree());
    await waitFor(() => expect(tauriMocks.listSaves).toHaveBeenCalledTimes(1));

    engine.saveCount = 1;
    rerender(tree());
    await waitFor(() => expect(tauriMocks.listSaves).toHaveBeenCalledTimes(2));

    engine.saveCount = 2;
    rerender(tree());
    await waitFor(() => expect(tauriMocks.listSaves).toHaveBeenCalledTimes(3));
  });

  it("shows a newly saved song in the list after the refresh", async () => {
    const { rerender, findByText } = render(tree());
    await waitFor(() => expect(tauriMocks.listSaves).toHaveBeenCalledTimes(1));

    tauriMocks.listSaves.mockResolvedValue([saveSummary("fresh", "song.mid")]);
    engine.saveCount = 1;
    rerender(tree());
    expect(await findByText("fresh")).toBeTruthy();
  });
});

describe("PlaybackTab max visible saves", () => {
  it("shows only the first N saves in the card and all of them in the Load Save dialog", async () => {
    tauriMocks.loadAppConfig.mockResolvedValue({ max_visible_saves: 2 });
    tauriMocks.listSaves.mockResolvedValue([
      saveSummary("one", "a.mid"),
      saveSummary("two", "a.mid"),
      saveSummary("three", "a.mid"),
    ]);
    render(tree());
    await waitFor(() => expect(document.querySelectorAll(".saved-songs-card .save-card").length).toBe(2));
    expect(screen.queryByText("three")).toBeNull();

    fireEvent.click(screen.getByTitle("All saves"));
    await waitFor(() => expect(document.querySelectorAll(".load-save-dialog__group .btn--item").length).toBe(3));
  });

  it("shows every save when there are fewer than the limit", async () => {
    tauriMocks.listSaves.mockResolvedValue([saveSummary("one", "a.mid"), saveSummary("two", "a.mid")]);
    render(tree());
    await waitFor(() => expect(document.querySelectorAll(".saved-songs-card .save-card").length).toBe(2));
  });
});

describe("PlaybackTab opening the Load Save dialog", () => {
  it("clicking a save card opens the dialog with that save selected and its details showing", async () => {
    tauriMocks.listSaves.mockResolvedValue([
      saveSummary("first", "a.mid", 90),
      saveSummary("second", "b.mid", 130),
    ]);
    render(tree());
    await waitFor(() => expect(document.querySelectorAll(".saved-songs-card .save-card").length).toBe(2));

    fireEvent.click(screen.getByText("second"));

    await waitFor(() => expect(document.querySelector(".load-save-dialog__details-title")?.textContent).toBe("b.mid"));
    const active = document.querySelector(".load-save-dialog__group .btn--active");
    expect(active?.textContent).toBe("second");
    expect(screen.getByText("130%")).toBeTruthy();
    expect(screen.queryByText("Select a save to see its details.")).toBeNull();
  });

  it("the All saves button opens the dialog with nothing selected", async () => {
    tauriMocks.listSaves.mockResolvedValue([saveSummary("first", "a.mid")]);
    render(tree());
    await waitFor(() => expect(document.querySelectorAll(".saved-songs-card .save-card").length).toBe(1));

    fireEvent.click(screen.getByTitle("All saves"));

    await waitFor(() => expect(screen.getByText("Select a save to see its details.")).toBeTruthy());
    expect(document.querySelector(".load-save-dialog__group .btn--active")).toBeNull();
  });

  it("reopening from All saves after a card click does not keep the earlier selection", async () => {
    tauriMocks.listSaves.mockResolvedValue([saveSummary("first", "a.mid")]);
    render(tree());
    await waitFor(() => expect(document.querySelectorAll(".saved-songs-card .save-card").length).toBe(1));

    fireEvent.click(screen.getByText("first"));
    await waitFor(() => expect(document.querySelector(".load-save-dialog__group .btn--active")).not.toBeNull());
    fireEvent.click(screen.getByText("Cancel"));
    await waitFor(() => expect(document.querySelector(".load-save-dialog")).toBeNull());

    fireEvent.click(screen.getByTitle("All saves"));
    await waitFor(() => expect(screen.getByText("Select a save to see its details.")).toBeTruthy());
  });
});
