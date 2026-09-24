import { Button } from "../../components/Button";

interface ApplyToastProps {
  visible: boolean;
  onApply: () => void;
  onDiscard: () => void;
  shake?: boolean;
}

export function ApplyToast({ visible, onApply, onDiscard, shake }: ApplyToastProps) {
  return (
    <div className={`apply-toast${visible ? " apply-toast--visible" : ""}${shake ? " apply-toast--shake" : ""}`}>
      <span className="apply-toast__msg">Your changes aren't reflected in the compiled playback yet.</span>
      <Button variant="accent" onClick={onApply}>
        Apply
      </Button>
      <Button onClick={onDiscard}>
        Discard
      </Button>
    </div>
  );
}
