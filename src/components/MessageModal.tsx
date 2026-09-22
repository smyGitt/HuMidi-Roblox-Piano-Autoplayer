import type { ReactNode } from "react";
import { Modal } from "./Modal";
import { Icon, type IconName } from "./Icon";

export interface MessageModalButton {
  label: string;
  onClick: () => void;
  variant?: "default" | "accent";
  leadingIcon?: IconName;
  trailingIcon?: IconName;
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
              {b.leadingIcon && <Icon name={b.leadingIcon} size={b.iconSize} />}
              {b.label}
              {b.trailingIcon && <Icon name={b.trailingIcon} size={b.iconSize} />}
            </button>
          ))}
        </>
      }
    >
      <p className="message-modal__text">{body}</p>
    </Modal>
  );
}
