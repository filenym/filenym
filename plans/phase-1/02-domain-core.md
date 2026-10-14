# Phase 1 Milestone 02: Domain Core

## Status

**Implemented and locally validated; not yet committed, pushed, or merged as of this review.** The domain policies were approved before implementation on 2026-10-15. Later milestones remain excluded. The implementation and validation record below describes the local work against the merged Milestone 01 foundation; it does not assert a GitHub Actions result.

This document preserves the approved design, implementation sequence, test requirements, acceptance criteria, and recorded outcomes. It is a completed milestone record, not an instruction to implement additional features.

## Objective and scope

Implement the smallest coherent, pure Rust domain model for one canonical physical file with zero or more logical aliases. Represent stable FileNym record identity, last known location, optional filesystem identity evidence, metadata, explicit availability, and conservative lifecycle changes. Enforce invariants through private fields, validated constructors, and atomic domain operations.

All new behavior belongs in `filenym-core`. It must be usable and testable without a desktop runtime, database, operating-system APIs, or filesystem access. Domain operations manipulate supplied values; they do not register a real file, discover changes, or verify physical identity.

This milestone includes domain-level alias creation, rename, deletion, and preservation during canonical state changes, plus unit and public-API invariant tests. It includes regression validation of the existing desktop application. It does not expose the new domain behavior through desktop controls.

## Repository evidence and authoritative decisions

Authoritative references: [engineering instructions](../../AGENTS.md), [problem statement](../../docs/problem-statement.md), [Phase 1 scope](../../docs/architecture/phase-1-scope.md), [domain model](../../docs/architecture/domain-model.md), [persistence](../../docs/architecture/persistence.md), [file lifecycle](../../docs/architecture/file-lifecycle.md), accepted ADRs [0001](../../docs/decisions/0001-local-first-desktop.md), [0002](../../docs/decisions/0002-aliases-are-metadata.md), and [0003](../../docs/decisions/0003-path-is-not-identity.md), and [Milestone 01](01-project-foundation.md). The design was checked against workspace/crate manifests, source, development scripts, frontend tests, Tauri configuration, and the foundation CI workflow.

- ADR 0001 establishes the local-first Tauri 2/React/TypeScript/Rust/SQLite baseline without a server or account.
- ADR 0002 makes aliases metadata, with no physical file creation or mutation.
- ADR 0003 separates stable application identity, mutable path, and platform identity evidence; Windows mechanisms require experiments.
- The workspace contains `filenym-core`, `filenym-ports`, `filenym-app`, `filenym-infra`, and `src-tauri`. Core has no dependencies. Ports depends on core; app and infra depend on core and ports; desktop depends on app and infra. At the start of this milestone all four non-desktop crates contained role documentation only. Core now contains the domain implementation; the other three remain placeholders.
- Desktop currently exposes only a version command. React renders branding, the tagline, and a version in desktop mode. No file or alias workflow exists.

Documentation context:

1. Overview/research documents retain research-stage language and future external presentation goals. Approved ADRs and Phase 1 scope govern this milestone; external presentation remains excluded.
2. Historical domain-document and ADR naming discrepancies are resolved: the repository uses `domain-model.md` and four-digit ADR names.
3. Conceptual layer diagrams describe responsibilities, not outbound dependencies from core. Actual manifests and Milestone 01's dependency rules make core inward-most and ports dependent on core.
4. Scope lists renamed/moved states; the domain-model document permits lifecycle events and left the exact distinction open. The approved Milestone 02 decision uses change results for rename/move and persistent status for availability/content state, without choosing a platform detection mechanism.

## Explicit non-goals

- No application services, repository/clock/ID-generation ports, SQLite schema, migrations, database location, serialization DTOs, or persistence implementation.
- No Tauri commands, React changes, watcher, filesystem calls, file registration service, startup reconciliation algorithm, relink UX, or replacement approval workflow.
- No Windows identity selection or experiments, file-ID reliability assumptions, hashing, content inspection, directory search, or movement inference.
- No hard links, symlinks, copying as aliases, virtual filesystems, WinFsp/FUSE, browser integration, external filename presentation, drag and drop, shell extensions, cloud services, authentication, telemetry, or AI naming.
- No changes to dependencies, manifests, lockfiles, CI, or established architectural boundaries were made for this milestone.
- No speculative implementations of Milestones 03–09. Later integrations consume domain values and must supply their own reviewed evidence and policies.

## Domain types and responsibilities

The approved responsibilities below are implemented as concrete domain types. Exact public APIs are described in the implementation record.

| Type                  | Responsibility and representation                                                                                                                                                                                                                                                         |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CanonicalFileId`     | Immutable, distinct newtype over an opaque, nonempty `String`. Supplied by the caller; never generated from a path or OS identity. No UUID-format or database-key commitment.                                                                                                             |
| `AliasId`             | Separate immutable newtype with the same opaque-value validation. Cannot be substituted for a canonical ID. Generation and registry-wide uniqueness are future application responsibilities.                                                                                              |
| `AliasName`           | Validated exact user-entered filename string plus a deterministic internal comparison key. No normalization that silently rewrites the display name or extension.                                                                                                                         |
| `KnownFileLocation`   | Validated supplied `PathBuf` representing last known location; derive filename on demand instead of storing a second independently mutable filename. Standard-library path manipulation is permitted, filesystem access is not.                                                           |
| `FileIdentity`        | Optional opaque evidence with nonempty scheme identifier and nonempty byte value. Examples in tests use synthetic schemes. Structural equality means equal evidence only, never proof of continuity. No Windows handles, volume structures, inode policy, or fallback identity synthesis. |
| `FileMetadata`        | Supplied file size (`u64`) and optional modification timestamp (`SystemTime`). Size zero and missing modification time are valid. Metadata equality is not physical identity.                                                                                                             |
| `FileStatus`          | `Available`, `Modified`, `Unavailable(UnavailableReason)`, `PossibleReplacement`. No `Deleted` assertion based only on lack of access.                                                                                                                                                    |
| `UnavailableReason`   | Small factual enum: `NotFound`, `PermissionDenied`, `Unknown`. Disconnected storage can remain unknown; do not invent platform diagnoses.                                                                                                                                                 |
| `FileChange`          | Ephemeral domain result: unchanged observation, modified, renamed, moved, combined rename/move, unavailable, possible replacement, recovered. No persisted event log or event bus. Combined results may also indicate content metadata changed.                                           |
| `LocationChange`      | Lexical `Unchanged`, `Renamed`, `Moved`, or `RenamedAndMoved` classification used in recovery results. It does not establish physical identity.                                                                                                                                           |
| `SameFileObservation` | Supplied location, metadata, optional evidence, and observation time, representing a caller-trusted same-file classification. Its constructor performs no identity verification.                                                                                                          |
| `FileObservation`     | Classified `ConfirmedSameFile(SameFileObservation)`, `Unavailable`, or `PossibleReplacement` input to the aggregate.                                                                                                                                                                      |
| `Alias`               | Alias ID, owner canonical ID, validated name, caller-supplied creation/update timestamps. Private fields; rename through owner aggregate only.                                                                                                                                            |
| `CanonicalFile`       | Stable ID, last known location, optional accepted identity evidence, accepted metadata, registration timestamp, last observation timestamp, status, and owned `Vec<Alias>`. Controls alias membership and atomic canonical state updates.                                                 |
| `DomainError`         | Typed expected domain failures; implements `Display` and `std::error::Error` manually using the standard library. No adapter error catch-all.                                                                                                                                             |

### Ownership and dependency boundaries

`CanonicalFile` is the consistency boundary for aliases. Only it can create, rename, or remove its aliases. Public accessors return read-only values and a borrowed alias slice. There are no public setters or mutable collection access. Aliases are constructed internally, deriving their owner ID from the aggregate.

This aggregate makes per-file uniqueness and ownership enforceable in memory. It does not dictate a SQLite storage layout or require SQL repositories to load entire aggregates forever. Later persistence plans must preserve these invariants transactionally; no repository design is needed here.

Types derive `Clone`, `Debug`, `PartialEq`, and `Eq` where meaningful; record IDs also derive `Hash`. Canonical entity equality compares domain snapshots, not physical files. No entity ordering or path-based identity matching is implemented. Ordinary error messages contain neither identity bytes nor full paths. No serialization, UUID, time, Unicode, or error-framework dependencies were added.

## Approved validation policies

### Alias names and uniqueness

The conservative logical-filename policy is independent of the host OS:

1. Reject empty and whitespace-only values; reject leading/trailing Unicode whitespace rather than silently trimming.
2. Reject `.` and `..`; both `/` and `\`; NUL and Unicode control characters; and the literal characters `< > : " | ? *`. A colon rejection also blocks Windows stream syntax.
3. Reject trailing periods and Windows device stems, case-insensitively, including a stem before an extension: `CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, and the superscript-digit COM/LPT variants. This is a documented product restriction, not a claim of exhaustive platform compatibility.
4. Accept internal spaces, multiple periods, dotfiles other than `.`/`..`, Unicode letters, and extensionless names. Do not reject an innocuous name merely because it contains `..` within a filename, such as `report..pdf`.
5. The entire name is limited to 255 UTF-8 bytes. This is a deterministic logical-alias limit, not an OS filename-length guarantee.
6. Duplicate comparison is ASCII case-insensitive using `to_ascii_lowercase`, scoped to one canonical file. Non-ASCII characters compare exactly, without Unicode normalization or Unicode case folding. Thus `Report.pdf` and `report.PDF` collide, while composed/decomposed Unicode names and non-ASCII case variants can remain distinct. This deliberately avoids pretending to match filesystem collation.
7. Preserve exact accepted input. Never append, strip, or replace extensions automatically. Explicit user input with a different extension is accepted as logical metadata; extension enforcement/warnings require a later product decision, not silent correction.
8. The same name may belong to different canonical records. Alias IDs must be unique within an aggregate; registry-wide ID allocation is outside core. A canonical filename need not differ from an alias name.
9. Renaming an alias excludes itself from duplicate detection. Identical spelling is a successful no-op without a timestamp update; a case-only spelling change updates the name and timestamp if no other alias collides.

The policy is also documented in the domain API. It is stricter than some native filesystems and does not promise that aliases can already be presented externally. Where multiple restrictions apply, validation reports the first matching reason; control characters are checked before whitespace.

### Record IDs, locations, and time

- Reject empty/whitespace-only IDs, leading/trailing whitespace, and control characters. Treat all other ID content as opaque; no public numeric database key or UUID generation.
- Reject an empty location, a location without a terminal filename, trailing separators, NUL, and explicit `.`/`..` components. Core does not call canonicalize, metadata, exists, or any OS API. It accepts structurally valid relative locations as well as absolute locations; future boundaries must resolve them safely. Core cannot guarantee existence, containment, permissions, or symlink safety.
- Host `PathBuf` syntax differs. Domain tests use host-native synthetic paths, including absolute aggregate fixtures and relative validation fixtures. Prefixes/components are interpreted by the standard library; an encoded separator scan catches dot components that Rust otherwise normalizes. Full adapter-level Windows path handling, absolute resolution, UNC policy, and security enforcement remain deferred.
- All time values are supplied; never call `SystemTime::now` in domain behavior. Creation/update timestamps must not precede registration/alias creation as applicable. Do not require filesystem modification time to follow registration; an old file is valid.
- Do not require observation timestamps to increase monotonically: wall-clock adjustment is possible. Event ordering and stale-observation detection require a later application policy. Invalid timestamp inputs for an alias mutation fail before mutation.

## Domain invariants

1. Canonical ID is stable through every observation/state change. IDs, path, metadata, and OS evidence are separate concepts.
2. A canonical file has zero or more aliases; an alias belongs to exactly its owning canonical record.
3. Alias create/rename/delete modifies only alias metadata, never canonical location, evidence, metadata, status, or observation timestamps.
4. Duplicate name keys and alias IDs within a canonical record are rejected. Duplicate failure leaves all existing state unchanged.
5. Canonical availability changes preserve every alias ID, name, owner, and timestamp, including when no physical file is accessible.
6. Unavailability and replacement suspicion preserve last accepted location, evidence, and metadata; only status and observation time change.
7. Same path, size, modification time, or opaque identity equality is insufficient for core to infer physical identity.
8. A supplied same-file observation can update accepted location and metadata only through an explicit trusted classification from the caller. Ambiguous/mismatching observations cannot use that update path silently.
9. Alias metadata management remains available in unavailable and replacement-suspected states. This does not imply access to file contents.
10. Validation precedes mutation; every `Err` leaves the entire aggregate unchanged. Alias deletion cannot remove a canonical registration.

## Lifecycle behavior and safe transition strategy

Use a small classified-observation input enum with three cases: `ConfirmedSameFile` carrying supplied location/metadata and optional compatible evidence, `Unavailable` carrying a reason, and `PossibleReplacement`. Every case carries a supplied observation time. Naming must make the trust boundary explicit: core accepts a classification; it does not perform identity verification.

An observation cannot replace stored identity evidence with a different scheme/value or erase it when evidence is absent. Ordinary same-file observations are first rejected with `UnresolvedReplacement` when the current status is PossibleReplacement. Otherwise, conflicting stored/supplied evidence returns `IdentityMismatch` with no mutation; the caller can separately mark possible replacement. Explicit recovery also rejects conflicting evidence. Previously absent evidence may be attached only through caller-classified same-file input. Structural equality alone does not authorize that classification.

| Current status      | Classified input                                        | Result                                                                                                                             |
| ------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Available/Modified  | Confirmed same file, unchanged location/metadata        | Available; refresh observation time; unchanged event                                                                               |
| Available/Modified  | Confirmed same file, metadata changed                   | Modified; update accepted metadata and observation time; modified event                                                            |
| Available/Modified  | Confirmed same file, location changed                   | Update location and observation time; renamed/moved/combined event; Modified only if content metadata changed, otherwise Available |
| Any                 | Unavailable                                             | Unavailable(reason); retain accepted location/evidence/metadata and aliases                                                        |
| Any                 | Possible replacement                                    | PossibleReplacement; retain accepted values and aliases                                                                            |
| Unavailable         | Confirmed same file                                     | Recover to Available or Modified; update accepted values; recovered event with relevant location/content changes                   |
| PossibleReplacement | Ordinary confirmed observation                          | Reject with `UnresolvedReplacement`; preserve all state                                                                            |
| PossibleReplacement | Explicit same-file recovery after external verification | Recover to Available or Modified; preserve ID/aliases; conflicting evidence still rejected                                         |

`recover_verified_same_file` is the explicit same-file recovery operation for unresolved replacement. It requires caller verification classification, preserves IDs/aliases, and rejects incompatible evidence. It is callable in any status: unavailable/replacement states produce `Recovered`, while available/modified states produce ordinary change results. No intentional physical replacement or unchecked relink operation exists. A later recovery plan must define how applications establish proof or obtain user intent; this milestone supplies no evidence policy or UX.

Rename/move events compare supplied lexical parent and filename components after same-file classification: filename-only change is rename, parent-only change is move, both is combined. This is reporting of supplied locations, not a search algorithm or proof that physical rename/move occurred. Path spelling differences may produce location events; no filesystem normalization guarantee is made.

`Modified` means the latest confirmed observation changed content-related metadata, not a permanent history or proof of changed bytes. An unchanged subsequent confirmed observation becomes Available. Rename/move are returned change results; status consumers do not depend on an event log. This interpretation was approved before implementation.

## Error-handling strategy

Use specific error variants with small reason enums where helpful:

- `InvalidRecordId`, `InvalidLocation`, `InvalidIdentityEvidence`, `InvalidTimestamp`.
- `InvalidAliasName(AliasNameError)` with distinguishable empty, whitespace, traversal/separator, forbidden character, device name, trailing period, and length reasons.
- `DuplicateAliasName`, `DuplicateAliasId`, `AliasNotFound`.
- `IdentityMismatch` and `UnresolvedReplacement` for prohibited canonical updates.

Unavailable and possible replacement are represented states, not automatic loss of records or generic failures. Permission denial can be represented as supplied unavailability reason; actual OS permission/persistence/filesystem errors belong in later layers. Adapter-specific SQLite, Tauri, and filesystem failures are excluded from core. Display messages describe the category without echoing full sensitive paths or identity tokens.

Fallible constructors and aggregate mutations return `Result`; infallible constructors accept validated values or unrestricted metadata/timestamps and return the value directly. Expected invalid input does not panic. Tests may unwrap valid fixtures. Aggregate mutations validate constraints before assigning fields. Future hydration APIs must use the same validation rather than unchecked field access; no serialization/hydration framework was introduced.

## Source structure and affected files

```text
crates/filenym-core/
  src/
    lib.rs             # role documentation, modules, intentional public exports
    ids.rs             # distinct supplied record IDs
    alias_name.rs      # validated name and comparison key
    alias.rs           # alias record, owner-controlled mutation
    location.rs        # supplied location value and lexical helpers
    identity.rs        # opaque optional evidence
    metadata.rs        # supplied metadata and timestamps
    lifecycle.rs       # status, reasons, classified input, change results
    canonical_file.rs  # aggregate and invariant-preserving operations
    error.rs           # typed domain errors
  tests/
    domain_invariants.rs # public API scenarios and rejection atomicity
```

Inline unit tests belong beside validation and transition logic. Public API tests verify behavior through exports rather than private fields. Avoid duplicating every unit assertion in integration tests.

Implementation modified `crates/filenym-core/src/lib.rs` and created the modules/test file shown above. This plan records approval and outcomes. No manifest, lockfile, other crate, frontend, CI, or architecture document modifications were made for Milestone 02. Wider architectural changes require separate review.

## Unit and public-API test matrix

| Area                  | Cases and required assertions                                                                                                                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Record IDs            | Valid opaque supplied IDs; empty/whitespace/control/edge whitespace rejected; canonical and alias IDs cannot substitute for one another (compile-fail documentation test if useful); IDs unchanged across lifecycle transitions       |
| Alias valid names     | Ordinary names, internal spaces, Unicode, dotfiles, extensionless names, multiple periods, explicit different extensions; exact string retained                                                                                       |
| Alias invalid names   | Empty, whitespace-only, leading/trailing whitespace, both separators, traversal tokens, colon/stream form, each forbidden character, NUL/control characters, trailing period, reserved stems with case/extension/superscript variants |
| Length/key boundaries | 255 bytes accepted, 256 rejected; multibyte boundaries; ASCII case collision; non-ASCII case and composed/decomposed forms follow documented exact behavior                                                                           |
| Alias ownership       | Zero aliases; create multiple aliases; owner ID derived from aggregate; duplicate alias ID/name rejected; same alias name on a different canonical allowed                                                                            |
| Alias rename/delete   | Rename valid, duplicate rejected, self-name no-op, case-only rename, unknown ID errors, delete only target alias, last alias removal leaves canonical; all failure snapshots unchanged                                                |
| Alias timing          | Supplied creation/update timestamps; invalid mutation time rejected; no-op leaves timestamp unchanged                                                                                                                                 |
| Metadata/location     | Zero size, absent modification time, old modification time, empty/filename-less/traversal locations; synthetic native paths without disk access                                                                                       |
| Identity separation   | Equal path/different canonical IDs; same ID/new path; absent evidence; invalid opaque evidence; equal metadata not identity; evidence conflict rejected; missing observation evidence does not erase stored evidence                  |
| Same-file lifecycle   | Unchanged, content metadata change, rename, move, combined rename/move, location plus metadata change; stable ID and aliases; event/status interpretation and supplied times                                                          |
| Unavailability        | Not found, permission denied, unknown; repeated input idempotent except supplied observation time; preserve accepted values and every alias field                                                                                     |
| Replacement           | Same-path replacement suspicion preserves accepted values; cannot silently recover; repeated suspicion safe; explicit verified recovery succeeds only with compatible evidence; rejected recovery atomic                              |
| Recovery              | Unavailable to same-file available/modified at same/new location; no inferred recovery from path alone; aliases retained; intentional replacement absent from API                                                                     |
| Status cross-product  | Table-driven tests cover every current status against each classified input, plus explicit recovery and evidence conflicts                                                                                                            |
| Side-effect boundary  | Public API sequence creates/renames/deletes aliases and applies lifecycle states entirely with in-memory fixtures; static inspection confirms no I/O, clock reads, platform imports, or outbound crate dependencies                   |

Use deterministic supplied timestamps, IDs, and evidence. No test needs to touch user files or even create physical canonical files. No Windows continuity claim can be established by synthetic domain tests.

## Complete desktop application validation

Milestone 02 requires domain coverage plus regression validation of all existing desktop behavior in development and production builds. New domain behavior is not connected to desktop controls, so alias/registration/relink UI workflows cannot be reported as tested. Later milestones must add desktop tests when those UI/commands exist.

Run on Windows using the existing toolchain and locked dependencies. Record OS/toolchain, commit/revision under test, commands, exit codes, and manual results in the implementation handoff. Manual GUI evidence is required; a frontend unit test alone does not prove the desktop binary works.

| Desktop check               | Procedure and expected result                                                                                                                                                         |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Existing frontend behavior  | Run existing Vitest suite, lint, typecheck, and frontend build; shell test passes without additions solely mirroring implementation                                                   |
| Development launch and IPC  | Launch `npm.cmd run tauri:dev`; window titled FileNym opens, branding/logo/tagline render, version reads `0.1.0` from real Tauri command, no visible error or crash                   |
| Window behavior             | Resize at default and configured minimum sizes, maximize/restore, inspect clipping and logo readability; no regression in existing content                                            |
| Close/reopen                | Close cleanly, terminate any remaining development process, relaunch; same shell/version visible, no new file-registry state or side effects                                          |
| Production build            | Run `npm.cmd run tauri:build -- --no-bundle`; start built executable at `target/release/filenym-desktop.exe`; repeat rendering, IPC, window, close/reopen checks without Vite running |
| Browser fallback            | Run frontend-only development mode; shell remains usable without desktop version and without invoking native file functionality                                                       |
| Feature coverage accounting | Match every domain feature to automated tests; record desktop exposure as absent for Milestone 02, rather than claiming an unimplemented end-to-end workflow passed                   |

No new E2E framework, automation dependency, installer distribution, or source modifications are required for these checks. Record any blocked GUI checks explicitly; acceptance requires completed visual inspection of the running application. Desktop regression failure blocks acceptance, and repairs outside this milestone require scope review.

When future desktop workflows are implemented, their own plans must test real UI-to-command-to-service behavior for registration, alias create/rename/delete/validation, retained metadata on unavailability, conservative replacement, verified recovery, and restart persistence. This is a coverage obligation, not implementation design for those milestones.

## Approved implementation sequence

The following sequence records the reviewed implementation tasks, now completed:

1. Obtain review of this plan, especially the decisions listed below. Record resolved choices in the plan before code begins.
2. Recheck the repository diff and crate boundaries; preserve unrelated work. Implement supplied ID, location, metadata, evidence, and error value types with focused tests.
3. Implement and document approved alias validation/comparison rules, including extension handling, Unicode behavior, byte limit, and reserved-name policy. Complete boundary tests before aggregate mutation logic.
4. Implement alias record and canonical aggregate construction with private fields/read-only access. Add create/rename/delete metadata-only operations and failure-atomicity tests.
5. Implement the reviewed status/event representation and classified observation transitions. Add evidence-conflict safeguards and explicit verified same-file recovery, without adapters or intentional replacement.
6. Add public API invariant scenarios and table-driven transition tests. Review every mutating operation for preserved canonical ID, aliases, accepted evidence, and atomic errors.
7. Run Rust formatting and focused core validation, then workspace and frontend foundation checks. Inspect manifests/imports and the diff for forbidden dependencies or side effects.
8. Run the complete desktop regression matrix in development and production modes; record results and untested/blocking conditions accurately.
9. Assess acceptance criteria, update only the plan's implementation status/results as appropriate, and report changed files, policies, test evidence, and remaining limitations. Do not broaden into a later milestone.

## Validation commands

Run from the repository root using PowerShell. These commands are retained for reproducibility; recorded implementation results appear below. Run each independently and inspect its result. This documentation review did not rerun application validation.

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check -p filenym-core --locked
cargo clippy -p filenym-core --all-targets --locked -- -D warnings
cargo test -p filenym-core --locked
cargo test -p filenym-core --doc --locked
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm.cmd run format:check
npm.cmd run lint
npm.cmd run typecheck
npm.cmd run test
npm.cmd run build
npm.cmd run tauri:dev
npm.cmd run tauri:build -- --no-bundle
git diff --check
git status --short
```

Use `npm.cmd ci` only if locked frontend dependencies need installing; do not update manifests or lockfiles. Run interactive desktop checks separately after automated commands. Verify the actual executable output path if build tooling reports a different target directory. Routine CI already covers workspace/frontend foundation checks; this milestone needs no CI configuration change.

## Acceptance criteria

- Plan policies reviewed before implementation; no unresolved blocking domain policy left to accidental coding choices.
- Core remains dependency-free and independent of ports/app/infra/Tauri/SQLite/React/OS APIs and filesystem I/O.
- Supplied record IDs remain stable and clearly distinct from mutable paths and optional filesystem evidence.
- Alias validation, per-canonical uniqueness, ownership, and metadata-only operations are enforced through public APIs.
- All rejected operations are atomic; aliases and accepted metadata survive unavailability/replacement suspicion.
- Lifecycle transition matrix and explicit same-file recovery are tested without asserting unproven platform semantics.
- No automatic physical replacement, unchecked relink, inferred identity from path, or extension rewriting exists.
- All required formatting, linting, compilation, unit/public API tests, frontend checks, and desktop development/production regression checks pass with recorded evidence.
- Every new feature has domain test coverage; desktop exposure limitations are stated accurately.
- Implementation diff stays within approved core source/test files and plan updates, with no dependency/lockfile, other layer, or architecture document change.

## Risks, assumptions, and unresolved decisions

### Approved design decisions

1. Alias policy: conservative forbidden/device names, 255 UTF-8-byte limit, rejection rather than trimming, ASCII-only case-insensitive uniqueness, and exact non-ASCII comparison. Unicode-normalization/case-folding alternatives require a separate dependency/policy assessment.
2. Extension policy: preserve exact user input and allow explicitly different extensions as metadata. If extension matching is desired, approve its detailed dotfile/multipart/extensionless rules before coding.
3. Aggregate boundary: canonical owns in-memory aliases and enforces per-file uniqueness; future persistence remains free to store separate related records.
4. Lifecycle interpretation: persistent availability/content status, ephemeral rename/move events, Modified as the latest observation change, and a guarded explicit same-file recovery path from PossibleReplacement.
5. Supplied opaque string IDs, standard-library timestamp/path values, and minimally opaque evidence rather than generated UUIDs, persistence formats, or a platform identity schema.

Design review approved these domain policies together, with explicit clarifications confirming their semantics. No ADR amendment or architectural rewrite was required.

### Deferred questions and practical limits

- Which Windows evidence survives rename/move, cross-volume operations, replacement, and removable/network storage remains unknown. Matching opaque values can be misleading, including reuse; experiments and future application policies are mandatory before real automatic tracking.
- Caller classification is a trust boundary, not a security proof. Future application services must never label an observation same-file based merely on path, size, timestamp, watcher event, or opaque token equality.
- `PathBuf` lexical operations are host-dependent. Core performs structural checks only; adapters must handle real path security and permissions. No portable filesystem equivalence is asserted.
- ASCII-only alias comparison permits visually similar or canonically equivalent Unicode names. This must be accepted as a documented logical policy rather than hidden behind platform assumptions.
- Aggregate ownership may require later efficient hydration/transaction choices. Do not preemptively build repositories or pagination abstractions to solve hypothetical scale.
- Wall-clock timestamp ordering and actual stale-observation handling are deferred. The core records supplied observations; it cannot reconstruct filesystem history.
- Desktop smoke checks validate the current shell and boundary wiring, not domain features inaccessible through UI. Full Phase 1 desktop workflow validation belongs to the milestones that expose those workflows.

## Completion record

Milestone 02 was implemented and locally validated following design approval. Verdict: **PASS** within the approved domain-only scope. No later milestone behavior was implemented. Commit/push/merge remain pending as of this review.

## Implementation and validation record

### Environment and revision

- Local validation date: 2026-10-15.
- Windows NT 10.0.26200.0; window DPI 120 (125% scaling).
- Rust `1.99.0 (b940084d7 2026-09-28)`; Cargo `1.99.0 (5f94df478 2026-08-27)`; Node `v22.21.1`; npm `10.9.4`.
- Base revision: `7d65d489952f53639f23748f726d294c41d4c8af` plus the uncommitted Milestone 02 changes described here.

### Implemented files and public boundary

Modified `crates/filenym-core/src/lib.rs` to export the domain API. Created `ids.rs`, `alias_name.rs`, `alias.rs`, `location.rs`, `identity.rs`, `metadata.rs`, `lifecycle.rs`, `canonical_file.rs`, and `error.rs` under core `src/`, and `tests/domain_invariants.rs`. Updated this plan with approval and validation evidence. No other tracked source/configuration documents changed.

Public value/entity types are `CanonicalFileId`, `AliasId`, `AliasName`, `KnownFileLocation`, `FileIdentity`, `FileMetadata`, `Alias`, and `CanonicalFile`. Lifecycle types are `FileStatus`, `UnavailableReason`, `SameFileObservation`, `FileObservation`, `LocationChange`, and `FileChange`. Errors are `DomainError` and `AliasNameError`.

Validated values expose `new` constructors and read-only accessors. Alias construction/mutation is crate-private. The aggregate exposes `create_alias`, `rename_alias`, `delete_alias`, `observe`, and `recover_verified_same_file`, plus read-only canonical/alias access. `SameFileObservation` is a documented caller trust boundary, not evidence verification. Recovery with incompatible evidence fails; no intentional replacement API exists.

Canonical and metadata constructors accept already validated values and supplied times; they are infallible where no further validation is required. All fallible validation and mutation APIs return typed errors. No mutable collection/field access is public. Constructors generate neither IDs nor timestamps. Core's manifest remains dependency-free.

Location validation rejects explicit dots even where Rust normalizes components, including Windows drive-relative `C:.\a` and `C:..\a`, by checking the encoded path after the platform-parsed prefix. This uses standard-library lexical operations only. It makes no claim of absolute resolution, existence, symlink containment, or Windows physical identity continuity.

### Automated results

The recorded local implementation runs below completed successfully. Test counts are corroborated by the source and captured command results: 6 inline unit tests, 12 public API tests, and 1 compile-fail doctest. Early validation failures were corrected before the final suites passed. These are local results, not GitHub Actions results or new runs performed during documentation review.

| Command                                                              | Final result                                                           |
| -------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `cargo fmt --all`                                                    | PASS; core source formatted                                            |
| `cargo fmt --all -- --check`                                         | PASS                                                                   |
| `cargo check -p filenym-core --locked`                               | PASS                                                                   |
| `cargo clippy -p filenym-core --all-targets --locked -- -D warnings` | PASS; no warnings                                                      |
| `cargo test -p filenym-core --locked`                                | PASS; 6 inline unit tests, 12 public API tests, 1 compile-fail doctest |
| `cargo test -p filenym-core --doc --locked`                          | PASS; distinct-ID compile-fail doctest                                 |
| `cargo check --workspace --locked`                                   | PASS; all workspace crates                                             |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`     | PASS; no warnings                                                      |
| `cargo test --workspace --locked`                                    | PASS; all 19 core tests, no tests in placeholder crates                |
| `npm.cmd run format:check`                                           | PASS                                                                   |
| `npm.cmd run lint`                                                   | PASS                                                                   |
| `npm.cmd run typecheck`                                              | PASS                                                                   |
| `npm.cmd run test`                                                   | PASS; existing frontend test, 1/1                                      |
| `npm.cmd run build`                                                  | PASS; Vite production frontend                                         |
| `npm.cmd run tauri:build -- --no-bundle`                             | PASS; final executable built after final core fixes                    |
| `git diff --check`                                                   | PASS                                                                   |

The table-driven tests cover all four statuses against same-file, unavailable (all three reasons), and replacement observations, plus explicit recovery and evidence conflicts. Additional loops cover every rename/move/content-change combination and every recovery location/content combination. Aggregate mutation rejections check whole-aggregate snapshots. Alias tests cover creation, owner derivation, case duplicates, duplicate IDs, per-file scope, Unicode case/composition differences, exact extension preservation, no-op and case-only renames, timestamp failures, unknown IDs, targeted deletion, and mutations during unresolved states. Validation tests cover every COM/LPT digit and superscript variant with extensions, forbidden forms, exact 255/256-byte boundaries, and multibyte limits.

Static source inspection found no filesystem I/O, process execution, operating-system identity APIs, watcher imports, Tauri/SQLite dependencies, or clock reads in core. Temporary desktop automation remained outside production source and added no dependencies.

### Desktop automation and visual inspection results

Window operations were driven by temporary PowerShell/UI Automation helpers against real running Tauri windows, followed by screenshot-based visual inspection for content, clipping, and readability. This was tool-driven testing with visual review, not a separate human acceptance test or a committed automated desktop test suite. No production UI or command was changed for testing.

| Check                                    | Result and evidence                                                                                                                                                                 |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Development launch                       | PASS; `npm.cmd run tauri:dev` built/launched the real window with FileNym title, both logos, branding/tagline, and visible `v0.1.0` from existing version IPC                       |
| Development window behavior              | PASS; default logical client 900x640 and minimum 640x440 (outer windows 1143x847 and 818x597 at 125% DPI); maximize/restore inspected without clipping at the configured sizes      |
| Development close/reopen                 | PASS; WindowPattern close exited cleanly; Tauri/Vite helper stopped, `tauri:dev` relaunched, shell/version restored; app closed again                                               |
| Production build/run                     | PASS; `target/release/filenym-desktop.exe` built and launched with no Vite listener for the initial production checks                                                               |
| Production rendering/IPC/window behavior | PASS; default/minimum/maximize/restore matched development rendering; real visible version verified; clean close succeeded                                                          |
| Production reopen                        | PASS; executable reopened, same branding/tagline/version visually verified after frontend dev server stopped; clean close succeeded                                                 |
| Browser fallback                         | PASS; frontend-only `npm.cmd run dev` served the app to installed headless Edge with an isolated temporary profile; screenshot showed logos/branding/tagline and no desktop version |
| Cleanup                                  | PASS; no FileNym app or Vite 5173 listener remained after regression checks                                                                                                         |

Screenshots and accessibility snapshots were local, temporary evidence and are not included as repository artifacts. The table preserves the implementation-time observations; it does not establish a durable desktop automation harness or hosted CI evidence. Accessibility alone was insufficient because the WebView tree intermittently omitted content. Visual inspection supplemented it. Window sizing was checked in logical units with DPI and non-client frame adjustments; incorrectly sized preliminary attempts were excluded from the final results.

No desktop alias, file registration, persistence, watcher, reconciliation, or relink workflow was tested or exposed. Those features do not exist in this milestone. Domain behavior was exercised through the pure Rust API only.

### Remaining limits

- Validation was performed on Windows; host-native lexical path behavior on other platforms is not experimentally validated here.
- Opaque evidence and `ConfirmedSameFile` are externally supplied facts. No Windows identity tracking mechanism, file continuity proof, or intentional replacement workflow was implemented.
- ASCII-only alias uniqueness deliberately leaves non-ASCII case and Unicode normalization variants distinct. Extension differences are deliberately accepted without rewriting.
- Domain observation times allow wall-clock regression; external event ordering remains deferred.
- Installer bundles and distribution were not tested; the approved production check used `--no-bundle`.
- Hosted GitHub Actions results were not inspected; local validation does not establish a successful hosted run.

All approved Milestone 02 acceptance checks passed within these stated boundaries. No manifests, lockfiles, dependencies, other crate implementations, React features, Tauri commands, or architecture documents changed.
