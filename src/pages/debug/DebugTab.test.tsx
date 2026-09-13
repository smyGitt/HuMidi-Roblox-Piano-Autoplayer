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
