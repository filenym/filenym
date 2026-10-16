import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, it, expect, vi } from "vitest";
import { useTheme } from "./useTheme";
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});
it("persists explicit themes and tolerates denied preference storage", async () => {
  const { result } = renderHook(useTheme);
  expect(result.current.theme).toBe("system");
  act(() => result.current.setTheme("dark"));
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe("dark"));
  expect(localStorage.getItem("filenym-theme")).toBe("dark");
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
    throw new Error("denied");
  });
  act(() => result.current.setTheme("light"));
  expect(document.documentElement.dataset.theme).toBe("light");
});
it("tracks system preference changes and removes the listener", () => {
  let change = () => {};
  const remove = vi.fn();
  vi.stubGlobal("matchMedia", () => ({
    matches: true,
    addEventListener: (_name: string, fn: () => void) => {
      change = fn;
    },
    removeEventListener: remove,
  }));
  const { result, unmount } = renderHook(useTheme);
  expect(document.documentElement.dataset.theme).toBe("dark");
  act(() => result.current.setTheme("light"));
  act(() => change());
  expect(document.documentElement.dataset.theme).toBe("light");
  unmount();
  expect(remove).toHaveBeenCalled();
  vi.unstubAllGlobals();
});
