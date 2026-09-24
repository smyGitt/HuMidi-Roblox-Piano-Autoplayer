import { useMemo, useState } from "react";
import { Modal } from "../components/Modal";
import { Button } from "../components/Button";
import { Label } from "../components/Label";

export interface SaveEntry {
  filepath: string;
  sourceMidiName: string;
  saveName: string;
  createdAt: string;
  tempo: number;
  pedalStyle: string;
  use88Key: boolean;
  humanization: string[];
}

interface LoadSaveDialogProps {
  saves: SaveEntry[];
  initialSelected?: string | null;
  onCancel: () => void;
  onLoad: (save: SaveEntry) => void;
  onRename: (filepath: string, newName: string) => void;
  onDelete: (filepath: string) => void;
}

export function LoadSaveDialog({ saves, initialSelected = null, onCancel, onLoad, onRename, onDelete }: LoadSaveDialogProps) {
  const [selected, setSelected] = useState<string | null>(initialSelected);

  const groups = useMemo(() => {
    const map = new Map<string, SaveEntry[]>();
    for (const s of saves) {
      const list = map.get(s.sourceMidiName) ?? [];
      list.push(s);
      map.set(s.sourceMidiName, list);
    }
    return Array.from(map.entries());
  }, [saves]);

  const selectedSave = saves.find((s) => s.filepath === selected) ?? null;

  return (
    <Modal
      title="Load Save"
      onClose={onCancel}
      width={820}
      height={520}
      footer={
        <>
          <Button disabled={!selectedSave} onClick={() => selectedSave && onRename(selectedSave.filepath, prompt("New name", selectedSave.saveName) ?? selectedSave.saveName)}>
            Rename
          </Button>
          <Button disabled={!selectedSave}
            onClick={() =>
              selectedSave &&
              window.confirm(`Delete "${selectedSave.saveName}"? This cannot be undone.`) &&
              onDelete(selectedSave.filepath)
            }
          >
            Delete
          </Button>
          <Button onClick={onCancel}>
            Cancel
          </Button>
          <Button
            variant="accent"
            disabled={!selectedSave}
            onClick={() => selectedSave && onLoad(selectedSave)}
          >
            Load
          </Button>
        </>
      }
    >
      <div className="load-save-dialog">
        <div className="load-save-dialog__tree">
          {groups.length === 0 && <span className="load-save-dialog__empty">No saves found.</span>}
          {groups.map(([midiName, entries]) => (
            <div key={midiName} className="load-save-dialog__group">
              <Label className="load-save-dialog__group-label">{midiName}</Label>
              {entries.map((s) => (
                <Button
                  key={s.filepath}
                  variant="item"
                  active={selected === s.filepath}
                  onClick={() => setSelected(s.filepath)}
                >
                  <Label>{s.saveName}</Label>
                </Button>
              ))}
            </div>
          ))}
        </div>
        <div className="load-save-dialog__details">
          {selectedSave ? (
            <>
              <div className="load-save-dialog__details-title">{selectedSave.sourceMidiName}</div>
              <div className="load-save-dialog__details-meta">{selectedSave.createdAt}</div>
              <div className="load-save-dialog__details-grid">
                <span>Tempo</span>
                <span>{selectedSave.tempo}%</span>
                <span>Pedal Style</span>
                <span>{selectedSave.pedalStyle}</span>
                <span>88-Key</span>
                <span>{selectedSave.use88Key ? "Yes" : "No"}</span>
              </div>
              <div className="load-save-dialog__details-subhead">Humanization</div>
              {selectedSave.humanization.length === 0 ? (
                <span className="load-save-dialog__placeholder">None selected</span>
              ) : (
                <ul className="load-save-dialog__hum-list">
                  {selectedSave.humanization.map((h) => (
                    <li key={h}>{h}</li>
                  ))}
                </ul>
              )}
            </>
          ) : (
            <span className="load-save-dialog__placeholder">Select a save to see its details.</span>
          )}
        </div>
      </div>
    </Modal>
  );
}
