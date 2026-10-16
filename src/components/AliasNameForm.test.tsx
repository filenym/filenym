import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, it, expect, vi } from "vitest";
import AliasNameForm from "./AliasNameForm";
import Confirmation from "./Confirmation";
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
it("inline rename preserves spelling and supplied revision, with Escape and IME safety", async () => {
  const save = vi.fn().mockResolvedValue(false),
    cancel = vi.fn();
  render(
    <AliasNameForm
      initialName="Original.PDF"
      initialRevision="9007199254740993"
      disabled={false}
      needsReview={false}
      onReview={vi.fn()}
      onSave={save}
      onCancel={cancel}
    />,
  );
  const input = screen.getByRole("textbox");
  fireEvent.change(input, { target: { value: "Original.pdf" } });
  fireEvent.keyDown(input, { key: "Enter", isComposing: true });
  expect(save).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  await waitFor(() => expect(save).toHaveBeenCalledWith("Original.pdf", "9007199254740993"));
  expect(input).toHaveValue("Original.pdf");
  fireEvent.keyDown(input, { key: "Escape" });
  expect(cancel).toHaveBeenCalledTimes(1);
});
it("invalid form never dispatches and keeps focus", () => {
  const save = vi.fn();
  render(
    <AliasNameForm
      initialName=""
      initialRevision="0"
      disabled={false}
      needsReview={false}
      onReview={vi.fn()}
      onSave={save}
      onCancel={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByRole("textbox"), { target: { value: "../bad" } });
  fireEvent.click(screen.getByRole("button", { name: "Save" }));
  expect(save).not.toHaveBeenCalled();
  expect(screen.getByRole("textbox")).toHaveFocus();
  expect(screen.getByRole("textbox")).toHaveAttribute("aria-invalid", "true");
});
it("confirmation opens a native modal and places Cancel at the safe default", () => {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
  const show = vi.fn(function (this: HTMLDialogElement) {
    this.open = true;
  });
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: show,
  });
  const cancel = vi.fn();
  try {
    render(
      <Confirmation title="Delete alias?" onCancel={cancel}>
        <p>The original stays unchanged.</p>
        <button>Delete alias</button>
      </Confirmation>,
    );
    expect(show).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("dialog")).toHaveAccessibleName("Delete alias?");
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("button", { name: "Cancel" }), { key: "Tab" });
    expect(screen.getByRole("button", { name: "Delete alias" })).toHaveFocus();
    fireEvent.keyDown(screen.getByRole("button", { name: "Delete alias" }), {
      key: "Tab",
      shiftKey: true,
    });
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(cancel).toHaveBeenCalledTimes(1);
  } finally {
    if (descriptor) Object.defineProperty(HTMLDialogElement.prototype, "showModal", descriptor);
    else Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  }
});
it("confirmation returns focus to its connected trigger after unmount", async () => {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  const trigger = document.createElement("button");
  trigger.textContent = "Open confirmation";
  document.body.append(trigger);
  trigger.focus();
  try {
    const view = render(
      <Confirmation title="Confirm?" onCancel={vi.fn()}>
        <p>Owned fixture.</p>
      </Confirmation>,
    );
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    view.unmount();
    await waitFor(() => expect(trigger).toHaveFocus());
  } finally {
    trigger.remove();
    if (descriptor) Object.defineProperty(HTMLDialogElement.prototype, "showModal", descriptor);
    else Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  }
});
