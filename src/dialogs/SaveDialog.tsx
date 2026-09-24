import { useState } from "react";
import { Modal } from "../components/Modal";
import { Button } from "../components/Button";
import { TextInput } from "../components/Field";

const SAVE_DIALOG_WIDTH = 420;
const SAVE_DIALOG_HEIGHT = 220;

interface SaveDialogProps {
  initialName: string;
  error: string | null;
  onCancel: () => void;
  onSave: (name: string) => void;
}

export function SaveDialog({ initialName, error, onCancel, onSave }: SaveDialogProps) {
  const [name, setName] = useState(initialName);

  return (
    <Modal
      title="Save playback"
      onClose={onCancel}
      hideClose
      width={SAVE_DIALOG_WIDTH}
      height={SAVE_DIALOG_HEIGHT}
      footer={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button variant="accent" onClick={() => onSave(name)}>
            Save
          </Button>
        </>
      }
    >
      <div className="save-dialog">
        <TextInput
          autoFocus
          onFocus={(e) => e.target.select()}
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onSave(name);
          }}
        />
        {error && <span className="save-dialog__error">{error}</span>}
      </div>
    </Modal>
  );
}
