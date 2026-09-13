import type { ReactNode } from "react";

interface ModalProps {
  title: string;
  onClose: () => void;
  footer?: ReactNode;
  width?: number;
  height?: number;
  children: ReactNode;
}

export function Modal({ title, onClose, footer, width = 600, height = 400, children }: ModalProps) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className="modal"
        style={{ width, height }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal__title-row">
          <span className="modal__title">{title}</span>
          <button className="modal__close-btn" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="modal__body">{children}</div>
        {footer && <div className="modal__footer">{footer}</div>}
      </div>
    </div>
  );
}
