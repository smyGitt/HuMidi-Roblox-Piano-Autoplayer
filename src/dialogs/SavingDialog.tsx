import { Modal } from "../components/Modal";
import { Spinner } from "../components/Spinner";

const SAVING_DIALOG_WIDTH = 320;
const SAVING_DIALOG_HEIGHT = 240;

export function SavingDialog() {
  return (
    <Modal title="Saving..." onClose={() => {}} hideClose width={SAVING_DIALOG_WIDTH} height={SAVING_DIALOG_HEIGHT}>
      <div className="saving-dialog">
        <Spinner />
        <span className="saving-dialog__note">
          Saving might take long if you have PedalAI selected, and have not generated pedal events yet.
        </span>
      </div>
    </Modal>
  );
}
