import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, it, expect, vi } from "vitest";
import { useRegistry } from "./useRegistry";
import { registryApi } from "../lib/registry";
import fixture from "../../tests/contracts/registry.json";
import type { CanonicalFileDto, Reply, RegistryDto } from "../lib/transport";
vi.mock("../lib/tauri", () => ({ isDesktop: () => true }));
vi.mock("../lib/registry", async (original) => ({
  ...(await original<typeof import("../lib/registry")>()),
  registryApi: {
    status: vi.fn(),
    list: vi.fn(),
    get: vi.fn(),
    select: vi.fn(),
    add: vi.fn(),
    reopen: vi.fn(),
  },
}));
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
function bootstrap() {
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
describe("registry async boundaries", () => {
  it("ignores an initial inventory arriving after a mutation", async () => {
    bootstrap();
    const list = deferred<Reply<RegistryDto>>();
    vi.mocked(registryApi.list).mockReturnValue(list.promise);
    vi.mocked(registryApi.select).mockResolvedValue({
      storageEpoch: "epoch",
      value: { kind: "added", file: fixture as CanonicalFileDto },
    });
    const { result } = renderHook(useRegistry);
    await waitFor(() => expect(result.current.enabled).toBe(true));
    await act(() => result.current.addFile());
    expect(result.current.entries).toHaveLength(1);
    await act(async () => {
      list.resolve({ storageEpoch: "epoch", value: { entries: [] } });
    });
    expect(result.current.entries).toHaveLength(1);
  });
  it("serializes repeated actions and blocks writes on an unexpected epoch response", async () => {
    bootstrap();
    const reply = deferred<Awaited<ReturnType<typeof registryApi.select>>>();
    vi.mocked(registryApi.select).mockReturnValue(reply.promise);
    const { result } = renderHook(useRegistry);
    await waitFor(() => expect(result.current.enabled).toBe(true));
    let first: Promise<void>;
    act(() => {
      first = result.current.addFile();
    });
    await act(() => result.current.addFile());
    expect(registryApi.select).toHaveBeenCalledTimes(1);
    await act(async () => {
      reply.resolve({
        storageEpoch: "old",
        value: { kind: "added", file: fixture as CanonicalFileDto },
      });
      await first;
    });
    expect(result.current.entries).toHaveLength(0);
    expect(result.current.status.phase).toBe("needsReopen");
    expect(result.current.enabled).toBe(false);
  });
  it("blocks new writes after unknown transport outcome until explicit reopen and reload", async () => {
    bootstrap();
    vi.mocked(registryApi.select).mockRejectedValue(new Error("sensitive native detail"));
    const { result } = renderHook(useRegistry);
    await waitFor(() => expect(result.current.enabled).toBe(true));
    await act(() => result.current.addFile());
    expect(result.current.status.phase).toBe("needsReopen");
    expect(result.current.error?.code).toBe("transportUnknown");
    await act(() => result.current.addFile());
    expect(registryApi.select).toHaveBeenCalledTimes(1);
    vi.mocked(registryApi.reopen).mockResolvedValue({
      storageEpoch: "new",
      value: { phase: "needsReload", code: null, affectedCanonicalIds: [] },
    });
    vi.mocked(registryApi.list).mockResolvedValue({ storageEpoch: "new", value: { entries: [] } });
    vi.mocked(registryApi.status).mockResolvedValue({
      storageEpoch: "new",
      value: { phase: "ready", code: null, affectedCanonicalIds: [] },
    });
    await act(() => result.current.reopen());
    expect(result.current.enabled).toBe(true);
    expect(registryApi.select).toHaveBeenCalledTimes(1);
  });
  it("ignores completion after unmount", async () => {
    bootstrap();
    const reply = deferred<Awaited<ReturnType<typeof registryApi.select>>>();
    vi.mocked(registryApi.select).mockReturnValue(reply.promise);
    const { result, unmount } = renderHook(useRegistry);
    await waitFor(() => expect(result.current.enabled).toBe(true));
    let first: Promise<void>;
    act(() => {
      first = result.current.addFile();
    });
    unmount();
    await act(async () => {
      reply.resolve({
        storageEpoch: "epoch",
        value: { kind: "added", file: fixture as CanonicalFileDto },
      });
      await first;
    });
    expect(registryApi.select).toHaveBeenCalledTimes(1);
  });
});
it("StrictMode ignores a superseded initialization response", async () => {
  bootstrap();
  const old = deferred<Awaited<ReturnType<typeof registryApi.status>>>();
  vi.mocked(registryApi.status).mockReturnValueOnce(old.promise);
  const { result } = renderHook(useRegistry, {
    reactStrictMode: true,
  });
  await waitFor(() => expect(result.current.enabled).toBe(true));
  await act(async () => {
    old.resolve({
      storageEpoch: "obsolete",
      value: { phase: "blocked", code: "databaseUnavailable", affectedCanonicalIds: [] },
    });
  });
  expect(result.current.enabled).toBe(true);
  expect(result.current.status.phase).toBe("ready");
});
it("selection navigation cannot overtake a pending resource read", async () => {
  bootstrap();
  const a = deferred<Awaited<ReturnType<typeof registryApi.get>>>();
  vi.mocked(registryApi.get).mockReturnValue(a.promise);
  const { result } = renderHook(useRegistry);
  await waitFor(() => expect(result.current.enabled).toBe(true));
  let first: Promise<void>;
  act(() => {
    first = result.current.select("fixture");
  });
  await act(() => result.current.select("other"));
  expect(registryApi.get).toHaveBeenCalledTimes(1);
  expect(result.current.selectedId).toBe("fixture");
  await act(async () => {
    a.resolve({ storageEpoch: "epoch", value: fixture as CanonicalFileDto });
    await first;
  });
  expect(result.current.selected?.id).toBe("fixture");
});
it("startup timeout leaves an explicit blocked state and ignores a late response", async () => {
  vi.useFakeTimers();
  try {
    bootstrap();
    const late = deferred<Awaited<ReturnType<typeof registryApi.status>>>();
    vi.mocked(registryApi.status).mockReturnValue(late.promise);
    const { result } = renderHook(useRegistry);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10001);
    });
    expect(result.current.status.phase).toBe("blocked");
    expect(result.current.enabled).toBe(false);
    await act(async () => {
      late.resolve({
        storageEpoch: "epoch",
        value: { phase: "ready", code: null, affectedCanonicalIds: [] },
      });
    });
    expect(result.current.status.phase).toBe("blocked");
    expect(registryApi.list).not.toHaveBeenCalled();
  } finally {
    vi.useRealTimers();
  }
});
