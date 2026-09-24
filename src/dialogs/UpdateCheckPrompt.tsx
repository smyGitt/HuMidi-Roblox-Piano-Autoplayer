import { Modal } from "../components/Modal";
import { Button } from "../components/Button";

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
          <Button onClick={() => onChoice(false)}>
            Nah
          </Button>
          <Button variant="accent" onClick={() => onChoice(true)}>
            Sure
          </Button>
        </>
      }
    >
      <p className="message-modal__text">
        Enable automatically checking for update when you start the application?
        <br />
        NOTE: You can change this setting whenever you want in the settings.
      </p>
    </Modal>
  );
}
