import { MessageModal } from "../components/MessageModal";

const DIALOG_WIDTH = 480;
const DIALOG_HEIGHT = 260;

interface BlockedInputDialogProps {
  message: string;
  onDismiss: () => void;
}

export function BlockedInputDialog({ message, onDismiss }: BlockedInputDialogProps) {
  return (
    <MessageModal
      title="Load blocked"
      width={DIALOG_WIDTH}
      height={DIALOG_HEIGHT}
      body={`Loading was stopped because the file contains input that is not a piano key press. ${message}`}
      buttons={[{ label: "OK", variant: "accent", onClick: onDismiss }]}
      onClose={onDismiss}
    />
  );
}
