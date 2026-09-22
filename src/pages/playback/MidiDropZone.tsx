import { useState, type DragEvent } from "react";
import { Card } from "../../components/Card";
import { Icon } from "../../components/Icon";
import { isTauri } from "../../lib/tauri";

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
        <Icon name="folder-open" size={48} />
        <span className="midi-drop-zone__hint">Drop a .mid file</span>
        <span className="midi-drop-zone__sub">or use the buttons below</span>
        <div className="midi-drop-zone__buttons">
          <button
            className="midi-drop-zone__btn"
            onClick={() => {
              if (isTauri()) {
                onBrowse();
                return;
              }
              const input = document.createElement("input");
              input.type = "file";
              input.accept = ".mid,.midi";
              input.onchange = () => {
                const file = input.files?.[0];
                if (file) onFileChosen(file.name);
              };
              input.click();
            }}
          >
            Browse...
          </button>
          <button className="midi-drop-zone__btn" onClick={onLoadSaved}>
            Load Save
          </button>
        </div>
      </div>
    </Card>
  );
}
