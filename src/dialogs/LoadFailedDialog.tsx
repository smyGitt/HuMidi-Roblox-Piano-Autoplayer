import { MessageModal } from "../components/MessageModal";

const DIALOG_WIDTH = 480;
const DIALOG_HEIGHT = 260;

interface LoadFailedDialogProps {
  message: string;
  onDismiss: () => void;
}

export function LoadFailedDialog({ message, onDismiss }: LoadFailedDialogProps) {
  return (
    <MessageModal
      title="Cannot open for editing"
      width={DIALOG_WIDTH}
      height={DIALOG_HEIGHT}
      body={message}
      buttons={[{ label: "OK", variant: "accent", onClick: onDismiss }]}
      onClose={onDismiss}
    />
  );
}
