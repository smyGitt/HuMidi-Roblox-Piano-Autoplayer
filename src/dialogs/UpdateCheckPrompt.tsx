import { Modal } from "../components/Modal";

interface UpdateCheckPromptProps {
  onChoice: (v: boolean) => void;
}

export function UpdateCheckPrompt({ onChoice }: UpdateCheckPromptProps) {
  return (
    <Modal
      title="Check for update on startup"
      onClose={() => {}}
      width={420}
      height={220}
      footer={
        <>
          <button className="modal__btn" onClick={() => onChoice(false)}>
            Nah
          </button>
          <button className="modal__btn modal__btn--accent" onClick={() => onChoice(true)}>
            Sure
          </button>
        </>
      }
    >
      <p className="update-check-prompt__text">
        Enable automatically checking for update when you start the application?
        <br />
        NOTE: You can change this setting whenever you want in the settings.
      </p>
    </Modal>
  );
}
