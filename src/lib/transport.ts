export type Decimal = string; // canonical decimal spelling, validated in Rust
// Same names are explicit Rust serde enum renames, not Debug formatting.
export type CommandCode =
  | "invalidCanonicalId"
  | "invalidAliasId"
  | "invalidRequest"
  | "invalidAliasName"
  | "duplicateAliasName"
  | "duplicateAliasId"
  | "duplicateCanonicalId"
  | "aliasNotFound"
  | "canonicalNotFound"
  | "conflict"
  | "busy"
  | "databaseUnavailable"
  | "permissionDenied"
  | "corruptDatabase"
  | "invalidStoredData"
  | "incompatibleDatabase"
  | "unsupportedPathEncoding"
  | "unrepresentableValue"
  | "uncertainOutcome"
  | "storageFailure"
  | "invalidAggregateChange"
  | "migrationFailed"
  | "databaseAlreadyExists"
  | "domainFailure"
  | "invalidTimestamp"
  | "selectionNotFound"
  | "selectionPermissionDenied"
  | "selectionUnsupported"
  | "selectionMetadataUnavailable"
  | "selectionInvalid"
  | "selectionChanged"
  | "pickerBusy"
  | "duplicatePath"
  | "staleSession"
  | "unsupportedPlatform"
  | "idGenerationFailed"
  | "internalFailure"
  | "transportUnknown";
export type AliasReason =
  | "empty"
  | "whitespace"
  | "traversalOrSeparator"
  | "forbiddenCharacter"
  | "deviceName"
  | "trailingPeriod"
  | "tooLong";
export type SelectionReason =
  | "directory"
  | "remoteLocation"
  | "reparsePoint"
  | "offlineFile"
  | "invalidPath"
  | "unsupportedDrive";
export type ErrorReason = AliasReason | SelectionReason;
export type SessionRequestDto = { storageEpoch: string };
export type RegisterSelectionRequestDto = SessionRequestDto & {
  selectionToken: string;
  decision: "cancel" | "viewExisting" | "addSeparately";
  candidateId: string | null;
};
export type AddAliasRequestDto = SessionRequestDto & {
  canonicalId: string;
  expectedRevision: Decimal;
  name: string;
};
export type RenameAliasRequestDto = AddAliasRequestDto & { aliasId: string };
export type DeleteAliasRequestDto = SessionRequestDto & {
  canonicalId: string;
  expectedRevision: Decimal;
  aliasId: string;
};
export type GetFileRequestDto = SessionRequestDto & { canonicalId: string };
export type TimestampDto = { seconds: Decimal; nanoseconds: number };
export type NativeTextDto = {
  encoding: "windowsUtf16";
  units: number[];
  display: string;
  escaped: boolean;
};
export type StatusDto =
  | { kind: "available" }
  | { kind: "modified" }
  | { kind: "unavailable"; reason: "notFound" | "permissionDenied" | "unknown" }
  | { kind: "possibleReplacement" };
export type AliasDto = {
  id: string;
  canonicalFileId: string;
  name: string;
  createdAt: TimestampDto;
  updatedAt: TimestampDto;
};
export type CanonicalFileDto = {
  id: string;
  revision: Decimal;
  filename: NativeTextDto;
  savedPath: NativeTextDto;
  sizeBytes: Decimal;
  modifiedAt: TimestampDto | null;
  registeredAt: TimestampDto;
  lastObservedAt: TimestampDto;
  status: StatusDto;
  aliases: AliasDto[];
};
export type RegistryEntryDto =
  | { kind: "file"; file: CanonicalFileDto }
  | { kind: "unreadable"; id: string; code: CommandCode }
  | { kind: "missing"; id: string };
export type RegistryDto = { entries: RegistryEntryDto[] };
export type SelectionOutcomeDto =
  | { kind: "cancelled" }
  | { kind: "added"; file: CanonicalFileDto }
  | {
      kind: "duplicateDecision";
      selectionToken: string;
      filename: NativeTextDto;
      duplicateCandidateIds: string[];
    };
export type DuplicateDecisionOutcomeDto =
  | { kind: "cancelled" }
  | { kind: "viewExisting"; file: CanonicalFileDto }
  | { kind: "added"; file: CanonicalFileDto };
export type RegistryStatusDto = {
  phase:
    "initializing" | "ready" | "blocked" | "needsReopen" | "reopening" | "needsReload" | "closing";
  code: CommandCode | null;
  affectedCanonicalIds: string[];
};
export type CommandErrorDto = {
  code: CommandCode;
  field: "canonicalId" | "aliasId" | "name" | "expectedRevision" | null;
  reason: ErrorReason | null;
  candidateIds: string[];
  attempted: { canonicalId: string | null; aliasId: string | null } | null;
  storageEpoch: string | null;
};
export type Reply<T> = { storageEpoch: string | null; value: T };
