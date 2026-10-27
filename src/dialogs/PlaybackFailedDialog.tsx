import { MessageModal } from "../components/MessageModal";

const DIALOG_WIDTH = 480;
const DIALOG_HEIGHT = 260;

interface PlaybackFailedDialogProps {
  message: string;
  onDismiss: () => void;
}

export function PlaybackFailedDialog({ message, onDismiss }: PlaybackFailedDialogProps) {
  return (
    <MessageModal
      title="Cannot start playback"
      width={DIALOG_WIDTH}
      height={DIALOG_HEIGHT}
      body={message}
      buttons={[{ label: "OK", variant: "accent", onClick: onDismiss }]}
      onClose={onDismiss}
    />
  );
}
