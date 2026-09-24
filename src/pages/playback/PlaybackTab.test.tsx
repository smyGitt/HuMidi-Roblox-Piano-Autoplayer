import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PlaybackConfigProvider } from "../../state/PlaybackConfigContext";
import { LogProvider } from "../../state/LogContext";
import { PlaybackTab } from "./PlaybackTab";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
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

function tree() {
  return (
    <PlaybackConfigProvider>
      <LogProvider>
        <PlaybackTab onEditTrackSelection={() => {}} />
      </LogProvider>
    </PlaybackConfigProvider>
  );
}

beforeEach(() => {
  engine.saveCount = 0;
  tauriMocks.isTauri.mockReturnValue(true);
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

    tauriMocks.listSaves.mockResolvedValue([
      {
        path: "C:\\saves\\fresh.json",
        filename: "fresh.json",
        song_name: "song.mid",
        created: "2026-09-24T10:00:00",
        last_accessed: "2026-09-24T10:00:00",
        tempo: 100,
        pedal_style: "ai",
        use_88_key_layout: false,
        humanization: [],
      },
    ]);
    engine.saveCount = 1;
    rerender(tree());
    expect(await findByText("fresh")).toBeTruthy();
  });
});
