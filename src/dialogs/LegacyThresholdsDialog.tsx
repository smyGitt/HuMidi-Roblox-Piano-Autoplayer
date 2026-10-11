import { MessageModal } from "../components/MessageModal";

const DIALOG_WIDTH = 480;
const DIALOG_HEIGHT = 260;

interface LegacyThresholdsDialogProps {
  onDismiss: () => void;
}

export function LegacyThresholdsDialog({ onDismiss }: LegacyThresholdsDialogProps) {
  return (
    <MessageModal
      title="Older save"
      width={DIALOG_WIDTH}
      height={DIALOG_HEIGHT}
      body="This save was made with a single On and Off threshold, so both were loaded as flat lines. Threshold curves are now supported in the Pedal AI card."
      buttons={[{ label: "OK", variant: "accent", onClick: onDismiss }]}
      onClose={onDismiss}
    />
  );
}
