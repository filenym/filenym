import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import AppShell from "./AppShell";
import { isDesktop } from "../lib/tauri";
import { registryApi } from "../lib/registry";
vi.mock("../lib/tauri", () => ({ isDesktop: vi.fn(() => true) }));
vi.mock("../lib/registry", async (original) => ({
  ...(await original<typeof import("../lib/registry")>()),
  registryApi: {
    status: vi.fn(),
    list: vi.fn(),
    get: vi.fn(),
    select: vi.fn(),
    add: vi.fn(),
    rename: vi.fn(),
    delete: vi.fn(),
    decide: vi.fn(),
    reopen: vi.fn(),
  },
}));
const timestamp = { seconds: "0", nanoseconds: 0 };
const native = { encoding: "windowsUtf16" as const, units: [65], display: "A.txt", escaped: false };
const file = {
  id: "record",
  revision: "0",
  filename: native,
  savedPath: { ...native, display: "C:\\fixture\\A.txt" },
  sizeBytes: "18446744073709551615",
  modifiedAt: null,
  registeredAt: timestamp,
  lastObservedAt: timestamp,
  status: { kind: "available" as const },
  aliases: [],
};
function bootstrap() {
  vi.mocked(isDesktop).mockReturnValue(true);
  vi.mocked(registryApi.status).mockResolvedValue({
    storageEpoch: "epoch",
    value: { phase: "ready", code: null, affectedCanonicalIds: [] },
  });
  vi.mocked(registryApi.list).mockResolvedValue({ storageEpoch: "epoch", value: { entries: [] } });
}
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
describe("desktop registry", () => {
  it("preserves branding/version and compact empty registry", async () => {
    bootstrap();
    render(<AppShell version="0.1.0" />);
    expect(screen.getByRole("heading", { name: "FileNym" })).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "FileNym logo" })).toHaveAttribute(
      "src",
      "/filenym-logo.png",
    );
    expect(screen.getByText("One file. Many names.")).toBeInTheDocument();
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
  });
  it("normal selection is one command and commits before alias draft is shown", async () => {
    bootstrap();
    vi.mocked(registryApi.select).mockResolvedValue({
      storageEpoch: "epoch",
      value: { kind: "added", file },
    });
    vi.mocked(registryApi.add).mockResolvedValue({
      storageEpoch: "epoch",
      value: {
        ...file,
        revision: "1",
        aliases: [
          {
            id: "alias",
            canonicalFileId: "record",
            name: "Context.PDF",
            createdAt: timestamp,
            updatedAt: timestamp,
          },
        ],
      },
    });
    render(<AppShell version={null} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByText("Saved location");
    fireEvent.click(screen.getByRole("button", { name: "+ Add" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Alias name" }), {
      target: { value: "Context.PDF" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await screen.findByText("Context.PDF");
    expect(registryApi.add).toHaveBeenCalledWith("epoch", "record", "0", "Context.PDF");
    expect(registryApi.decide).not.toHaveBeenCalled();
  });
  it.each(["conflict", "busy"])(
    "%s preserves exact draft and revision until explicit review",
    async (code) => {
      bootstrap();
      vi.mocked(registryApi.select).mockResolvedValue({
        storageEpoch: "epoch",
        value: { kind: "added", file },
      });
      vi.mocked(registryApi.add).mockRejectedValue({
        code,
        field: null,
        reason: null,
        candidateIds: [],
        attempted: null,
        storageEpoch: "epoch",
      });
      render(<AppShell version={null} />);
      await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
      fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
      await screen.findByText("Saved location");
      fireEvent.click(screen.getByRole("button", { name: "+ Add" }));
      const input = screen.getByRole("textbox", { name: "Alias name" });
      fireEvent.change(input, { target: { value: "Draft.txt" } });
      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await screen.findByRole("alert");
      expect(input).toHaveValue("Draft.txt");
      expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
      expect(registryApi.add).toHaveBeenCalledTimes(1);
    },
  );
});

it("browser preview disables native operations without pretending to persist", async () => {
  vi.mocked(isDesktop).mockReturnValue(false);
  render(<AppShell version={null} />);
  await screen.findByText(/This browser preview does not save records/);
  expect(screen.getByRole("button", { name: "+ Add file" })).toBeDisabled();
  expect(registryApi.status).not.toHaveBeenCalled();
});
it("untrusted display strings remain literal text", async () => {
  bootstrap();
  const label = "<img src=x onerror=alert(1)>\u202e.txt";
  vi.mocked(registryApi.select).mockResolvedValue({
    storageEpoch: "epoch",
    value: { kind: "added", file: { ...file, filename: { ...native, display: label } } },
  });
  const view = render(<AppShell version={null} />);
  await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
  await screen.findByText("Saved location");
  expect(view.container.querySelector('img[src="x"]')).toBeNull();
  expect(screen.getAllByText(label).length).toBeGreaterThan(0);
});
function modalUnit() {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, "showModal");
  Object.defineProperty(HTMLDialogElement.prototype, "showModal", {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  return () => {
    if (descriptor) Object.defineProperty(HTMLDialogElement.prototype, "showModal", descriptor);
    else Reflect.deleteProperty(HTMLDialogElement.prototype, "showModal");
  };
}
it("duplicate decisions cancel, view and save separately only on explicit choice", async () => {
  const restore = modalUnit();
  try {
    bootstrap();
    vi.mocked(registryApi.select).mockResolvedValue({
      storageEpoch: "epoch",
      value: {
        kind: "duplicateDecision",
        selectionToken: "token",
        filename: native,
        duplicateCandidateIds: ["record"],
      },
    });
    vi.mocked(registryApi.decide)
      .mockResolvedValueOnce({ storageEpoch: "epoch", value: { kind: "cancelled" } })
      .mockResolvedValueOnce({ storageEpoch: "epoch", value: { kind: "viewExisting", file } })
      .mockResolvedValueOnce({
        storageEpoch: "epoch",
        value: { kind: "added", file: { ...file, id: "separate" } },
      });
    render(<AppShell version={null} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByRole("dialog");
    expect(registryApi.decide).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "View existing 1" }));
    await screen.findByText("Saved location");
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "Add separately" }));
    await screen.findByText("2 registered");
    expect(registryApi.decide).toHaveBeenNthCalledWith(1, "epoch", "token", "cancel", null);
    expect(registryApi.decide).toHaveBeenNthCalledWith(
      2,
      "epoch",
      "token",
      "viewExisting",
      "record",
    );
    expect(registryApi.decide).toHaveBeenNthCalledWith(3, "epoch", "token", "addSeparately", null);
  } finally {
    restore();
  }
});
it("inline rename and safe delete use captured revisions and keep the parent", async () => {
  const restore = modalUnit();
  try {
    bootstrap();
    const alias = {
      id: "alias",
      canonicalFileId: "record",
      name: "Original.pdf",
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    const original = { ...file, aliases: [alias] };
    vi.mocked(registryApi.select).mockResolvedValue({
      storageEpoch: "epoch",
      value: { kind: "added", file: original },
    });
    vi.mocked(registryApi.rename).mockResolvedValue({
      storageEpoch: "epoch",
      value: { ...original, revision: "1", aliases: [{ ...alias, name: "Renamed.PDF" }] },
    });
    vi.mocked(registryApi.delete).mockResolvedValue({
      storageEpoch: "epoch",
      value: { ...file, revision: "2" },
    });
    render(<AppShell version={null} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByText("Saved location");
    fireEvent.click(screen.getByLabelText("Actions for Original.pdf"));
    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    fireEvent.change(screen.getByRole("textbox", { name: "Alias name" }), {
      target: { value: "Renamed.PDF" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    await screen.findByText("Renamed.PDF");
    expect(registryApi.rename).toHaveBeenCalledWith("epoch", "record", "0", "alias", "Renamed.PDF");
    fireEvent.click(screen.getByLabelText("Actions for Renamed.PDF"));
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await screen.findByRole("dialog");
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(registryApi.delete).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "Delete alias" }));
    await screen.findByText("No aliases yet. Add a name for this context.");
    expect(registryApi.delete).toHaveBeenCalledWith("epoch", "record", "1", "alias");
    expect(screen.getByText("Saved location")).toBeInTheDocument();
  } finally {
    restore();
  }
});
it("an unknown duplicate save receipt dismisses the dialog so explicit recovery stays reachable", async () => {
  const restore = modalUnit();
  try {
    bootstrap();
    vi.mocked(registryApi.select).mockResolvedValue({
      storageEpoch: "epoch",
      value: {
        kind: "duplicateDecision",
        selectionToken: "token",
        filename: native,
        duplicateCandidateIds: ["record"],
      },
    });
    vi.mocked(registryApi.decide).mockRejectedValue(new Error("native detail"));
    render(<AppShell version={null} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "+ Add file" })).toBeEnabled());
    fireEvent.click(screen.getByRole("button", { name: "+ Add file" }));
    await screen.findByRole("dialog");
    fireEvent.click(screen.getByRole("button", { name: "Add separately" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(screen.getByRole("button", { name: "Reopen and review saved registry" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "+ Add file" })).toBeDisabled();
  } finally {
    restore();
  }
});
