import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { registryApi, isError, unknownError } from "../lib/registry";
import { isDesktop } from "../lib/tauri";
import type {
  CanonicalFileDto,
  CommandErrorDto,
  RegistryEntryDto,
  RegistryStatusDto,
  SelectionOutcomeDto,
} from "../lib/transport";

function advanceGeneration(counter: { current: number }) {
  return ++counter.current;
}
function ensureEpoch(actual: string | null, expected: string | null) {
  if (actual !== expected) throw { ...unknownError(), code: "staleSession" };
}
export function useRegistry() {
  const [entries, setEntries] = useState<RegistryEntryDto[]>([]),
    [selectedId, setSelectedId] = useState<string | null>(null);
  const [status, setStatus] = useState<RegistryStatusDto>({
    phase: "initializing",
    code: null,
    affectedCanonicalIds: [],
  });
  const [error, setError] = useState<CommandErrorDto | null>(null),
    [pending, setPending] = useState(false),
    [duplicate, setDuplicate] = useState<Extract<
      SelectionOutcomeDto,
      { kind: "duplicateDecision" }
    > | null>(null);
  const [notice, setNotice] = useState("");
  const epoch = useRef<string | null>(null),
    sequence = useRef(0),
    busy = useRef(false),
    alive = useRef(false),
    unsafe = useRef(false);
  const accept = useCallback((file: CanonicalFileDto) => {
    setEntries((old) => {
      const exists = old.some((e) => e.kind === "file" && e.file.id === file.id);
      return exists
        ? old.map((e) => (e.kind === "file" && e.file.id === file.id ? { kind: "file", file } : e))
        : [...old, { kind: "file", file }];
    });
  }, []);
  const fail = useCallback((value: unknown) => {
    const e = isError(value) ? value : unknownError();
    setError(e);
    if (["uncertainOutcome", "transportUnknown", "staleSession"].includes(e.code)) {
      unsafe.current = true;
      setStatus({
        phase: "needsReopen",
        code: e.code,
        affectedCanonicalIds: e.attempted?.canonicalId ? [e.attempted.canonicalId] : [],
      });
    } else if (
      [
        "databaseUnavailable",
        "permissionDenied",
        "corruptDatabase",
        "incompatibleDatabase",
        "invalidStoredData",
        "storageFailure",
      ].includes(e.code)
    ) {
      setStatus({ phase: "blocked", code: e.code, affectedCanonicalIds: [] });
    }
    return e;
  }, []);
  useEffect(() => {
    if (!isDesktop()) return;
    let active = true;
    let dispose: UnlistenFn | undefined;
    void listen("registry-closing", () => {
      if (active) {
        unsafe.current = true;
        setStatus({ phase: "closing", code: null, affectedCanonicalIds: [] });
        setNotice("Finishing save…");
      }
    })
      .then((unlisten) => {
        if (active) dispose = unlisten;
        else unlisten();
      })
      .catch(() => {
        /* No state change is inferred from a missing notification channel. */
      });
    return () => {
      active = false;
      dispose?.();
    };
  }, []);
  useEffect(() => {
    alive.current = true;
    let timer: ReturnType<typeof setTimeout>;
    const run = advanceGeneration(sequence);
    const startupTimeout = setTimeout(() => {
      if (alive.current && sequence.current === run) {
        advanceGeneration(sequence);
        clearTimeout(timer);
        fail({ ...unknownError(), code: "databaseUnavailable" });
        setNotice(
          "Opening took longer than expected. Reopen explicitly; the original operation is not cancelled or retried.",
        );
      }
    }, 10000);
    async function initialize() {
      if (!isDesktop()) {
        clearTimeout(startupTimeout);
        setStatus({ phase: "blocked", code: "unsupportedPlatform", affectedCanonicalIds: [] });
        return;
      }
      try {
        const reply = await registryApi.status();
        if (!alive.current || sequence.current !== run) return;
        setStatus(reply.value);
        epoch.current = reply.storageEpoch;
        if (reply.value.phase === "initializing" || reply.value.phase === "reopening") {
          timer = setTimeout(() => void initialize(), 150);
          return;
        }
        if (reply.storageEpoch && ["ready", "needsReload"].includes(reply.value.phase)) {
          const list = await registryApi.list(reply.storageEpoch);
          if (alive.current && sequence.current === run) {
            ensureEpoch(list.storageEpoch, epoch.current);
            setEntries(list.value.entries);
            if (reply.value.phase === "needsReload") {
              const checked = await registryApi.status();
              if (!alive.current || sequence.current !== run) return;
              ensureEpoch(checked.storageEpoch, epoch.current);
              unsafe.current = checked.value.phase !== "ready";
              setStatus(checked.value);
            } else {
              setStatus({ ...reply.value, phase: "ready" });
            }
          }
        }
        clearTimeout(startupTimeout);
      } catch (e) {
        clearTimeout(startupTimeout);
        if (alive.current && sequence.current === run) {
          const error = fail(e);
          if (!["uncertainOutcome", "transportUnknown", "staleSession"].includes(error.code))
            setStatus({ phase: "blocked", code: error.code, affectedCanonicalIds: [] });
        }
      }
    }
    void initialize();
    return () => {
      alive.current = false;
      advanceGeneration(sequence);
      clearTimeout(timer);
      clearTimeout(startupTimeout);
    };
  }, [fail]);
  async function execute<T>(job: (session: string) => Promise<T>): Promise<T | undefined> {
    if (busy.current || !epoch.current || unsafe.current) return;
    busy.current = true;
    setPending(true);
    const run = advanceGeneration(sequence);
    try {
      return await job(epoch.current);
    } catch (e) {
      if (alive.current && sequence.current === run) fail(e);
      return undefined;
    } finally {
      busy.current = false;
      if (alive.current && sequence.current === run) setPending(false);
    }
  }
  async function refresh() {
    return execute(async (session) => {
      const reply = await registryApi.list(session);
      if (!alive.current) return;
      ensureEpoch(reply.storageEpoch, epoch.current);
      setEntries(reply.value.entries);
      setError(null);
      setNotice("Saved registry reloaded. Review your draft before submitting.");
      return reply.value;
    });
  }
  async function select(id: string) {
    if (busy.current) return;
    setSelectedId(id);
    await execute(async (session) => {
      const reply = await registryApi.get(session, id);
      if (alive.current) {
        ensureEpoch(reply.storageEpoch, epoch.current);
        accept(reply.value);
      }
    });
  }
  async function addFile() {
    await execute(async (session) => {
      const reply = await registryApi.select(session);
      if (!alive.current) return;
      ensureEpoch(reply.storageEpoch, epoch.current);
      if (reply.value.kind === "duplicateDecision") setDuplicate(reply.value);
      else if (reply.value.kind === "added") {
        accept(reply.value.file);
        setSelectedId(reply.value.file.id);
        setError(null);
        setNotice("File registered. Its physical name and contents were preserved.");
      }
    });
  }
  async function decide(
    decision: "cancel" | "viewExisting" | "addSeparately",
    candidateId: string | null,
  ) {
    const choice = duplicate;
    if (!choice) return;
    if (decision === "cancel") setDuplicate(null);
    await execute(async (session) => {
      try {
        const reply = await registryApi.decide(
          session,
          choice.selectionToken,
          decision,
          candidateId,
        );
        if (!alive.current) return;
        ensureEpoch(reply.storageEpoch, epoch.current);
        setDuplicate(null);
        if (reply.value.kind !== "cancelled") {
          accept(reply.value.file);
          setSelectedId(reply.value.file.id);
          setNotice(
            reply.value.kind === "added"
              ? "Separate registry entry saved."
              : "Existing registry entry selected.",
          );
        }
        setError(null);
      } catch (e) {
        if (
          decision === "viewExisting" ||
          !isError(e) ||
          [
            "selectionInvalid",
            "selectionChanged",
            "selectionUnsupported",
            "selectionNotFound",
            "uncertainOutcome",
            "transportUnknown",
            "staleSession",
          ].includes(e.code)
        )
          setDuplicate(null);
        throw e;
      }
    });
  }
  async function mutate(
    kind: "add" | "rename" | "delete",
    file: CanonicalFileDto,
    revision: string,
    aliasId: string | null,
    name: string,
  ) {
    return execute(async (session) => {
      const reply =
        kind === "add"
          ? await registryApi.add(session, file.id, revision, name)
          : kind === "rename"
            ? await registryApi.rename(session, file.id, revision, aliasId!, name)
            : await registryApi.delete(session, file.id, revision, aliasId!);
      if (!alive.current) return;
      ensureEpoch(reply.storageEpoch, epoch.current);
      accept(reply.value);
      setError(null);
      setNotice(
        kind === "delete" ? "Alias deleted. The original file was preserved." : "Alias saved.",
      );
      return reply.value;
    });
  }
  async function reopen() {
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    advanceGeneration(sequence);
    setDuplicate(null);
    try {
      const reply = await registryApi.reopen();
      if (!alive.current) return;
      epoch.current = reply.storageEpoch;
      setStatus(reply.value);
      if (!reply.storageEpoch) throw unknownError();
      const list = await registryApi.list(reply.storageEpoch);
      if (!alive.current) return;
      ensureEpoch(list.storageEpoch, epoch.current);
      setEntries(list.value.entries);
      const checked = await registryApi.status();
      if (!alive.current) return;
      ensureEpoch(checked.storageEpoch, epoch.current);
      setStatus(checked.value);
      unsafe.current = checked.value.phase !== "ready";
      if (!unsafe.current) {
        setError(null);
        setNotice(
          "Registry reopened and saved records reloaded. Review the result before making another change.",
        );
      }
    } catch (e) {
      if (alive.current) fail(e);
    } finally {
      busy.current = false;
      if (alive.current) setPending(false);
    }
  }
  const selected = entries.find((e) => e.kind === "file" && e.file.id === selectedId);
  return {
    entries,
    selected: selected?.kind === "file" ? selected.file : null,
    selectedId,
    status,
    error,
    pending,
    duplicate,
    notice,
    enabled: isDesktop() && status.phase === "ready" && !pending,
    refresh,
    select,
    addFile,
    decide,
    mutate,
    reopen,
    back: () => {
      if (!busy.current) setSelectedId(null);
    },
  };
}
