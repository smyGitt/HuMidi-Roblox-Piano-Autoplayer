import { SpinnerGapIcon } from "@phosphor-icons/react";

export function Spinner() {
  return (
    <span className="spinner" role="status" aria-label="Loading">
      <SpinnerGapIcon weight="duotone" />
    </span>
  );
}
