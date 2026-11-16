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
  aiCurves: null,
  defaultAiCurves: null,
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
    <LogProvider>
      <AppSettingsProvider>
        <PlaybackConfigProvider>
          <PlaybackTab onEditTrackSelection={() => {}} />
        </PlaybackConfigProvider>
      </AppSettingsProvider>
    </LogProvider>
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

describe("PlaybackTab humanize explanations", () => {
  const EXPLANATIONS = [
    "turn all the human touches on or off at once",
    "each hand gets its own slightly different timing",
    "notes in a chord start just a hair apart, like a real hand",
    "notes start a little early or late, like a real player",
    "some notes get held a bit longer or shorter",
    "the speed drifts a little faster and slower as the song goes",
    "makes the speed changes go the opposite way",
    "your hands slowly slip out of sync with each other",
    "once in a while a note gets missed or played wrong",
  ];

  it("shows every explanation behind an info icon, none as a line of text", async () => {
    render(tree());
    fireEvent.click(screen.getByText("Humanize"));
    await waitFor(() => expect(document.querySelector(".playback-tab__humanize-page")).not.toBeNull());
    const page = document.querySelector(".playback-tab__humanize-page") as HTMLElement;
    for (const text of EXPLANATIONS) expect(screen.queryByText(text)).toBeNull();
    const tips = [...page.querySelectorAll(".info-tip")] as HTMLElement[];
    expect(tips).toHaveLength(EXPLANATIONS.length);
    const shown: (string | null)[] = [];
    for (const tip of tips) {
      fireEvent.mouseEnter(tip);
      shown.push(screen.getByRole("tooltip").textContent);
      fireEvent.mouseLeave(tip);
    }
    expect(shown).toEqual(EXPLANATIONS);
    expect(page.querySelector(".check-pair__desc")).toBeNull();
    expect(page.querySelector(".toggle-switch__description")).toBeNull();
  });

  it("puts the Invert Sway icon right after its checkbox label, in its own row", async () => {
    render(tree());
    fireEvent.click(screen.getByText("Humanize"));
    await waitFor(() => expect(screen.getByText("Invert Sway")).toBeTruthy());
    const label = screen.getByText("Invert Sway").closest("label") as HTMLElement;
    expect(label.parentElement?.className).toBe("control-row hum-row__indent");
    expect(label.nextElementSibling?.className).toBe("info-tip");
  });
});

describe("PlaybackTab generate pedal events button follows the pedal style", () => {
  async function openPlaybackPage() {
    render(tree());
    fireEvent.click(screen.getByText("Playback"));
    await waitFor(() => expect(screen.getByText("Generate pedal events")).toBeTruthy());
  }

  function generateButton(): HTMLButtonElement {
    return screen.getByText("Generate pedal events") as HTMLButtonElement;
  }

  function chooseStyle(label: string) {
    const select = screen.getByText("Pedal Style").closest(".control-row")!.querySelector("select") as HTMLSelectElement;
    fireEvent.change(select, { target: { value: label } });
  }

  it("is enabled for the default style once the notes are compiled", async () => {
    engine.hasCompiledNotes = true;
    await openPlaybackPage();
    expect(generateButton().disabled).toBe(false);
  });

  it("is disabled when the pedal style is None", async () => {
    engine.hasCompiledNotes = true;
    await openPlaybackPage();
    chooseStyle("None");
    await waitFor(() => expect(generateButton().disabled).toBe(true));
  });

  it("is enabled again as soon as the style is anything but None", async () => {
    engine.hasCompiledNotes = true;
    await openPlaybackPage();
    chooseStyle("None");
    await waitFor(() => expect(generateButton().disabled).toBe(true));
    for (const label of ["PedalAI", "Adaptive", "Rhythmic", "Harmonic"]) {
      chooseStyle(label);
      await waitFor(() => expect(generateButton().disabled).toBe(false));
    }
  });

  it("stays disabled without compiled notes whatever the style", async () => {
    engine.hasCompiledNotes = false;
    await openPlaybackPage();
    expect(generateButton().disabled).toBe(true);
    chooseStyle("Rhythmic");
    await waitFor(() => expect(generateButton().disabled).toBe(true));
  });
});

describe("PlaybackTab humanize card reset buttons", () => {
  async function openHumanize() {
    render(tree());
    fireEvent.click(screen.getByText("Humanize"));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Vary Timing" })).toBeTruthy());
  }

  function switchNamed(name: string): HTMLElement {
    return screen.getByRole("switch", { name });
  }

  function cardReset(title: string): HTMLElement {
    const row = screen.getByText(title).closest(".card__title-row") as HTMLElement;
    return row.querySelector("button[title='Reset']") as HTMLElement;
  }

  it("gives Timing & Feel and Hands & Imperfection a Reset button in the title row", async () => {
    await openHumanize();
    expect(cardReset("TIMING & FEEL")).not.toBeNull();
    expect(cardReset("HANDS & IMPERFECTION")).not.toBeNull();
  });

  it("Timing & Feel reset turns its switches off and restores its sliders", async () => {
    await openHumanize();
    fireEvent.click(switchNamed("Vary Timing"));
    fireEvent.click(switchNamed("Tempo Sway"));
    await waitFor(() => expect(switchNamed("Vary Timing").getAttribute("aria-checked")).toBe("true"));
    const spinbox = document.querySelectorAll(".slider-spinbox__spinbox")[0] as HTMLInputElement;
    fireEvent.change(spinbox, { target: { value: "0.08" } });
    await waitFor(() => expect(spinbox.value).toBe("0.08"));
    fireEvent.click(cardReset("TIMING & FEEL"));
    await waitFor(() => expect(switchNamed("Vary Timing").getAttribute("aria-checked")).toBe("false"));
    expect(switchNamed("Tempo Sway").getAttribute("aria-checked")).toBe("false");
    expect(spinbox.value).toBe("0.01");
  });

  it("Timing & Feel reset also clears Invert Sway", async () => {
    await openHumanize();
    fireEvent.click(switchNamed("Tempo Sway"));
    const invert = screen.getByText("Invert Sway").closest("label")!.querySelector("input") as HTMLInputElement;
    await waitFor(() => expect(invert.disabled).toBe(false));
    fireEvent.click(invert);
    await waitFor(() => expect(invert.checked).toBe(true));
    fireEvent.click(cardReset("TIMING & FEEL"));
    await waitFor(() => expect(invert.checked).toBe(false));
  });

  it("Hands & Imperfection reset turns its switches off and restores its sliders", async () => {
    await openHumanize();
    fireEvent.click(switchNamed("Hand Drift"));
    fireEvent.click(switchNamed("Mistake Chance"));
    await waitFor(() => expect(switchNamed("Hand Drift").getAttribute("aria-checked")).toBe("true"));
    fireEvent.click(cardReset("HANDS & IMPERFECTION"));
    await waitFor(() => expect(switchNamed("Hand Drift").getAttribute("aria-checked")).toBe("false"));
    expect(switchNamed("Mistake Chance").getAttribute("aria-checked")).toBe("false");
  });

  it("each reset leaves the other card's settings alone", async () => {
    await openHumanize();
    fireEvent.click(switchNamed("Vary Timing"));
    fireEvent.click(switchNamed("Hand Drift"));
    await waitFor(() => expect(switchNamed("Hand Drift").getAttribute("aria-checked")).toBe("true"));
    fireEvent.click(cardReset("TIMING & FEEL"));
    await waitFor(() => expect(switchNamed("Vary Timing").getAttribute("aria-checked")).toBe("false"));
    expect(switchNamed("Hand Drift").getAttribute("aria-checked")).toBe("true");
    fireEvent.click(switchNamed("Vary Timing"));
    fireEvent.click(cardReset("HANDS & IMPERFECTION"));
    await waitFor(() => expect(switchNamed("Hand Drift").getAttribute("aria-checked")).toBe("false"));
    expect(switchNamed("Vary Timing").getAttribute("aria-checked")).toBe("true");
  });
});
