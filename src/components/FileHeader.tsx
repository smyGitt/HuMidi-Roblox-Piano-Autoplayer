import { useState } from "react";
import { FloppyDiskIcon, FolderSimpleIcon, MusicNoteIcon, XSquareIcon } from "@phosphor-icons/react";
import { Button } from "./Button";
import { usePlaybackEngine } from "../state/PlaybackEngineContext";
import { checkSaveName, isTauri } from "../lib/tauri";
import { defaultSaveName } from "../lib/saveName";
import { SaveDialog } from "../dialogs/SaveDialog";
import { SavingDialog } from "../dialogs/SavingDialog";
import { pickMidiFile } from "../lib/pickMidiFile";
import { Label } from "./Label";

export function FileHeader() {
  const engine = usePlaybackEngine();
  const [saveDialogOpen, setSaveDialogOpen] = useState(false);
  const [saveDialogName, setSaveDialogName] = useState("");
  const [saveError, setSaveError] = useState<string | null>(null);
  const meta = engine.fileName ? `${engine.parts.length} track(s)` : "";

  function openSaveDialog() {
    setSaveDialogName(defaultSaveName(engine.fileName));
    setSaveError(null);
    setSaveDialogOpen(true);
  }

  async function handleSave(name: string) {
    let finalName = name;
    if (isTauri()) {
      try {
        finalName = await checkSaveName(name);
      } catch (e) {
        setSaveError(String(e));
        return;
      }
    }
    setSaveDialogOpen(false);
    const result = await engine.save(finalName);
    if ("error" in result) {
      setSaveDialogName(finalName);
      setSaveError(result.error);
      setSaveDialogOpen(true);
    }
  }

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
        <Label className="file-header__name" slide>{engine.fileName || "No file loaded"}</Label>
        {meta && <Label className="file-header__meta">{meta}</Label>}
      </div>
      <Button
        variant="icon"
        subtle
        size="md"
        onClick={openSaveDialog}
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
      {saveDialogOpen && (
        <SaveDialog
          initialName={saveDialogName}
          error={saveError}
          onCancel={() => setSaveDialogOpen(false)}
          onSave={(name) => void handleSave(name)}
        />
      )}
      {engine.isSaving && <SavingDialog />}
    </div>
  );
}
