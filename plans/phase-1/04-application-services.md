# Phase 1 Milestone 04: Application Services

## Status

**Implemented and locally validated; not committed or pushed.** The design was prepared on 2026-10-15 against `main` at `8dce1ac` (Milestone 03 merge, pull request #13), then approved for implementation on 2026-10-16. This document preserves the reviewed design, rationale, acceptance matrix, and executed validation record. Before implementation the only untracked file was this plan. No hosted CI result is asserted.

## Objective and scope

Implement a small synchronous application service that coordinates existing domain operations and the aggregate repository. Include registration from caller-supplied facts, alias creation/rename/deletion, retrieval, and an ID inventory. Successful mutations return the committed aggregate and storage revision; rejected requests cannot publish a speculative success or partially persisted aggregate.

Test every included use case through the real application-service-to-SQLite boundary, including failure paths, reopen/restart, and physical-file safety. Retain full regression coverage of the existing desktop shell. Desktop registration and alias workflows remain unavailable until a later approved adapter/UI milestone.

## Repository review and authoritative decisions

Reviewed [engineering instructions](../../AGENTS.md), [problem statement](../../docs/problem-statement.md), architecture documents ([scope](../../docs/architecture/phase-1-scope.md), [domain model](../../docs/architecture/domain-model.md), [persistence](../../docs/architecture/persistence.md), [lifecycle](../../docs/architecture/file-lifecycle.md)), accepted ADRs [0001](../../docs/decisions/0001-local-first-desktop.md), [0002](../../docs/decisions/0002-aliases-are-metadata.md), [0003](../../docs/decisions/0003-path-is-not-identity.md), and milestone records [01](01-project-foundation.md), [02](02-domain-core.md), [03](03-sqlite-persistence.md). Source review covered core value types/aggregate/lifecycle/rehydration, port contracts, SQLite codecs/connection/migrations/repository, existing tests, crate manifests, frontend scripts, desktop shell/configuration, and [foundation CI](../../.github/workflows/foundation.yml).

Verified facts and discrepancies affecting this design:

1. Git history contains merged domain and persistence implementations. Milestone 02/03 documents still describe their pre-commit completion snapshots. Their status lines are historical; they do not mean code is missing. Those historical records were not modified during this milestone.
2. At the reviewed baseline, `filenym-app` contained only role documentation and depended on core and ports. No application service existed then; M04 now implements the service while leaving ID generation, clocks, transport, and production DB composition deferred.
3. Actual `CanonicalFileRepository` methods are `insert`, `load`, `update`, and `list_ids`, all with mutable receivers. Load returns `Option<StoredCanonicalFile>`; insert/update return `PersistenceRevision`. No separate alias repository, deletion of a registration, full-list snapshot, or transaction handle exists or is needed here.
4. `CanonicalFile::new` is infallible for validated inputs, initially Available with no aliases and observation time equal to registration time. It does not inspect a physical file. Architecture registration describes an eventual full physical-file workflow; M04 supplies only its metadata orchestration portion. Available here is the caller's initial assertion, not an independently verified availability result.
5. IDs are opaque strings, not UUIDs; alias IDs are unique per canonical aggregate, not registry-wide. Names use ASCII case folding only. Relative native locations are currently valid; no absolute-path requirement or filesystem normalization should be silently introduced.
6. Domain identical-name rename validates the timestamp and then leaves alias fields unchanged. SQLite increments revisions on every successful update, including unchanged aggregates. The chosen service policy below preserves both semantics.
7. `Modified` is a transient metadata-observation status; rename/move are change results. All three unavailable reasons and PossibleReplacement persist. Alias edits are allowed in every status and do not clear it.
8. `UncertainOutcome` requires retiring the adapter and fresh reopen/reload. It must not become Conflict, a retry, or an assertion of rollback. Database unavailability is distinct from a canonical file's persisted Unavailable status.
9. At the reviewed baseline, existing tests covered domain invariants and repository correctness, including failure injection and subprocess recovery, but no service orchestration. M03 recorded 54 Rust tests, including one worker entry. The completed M04 validation below includes those inherited suites and the new service tests.
10. Overview research language and conceptual repository diagrams do not override approved ADRs, concrete inward dependency rules, or Phase 1 exclusions. No accepted architectural contradiction requires an ADR change for this design.

## Non-goals

- No React screens, Tauri commands/state/composition, file picker, production app-data directory resolution, DB creation/opening inside the service, or frontend serialization.
- No OS file inspection, content reads, identity detection, path resolution/canonicalization, existence/permission probes, symlink handling, or physical file mutation.
- No watcher, rename/move detection, startup reconciliation, observation/recovery commands, intentional replacement, relink UX, or Milestone 05 implementation.
- No registration removal, cross-record transaction, automatic deduplication by location/evidence, full inventory summaries, pagination, search, cache, async runtime, command bus, event log, clock/ID provider, or generic service framework.
- No new third-party dependencies, schema/migration changes, unchecked domain access, or alteration of existing validation/lifecycle rules.
- No hard/symbolic links, virtual filesystems, external filename presentation, upload/browser integration, cloud, accounts, telemetry, or AI features.

## Architecture and dependency direction

```text
Future React -> future thin Tauri adapter/composition
                            |                 |
                            v                 v
                       filenym-app      filenym-infra (SQLite)
                          |    |             |    |
                          |    +----------> ports <+
                          v                   |
                         core <---------------+
                          |
                    Rust standard library

Test composition only:
infra integration test -> app + SqliteRepository -> ports/core
```

Production `filenym-app` imports core, ports, and standard-library types only. Infra implements the repository trait; the caller constructs the adapter and lends it to the service. No connection ownership, database path, SQL, thread synchronization, or OS adapter enters application APIs. Core and ports need no changes. Infra production source and migrations need no changes.

Place SQLite-backed service tests in infra, with **a dev-dependency from infra to app**. This preserves the production graph and avoids app depending on infra, even for tests. Both already depend inward on core/ports; this test-only edge has no cycle. Existing workspace CI discovers the tests without configuration changes. The lockfile metadata change names that new local dev-dependency only; no package version upgrades were made.

## Implemented public APIs

The following records the exported request/error types and service signatures; method bodies and imports are omitted for readability. Imports are existing types from `filenym_core`, `filenym_ports`, and `std::time::SystemTime`. Requests are owned data, derive `Clone, Debug, PartialEq, Eq`, and intentionally allow raw strings pending service validation. They are not serde DTOs and are not trusted until validated.

```rust
pub struct RegisterCanonicalFile {
    pub canonical_id: String,
    pub location: KnownFileLocation,
    pub identity: Option<FileIdentity>,
    pub metadata: FileMetadata,
    pub registered_at: SystemTime,
}

pub struct AddAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
    pub name: String,
    pub created_at: SystemTime,
}

pub struct RenameAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
    pub name: String,
    pub updated_at: SystemTime,
}

pub struct DeleteAlias {
    pub canonical_id: String,
    pub expected_revision: PersistenceRevision,
    pub alias_id: String,
}

pub enum ApplicationError {
    InvalidCanonicalId,
    InvalidAliasId,
    CanonicalNotFound,
    AliasNotFound,
    DuplicateCanonicalId,
    DuplicateAliasId,
    DuplicateAliasName,
    Conflict,
    Domain(DomainError),
    Storage(PersistenceError),
}

pub struct FileRegistryService<'a, R: CanonicalFileRepository + ?Sized> {
    repository: &'a mut R,
}

// Signatures in impl<'a, R: CanonicalFileRepository + ?Sized>:
pub fn new(repository: &'a mut R) -> Self;
pub fn register_canonical_file(
    &mut self, request: RegisterCanonicalFile,
) -> Result<StoredCanonicalFile, ApplicationError>;
pub fn add_alias(
    &mut self, request: AddAlias,
) -> Result<StoredCanonicalFile, ApplicationError>;
pub fn rename_alias(
    &mut self, request: RenameAlias,
) -> Result<StoredCanonicalFile, ApplicationError>;
pub fn delete_alias(
    &mut self, request: DeleteAlias,
) -> Result<StoredCanonicalFile, ApplicationError>;
pub fn get_canonical_file(
    &mut self, id: &str,
) -> Result<StoredCanonicalFile, ApplicationError>;
pub fn list_registered_file_ids(
    &mut self,
) -> Result<Vec<CanonicalFileId>, ApplicationError>;
```

Keep the service field private. No raw repository getter, public save-arbitrary-aggregate method, snapshot reconstruction endpoint, or lifecycle forwarding method is exposed. A short borrow scope permits the composition root to drop the service then close/reopen the adapter. Accepting `?Sized` allows either a concrete repository or an existing trait object without requiring boxing or dynamic dispatch.

Reuse `StoredCanonicalFile` as the concrete application result instead of duplicating it: it already contains the complete validated domain value and committed revision, has read-only accessors, and contains no SQLite types. Returned values are detached snapshots, not live references or write capabilities; callers cannot persist arbitrary modifications through this service. They may become stale immediately after another writer commits. Future transport adapters must map them to transport-safe DTOs, including explicit timestamp/path/revision representations, in their own milestone.

`ApplicationError` derives `Clone, Copy, Debug, PartialEq, Eq` and implements `Display` and `std::error::Error`. Display uses static category messages, with existing safe nested domain/persistence messages and error sources. Do not echo request values. Deriving Debug on requests/results is for Rust development, not authorization to log user metadata.

## Input ownership and validation

The caller supplies stable canonical/alias IDs and all timestamps. No `SystemTime::now`, random IDs, UUID library, filesystem identity synthesis, or ID/clock port is needed. IDs remain independent of path/evidence. Canonical IDs must be unique in the registry; alias IDs must be unique within their owner. Duplicate IDs are rejected, never silently replaced or regenerated. ID generation belongs to a later composition policy; M04 does not promise global alias-ID uniqueness.

Use `CanonicalFileId::new`, `AliasId::new`, and `AliasName::new` inside methods, even for requests constructed directly by Rust callers. Deterministic validation order is canonical ID, alias ID if present, then name if present, before any repository access. Empty/edge-whitespace/control IDs fail with the corresponding application ID error. Do not impose a UUID format or lowercase/trim IDs.

Location, evidence, and metadata are already typed inputs. Their constructors enforce current domain rules before registration requests can be formed; the service need not repeat those constructors or translate an invalid native path into an alias error. Invalid location/evidence construction remains `DomainError::InvalidLocation`/`InvalidIdentityEvidence` at the caller boundary and is covered by core tests. M04 introduces no raw location/evidence parser. Registration accepts valid relative and nonexistent synthetic paths unchanged. It does not certify existence or permission. A future physical registration adapter must validate a supported existing file and supply trusted facts; it cannot infer identity from path or metadata alone.

Domain owns alias rules: reject rather than trim, preserve extension/spelling, 255 UTF-8 bytes, reserved stems/forbidden forms, ASCII-only per-owner uniqueness, exact non-ASCII comparison. Visually similar Unicode and normalization variants may remain distinct. In-context timestamp validation remains in the aggregate after load; no stricter clock monotonicity is added. Creation must be at/after registration; rename time must be at/after alias creation but may precede its previous update. Identical-name rename still validates the supplied time. Delete has no timestamp because no domain deletion timestamp exists.

## Use-case behavior and output contracts

| Use case            | Steps and result                                                                                                                                                                                                                                    | Failure and safety behavior                                                                                                                                                                                                                                        |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Register            | Validate canonical ID; construct `CanonicalFile::new` from supplied facts; insert once; return `StoredCanonicalFile::new(file, returned_revision)`. Real adapter returns revision 0, Available, no aliases, observation time equal to registration. | No pre-load/path search/deduplication. Existing canonical ID fails. Different IDs at the same path or with equal opaque evidence are allowed; M04 cannot establish they are physically identical. No implicit initial aliases.                                     |
| Add alias           | Validate strings; load; require caller revision matches; call `create_alias`; update using loaded revision; return mutated aggregate with returned revision.                                                                                        | Duplicate ID/name, invalid creation time, missing record, and conflicts fail without losing aliases/canonical facts. Appended alias receives owner from aggregate.                                                                                                 |
| Rename alias        | Same validation/load/revision sequence; call `rename_alias`; update once even if domain state is unchanged; return committed aggregate/revision.                                                                                                    | Stable alias ID, owner, creation time, and order remain. Case-only rename changes spelling/update time. Exact-name rename preserves all domain fields but increments storage revision on successful update. Missing alias, collision, or invalid time fails.       |
| Delete alias        | Validate IDs; load/revision check; call `delete_alias`; update once; return committed aggregate/revision.                                                                                                                                           | Only targeted metadata removed. Remaining order/fields stay intact; removing the final alias leaves parent. Unknown alias is an error, not idempotent success. Canonical file is never deleted.                                                                    |
| Get canonical       | Validate ID; call load once; return complete stored aggregate and revision; None becomes CanonicalNotFound.                                                                                                                                         | No observation, status rewrite, or storage update. Database error is not converted to a missing record.                                                                                                                                                            |
| List registered IDs | Call `list_ids` once; return its deterministic binary-ordered IDs, including all statuses; empty registry returns empty vector.                                                                                                                     | No N+1 loads or skipped errors. IDs are inventory, not complete summaries or proof all stored aggregates are valid. Use get for full validation; missing/corrupt record during subsequent get is handled explicitly. No atomic full-registry snapshot is promised. |

There is no service cache. Mutations use a local loaded aggregate and return it only after update succeeds. Construct the result with the **repository-returned** revision, not a guessed increment; do not reload after commit and accidentally return another writer's state or turn a successful commit into a read failure. Repository atomicity is the transaction boundary; service load/domain/update is not a single database transaction and does not claim to be one.

## Revision, concurrency, and recovery policy

Every alias mutation requires a concrete caller `expected_revision`; there is no optional or unconditional variant. A caller obtains it through registration/get/mutation output. If only IDs were listed, get the aggregate/revision before editing.

1. Validate raw strings without repository calls.
2. Load the latest persisted aggregate; absence is CanonicalNotFound.
3. Compare caller expected revision with loaded revision. Any inequality, including a future revision, returns Conflict with no domain mutation/update.
4. Apply exactly one existing domain operation. Domain rejection causes no update.
5. Call `update(&file, loaded_revision)` exactly once. A second writer between load and update is detected by the repository and also yields Conflict.
6. Return success only after update reports committed success, with its returned revision. Never retry/rebase silently, merge snapshots, increment revisions locally, or change supplied IDs/times to force a retry.

Conflict takes precedence over contextual alias errors when the caller is stale; malformed primitive strings take precedence before load. A current revision targeting a missing alias yields AliasNotFound. Matching revision plus identical-name rename still attempts a conditional write and increments revision, ensuring the same conflict/commit semantics for every accepted alias command. This makes the domain no-op a successful storage write; approve this explicit tradeoff rather than accidentally optimizing away conflict detection.

Busy, PermissionDenied, or other deterministic failures are errors, never success. Revisions and state remain as guaranteed by the repository's deterministic rollback contract. A caller may choose an explicit retry after fresh get/revision review; service does not implement a retry policy. `UncertainOutcome` preserves uncertainty: discard the adapter at composition boundary, explicitly reopen existing storage, reload using stable IDs, and determine actual state before deciding a new action. Registration is not idempotent across uncertain retries; do not blindly register again or reinterpret DuplicateCanonicalId as success. M04 adds no adapter factory/reopen service.

Revision exhaustion remains `Storage(UnrepresentableValue)` with no durable change. There is no cross-record transaction and no guarantee that list followed by get observes one registry-wide instant. Subsequent external writers may invalidate any response revision immediately; that is expected optimistic concurrency behavior.

## Error mapping

Use small operation-aware mapping helpers, not arbitrary strings or SQL-code inspection. Apply the following normalized mappings so callers do not need to recognize the same business error in two wrappers.

| Input/error source                                                         | Application result                               |
| -------------------------------------------------------------------------- | ------------------------------------------------ |
| Canonical/alias ID constructor InvalidRecordId                             | InvalidCanonicalId / InvalidAliasId respectively |
| Load returns None; persistence NotFound during update                      | CanonicalNotFound                                |
| Domain AliasNotFound                                                       | AliasNotFound                                    |
| Domain DuplicateAliasId / DuplicateAliasName                               | Matching top-level duplicate variant             |
| Persistence DuplicateCanonicalId / DuplicateAliasId / DuplicateAliasName   | Matching top-level duplicate variant             |
| Caller revision mismatch; persistence Conflict                             | Conflict                                         |
| Other DomainError, including InvalidAliasName(reason) and InvalidTimestamp | Domain(original error)                           |
| All other PersistenceError variants                                        | Storage(original category), unchanged            |

The storage wrapper retains InvalidStoredData, CorruptDatabase, IncompatibleDatabase, UnsupportedPathEncoding, UnrepresentableValue, Busy, PermissionDenied, Unavailable, InvalidAggregateChange, MigrationFailed, DatabaseAlreadyExists, StorageFailure, and UncertainOutcome. Some are normally initialization-only and cannot arise from real CRUD, but injected repository errors must still propagate without panic or lossy classification. Do not invent a SQLite or OS error conversion in app. Initialization errors remain the caller's responsibility before constructing the service.

Unavailable/PossibleReplacement on a retrieved record is successful data, not an application storage error. Missing registration is unrelated to a file being missing on disk. IdentityMismatch/UnresolvedReplacement remain valid nested domain errors but have no new lifecycle command to trigger them here. Safe Display/source tests assert no request path, name, SQL-like input, or identity payload appears.

## Lifecycle decision

**Defer observation and verified-recovery services to the later filesystem/lifecycle milestones (06–08), subject to their reviewed plans.** Existing domain APIs are sufficient to represent them, but M04 has neither a trusted observer nor an experimental identity policy. Exposing a convenience ConfirmedSameFile/recover endpoint now would broaden the application trust boundary before its caller contract exists.

M04 must retrieve and preserve every existing lifecycle state while managing aliases, including Unavailable(PermissionDenied) and PossibleReplacement. Integration fixtures may seed states through existing core/repository APIs solely to test preservation. Such fixture setup is not a production lifecycle service or evidence of OS tracking. No observation classification, replacement acceptance, or recovery is inferred during get/list/register/alias edits.

## Test organization and isolation

- App public-API tests use a tiny handwritten repository spy/scripted fake implementing the four current methods. Record call order, IDs, expected revisions, snapshots, and injected outcomes. Fail unexpected calls. Support only scenarios needed to prove orchestration, including injected update conflicts; real intervening writes are tested in infra; do not recreate SQL/codec behavior or a second generic repository framework.
- Infra integration tests compose the real service with the public SqliteRepository. Reuse `tests/support/mod.rs` owned temporary directory helper. Keep SQL failure injection in infra tests, never production services or core. Open the adapter before adding fixture triggers because schema verification correctly rejects extra objects; remove triggers/test tables before reopen.
- Add a separate service subprocess test and worker entry. Child modes call actual service methods, not raw SQL, using explicit owned DB arguments. Parent waits for successful child exit with a 15-second deadline per mode; all handles are closed before cleanup. Do not rely on sleeps to create races. Count worker entries separately from substantive tests in completion reporting.
- Use fixed supplied times, including pre-epoch values and host-representable fractional seconds. Windows domain SystemTime supports 100 ns resolution; exact storage-format nanoseconds beyond host precision remain rejected, not silently rounded. Keep existing native codec tests rather than duplicating their entire matrix in services.
- All fixtures are exclusively created owned directories. Validate resolved cleanup targets remain under their intended temporary root. Only fixture code touches sentinel files; production services touch no physical file. Restore changed fixture permissions and kill/reap test children in bounded cleanup paths.

## Automated feature and edge-case matrix

Every row is mandatory coverage; multiple assertions/table cases may share a test. U means app orchestration tests; I means real service/SQLite tests. Existing core/infra suites remain required regression evidence. No fixed new test count is promised before implementation; completion must map each row to named executed tests and record blocked cases.

| ID  | Layer             | Cases and required assertions                                                                                                                                                                                                                                               |
| --- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | U/I               | Register minimal zero-size/no evidence/no modification-time record; exact ID/location/registration/status/observation fields and revision 0; one insert, no load/update/list.                                                                                               |
| R2  | U/I               | Duplicate canonical ID, including different supplied metadata; typed duplicate, prior full aggregate/revision unchanged after reopen; no overwrite or fallback ID.                                                                                                          |
| R3  | U/I               | Different IDs sharing path and/or synthetic identity evidence allowed; nonexistent relative/absolute host paths accepted without access; no path-derived ID/deduplication.                                                                                                  |
| V1  | U                 | Invalid canonical IDs across register/get/all mutations; invalid alias IDs on all alias commands; empty/control/edge-whitespace cases; no repository calls. Distinct ID error categories and validation precedence.                                                         |
| V2  | U/I               | Add and rename: empty/whitespace/edge Unicode whitespace; dot/dotdot/separators/traversal; forbidden punctuation/colon/control/NUL; trailing period; reserved device stems including extensions/superscript digits; exact domain reason; no write.                          |
| V3  | U/I               | ASCII 255/256 and multibyte 255/256-byte names; internal spaces, dotfiles, extensionless/multipart/report..pdf accepted; exact spelling and explicit changed extension retained through reopen.                                                                             |
| V4  | U/I               | ASCII case duplicate rejects; non-ASCII case and composed/decomposed Unicode remain distinct; same name and alias ID across different owners allowed; quoted/SQL-like accepted strings are data.                                                                            |
| A1  | U/I               | Add zero/one/multiple aliases; append order/owner/times; duplicate same-owner ID and name reject; full canonical facts and unrelated aliases preserved; returned revision matches stored revision.                                                                          |
| A2  | U/I               | Rename success/case-only/exact identical spelling; alias ID/owner/created/order preserved; case-only time changes; identical spelling keeps time but commits revision +1; collision with another alias rejects.                                                             |
| A3  | U/I               | Delete first/middle/last/only alias; exact survivor order and fields; parent survives; missing alias rejects, including repeated delete with a refreshed revision.                                                                                                          |
| T1  | U/I               | Create before registration and rename before creation fail, no update; equality boundary succeeds; rename time earlier than previous update but not creation succeeds; old/missing metadata modification time allowed. Identical-name rename with invalid time still fails. |
| Q1  | U/I               | Get present returns exact aggregate/revision; missing becomes CanonicalNotFound; invalid stored aggregate never becomes None; get makes one load and no writes.                                                                                                             |
| Q2  | U/I               | Empty/multiple inventory; binary ID order, every status included; exactly one list call and zero loads/writes. List does not claim child validation; malformed child permits ID inventory but get fails explicitly. Malformed ID fails inventory.                           |
| M1  | U/I               | Missing canonical for add/rename/delete, distinct from missing alias and unavailable status; no update; no stub aggregate creation.                                                                                                                                         |
| C1  | U/I               | All alias commands require matching revision; stale and future revisions conflict before domain/update even if requested alias missing; state/revision unchanged.                                                                                                           |
| C2  | U/I               | Race after load before update: second connection commits, first service update conflicts with no retry. Test add, rename, delete, and identical-name rename; winning state/revision preserved.                                                                              |
| C3  | U/I               | Success returns repository revision, not guessed value; failed write consumes no revision under deterministic rollback; max revision overflow fails atomically; explicit reload/review with new revision then success.                                                      |
| F1  | U                 | Inject every persistence variant for applicable insert/load/update/list paths, table-test exact mapping; no subsequent calls or false success. Load errors prevent mutations; domain errors prevent update.                                                                 |
| F2  | I                 | Fail insert after parent/alias write and updates after parent change, alias deletion, alias insertion; service returns error; prior complete aggregate/revision (or absent new parent) verified immediately and after reopen.                                               |
| F3  | I                 | Commit-time deferred constraint failure returns error, not response; deterministic rollback keeps exact prior aggregate/revision and unrelated records. Remove fixture objects before reopening.                                                                            |
| F4  | U/I               | Busy during registration and each alias write with shorter bounded adapter timeout; safe error/no retry; explicit fresh get/retry after unlock succeeds; no hung tests or partial state.                                                                                    |
| F5  | U plus existing I | UncertainOutcome maps unchanged and makes no retry/success claim. Simulated adapter disposition remains covered by M03; fresh service/reopened adapter can get stable IDs to inspect actual state. Do not claim fake uncertainty proves hardware failure handling.          |
| F6  | I                 | Invalid stored alias key/name/ordinal/timestamp and invalid inventory ID yield safe failure; no cleanup/repair/rewrite. Corrupt/incompatible/missing/read-only DB open remains covered by M03; service does not create a DB on failure.                                     |
| L1  | I                 | For Available, Modified, all three Unavailable reasons, PossibleReplacement: get/list and add/rename/delete work; accepted location/evidence/metadata/status/registration/observation untouched through reopen. No synthetic observe/recover calls.                         |
| S1  | U/I               | Complete sequential journey and subprocess restart described below; exact expected aggregate equality at every checkpoint, not just counts. Include unaffected sibling canonical record.                                                                                    |
| S2  | I                 | Owned physical sentinel contents, filename/existence, size, modification time and sibling files unchanged after registration/alias successes, rejections, deletion of final alias, and reopen; aliases create no new files. Nonexistent supplied location also succeeds.    |
| E1  | U                 | Error Display/source categories contain no sensitive fixture names/path/evidence or SQL-like input; no panic on expected invalid input; request Debug is not used for automatic logging.                                                                                    |
| B1  | Static/build      | App production dependencies/imports limited to core/ports/std; no clocks, physical I/O, SQL/Tauri/OS APIs, new public save/lifecycle bypass, or new third-party packages. Core/ports/infra production diff remains empty.                                                   |

### End-to-end service acceptance journeys

These are end-to-end tests of the **M04 service boundary through SQLite**, not UI automation. Drive commands through public service methods and validate with service get after each step; direct repository/SQL setup is permitted only for controlled failure/lifecycle fixtures.

1. **Complete metadata journey:** create isolated DB; register A and unrelated B; get/list; add multiple aliases with exact names/times; case-only rename; identical-name rename; reject duplicate/invalid/stale commands; delete a middle alias; close all handles; reopen with a fresh service; compare every persisted field/revision and B; delete remaining aliases; verify A still exists; reopen again. Successful add/rename/delete writes advance revision exactly once; reads/rejections do not.
2. **Two callers:** both get revision n; caller B mutates to n+1; caller A's command with n conflicts; reload A, explicitly submit a reviewed command with n+1, and verify both intended changes survive. Separately interpose B between A's internal load and update via a test-only delegating repository wrapper around the real adapter, using a second connection. No production race hook or retry is added.
3. **Failure and recovery:** inject deterministic failures at each write stage, inspect prior aggregate/revision, remove fixture objects, reopen, and explicitly resubmit with freshly loaded revision. Then inject uncertain result with a spy, assert no retry, and document the required external reopen/reload decision without pretending to prove disk durability.
4. **Process restart:** writer child creates service, registers/adds/renames/deletes, exits successfully after commit; new reader child opens existing DB and verifies exact expected aggregate/revision via service; another child mutates with the returned revision and parent reopens to verify. Missing DB reader mode must fail without creating a replacement DB. M03's bounded pre-commit termination test remains regression coverage; no claim that a UI restart performs filesystem reconciliation.
5. **Unavailable/replacement metadata:** seed each status using trusted synthetic fixtures; perform alias journey through service, close/reopen, and assert status plus accepted evidence/location/metadata retained. No file access/relink/recovery is attempted or implied.

No case is considered resolved merely because a test is listed here. Implementation acceptance requires executing its assertions, fixing within-scope failures, and recording results. New evidence contradicting an approved rule must be reported for review rather than bypassed or silently changing the rule.

## Complete existing desktop regression and future UI handoff

Run all automated frontend/workspace checks, then validate the complete currently exposed desktop behavior on the supported Windows environment. These checks detect integration regressions even though new services are not wired into the shell.

| Mode             | Required steps and observable result                                                                                                                                                                                                                                         |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Development      | `npm.cmd run tauri:dev`; real window title FileNym, logos/branding/tagline, visible version from actual IPC; default 900x640 and minimum 640x440 logical client size with DPI adjustment; maximize/restore without clipping; clean close/reopen with same rendering/version. |
| Production       | Build `--no-bundle`; stop Vite; run emitted executable (normally `target/release/filenym-desktop.exe`); repeat rendering/version/window/close/reopen checks without dev server.                                                                                              |
| Browser fallback | Frontend-only Vite/browser launch renders branding/tagline, omits native version, and attempts no native persistence.                                                                                                                                                        |
| Cleanup/evidence | Bounded rendering waits before visual acceptance; capture actual observations; separate automated command results from tool-driven/visual/manual checks. Stop only test-owned app/server/processes; no residual listener or fixture handles.                                 |

No new E2E framework, Tauri command, UI harness requirement, or production DB initialization is introduced just to test services. Record any GUI block as untested; it prevents an unqualified milestone PASS. Screenshots/helpers/build outputs are temporary, not files to commit. No separate human acceptance test or hosted CI run is claimed without evidence.

When future desktop workflows are approved, their plans must add real UI-to-command-to-service-to-SQLite tests for register/get/list/add/rename/delete, validation messages, stale-edit conflicts/reload, safe handling of Busy/Unavailable/UncertainOutcome, and restart persistence. Later lifecycle UI must cover unavailability, replacement suspicion, and verified same-file recovery using experimentally established observations. These are explicit handoff obligations; implementing them is outside M04.

## Implementation files

| File                                                        | Implemented change                                                                                                      |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `crates/filenym-app/src/lib.rs`                             | Modify role docs, private module declarations, intentional exports.                                                     |
| `crates/filenym-app/src/requests.rs`                        | Create the four request structs and API documentation.                                                                  |
| `crates/filenym-app/src/error.rs`                           | Create typed application error and safe mappings/display/source.                                                        |
| `crates/filenym-app/src/service.rs`                         | Create one cohesive service and a small shared load/revision helper if duplication warrants it.                         |
| `crates/filenym-app/tests/application_services.rs`          | Create public API orchestration, validation, call-order, error, conflict tests.                                         |
| `crates/filenym-app/tests/support/mod.rs`                   | Create minimal handwritten repository spy/scripted fake and deterministic fixtures.                                     |
| `crates/filenym-infra/Cargo.toml`                           | Add local `filenym-app` dev-dependency only.                                                                            |
| `crates/filenym-infra/tests/application_services.rs`        | Create real SQLite feature/journey/failure/physical-safety tests.                                                       |
| `crates/filenym-infra/tests/application_service_restart.rs` | Create bounded worker/process restart service acceptance tests.                                                         |
| `crates/filenym-infra/tests/support/mod.rs`                 | Reuse owned-directory helper; extend narrowly only if needed for service fixture construction. Preserve existing tests. |
| `Cargo.lock`                                                | Only local dependency metadata if Cargo requires it; no resolution upgrades.                                            |
| `plans/phase-1/04-application-services.md`                  | Record approval, final APIs/coverage/results/deviations/limits after implementation.                                    |

No core/ports changes, infra production edits, SQL migrations, app manifest dependencies, frontend/Tauri code, accepted architecture docs, or CI configuration changes were made. The initial planning task created only this plan; the implementation changes are recorded below. The shared infra test support required no edits.

## Approved implementation sequence and step acceptance

| Step                                  | Files                                                   | Acceptance condition                                                                                                                        |
| ------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Review decisions                   | This plan; read current manifests/source/history        | Approval of decisions below recorded before implementation; actual baseline/API/dependency differences reassessed.                          |
| 2. Define API/data contracts          | app requests/lib/service declarations                   | Four requests, six use cases, protected repository field, concrete stored result; signatures match approved policy.                         |
| 3. Define errors                      | app error + focused tests                               | Field-specific invalid IDs; exact normalized mapping for every domain/storage category; safe Display/source.                                |
| 4. Implement registration             | app service + app tests                                 | Supplied facts only, one insert, exact returned revision, duplicate rejection, zero physical access.                                        |
| 5. Implement alias mutations          | app service + app tests                                 | Validate/load/revision/domain/update order; preserved metadata; no-op revision policy explicit; no write after domain failure.              |
| 6. Implement reads                    | app service + app tests                                 | One load/list call, missing-vs-error distinction, full retrieval, IDs-only deterministic inventory.                                         |
| 7. Complete concurrency/failure tests | app tests/support                                       | Stale caller and update race both conflict; no retries; all storage errors, uncertain outcome, overflow, no false success.                  |
| 8. Add real service acceptance        | infra manifest/tests/support; lock metadata if needed   | Matrix I cases/journeys complete; isolated DBs, exact reopen state, rollback/locking/status/sentinel proof; production graph unchanged.     |
| 9. Add process restart proof          | infra restart test                                      | Fresh processes call real services, bounded handshakes/cleanup, committed state/revisions exact and missing DB not recreated.               |
| 10. Validate regression               | Existing scripts/test suites; no unrelated source edits | Rust/frontend checks and development/production/browser matrix pass; actual commands, test names/counts, platform/GUI blocks recorded.      |
| 11. Finalize public record            | This plan                                               | Final API/schema-boundary/paths/results audited; Markdown/links/diff clean; no unsupported claims, no later scope, report exact Git status. |

A failure outside approved scope is reported with evidence; do not silently repair architecture, UI, or OS tracking. Do not skip required tests or weaken checks to obtain a pass.

## Acceptance criteria

- All six use cases work through the public service API and the existing repository trait without changing core invariants or persistence semantics.
- Every mutation validates raw inputs, loads current state, checks mandatory caller revision, invokes exactly the intended domain operation, and persists once using the loaded revision; failed operations never return success or silently retry.
- Aliases/IDs/order/timestamps/extensions/status/accepted canonical facts survive exact reload/reopen; all allowed lifecycle states permit metadata management. No physical canonical file operation occurs.
- Every matrix row has named executed coverage and recorded results, including real SQLite service journeys, failure rollback, concurrency, fresh process restart, safe errors, and owned-file preservation. Existing core/ports/infra tests remain green.
- No unqualified PASS with required blocked/unexecuted checks; distinguish simulated storage errors, deterministic rollback, actual process evidence, and untested hardware durability/platform behavior.
- All Rust/frontend checks and desktop development/production/browser regressions pass with evidence separated by automated versus visual/manual method. No desktop alias-workflow claim is made.
- Dependencies remain inward; only a local infra-to-app dev-dependency is added. No new third-party packages, SQL format change, application startup wiring, lifecycle service, or Milestone 05 work.
- Final document reflects actual implementation/results, all relative links/source paths are verified, formatting and scope audit pass, and no commit/push/branch change occurs unless separately requested.

## Validation commands

After implementation, run each command from repository root and inspect its exit status. Establish the local dev-dependency lock metadata once if needed, then use `--locked`; do not upgrade dependencies. `npm.cmd ci` is only needed if locked frontend dependencies are absent.

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check -p filenym-app --locked
cargo clippy -p filenym-app --all-targets --locked -- -D warnings
cargo test -p filenym-app --locked
cargo test -p filenym-infra --test application_services --locked
cargo test -p filenym-infra --test application_service_restart --locked
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo tree -p filenym-app --edges normal
npm.cmd run format:check
npm.cmd run lint
npm.cmd run typecheck
npm.cmd run test
npm.cmd run build
npm.cmd run tauri:dev
npm.cmd run tauri:build -- --no-bundle
.\node_modules\.bin\prettier.cmd --check plans/phase-1/04-application-services.md
git diff --check
git status --short --untracked-files=all
```

Run desktop commands as separate interactive checks with bounded cleanup, not a blocking batch. Inspect normal dependency tree and actual source imports for forbidden outward references. Existing Windows CI runs workspace tests, so new integration tests are automatically included; inspect an actual hosted run before reporting CI success. Additional Unix/macOS validation is useful but no unsupported platform guarantee or new CI job is introduced here.

The initial documentation-only planning review checked Markdown, links, and whitespace without running application tests. Implementation validation below supersedes that limited review. Review untracked source/documents explicitly because ordinary Git diff does not display them.

## Approved decisions and remaining limits

Implementation approval explicitly covered the following decisions:

1. **Boundary:** registration records caller-supplied facts, accepts current structural relative-path policy, initializes the domain's Available state without physical verification, and allows same-path/evidence records with different IDs. Physical duplicate detection and supported-file validation remain deferred.
2. **Concurrency:** caller expected revision is mandatory on every alias mutation; compare before contextual domain validation and persist using loaded revision; no automatic retries. Exact-name rename commits and advances storage revision while leaving domain fields unchanged.
3. **API:** raw ID/name strings validated inside service, other registration facts supplied as existing domain values, caller-owned IDs/times, borrowed generic repository service, and existing StoredCanonicalFile responses. No generator/clock or serialization dependency.
4. **Read scope:** list IDs only through existing port; get provides full aggregate/revision. No atomic full-registry snapshot or silent per-record error suppression.
5. **Lifecycle:** preserve all stored states during alias management but expose no observation/recovery service until its trusted caller/identity policy is reviewed.
6. **Test placement:** local infra-to-app dev-dependency, app spies for orchestration, infra real SQLite/process acceptance tests, and complete existing desktop regression with future UI workflows explicitly deferred.

These choices were approved together and implemented without changing an accepted ADR. No policy decision remains blocked; later changes require explicit review rather than silently selecting a different policy.

Remaining limits are inherited or deliberately deferred: unbounded inventory size is acceptable for the initial registry; host-native path/time portability is limited; ASCII-only Unicode comparison may permit visually similar names; caller facts are not authenticated; uncertain writes cannot be blindly retried; ID generation/idempotency and production DB wiring need later reviewed policies. Windows file identity reliability, startup observations, recovery verification, and hardware power-loss durability remain unestablished. No accepted ADR is changed and no future physical-file behavior is assumed reliable by this milestone.

## Implementation and validation record

### Actual architecture, API, and refinements

Local validation completed on 2026-10-16 against `8dce1ac` plus these uncommitted changes. The branch remained `main`. The service implements all six operations above, using four public request types and ApplicationError; no transport DTOs or lifecycle endpoints were added. Core/ports and infra production source/schema are unchanged. The service has no clocks, physical filesystem access, SQLite types, or adapter creation/recovery logic.

The concrete implementation uses private `load_current` and `save` helpers to share revision checking and committed response construction. Domain/persistence error conversions use `From` implementations; invalid ID validation uses field-specific mapping. Nested errors expose safe typed sources; normalized business errors have no underlying engine diagnostic. The repository spy strictly consumes scripted calls, checking IDs, full aggregate snapshots and revisions. It does not implement a second persistence engine.

One applicability refinement is important for F2: service registration always inserts an aggregate with **zero aliases**, so failure during registration's alias-row insertion cannot occur. New service tests inject failure before/after parent insertion; existing M03 aggregate-insert tests cover failure after alias insertion. All service alias writes test parent update, alias deletion, alias insertion, and commit failure. This is coverage of the actual contract, not an expansion to accept prepopulated registration aggregates.

The shared infra test helper was reused without modification. Test children use successful exit as the commit/restart barrier, with bounded polling and kill/wait cleanup; the short polling delay does not determine a race outcome. A test-only delegating adapter injects a real second-connection write exactly at the first service's update boundary. No production test hook, automatic retry, or filesystem classification was introduced.

`filenym-infra` adds only `[dev-dependencies] filenym-app = { path = "../filenym-app" }`. Its production dependencies are unchanged. Cargo.lock adds exactly `"filenym-app"` to the local infra package dependency list; no package was added or version/checksum upgraded. The normal app tree is app -> core/ports -> core, and Cargo accepted the test graph without a cycle.

### Named executed coverage

All test names in these tables passed in the final workspace run. IDs refer to the required matrix above; multiple test cases execute within each named test. Existing domain/repository suites also passed unchanged. No tests were ignored to obtain a pass.

Application tests: `crates/filenym-app/tests/application_services.rs`.

| Test name                                                                            | Coverage                                                                                                                |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| `registration_and_reads_use_only_the_required_calls_and_return_repository_revisions` | R1, Q1, Q2, C3; registration/get/list call order, missing record, trait-object injection, non-guessed returned revision |
| `mutations_load_apply_and_write_once_without_post_commit_reads`                      | A1?A3, C3, S1; exact expected snapshots/revision and no post-commit read                                                |
| `identical_rename_still_updates_and_invalid_time_prevents_write`                     | A2, T1; identical spelling conditional write and invalid no-op timestamp                                                |
| `primitive_validation_precedes_repository_access_in_field_order`                     | V1; every ID-bearing operation and canonical/alias/name precedence                                                      |
| `every_alias_validation_category_is_preserved_without_repository_access`             | V2, V3; add/rename, all forbidden categories/device digit variants, byte boundaries                                     |
| `contextual_domain_failures_never_write`                                             | A1?A3, T1, F1; duplicates, unknown alias, invalid timestamps, collision                                                 |
| `missing_canonical_and_stale_or_future_revisions_precede_contextual_errors`          | M1, C1; all alias commands, stale/future revisions, absent aggregate                                                    |
| `every_persistence_error_propagates_without_retry_or_false_success`                  | R2, C2?C3, F1, F4?F5; all 18 persistence categories at insert/list/get and each alias load/update                       |
| `domain_mapping_and_error_sources_are_safe_and_complete`                             | E1; normalized/nested domain categories, safe sources/displays, uncertainty instruction                                 |

Real SQLite service tests: `crates/filenym-infra/tests/application_services.rs`.

| Test name                                                                  | Coverage                                                                                                                     |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `complete_service_journey_preserves_exact_snapshots_across_reopens`        | R1?R2, A1?A3, Q1?Q2, C1, S1; full sequential journey, no-op revision, sibling preservation, zero remaining aliases           |
| `supplied_registration_facts_require_no_filesystem_resolution`             | R1, R3; nonexistent absolute location, pre-epoch supplied registration, equal evidence/path under distinct IDs, exact reopen |
| `unicode_name_boundaries_and_owner_scope_round_trip_through_services`      | V2?V4, A1, R3; exact Unicode/extension spellings, ASCII/multibyte byte limits, same alias ID/name on another owner           |
| `timestamp_boundaries_missing_records_and_metadata_are_preserved`          | T1, M1, Q1; equality boundary, backward valid update, rejected creation/rename time, canonical fact preservation             |
| `every_persisted_lifecycle_state_survives_all_alias_commands_and_reopen`   | L1, Q2; all six status/reason values, alias management, earlier observation time preserved                                   |
| `caller_staleness_and_intervening_sqlite_writers_conflict_without_retries` | C1?C3; stale caller, future revision, explicit reload/retry, real intervening writer for add/rename/delete/identical rename  |
| `write_stage_failures_rollback_service_commands_and_allow_explicit_retry`  | F2, C3, S1; write-stage rollback, reopen, explicit resubmission and unaffected sibling                                       |
| `commit_failure_never_returns_success_and_preserves_previous_snapshot`     | F3; deferred constraint at commit, all alias command forms, exact prior snapshot/revision and reopen                         |
| `service_writes_obey_bounded_locking_and_explicit_reload_retry`            | F4; 25 ms configured wait for registration and alias writes, Busy without retries, explicit success after unlock             |
| `invalid_stored_data_inventory_and_revision_exhaustion_are_never_repaired` | Q1?Q2, C3, F6; malformed names/keys/order/times, inventory ID rejection, max revision atomic failure                         |
| `service_alias_journey_never_changes_owned_physical_files`                 | S2; sentinel/sibling preservation, logical alias names never created as files, final alias deletion and reopen               |

Fresh-process tests: `crates/filenym-infra/tests/application_service_restart.rs`.

| Test name                                                                       | Coverage                                                                                             |
| ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `fresh_process_service_journey_preserves_committed_state_and_missing_db_safety` | S1, Q1, F6; missing-open, writer, reader, next-writer processes; exact fields and revisions 4 then 5 |
| `application_service_restart_worker`                                            | Harness entry for those child modes; normally returns immediately when no test environment is set    |

B1 was verified by normal dependency tree, manifest/lockfile inspection, source/import review, and Git scope audit. F5's adapter retirement is additionally covered by existing `unexpected_commit_state_retires_the_adapter`; missing/read-only/corrupt/schema checks and pre-commit termination remain covered by M03 tests. Lifecycle conflict/recovery tests remain in core/infra; no application recovery endpoint was added.

Final test accounting: **76 Rust tests passed**: 9 app, 22 core including the compile-fail doctest, 43 infra, and 2 ports. M04 adds **22 counted entries**: 9 app + 11 SQLite service + 2 restart. One new entry is the subprocess worker; 21 new entries contain substantive assertions. The overall total includes two worker entries (one inherited from M03). Table-driven cases are not counted as separate Rust tests.

### Automated project validation

| Command/check                                                                          | Result                                                                          |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `cargo fmt --all` and `cargo fmt --all -- --check`                                     | PASS                                                                            |
| `cargo check -p filenym-app --locked`                                                  | PASS                                                                            |
| `cargo clippy -p filenym-app --all-targets --locked -- -D warnings`                    | PASS                                                                            |
| `cargo test -p filenym-app --locked`                                                   | PASS; 9 tests                                                                   |
| Both focused infra service/restart test commands above                                 | PASS; also included in final workspace run                                      |
| `cargo check --workspace --locked`                                                     | PASS                                                                            |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`                       | PASS; no warnings                                                               |
| `cargo test --workspace --locked`                                                      | PASS; 76 tests including the doctest                                            |
| `cargo tree -p filenym-app --edges normal`                                             | PASS; only core/ports/std boundary                                              |
| `npm.cmd run format:check`                                                             | PASS                                                                            |
| `npm.cmd run lint`                                                                     | PASS                                                                            |
| `npm.cmd run typecheck`                                                                | PASS                                                                            |
| `npm.cmd run test`                                                                     | PASS; existing frontend test 1/1                                                |
| `npm.cmd run build`                                                                    | PASS                                                                            |
| `npm.cmd run tauri:build -- --no-bundle`                                               | PASS; rebuilt production executable                                             |
| Installed Prettier document check, relative links/source paths, whitespace/scope audit | PASS; formatted document, checked links/current source references and Git scope |

The only post-build source refinements were stronger test coverage/readability; the final sibling/sentinel assertions also passed the focused infra suite and workspace Clippy; production application source was unchanged after the executable build. No new tooling dependency was installed. These are local Windows results; the configured GitHub Actions workflow was inspected but no hosted run is asserted.

### Desktop regression evidence

| Existing behavior                       | Result and method                                                                                                                                                           |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Development launch/branding/version IPC | PASS; real Tauri window, logos/tagline and visible `v0.1.0`; accessibility text and screenshot review                                                                       |
| Window sizes/maximize/restore           | PASS in development and production; 900x640 default and 640x440 minimum logical client targets at 125% DPI, outer captures 1143x847 and 818x597; maximized content readable |
| Development close/reopen                | PASS; closed cleanly, tauri:dev relaunched, settled rendering/version inspected, closed again                                                                               |
| Production without Vite                 | PASS; rebuilt executable launched with port 5173 not listening, branding/version/window checks; clean close and fresh reopen inspected, closed again                        |
| Frontend-only browser fallback          | PASS; installed headless browser, isolated temporary profile, exit 0, DOM branding assertions and screenshot review; no native version rendered                             |
| Cleanup                                 | PASS; test app and Vite sessions closed; no FileNym process or port 5173 listener remained                                                                                  |

These GUI checks used tool-driven window actions and visual review, not a separate human acceptance run or committed desktop E2E suite. The temporary helper initially selected Tauri's hidden event window instead of its visible titled window; it was corrected to select the FileNym window within the test process before accepting GUI results. Browser process-wrapper exit-code inspection was also corrected; final direct invocation returned exit 0 and passed DOM assertions. These were harness corrections, not production changes. Temporary helpers/profiles/captures are not milestone source or files to stage.

No desktop alias-management or database-backed UI workflow was tested: these features remain unwired and absent. The Rust service/SQLite/process tests establish the implemented application behavior.

### Known limitations and completion scope

All required Windows implementation checks passed; no required feature test is blocked or deliberately skipped. Additional Unix/macOS execution remains untested; existing host-native path and SystemTime precision limits remain explicit. Simulated uncertain errors and process restart/termination do not establish behavior under arbitrary hardware power loss or real disk-I/O failures. Permission/storage failures are preserved as typed categories without claiming automatic recovery.

Registration still trusts supplied facts and does not verify file existence/identity. Same-path/evidence records can coexist. Listing IDs is not an atomic full-registry snapshot, and responses may immediately become stale. Identical-name rename intentionally advances storage revision. ID allocation, retry/idempotency decisions, production database composition, observation/recovery workflows, UI and Tauri integration remain deferred.

Core/ports/SQLite production behavior, accepted ADRs, React/Tauri source and CI configuration remain unchanged. No Milestone 05, filesystem observation, watcher, identity detection, reconciliation, relink, physical aliasing, external presentation, or cloud functionality was implemented. No commit, push, or branch change was performed.

**Final verdict: PASS within the approved Windows Milestone 04 scope.** This document was checked against the implemented APIs, files, dependency diff, executed tests, and desktop observations and finalized for public inclusion. No publication or commit was performed.
