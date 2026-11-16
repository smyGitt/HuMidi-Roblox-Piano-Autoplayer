import { ArrowCounterClockwiseIcon } from "@phosphor-icons/react";
import { Button } from "./Button";

export function ResetButton({ onReset }: { onReset: () => void }) {
  return (
    <Button variant="icon" subtle danger onClick={onReset} title="Reset">
      <ArrowCounterClockwiseIcon weight="fill" />
    </Button>
  );
}
