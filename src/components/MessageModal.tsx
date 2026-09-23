import type { ReactNode } from "react";
import { Modal } from "./Modal";
import type { Icon as PhosphorIcon } from "@phosphor-icons/react";

export interface MessageModalButton {
  label: string;
  onClick: () => void;
  variant?: "default" | "accent";
  leadingIcon?: PhosphorIcon;
  trailingIcon?: PhosphorIcon;
  iconSize?: number;
  disabled?: boolean;
}

interface MessageModalProps {
  title: string;
  body: ReactNode;
  buttons: MessageModalButton[];
  onClose?: () => void;
  width?: number;
  height?: number;
}

export function MessageModal({ title, body, buttons, onClose, width = 420, height = 220 }: MessageModalProps) {
  return (
    <Modal
      title={title}
      onClose={onClose ?? (() => {})}
      width={width}
      height={height}
      footer={
        <>
          {buttons.map((b) => (
            <button
              key={b.label}
              className={`modal__btn${b.variant === "accent" ? " modal__btn--accent" : ""}`}
              onClick={b.onClick}
              disabled={b.disabled}
            >
              {b.leadingIcon && <b.leadingIcon size={b.iconSize ?? 20} weight="duotone" />}
              {b.label}
              {b.trailingIcon && <b.trailingIcon size={b.iconSize ?? 20} weight="duotone" />}
            </button>
          ))}
        </>
      }
    >
      <p className="message-modal__text">{body}</p>
    </Modal>
  );
}
