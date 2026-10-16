import { useEffect, useRef, type ReactNode } from "react";
export default function Confirmation({
  title,
  children,
  onCancel,
  disabled = false,
}: {
  title: string;
  children: ReactNode;
  onCancel: () => void;
  disabled?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement;
    const dialog = ref.current;
    dialog?.showModal();
    cancel.current?.focus();
    return () => {
      requestAnimationFrame(() => {
        if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
      });
    };
  }, []);
  return (
    <dialog
      ref={ref}
      aria-labelledby="confirmation-title"
      onKeyDown={(e) => {
        if (e.key !== "Tab") return;
        const buttons = Array.from(
          e.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"),
        );
        const first = buttons[0];
        const last = buttons[buttons.length - 1];
        if (!first) {
          e.preventDefault();
        } else if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }}
      onCancel={(e) => {
        e.preventDefault();
        if (!disabled) onCancel();
      }}
    >
      <h2 id="confirmation-title">{title}</h2>
      {children}
      <div className="dialog-actions">
        <button ref={cancel} disabled={disabled} onClick={onCancel}>
          Cancel
        </button>
      </div>
    </dialog>
  );
}
