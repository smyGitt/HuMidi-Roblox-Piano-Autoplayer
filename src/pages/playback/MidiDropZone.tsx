import { useState, type DragEvent } from "react";
import { Card } from "../../components/Card";
import { FolderOpenIcon } from "@phosphor-icons/react";
import { isTauri } from "../../lib/tauri";
import { pickMidiFile } from "../../lib/pickMidiFile";
import { Button } from "../../components/Button";

interface MidiDropZoneProps {
  onFileChosen: (name: string) => void;
  onLoadSaved: () => void;
  onBrowse: () => void;
}

function isMidiFile(name: string) {
  const lower = name.toLowerCase();
  return lower.endsWith(".mid") || lower.endsWith(".midi");
}

export function MidiDropZone({ onFileChosen, onLoadSaved, onBrowse }: MidiDropZoneProps) {
  const [dragActive, setDragActive] = useState(false);

  function handleDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragActive(false);
    const file = e.dataTransfer.files[0];
    if (file && isMidiFile(file.name)) onFileChosen(file.name);
  }

  return (
    <Card title="REPLACE" dashed dragActive={dragActive}>
      <div
        className="midi-drop-zone"
        onDragOver={(e) => {
          e.preventDefault();
          setDragActive(true);
        }}
        onDragLeave={() => setDragActive(false)}
        onDrop={handleDrop}
      >
        <FolderOpenIcon size={48} weight="duotone" />
        <span className="midi-drop-zone__hint">Drop a .mid file</span>
        <span className="midi-drop-zone__sub">or use the buttons below</span>
        <div className="midi-drop-zone__buttons">
          <Button onClick={() => (isTauri() ? onBrowse() : pickMidiFile(onFileChosen))}>
            Browse...
          </Button>
          <Button onClick={onLoadSaved}>
            Load Save
          </Button>
        </div>
      </div>
    </Card>
  );
}
