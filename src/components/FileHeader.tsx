import { FloppyDiskIcon, FolderSimpleIcon, MusicNoteIcon, XSquareIcon } from "@phosphor-icons/react";
import { Button } from "./Button";
import { usePlaybackEngine } from "../state/PlaybackEngineContext";
import { isTauri } from "../lib/tauri";
import { pickMidiFile } from "../lib/pickMidiFile";

export function FileHeader() {
  const engine = usePlaybackEngine();
  const meta = engine.fileName ? `${engine.parts.length} track(s)` : "";

  function handleReplace() {
    if (isTauri()) {
      void engine.openFileBrowser();
      return;
    }
    pickMidiFile((name) => void engine.loadFile(name, name));
  }

  return (
    <div className="file-header">
      <div className="file-header__tile">
        <MusicNoteIcon weight="duotone" />
      </div>
      <div className="file-header__info">
        <span className="file-header__name">{engine.fileName || "No file loaded"}</span>
        {meta && <span className="file-header__meta">{meta}</span>}
      </div>
      <Button
        variant="icon"
        subtle
        size="md"
        onClick={() => void engine.save()}
        disabled={!engine.hasCompiledNotes}
        aria-label="Save playback"
        title="Save playback"
      >
        <FloppyDiskIcon weight="duotone" />
      </Button>
      <Button variant="icon" subtle size="md" onClick={handleReplace} aria-label="Replace">
        <FolderSimpleIcon weight="duotone" />
      </Button>
      <Button variant="icon" subtle size="md" onClick={() => void engine.clearSong()} aria-label="Clear">
        <XSquareIcon weight="duotone" />
      </Button>
    </div>
  );
}
