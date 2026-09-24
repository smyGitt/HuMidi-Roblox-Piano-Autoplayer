import { describe, expect, it } from "vitest";
import { defaultSaveName, saveTimestamp } from "./saveName";
import { stripMidiExtension } from "./midiName";

describe("saveTimestamp", () => {
  it("formats local time as YYYYMMDD_HHMMSS with zero padding", () => {
    expect(saveTimestamp(new Date(2026, 0, 5, 3, 4, 9))).toBe("20260105_030409");
  });

  it("uses 24 hour digits", () => {
    expect(saveTimestamp(new Date(2026, 8, 23, 15, 45, 12))).toBe("20260923_154512");
  });
});

describe("defaultSaveName", () => {
  it("is the file name without its MIDI extension plus the timestamp suffix", () => {
    expect(defaultSaveName("song.mid", new Date(2026, 8, 23, 15, 45, 12))).toBe("song_20260923_154512");
    expect(defaultSaveName("Song.MIDI", new Date(2026, 8, 23, 15, 45, 12))).toBe("Song_20260923_154512");
  });

  it("keeps a name that has no MIDI extension", () => {
    expect(defaultSaveName("notes", new Date(2026, 0, 1, 0, 0, 0))).toBe("notes_20260101_000000");
  });
});

describe("stripMidiExtension", () => {
  it("only strips a trailing .mid or .midi", () => {
    expect(stripMidiExtension("a.mid.txt")).toBe("a.mid.txt");
    expect(stripMidiExtension("a.mid")).toBe("a");
    expect(stripMidiExtension("a.midi")).toBe("a");
  });
});
