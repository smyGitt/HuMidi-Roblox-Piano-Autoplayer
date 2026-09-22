import { Modal } from "../components/Modal";

interface UpdateCheckPromptProps {
  onChoice: (v: boolean) => void;
}

export function UpdateCheckPrompt({ onChoice }: UpdateCheckPromptProps) {
  return (
    <Modal
      title="Check for updates on startup?"
      onClose={() => {}}
      width={420}
      height={160}
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
      <p>Check for updates on startup?</p>
    </Modal>
  );
}
