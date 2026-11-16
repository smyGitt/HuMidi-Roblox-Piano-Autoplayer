import { CopySimpleIcon } from "@phosphor-icons/react";
import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LogProvider, useLog } from "../../state/LogContext";
import { DebugTab } from "./DebugTab";

function Harness({ onReady }: { onReady: (log: ReturnType<typeof useLog>) => void }) {
  const log = useLog();
  onReady(log);
  return <DebugTab />;
}

function renderDebugTab() {
  let log!: ReturnType<typeof useLog>;
  const utils = render(
    <LogProvider>
      <Harness onReady={(l) => (log = l)} />
    </LogProvider>,
  );
  return { ...utils, get log() { return log; } };
}

describe("DebugTab > Session Snapshot", () => {
  it("shows a dash for every row before anything is loaded", () => {
    renderDebugTab();
    const dashes = screen.getAllByText("-");
    expect(dashes.length).toBe(8);
  });

  it("reflects real snapshot values once updateSnapshot is called", () => {
    const { log } = renderDebugTab();
    act(() => log.updateSnapshot({ file: "song.mid", source: "MIDI file", tracks: 3, notes: 214 }));
    expect(screen.getByText("song.mid")).toBeInTheDocument();
    expect(screen.getByText("MIDI file")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("214")).toBeInTheDocument();
  });

  it("clearSnapshot puts every row back to a dash", () => {
    const { log } = renderDebugTab();
    act(() => log.updateSnapshot({ file: "song.mid", tracks: 3 }));
    act(() => log.clearSnapshot());
    const dashes = screen.getAllByText("-");
    expect(dashes.length).toBe(8);
  });
});

describe("DebugTab > Copy Log button", () => {
  function pathsOf(html: string): string[] {
    return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
  }

  it("is an icon-only button with the filled copy icon", () => {
    renderDebugTab();
    const copy = screen.getByLabelText("Copy Log") as HTMLElement;
    expect(copy.textContent).toBe("");
    const expected = render(<CopySimpleIcon weight="fill" />);
    expect(pathsOf(copy.innerHTML)).toEqual(pathsOf(expected.container.innerHTML));
    expect(screen.queryByText("Copy Log")).toBeNull();
  });
});

describe("DebugTab > right side panel", () => {
  it("has the Playback-style right panel holding Filter, Levels and Session Snapshot", () => {
    renderDebugTab();
    const panel = document.querySelector("aside.side-panel") as HTMLElement;
    expect(panel).not.toBeNull();
    const titles = [...panel.querySelectorAll(".card__title")].map((title) => title.textContent);
    expect(titles).toEqual(["Filter", "Levels", "Session Snapshot"]);
  });

  it("keeps the log console and the footer buttons outside the panel", () => {
    renderDebugTab();
    const panel = document.querySelector("aside.side-panel") as HTMLElement;
    expect(panel.querySelector(".debug-tab__console")).toBeNull();
    expect(document.querySelector(".debug-tab__main .debug-tab__console")).not.toBeNull();
    expect(document.querySelector(".debug-tab__main .debug-tab__footer")).toBeNull();
  });

  it("no longer has the old fixed column next to the console", () => {
    renderDebugTab();
    expect(document.querySelector(".debug-tab__body > .debug-tab__sidebar")).toBeNull();
  });
});

describe("DebugTab > console card footer", () => {
  it("holds Clear, Copy Log and Export Log in the footer of the log card", () => {
    renderDebugTab();
    const card = document.querySelector(".debug-tab__console-card") as HTMLElement;
    const footer = card.querySelector(".card__footer") as HTMLElement;
    expect(footer).not.toBeNull();
    expect(footer.contains(screen.getByText("Clear"))).toBe(true);
    expect(footer.contains(screen.getByLabelText("Copy Log"))).toBe(true);
    expect(footer.contains(screen.getByText("Export Log"))).toBe(true);
    expect(card.querySelector(".card__body")?.contains(footer)).toBe(false);
  });

  it("has no separate footer row under the card any more", () => {
    renderDebugTab();
    expect(document.querySelector(".debug-tab__footer")).toBeNull();
  });
});
