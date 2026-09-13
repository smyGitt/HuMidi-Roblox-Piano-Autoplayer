import { useState } from "react";
import { Modal } from "../components/Modal";

export type HandRole = "Auto-Detect" | "Left Hand" | "Right Hand";

export interface TrackRow {
  index: number;
  name: string;
  instrument: string;
  noteCount: number;
  channel: number;
}

interface TrackSelectionRowState {
  included: boolean;
  role: HandRole;
}

interface TrackSelectionDialogProps {
  tracks: TrackRow[];
  onCancel: () => void;
  onConfirm: (selection: { index: number; role: HandRole }[]) => void;
}

export function TrackSelectionDialog({ tracks, onCancel, onConfirm }: TrackSelectionDialogProps) {
  const [rows, setRows] = useState<Record<number, TrackSelectionRowState>>(() =>
    Object.fromEntries(
      tracks.map((t) => [t.index, { included: t.channel !== 9, role: "Auto-Detect" as HandRole }]),
    ),
  );

  function updateRow(index: number, patch: Partial<TrackSelectionRowState>) {
    setRows((r) => ({ ...r, [index]: { ...r[index], ...patch } }));
  }

  return (
    <Modal
      title="Select Tracks"
      onClose={onCancel}
      width={720}
      height={400}
      footer={
        <>
          <button className="modal__btn" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="modal__btn modal__btn--accent"
            onClick={() =>
              onConfirm(
                tracks
                  .filter((t) => rows[t.index].included)
                  .map((t) => ({ index: t.index, role: rows[t.index].role })),
              )
            }
          >
            OK
          </button>
        </>
      }
    >
      <p className="track-selection__info">
        Choose which tracks to play and override hand assignment where needed. Drum tracks are excluded by default.
      </p>
      <table className="track-selection__table">
        <thead>
          <tr>
            <th>Play</th>
            <th>Track Name</th>
            <th>Instrument</th>
            <th>Notes</th>
            <th>Hand Assignment</th>
          </tr>
        </thead>
        <tbody>
          {tracks.map((t) => (
            <tr key={t.index}>
              <td>
                <input
                  type="checkbox"
                  checked={rows[t.index].included}
                  onChange={(e) => updateRow(t.index, { included: e.target.checked })}
                />
              </td>
              <td>{t.name}</td>
              <td>{t.instrument}</td>
              <td>{t.noteCount}</td>
              <td>
                <select
                  value={rows[t.index].role}
                  onChange={(e) => updateRow(t.index, { role: e.target.value as HandRole })}
                >
                  <option>Auto-Detect</option>
                  <option>Left Hand</option>
                  <option>Right Hand</option>
                </select>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Modal>
  );
}
