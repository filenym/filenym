import { invoke } from "@tauri-apps/api/core";
import type {
  CanonicalFileDto,
  CommandErrorDto,
  Reply,
  RegistryDto,
  RegistryStatusDto,
  SelectionOutcomeDto,
  DuplicateDecisionOutcomeDto,
  TimestampDto,
  NativeTextDto,
} from "./transport";

const codes = new Set(
  "invalidCanonicalId invalidAliasId invalidRequest invalidAliasName duplicateAliasName duplicateAliasId duplicateCanonicalId aliasNotFound canonicalNotFound conflict busy databaseUnavailable permissionDenied corruptDatabase invalidStoredData incompatibleDatabase unsupportedPathEncoding unrepresentableValue uncertainOutcome storageFailure invalidAggregateChange migrationFailed databaseAlreadyExists domainFailure invalidTimestamp selectionNotFound selectionPermissionDenied selectionUnsupported selectionMetadataUnavailable selectionInvalid selectionChanged pickerBusy duplicatePath staleSession unsupportedPlatform idGenerationFailed internalFailure transportUnknown".split(
    " ",
  ),
);
const reasons = new Set(
  "empty whitespace traversalOrSeparator forbiddenCharacter deviceName trailingPeriod tooLong directory remoteLocation reparsePoint offlineFile invalidPath unsupportedDrive".split(
    " ",
  ),
);
type ObjectValue = Record<string, unknown>;
function object(v: unknown): v is ObjectValue {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}
function string(v: unknown): v is string {
  return typeof v === "string";
}
function nullableString(v: unknown) {
  return v === null || string(v);
}
function strings(v: unknown): v is string[] {
  return Array.isArray(v) && v.every(string);
}
function decimal(v: unknown, max: bigint, signed = false) {
  return (
    string(v) &&
    (signed ? /^(0|-[1-9]\d*|[1-9]\d*)$/ : /^(0|[1-9]\d*)$/).test(v) &&
    BigInt(v) <= max &&
    BigInt(v) >= (signed ? -9223372036854775808n : 0n)
  );
}
export function isTimestamp(v: unknown): v is TimestampDto {
  return (
    object(v) &&
    decimal(v.seconds, 9223372036854775807n, true) &&
    Number.isInteger(v.nanoseconds) &&
    Number(v.nanoseconds) >= 0 &&
    Number(v.nanoseconds) < 1e9
  );
}
export function isNativeText(v: unknown): v is NativeTextDto {
  return (
    object(v) &&
    v.encoding === "windowsUtf16" &&
    string(v.display) &&
    typeof v.escaped === "boolean" &&
    Array.isArray(v.units) &&
    v.units.every((n) => Number.isInteger(n) && n >= 0 && n <= 65535)
  );
}
export function isFile(v: unknown): v is CanonicalFileDto {
  if (
    !object(v) ||
    !string(v.id) ||
    !decimal(v.revision, 9223372036854775807n) ||
    !isNativeText(v.filename) ||
    !isNativeText(v.savedPath) ||
    !decimal(v.sizeBytes, 18446744073709551615n) ||
    !(v.modifiedAt === null || isTimestamp(v.modifiedAt)) ||
    !isTimestamp(v.registeredAt) ||
    !isTimestamp(v.lastObservedAt) ||
    !object(v.status)
  )
    return false;
  if (!(
    v.status.kind === "available" ||
    v.status.kind === "modified" ||
    v.status.kind === "possibleReplacement" ||
    (v.status.kind === "unavailable" &&
      ["notFound", "permissionDenied", "unknown"].includes(String(v.status.reason)))
  ))
    return false;
  return (
    Array.isArray(v.aliases) &&
    v.aliases.every(
      (a) =>
        object(a) &&
        string(a.id) &&
        a.canonicalFileId === v.id &&
        string(a.name) &&
        isTimestamp(a.createdAt) &&
        isTimestamp(a.updatedAt),
    )
  );
}
export function isError(v: unknown): v is CommandErrorDto {
  return (
    object(v) &&
    codes.has(String(v.code)) &&
    (v.field === null ||
      ["canonicalId", "aliasId", "name", "expectedRevision"].includes(String(v.field))) &&
    (v.reason === null || reasons.has(String(v.reason))) &&
    strings(v.candidateIds) &&
    nullableString(v.storageEpoch) &&
    (v.attempted === null ||
      (object(v.attempted) &&
        nullableString(v.attempted.canonicalId) &&
        nullableString(v.attempted.aliasId)))
  );
}
export function isRegistry(v: unknown): v is RegistryDto {
  return (
    object(v) &&
    Array.isArray(v.entries) &&
    v.entries.every(
      (e) =>
        object(e) &&
        ((e.kind === "file" && isFile(e.file)) ||
          (e.kind === "missing" && string(e.id)) ||
          (e.kind === "unreadable" && string(e.id) && codes.has(String(e.code)))),
    )
  );
}
export function isStatus(v: unknown): v is RegistryStatusDto {
  return (
    object(v) &&
    [
      "initializing",
      "ready",
      "blocked",
      "needsReopen",
      "reopening",
      "needsReload",
      "closing",
    ].includes(String(v.phase)) &&
    (v.code === null || codes.has(String(v.code))) &&
    strings(v.affectedCanonicalIds)
  );
}
function isSelection(v: unknown): v is SelectionOutcomeDto {
  return (
    object(v) &&
    (v.kind === "cancelled" ||
      (v.kind === "added" && isFile(v.file)) ||
      (v.kind === "duplicateDecision" &&
        string(v.selectionToken) &&
        isNativeText(v.filename) &&
        strings(v.duplicateCandidateIds)))
  );
}
function isDecision(v: unknown): v is DuplicateDecisionOutcomeDto {
  return (
    object(v) &&
    (v.kind === "cancelled" ||
      (["viewExisting", "added"].includes(String(v.kind)) && isFile(v.file)))
  );
}
export function unknownError(): CommandErrorDto {
  return {
    code: "transportUnknown",
    field: null,
    reason: null,
    candidateIds: [],
    attempted: null,
    storageEpoch: null,
  };
}
export async function call<T>(
  command: string,
  request: unknown,
  guard: (v: unknown) => v is T,
): Promise<Reply<T>> {
  try {
    const reply: unknown = await invoke(command, request === undefined ? undefined : { request });
    if (!object(reply) || !nullableString(reply.storageEpoch) || !guard(reply.value))
      throw unknownError();
    return reply as Reply<T>;
  } catch (e: unknown) {
    throw isError(e) ? e : unknownError();
  }
}
export const registryApi = {
  status: () => call("registry_status", undefined, isStatus),
  reopen: () => call("reopen_registry", undefined, isStatus),
  list: (storageEpoch: string) => call("list_registered_files", { storageEpoch }, isRegistry),
  get: (storageEpoch: string, canonicalId: string) =>
    call(
      "get_canonical_file",
      { storageEpoch, canonicalId },
      (v): v is CanonicalFileDto => isFile(v) && v.id === canonicalId,
    ),
  select: (storageEpoch: string) => call("select_existing_file", { storageEpoch }, isSelection),
  decide: (
    storageEpoch: string,
    selectionToken: string,
    decision: "cancel" | "viewExisting" | "addSeparately",
    candidateId: string | null,
  ) =>
    call(
      "register_selected_file",
      { storageEpoch, selectionToken, decision, candidateId },
      isDecision,
    ),
  add: (storageEpoch: string, canonicalId: string, expectedRevision: string, name: string) =>
    call("add_alias", { storageEpoch, canonicalId, expectedRevision, name }, isFile),
  rename: (
    storageEpoch: string,
    canonicalId: string,
    expectedRevision: string,
    aliasId: string,
    name: string,
  ) => call("rename_alias", { storageEpoch, canonicalId, expectedRevision, aliasId, name }, isFile),
  delete: (storageEpoch: string, canonicalId: string, expectedRevision: string, aliasId: string) =>
    call("delete_alias", { storageEpoch, canonicalId, expectedRevision, aliasId }, isFile),
};
export function errorMessage(e: CommandErrorDto): string {
  switch (e.code) {
    case "invalidAliasName":
      return (
        (
          {
            empty: "Enter an alias name.",
            whitespace: "Remove whitespace at the beginning or end.",
            traversalOrSeparator: "Enter a name without a path or traversal.",
            forbiddenCharacter: "Remove forbidden characters from the name.",
            deviceName: "This name is reserved by Windows.",
            trailingPeriod: "The name cannot end with a period.",
            tooLong: "Use at most 255 UTF-8 bytes.",
          } as Record<string, string>
        )[e.reason ?? ""] ?? "Choose a valid alias name."
      );
    case "duplicateAliasName":
      return "This file already has an alias with that name (ASCII case is ignored).";
    case "conflict":
      return "This record changed. Reload, review your draft, then submit again.";
    case "invalidTimestamp":
      return "The system clock precedes this record. Check the clock before trying again.";
    case "selectionUnsupported":
      return "Select an ordinary local file. File links, cloud placeholders, remote and recall-sensitive locations are unsupported.";
    case "selectionChanged":
      return "The selected file changed. Select it again.";
    case "selectionNotFound":
      return "The selected file is no longer available.";
    case "selectionPermissionDenied":
    case "permissionDenied":
      return "Permission was denied. Your saved data has been retained.";
    case "busy":
      return "The registry is busy. Wait, then explicitly try again.";
    case "uncertainOutcome":
    case "transportUnknown":
      return "The result is unknown. Reopen the registry and review saved records before any new change. Do not repeat the previous action blindly.";
    case "staleSession":
      return "The storage session changed. Reload and review saved records.";
    case "canonicalNotFound":
    case "aliasNotFound":
      return "This saved record no longer exists. Reload the registry before making another change.";
    case "databaseUnavailable":
      return "The saved registry is unavailable. Reopen it when its location is accessible. It will not be recreated automatically.";
    case "corruptDatabase":
    case "invalidStoredData":
    case "incompatibleDatabase":
    case "migrationFailed":
      return "Saved data could not be read safely. The registry has been preserved; use a compatible FileNym version and reopen it.";
    case "unsupportedPathEncoding":
    case "unrepresentableValue":
      return "This saved value cannot be represented on this system. It has not been rounded or replaced.";
    case "selectionInvalid":
      return "This selection is no longer valid. Select the file again.";
    case "selectionMetadataUnavailable":
      return "The selected file's metadata could not be inspected safely. Select it again when it is accessible.";
    case "pickerBusy":
      return "Finish or cancel the open file picker first.";
    case "idGenerationFailed":
    case "duplicateCanonicalId":
    case "duplicateAliasId":
      return "A new record could not be created safely. No change was confirmed; try the action again.";
    case "invalidCanonicalId":
    case "invalidAliasId":
    case "invalidRequest":
      return "The request is invalid. Reload saved records before trying again.";
    default:
      return "The operation could not finish. Saved metadata has been retained. Reload and review before trying again.";
  }
}
