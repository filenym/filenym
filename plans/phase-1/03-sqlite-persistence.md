# Phase 1 Milestone 03: SQLite Persistence

## Status

**Implemented and locally validated; not yet committed or merged.** The persistence design and domain API refinements were approved before implementation on 2026-10-15. This document retains the approved scope, rationale, test requirements, and implementation record. No GitHub Actions success is asserted.

Implemented against merged Milestone 02 at `f88234a` (pull request #12, domain implementation commit `3282181`). The branch remained `main`; no commit or push was performed during implementation.

## Objective and scope

Persist validated canonical aggregates and their logical aliases locally in SQLite, preserving exact domain state across connection closure and process restart. Database operations affect FileNym metadata only. They must never create, read, copy, rename, move, or delete registered physical files.

Include a minimal repository contract, SQLite adapter, explicit initial migration, connection initialization, lossless value codecs, validated domain rehydration, atomic aggregate insertion/update, conflict detection, and comprehensive automated tests. Include regression validation of all existing desktop behavior. Persistence is exercised through Rust repository APIs; no new desktop controls were introduced.

## Repository evidence and architectural constraints

Reviewed the engineering instructions, [problem statement](../../docs/problem-statement.md), [Phase 1 scope](../../docs/architecture/phase-1-scope.md), [domain model](../../docs/architecture/domain-model.md), [persistence guidance](../../docs/architecture/persistence.md), [file lifecycle](../../docs/architecture/file-lifecycle.md), all accepted ADRs, both preceding milestone plans, workspace/crate manifests, domain source and tests, frontend tests, Tauri configuration, and the foundation CI workflow.

- [ADR 0001](../../docs/decisions/0001-local-first-desktop.md): local-first desktop, SQLite metadata, no server/account/cloud requirement.
- [ADR 0002](../../docs/decisions/0002-aliases-are-metadata.md): aliases are metadata; no file presentation or physical copies.
- [ADR 0003](../../docs/decisions/0003-path-is-not-identity.md): application record IDs, mutable locations, and optional filesystem evidence remain separate.
- Core owns aliases in an ordered `Vec<Alias>`. IDs and names are validated values. Alias uniqueness is scoped to an aggregate and folds ASCII case only. Accepted spelling/extensions and non-ASCII distinctions are preserved.
- `CanonicalFile::new` starts Available, sets observation time to registration time, and has no aliases. Alias mutation and lifecycle observation methods cannot restore every stored snapshot exactly. Rehydration is therefore a concrete API gap, not a reason to weaken field protection.
- `FileMetadata::size` is `u64`; timestamps are supplied `SystemTime`; paths are native `PathBuf`; identity is optional opaque scheme/bytes. At the milestone start no SQLite dependency or persistence interface existed; both are now implemented behind the established boundaries.
- Core is dependency-free; ports depends on core; app and infra depend on core and ports; desktop composes app and infra. Preserve this graph.

Documentation differences and missing requirements:

1. Milestone 02's status is a pre-merge review snapshot; current Git history confirms its merge. That historical status was not rewritten during this milestone.
2. Architecture persistence fields/repositories are explicitly conceptual. Do not duplicate a filename already derived from location, invent canonical creation/update timestamps absent from core, or introduce separate alias repositories that bypass aggregate invariants.
3. Domain alias IDs are enforced per aggregate, not globally. The implemented composite alias primary key preserves this policy; making alias IDs globally unique would require an additional approved rule.
4. Lifecycle change results are ephemeral. Persist statuses and accepted facts, not rename/move events or an event history. Loading stored Modified must not change it to Available; loading PossibleReplacement must not constitute recovery.
5. Domain permits backward observation clocks, modification times older than registration, and structurally valid relative paths. Persistence must not silently add monotonic-clock, absolute-path, or physical identity assumptions.
6. Overview/research language about external presentation does not expand the approved Phase 1 scope. Windows identity mechanisms remain experimentally unresolved.

## Explicit non-goals

- No application services, registration workflow, Tauri commands/composition changes, React screens, production database startup wiring, or new UI persistence workflows.
- No watchers, Windows file identity detection, filesystem searches, startup filesystem reconciliation, relink UX, or intentional replacement workflow.
- No physical alias files, copying mechanism, hard links, symlinks, virtual filesystems, browser/upload integration, shell extensions, cloud storage, accounts, telemetry, or AI features.
- No ORM, async runtime, connection pool, generic repository framework, caching, event sourcing, backup/sync feature, encryption infrastructure, or automatic database repair.
- No canonical registration removal API in this milestone. Alias deletion is persisted as an aggregate update; deliberate registration removal requires a later reviewed use case.
- No Milestone 04 implementation. Database reopen tests simulate persistence restart only, not application services or filesystem reconciliation.

## Crate changes and dependency

| Crate                             | Implemented change                                                                                                                                                                                |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `filenym-core`                    | Add narrowly scoped validated snapshot/restore APIs and a read-only alias comparison-key accessor, approved before implementation. Remain independent of SQLite, serialization, OS APIs, and I/O. |
| `filenym-ports`                   | Add one aggregate repository trait, storage revision/result values, and typed persistence errors expressed in domain terms.                                                                       |
| `filenym-infra`                   | Implement SQLite connection/migrations, repository, value codecs, and integration tests. All SQL and native path encoding stay here.                                                              |
| `filenym-app`, `src-tauri`, React | No production changes. Run existing regression checks.                                                                                                                                            |

Infra uses `rusqlite = { version = "=0.40.2", default-features = false, features = ["bundled"] }`. The published tagged manifest identifies active development and was reviewed before dependency selection. It declares no `rust-version`; compatibility was demonstrated with Rust 1.99.0 and the existing Windows native build toolchain, rather than inventing an upstream MSRV. Bundling supplies direct synchronous SQLite bindings without an ORM or a separately installed runtime engine. Tests report SQLite 3.53.2 through `libsqlite3-sys` 0.38.2. `Cargo.lock` includes the resolved dependency graph and must accompany the eventual commit. See the [selected manifest](https://raw.githubusercontent.com/rusqlite/rusqlite/v0.40.2/Cargo.toml) and [published API](https://docs.rs/rusqlite/0.40.2/rusqlite/).

No additional time, UUID, serialization, migration, or test framework dependency was added. Standard-library facilities implement timestamp/path codecs and owned temporary test directories. Native bundled compilation and test isolation passed on Windows.

## Minimal persistence contract

The synchronous `CanonicalFileRepository` trait in ports uses mutable receiver methods to make transaction/connection ownership explicit:

| Operation                                   | Semantics                                                                                                                                                                                                                              |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `insert(&CanonicalFile)`                    | Insert a new aggregate and return initial `PersistenceRevision(0)`. Existing canonical ID is an error; never overwrite it.                                                                                                             |
| `load(&CanonicalFileId)`                    | Return `Option<StoredCanonicalFile>`: None only when the parent ID is absent; present invalid data returns an error. Stored value contains validated aggregate plus revision.                                                          |
| `update(&CanonicalFile, expected_revision)` | Update an existing aggregate atomically, requiring the current revision. Return incremented revision. Missing parent and stale revision are distinct errors.                                                                           |
| `list_ids()`                                | Return validated canonical IDs in deterministic binary order, including unavailable/replacement records. Justified as the minimal inventory needed by later loading/reconciliation; no filtering, pagination, or service is added now. |

Concrete `StoredCanonicalFile` and `PersistenceRevision` values keep storage coordination separate from file identity and domain timestamps. No generic transaction/unit-of-work abstraction was introduced. Prevent lost updates from independently loaded snapshots: exactly one writer with a given revision succeeds; the other must reload. Checked revision increment rejects exhaustion without modifying data.

No ID generation, clock reads, lifecycle inference, or physical file access occurs in the repository. A missing canonical file on disk is unrelated to a missing registration in SQLite. Repository callers supply domain-approved snapshots; infrastructure must not manufacture ConfirmedSameFile classifications while loading or saving.

## Version 1 schema

The initial migration is compiled into infra from [0001_initial.sql](../../crates/filenym-infra/migrations/0001_initial.sql). The tables below describe the implemented STRICT schema; the selected bundled engine supports STRICT tables. [SQLite STRICT table behavior](https://www.sqlite.org/stricttables.html).

### `canonical_files`

| Columns                                        | Storage and rules                                                                                                                    |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `id`                                           | TEXT, non-null primary key, binary comparison. Caller-supplied FileNym ID; no integer row ID exposed as domain identity.             |
| `path_encoding`, `path_value`                  | TEXT encoding tag and nonempty BLOB, described below. No path uniqueness constraint.                                                 |
| `identity_scheme`, `identity_value`            | Both NULL for absent evidence, or nonempty TEXT and nonempty BLOB together. No uniqueness constraint and no decoding of OS evidence. |
| `size_u64_be`                                  | Exactly 8-byte BLOB containing unsigned big-endian `u64`. Supports zero through `u64::MAX` without a signed integer cast.            |
| `modified_seconds`, `modified_nanoseconds`     | Optional normalized timestamp pair; both NULL or both present.                                                                       |
| `registered_seconds`, `registered_nanoseconds` | Required supplied registration timestamp pair.                                                                                       |
| `observed_seconds`, `observed_nanoseconds`     | Required supplied last-observation pair; may precede registration.                                                                   |
| `status`                                       | TEXT discriminator: `available`, `modified`, `unavailable`, `possible_replacement`. Unknown values are rejected.                     |
| `unavailable_reason`                           | `not_found`, `permission_denied`, or `unknown` only when status is unavailable; NULL otherwise.                                      |
| `revision`                                     | Nonnegative INTEGER, initially zero, checked increment on successful aggregate update.                                               |

Explicit constraints must enforce paired NULL rules, enum membership, reason/status compatibility, blob lengths, and timestamp nanosecond range. Avoid CHECK expressions that accidentally admit partial NULL states through SQL's three-valued logic. No copies of registered file content are stored.

### `aliases`

| Columns                                  | Storage and rules                                                                                                                                                      |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `canonical_file_id`, `id`                | Required TEXT; composite primary key `(canonical_file_id, id)`. Owner references `canonical_files.id`, with `ON DELETE RESTRICT`, `ON UPDATE RESTRICT`.                |
| `name`                                   | Required TEXT preserving exact accepted Unicode spelling and extension; byte length 1–255 using its UTF-8 BLOB length. Domain validation handles full filename policy. |
| `name_key`                               | Required BLOB of the domain's ASCII-folded comparison key in UTF-8; unique `(canonical_file_id, name_key)`.                                                            |
| `position`                               | Nonnegative INTEGER with unique `(canonical_file_id, position)`; preserves the aggregate Vec order. Decoder requires contiguous positions from zero.                   |
| `created_seconds`, `created_nanoseconds` | Required supplied creation timestamp.                                                                                                                                  |
| `updated_seconds`, `updated_nanoseconds` | Required supplied update timestamp.                                                                                                                                    |

Index coverage comes from the primary key and owner-scoped uniqueness constraints. Do not add speculative path/identity/search indexes. Neither unavailable status nor replacement suspicion deletes rows. A direct parent delete with aliases fails; no cascading deletion policy or physical file operation is attached to it. Foreign keys must be enabled for every connection. [SQLite foreign-key enforcement](https://www.sqlite.org/foreignkeys.html).

### Uniqueness and integrity boundaries

Core remains the naming-policy authority. Expose `AliasName::comparison_key()` read-only rather than duplicating its logic in infra. Bind its UTF-8 bytes as BLOB so SQL comparison is exact byte comparison; recompute/compare the stored key during load. A mismatching key is invalid stored data, never silently repaired. Full aggregate restore also checks duplicate domain keys.

Do not use Unicode normalization, full Unicode case folding, or a custom collation. SQLite's built-in NOCASE handles ASCII case rather than full Unicode equivalence; it is not the selected uniqueness mechanism. [SQLite datatype/collation documentation](https://www.sqlite.org/datatype3.html).

SQLite protects structural integrity, ownership, storage classes, supported enum values, and unique IDs/keys/positions. Core validates IDs, names, timestamps relative to registration/creation, and aggregate ownership. Infra validates binary encoding, discriminators, revision ranges, ordinal continuity, and comparison-key consistency. Direct malformed SQL fixtures may pass some structural constraints, but must still fail validated load; supported writes only use repository APIs.

## Lossless serialization

### Paths

The codec persists the supplied native path exactly, without canonicalize, lossy conversion, normalization, environment expansion, or filename duplication. Implemented format tags:

- `windows_utf16le_v1`: Windows native code units via `encode_wide`, serialized as little-endian 16-bit values; restore with `OsString::from_wide`. Preserve unpaired surrogate code units if accepted by the domain.
- `unix_bytes_v1`: Unix native bytes via `OsStrExt`/`OsStringExt`. Preserve non-UTF-8 bytes if accepted by the domain.

Conversion helpers are cfg-specific infrastructure code, not filesystem identity detection. Do not persist `OsStr::as_encoded_bytes()` as an unspecified cross-version format; Rust documents restrictions on transporting that representation. Windows encoding does preserve native code units without requiring well-formed UTF-16. [Rust OsStr encoding contract](https://doc.rust-lang.org/std/ffi/struct.OsStr.html#method.as_encoded_bytes), [Windows native encoding](https://doc.rust-lang.org/std/os/windows/ffi/trait.OsStrExt.html#tymethod.encode_wide).

Unknown/foreign-platform tags fail explicitly with `UnsupportedPathEncoding`; do not reinterpret a Windows blob as a Unix path. A tagged format is portable as stored data, not a promise that a path is usable on another OS. Reject malformed lengths/embedded NUL/domain-invalid locations during load. Core accepts some relative paths; store them unchanged and leave safe resolution to later boundaries. Test drive paths, Unicode, UNC/extended forms accepted by the current domain, relative paths, and separator spelling on their native host without creating those files.

### Size, identity, and timestamps

Opaque identity bytes use bound BLOB parameters, including zero bytes; absent evidence remains NULL, distinct from an invalid empty payload. Preserve scheme strings exactly, including embedded NUL allowed by the existing opaque-evidence domain API. SQL nonempty validation measures UTF-8 BLOB length to avoid text-length truncation at NUL; domain reconstruction rechecks validity.

Store file size in the fixed-width representation above because the domain admits unsigned values outside SQLite's signed integer range. Tests must prove the maximum value round trips. No hash/content read or numeric coercion is permitted.

Timestamp pairs use signed `i64` Unix seconds rounded down plus nanoseconds in `0..1_000_000_000`. Thus half a second before epoch is seconds -1 and nanoseconds 500,000,000. Preserve every host-representable timestamp and absence exactly; use checked arithmetic and `SystemTime` checked addition/subtraction. A value outside the codec or host's representable range returns a typed error, without truncation, saturation, or partial write. The decoder re-encodes the resulting `SystemTime` and rejects any host rounding. Windows `SystemTime` has 100 ns resolution, so finer stored values fail explicitly instead of being silently rounded. The storage pair supports nanoseconds; the domain value remains limited by the host. Do not store all nanoseconds in one `i64` or use floating-point seconds. [Rust checked SystemTime arithmetic](https://doc.rust-lang.org/std/time/struct.SystemTime.html#method.checked_add).

Observation timestamps need not be monotonic. Modification timestamps need not follow registration. Alias creation must not precede registration; updated time must not precede alias creation, but may precede a previous update under the existing wall-clock policy. Restore those same rules, not stricter invented clock semantics.

## Validated domain rehydration

Existing public APIs cannot exactly reconstruct stored status, distinct observation time, and alias creation/update times without replaying synthetic events or violating invariant protection. Replaying observations would conflate decoding with trusted filesystem classification and can clear Modified or recovery guards. Do not do it.

The `rehydration` module implements `CanonicalFileSnapshot`, `AliasSnapshot`, and `CanonicalFile::restore(snapshot) -> Result<CanonicalFile, DomainError>`. Snapshots contain supplied domain values/timestamps, with private fields and explicit constructors; they contain no SQLite types, row names, encoded bytes, revisions, or serde derives. Their role is immutable validated reconstruction input, not public entity setters.

Restoration must:

1. Use existing validated ID/name/location/identity values; infra invokes their constructors before assembling snapshots.
2. Check each alias owner equals canonical ID; reject duplicate alias IDs and comparison keys; preserve the supplied Vec order.
3. Check creation at/after registration and update at/after creation. Reject all invalid aliases before producing any aggregate.
4. Preserve canonical ID, accepted location/evidence/metadata, registration/observation times, exact status/reason, and every alias field without clock reads or lifecycle methods.
5. Represent an arbitrary valid stored status; this operation does not assert availability or verify identity. Future application boundaries must still authorize lifecycle changes.

Alias's entity constructor/mutation remains private to core and ownership is controlled by the aggregate. `DomainError::AliasOwnershipMismatch` reports inconsistent restoration ownership; existing duplicate/timestamp errors are reused. No unchecked setters, invalid default placeholders, or raw reconstruction bypasses. Add core tests proving valid snapshot equality and rejection of each invariant violation. A read-only `AliasName::comparison_key()` exposes the existing key, without changing equality or normalization behavior.

These API additions were explicitly approved before implementation. Existing constructors, mutation semantics, and alias/lifecycle policies remain unchanged. `AliasSnapshot::new` validates creation/update ordering. `CanonicalFileSnapshot::new(canonical, status, observed_at, aliases)` uses a valid canonical value as its facts/registration input and validates the replacement status/time/alias collection through restoration. `CanonicalFileSnapshot::from(&file)` captures a complete existing aggregate. These are reconstruction inputs, not mutable entity setters; status assignment does not verify physical identity.

## Transactions, revisions, and error handling

### Writes and reads

- Convert/validate all output encodings before opening a write transaction. In-memory aggregate input is never mutated by persistence operations.
- Insert parent and aliases in one immediate transaction; duplicate parent ID fails rather than becoming an upsert.
- Update obtains an immediate write transaction, loads/validates the current record, checks expected revision, and rejects missing/stale records distinctly. Preserve registration time; reject conflicting or erased accepted identity evidence, consistent with the current domain's lack of intentional replacement. Attaching evidence when previously absent remains allowed for a caller-approved snapshot.
- Conditionally update parent fields and checked revision, replace that parent's alias rows using bound DELETE/INSERT statements, and commit once. This correctly handles alias rename/delete/order changes without transient uniqueness collisions outside the transaction. No `INSERT OR REPLACE`, `OR IGNORE`, or automatic upsert.
- Any encode, SQL, constraint, or commit failure is returned as failure. Roll back the entire operation; never report success before commit. If rollback fails or connection state is uncertain, close/retire the adapter and report storage failure rather than retrying on that connection.
- Load parent and aliases inside one read transaction, sort by stored position, decode and validate completely, then return. Never return half an aggregate or silently skip corrupt rows. `list_ids` guarantees valid IDs, not validation of every child's metadata; loading each aggregate supplies that validation.
- Database operations validate stored snapshots, not all historical lifecycle transitions. Canonical snapshots originate from approved domain operations; this adapter is not a replacement decision service.

SQLite allows one simultaneous write transaction; immediate transactions obtain the writer reservation at the start. Tests must cover begin/commit contention and bounded failure rather than relying on timing assumptions. [SQLite transaction semantics](https://www.sqlite.org/lang_transaction.html).

### Port errors

The typed `PersistenceError` in ports implements standard error/display traits without exposing rusqlite:

| Category                                                         | Meaning                                                                                    |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `DuplicateCanonicalId`, `DuplicateAliasId`, `DuplicateAliasName` | Specific uniqueness violation; never overwrite existing user intent                        |
| `NotFound`                                                       | Update target absent; load uses None for absence                                           |
| `Conflict`                                                       | Expected storage revision is stale; reload before changing data                            |
| `InvalidAggregateChange`                                         | Registration time change or incompatible/erased accepted evidence during update            |
| `InvalidStoredData`, `CorruptDatabase`                           | Invalid domain/codec values versus engine-reported malformed storage                       |
| `IncompatibleDatabase`                                           | Wrong application/schema identity, unsupported newer version, or inconsistent schema shape |
| `DatabaseAlreadyExists`                                          | Explicit creation found an existing target; no adoption or overwrite                       |
| `UnsupportedPathEncoding`, `UnrepresentableValue`                | Explicit portability/encoding limit                                                        |
| `Busy`, `PermissionDenied`, `Unavailable`                        | Lock contention, identified OS denial, or inaccessible/missing storage respectively        |
| `MigrationFailed`, `StorageFailure`                              | Migration failure or remaining engine/I/O/resource failures                                |

`UncertainOutcome` additionally reports failed commit/rollback disposition requiring retirement of the connection and a fresh reopen/reload. It never claims deterministic rollback after an ambiguous I/O failure.

Map extended result codes with operation context; canonical not-found and foreign-key failures are not interchangeable. Preserve internal diagnostic sources inside infra where useful, but port/display messages omit SQL, full paths, alias names, and evidence bytes. Do not guess that every cannot-open error is permission denial. Unknown categories remain storage/unavailability failures. No UI transport mapping is implemented here.

For a commit I/O failure with uncertain outcome, do not claim that the disk is unchanged or automatically replay writes. Return failure, retire/reopen safely, and require reload. Tests distinguish deterministic rollback failures from ambiguous hardware outcomes.

## Initialization, location, and migrations

### Local location and caller ownership

The future desktop composition root will resolve its OS-appropriate local app-data directory and pass `filenym.sqlite3` beneath it to infra. Do not hard-code a username, repository root, documents folder, or environment variable fallback. Milestone 03 implements an adapter accepting an explicit absolute database path; it does not add Tauri location resolution, automatic production initialization, or directory selection APIs.

Caller provisions the parent directory. Separate explicit `create_new` from `open_existing` infrastructure entry points: creation reserves the named file exclusively and refuses an existing target; reopening never uses SQLite's CREATE flag and returns Unavailable if the database is absent. This prevents a missing known database from silently becoming an empty registry. Open/create only the named database and engine-managed journal sidecars, respecting OS permissions. Reject unsupported path input before open; no URI filename interpretation or canonical-file path derived from an alias. Registered-file location and database location are separate values and responsibilities. Tests use owned temporary directories exclusively.

### Connection policy

Each adapter owns one synchronous connection, used serially. No pool/shared mutable connection/async framework. Independent adapters or processes may open the same local database; revisions handle stale snapshots and SQLite handles write locking.

Connections explicitly set a default 5-second busy timeout, enable/verify foreign keys, select WAL, and verify `synchronous=FULL`. Public `_with_timeout` creation/open variants permit a shorter wait up to 5 seconds without changing durability or foreign-key policy. SQLite's ordinary checkpoint mechanism is retained. WAL permits readers alongside a writer but still has contention and requires local shared-memory support; do not target network-hosted databases. Refuse inability to establish the approved mode rather than silently changing durability semantics. [SQLite WAL constraints](https://www.sqlite.org/wal.html), [connection timeout API](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Connection.html#method.busy_timeout).

Verify configured values after setting them. SQLite read-only fallback is explicitly rejected using `is_readonly("main")`. The settings were reviewed and verified in file-backed Windows tests; they do not establish hardware power-loss guarantees. Use file-backed tests for all acceptance cases; in-memory tests may supplement codecs but cannot prove reopen/WAL behavior. Restrict configurable options to reviewed connection setup and test timeout; do not make durability weaker through ordinary public defaults.

### Schema ownership and versioning

Use `PRAGMA user_version` as the application migration version (initial 1) and a fixed FileNym `application_id` marker. The private marker is `0x464e594d` ("FNYM"), verified together with the exact schema; no globally registered file-type claim is made. Do not use SQLite's internal `schema_version` as the application migration counter. [Application version/identity pragmas](https://www.sqlite.org/pragma.html#pragma_user_version).

1. Open the explicit database with ordinary non-URI flags. Establish connection-local safety settings and inspect version/application ID/schema before any file-altering journal-mode change.
2. Initialize only an empty unversioned database reserved by explicit creation at an app-owned location. Existing-file open must verify FileNym ownership before applying pending migrations; it does not adopt an unmarked empty/populated database. Refuse an unknown application ID or newer/inconsistent FileNym version. Never adopt arbitrary existing user tables.
3. Establish the approved journal mode outside a transaction. Acquire an immediate migration transaction and recheck ownership/version after the lock, handling another opener having already initialized it.
4. Apply ordered embedded SQL migrations and update application marker/user_version in the same transaction. For this milestone only `0001_initial.sql` is a production migration; no speculative version 2 is shipped.
5. Validate expected tables/columns/keys/constraints and foreign-key consistency; commit the whole pending sequence, or roll back it all. Never expose repository operations until initialization succeeds.
6. Reopening current version is idempotent. Committed migrations are immutable; later schema changes append a migration. Unknown versions do not trigger downgrade, reset, or deletion.

The only production migration is the explicit version 0-to-1 initialization. Its immutable SQL is compiled in; schema verification compares table DDL against that source, checks expected objects/index count, and runs `quick_check` plus `foreign_key_check`. No migration framework, checksum dependency, down migration, or auto-repair was introduced. A future format version requires an appended reviewed migration and version-specific validation; version 2 is exercised only as a test fixture, never supported by production code. Test failed initial and failed later-fixture migration sequences, including rollback of schema and version changes. Journal-mode initialization may create headers/sidecars before a failed migration; logical schema/records/version rollback is the guarantee, not byte-for-byte file immutability. A failed fresh initialization can leave an empty, unowned database artifact: retain it and report failure, without automatic deletion/adoption. Test-only migration retry uses a known owned fixture; production recovery of failed creation requires an explicit later caller decision rather than silent reset.

Run integrity/schema checks during initialization, including foreign-key checking; malformed record values are always checked on load. Physical corrupt/unrecognized storage fails without automatic rebuilding, renaming, or deletion. No backup feature is introduced. Hardware failure, disk exhaustion, permission changes, and failed rollback may preclude a guarantee of unchanged durable bytes; report them honestly and never discard recoverable metadata as a repair strategy.

## Security and data integrity

- All paths, names, IDs, bytes, and stored rows are untrusted. Use bound SQL parameters; only compiled migration/query strings define SQL structure.
- Do not dynamically load extensions, execute user-supplied SQL, enable writable-schema modes, or change OS permission boundaries.
- Validate storage classes, paired NULLs, enums, key bytes, timestamp arithmetic, blob lengths, and hydration rules. Errors must not repair, trim, normalize, or drop offending values silently.
- Adapter tests may alter deliberately corrupted fixtures through private/test-only raw connections. Production repository API exposes no arbitrary SQL connection or raw setters.
- Inspect save/load behavior on records with nonexistent physical paths: availability/evidence remain stored facts and no filesystem access occurs. Only the database and its sidecars may change.
- SQLite structural validation is not protection against a hostile editor forging a fully consistent snapshot. Rehydration validates domain invariants, not authenticity or physical identity.

## Complete feature test matrix

Every persistence feature below remains an acceptance requirement. Automated coverage and execution results are recorded at the end of this document; host-specific limits are explicit.

| Feature               | Required cases and assertions                                                                                                                                                                                                                                                                                                                       |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Domain restore        | Whole-aggregate equality for all statuses/reasons; zero/multiple aliases; distinct creation/update/observation times; exact order; duplicate names/IDs, owner mismatch, invalid times rejected; existing domain tests remain passing                                                                                                                |
| Comparison key API    | ASCII-case equivalence and exact Unicode differences match existing rules; read-only accessor introduces no mutation path                                                                                                                                                                                                                           |
| Initialization        | Exclusive creation/new database/version 1; existing-target creation refused; missing open-existing fails without creating a file; exact schema/constraints; reopen idempotence; wrong application marker; unmarked empty/populated DB rejected; newer/negative/malformed versions rejected; no reset of existing records                            |
| Migrations            | Initial failure rolls back DDL/marker/version; test-only later migration failure preserves version 1 data/schema; controlled fixture retry after removing injected failure; concurrent exclusive creation has one winner and typed existing-target failures; existing-owned migration concurrency serializes or returns Busy without mixed versions |
| Insert                | Zero/one/many aliases; every canonical field round trips; parent duplication fails without overwrite; alias failure rolls back the newly inserted parent                                                                                                                                                                                            |
| Load/list             | Present/absent ID; empty inventory; deterministic ordering; unavailable/replacement entries included; parent and aliases read from one snapshot; malformed IDs fail inventory, child corruption fails aggregate load                                                                                                                                |
| Aggregate update      | Canonical metadata/status/location change; alias create/rename/delete/case-only/no-op; all aliases removed without removing parent; order preserved; untouched siblings unchanged; missing target fails                                                                                                                                             |
| Revisions             | Two adapters load same revision; first update succeeds and second conflicts; reload then succeeds; checked overflow atomic; revision changes only on successful writes; failed commit is not success                                                                                                                                                |
| Unique IDs/ownership  | Canonical IDs distinct; duplicate same-owner alias ID rejected; same alias ID on different owner allowed; ownerless alias rejected by foreign key; direct parent delete restricted; owner update cannot silently reparent                                                                                                                           |
| Names/keys            | Exact spelling/extensions; ASCII case duplicates; same name across parents allowed; non-ASCII case/composed/decomposed names remain distinct; internal spaces/dotfiles/multipart/extensionless names; 255-byte and multibyte boundaries; invalid names and mismatched key blob fail validated load                                                  |
| Path codecs           | Native Unicode, relative, drive/UNC/extended forms accepted by core; byte/code-unit equality; non-UTF-8 Unix and unpaired-surrogate Windows fixtures; unknown/foreign tag, NUL, odd UTF-16 length, empty/traversal/terminal-less paths rejected; no lossy fallback                                                                                  |
| Identity/size         | Absent versus present evidence; exact schemes and bytes including zero; incomplete/empty identity rejected; accepted evidence cannot be erased/replaced on update; zero, signed-boundary, above-i64 and `u64::MAX` sizes round trip; malformed size blob fails                                                                                      |
| Time codecs           | Epoch, pre-epoch integral/fractional, nanosecond boundaries, missing modified time, older modification time, backward observation/update clocks allowed by core; inconsistent NULLs/out-of-range nanos/host-overflow rejected; no precision loss                                                                                                    |
| Lifecycle persistence | Available, Modified, each unavailable reason, PossibleReplacement round trip and reopen; aliases/accepted facts retained; load does not call observe or recover; same-path/different-ID records remain distinct; guarded recovery behavior still enforced after load                                                                                |
| SQL constraints       | Exercise every CHECK, uniqueness, storage-class and foreign-key rule with isolated raw fixtures; distinguish SQL rejection from invalid stored data detection; foreign keys enabled on every opened connection                                                                                                                                      |
| Rollback              | Inject failure after parent write, after alias deletion, during alias insertion, and before commit using test-only triggers/failure seams; previous complete snapshot and revision survive deterministic rollback and reopen                                                                                                                        |
| Locking/WAL           | Two file-backed connections; bounded Busy; release lock then retry; read snapshot consistency while another writer commits; configured WAL/FULL/timeout verified; no silent fallback if mode unsupported                                                                                                                                            |
| Error/security cases  | Read-only/missing-parent/inaccessible fixture paths, directory-as-file, corrupt bytes, malformed rows/schema; appropriate safe category; no leaked SQL/path/evidence in display; quoted IDs/scheme/name inputs stay bound data                                                                                                                      |
| Restart/durability    | Close/drop every handle, reopen a file database, compare exact aggregates/revisions; subprocess writer exits then fresh reader verifies committed data; controlled child termination before commit preserves prior snapshot; inspect integrity afterward                                                                                            |
| Physical-file safety  | Owned sentinel file contents/name/metadata unchanged after persistence and alias operations; registration at nonexistent paths never inspected; deleting alias metadata leaves sentinel and canonical row intact                                                                                                                                    |
| Adapter boundary      | Core still dependency-free; SQLite imports/SQL/path-native codecs confined to infra; ports expose no rusqlite/Tauri values; app/desktop remain unchanged                                                                                                                                                                                            |

Use a standard-library-owned temporary directory guard with unique process/counter names and exclusive directory creation, never a hard-coded shared test database. Close child/SQLite handles before cleanup on Windows; cleanup targets only directories created by that guard. Every test can run independently and in parallel. Crash/restart children are test fixtures, not application services. Use synchronization/handshakes for concurrency/failure stages rather than sleep-based races.

Permission and crash tests must have reproducible platform-specific setup and restore test permissions safely. If the environment cannot perform a required case, record it as blocked/untested; do not silently skip and issue a full PASS. Tests cannot establish power-loss durability on arbitrary hardware; claim only demonstrated transaction/process-restart behavior.

## Desktop regression coverage

All new persistence features are exercised through Rust APIs. No database-backed desktop workflows exist in this milestone; do not claim UI registration, alias persistence, or filesystem recovery testing. Run the complete existing-app regression after implementation:

| Check                               | Evidence required                                                                                                                                              |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Automated frontend/workspace checks | Existing tests, format/lint/typecheck/build and all new Rust suites pass                                                                                       |
| Development desktop                 | Tauri launch/title/branding/logos/tagline; visible version from real IPC; default/minimum logical sizes adjusted for DPI; maximize/restore; clean close/reopen |
| Production desktop                  | Build with `--no-bundle`; run built executable without Vite; repeat rendering/IPC/window/close/reopen checks                                                   |
| Browser fallback                    | Frontend-only launch renders shell without native version or commands                                                                                          |
| Scope boundary                      | Desktop does not initialize production DB automatically; repository restart proof is a Rust test, not a desktop persistence workflow                           |

Record automated commands/exit codes separately from tool-driven GUI actions and visual inspection. Keep manual evidence accurate, state platform limits, and do not claim a separate human tester or hosted CI run unless performed/verified. Later milestones must add end-to-end tests for each new UI/service integration without implementing them now.

## Implementation files

```text
crates/filenym-core/src/rehydration.rs       # approved snapshots/restore validation
crates/filenym-core/src/alias_name.rs        # read-only comparison key
crates/filenym-core/src/canonical_file.rs    # invariant-preserving restore entry point
crates/filenym-core/src/error.rs             # typed ownership mismatch
crates/filenym-core/src/lib.rs               # reviewed public exports
crates/filenym-core/tests/rehydration.rs
crates/filenym-ports/src/persistence.rs      # repository/revision/result/error contract
crates/filenym-ports/src/lib.rs
crates/filenym-infra/Cargo.toml              # rusqlite bundled dependency
crates/filenym-infra/src/lib.rs
crates/filenym-infra/src/sqlite/mod.rs
crates/filenym-infra/src/sqlite/connection.rs
crates/filenym-infra/src/sqlite/migrations.rs
crates/filenym-infra/src/sqlite/repository.rs
crates/filenym-infra/src/sqlite/codec.rs
crates/filenym-infra/migrations/0001_initial.sql
crates/filenym-infra/tests/sqlite_persistence.rs
crates/filenym-infra/tests/sqlite_failures.rs
crates/filenym-infra/tests/support/           # owned temporary DB/process fixtures
Cargo.lock
plans/phase-1/03-sqlite-persistence.md        # approval/completion evidence
```

No frontend/app/Tauri source, root manifests, other lockfiles, ADRs, or CI change is expected. Existing workspace CI runs all Rust suites, including new adapter tests. Any newly required native CI prerequisite must be justified separately rather than silently added. Avoid source-module growth into a generic migration or repository framework.

## Approved implementation sequence

The following records the approved sequence, completed during this milestone.

1. Record review approval, including core API, schema/codecs, key scope, revisions, WAL/durability, dependency, and location boundary. Confirm Milestone 02 code is present in the implementation checkout.
2. Implement/test validated rehydration and comparison-key access without changing existing policies; all original core tests must remain green.
3. Introduce minimal ports/revision/error types; document missing/conflicting/error semantics and mutable connection ownership.
4. Recheck rusqlite maintenance/release/MSRV/build requirements; add only approved features to infra and update the Rust lockfile.
5. Implement pure codec tests before database writes, including full `u64`, pre-epoch nanos, native path edge cases, and incompatible tags.
6. Add initial migration and connection initialization; test versioning, unrelated/newer/corrupt databases, foreign keys, mode verification, and migration rollback before repository CRUD.
7. Implement insert/load/list/update with atomic alias replacement and revision checks; test exact aggregate equality, safe error mapping, constraint failures, and malformed stored values.
8. Add controlled locking, failure-injection, subprocess restart/termination, and physical sentinel tests. Each feature matrix row must have identifiable test coverage and results.
9. Run focused Rust validation, workspace and frontend checks, production build, and all existing desktop regressions. Record blocked/untested cases explicitly; broaden testing only for new failures/changes.
10. Inspect dependency boundaries and diff, update this plan with evidence and limitations, and report the verdict. No later use cases are included automatically.

## Acceptance criteria

- Reviewed core rehydration API enforces existing invariants and exactly restores snapshots without filesystem classification or public setters.
- Schema/migration initialization is explicit, idempotent, ownership/version checked, and failure-safe; every connection enforces foreign keys and reviewed durability/timeout settings.
- Every canonical/alias field, order, status/reason, opaque byte sequence, supported native path, full-size value, and representable timestamp round trips exactly through close/reopen.
- Duplicate/conflicting/missing/invalid/unavailable cases produce typed safe errors; no overwrite, silent repair, partial aggregate, lost-update acceptance, or automatic record/alias loss occurs.
- Deterministic rollback, corruption, migration failure, locking, fresh-process reopen, and physical-file safety tests pass with documented evidence.
- Every feature matrix row is covered. Any required blocked test prevents claiming an unqualified PASS; hardware durability and foreign-platform path limits remain explicit.
- Rust formatting/Clippy/core/ports/infra/workspace checks, frontend checks, and development/production desktop regressions pass. No database-backed desktop workflow is claimed.
- No source/dependency inversion, domain SQLite dependency, UI/service expansion, or Phase 1 non-goal is introduced. No Milestone 04 feature is implemented.

## Validation commands for implementation

Run separately from the repository root; these commands are retained for reproduction of the implemented validation. Existing `npm` scripts cover frontend/configuration files rather than Markdown; use the already installed Prettier binary to check this plan explicitly.

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p filenym-core --locked
cargo test -p filenym-core --doc --locked
cargo test -p filenym-ports --locked
cargo test -p filenym-infra --locked
cargo test --workspace --locked
npm.cmd run format:check
npm.cmd run lint
npm.cmd run typecheck
npm.cmd run test
npm.cmd run build
npm.cmd run tauri:dev
npm.cmd run tauri:build -- --no-bundle
.\node_modules\.bin\prettier.cmd --check plans/phase-1/03-sqlite-persistence.md
git diff --check
git status --short
```

Use `npm.cmd ci` only if locked frontend dependencies need installation. Initial Cargo resolution/lockfile update is an authorized implementation step only after dependency approval; use `--locked` for subsequent checks. Record actual SQLite version/build features used by tests. Portable Rust checks and native path codec tests on Unix are desirable additional evidence when an environment is available; no new CI platform is silently required by this plan.

## Approved decisions and remaining risks

1. **Core API:** approved validated snapshots/restore, owner-mismatch error, and read-only comparison key; implemented without weakening existing policies.
2. **Schema/codecs:** approved STRICT tables, per-owner alias IDs, key BLOB uniqueness, ordered aliases, 8-byte unsigned size, timestamp pairs, and tagged native paths; implemented format version 1.
3. **Transaction policy:** revision-based conflict handling and whole-aggregate replacement. Small registry workloads justify simplicity; larger-scale incremental storage is deferred.
4. **Connection/durability:** bundled rusqlite, explicit 5-second busy timeout, WAL/FULL on local storage, refusal to silently fall back. Verify behavior on supported Windows setup before relying on it.
5. **Initialization boundary:** explicit app-owned absolute DB path with existing parent; production app-data resolution/wiring deferred. The private application marker and strict schema-ownership checks are implemented; production directory selection remains deferred.

Unresolved technical limits are not permission to invent filesystem assumptions: foreign-platform paths are deliberately not reinterpreted; extreme `SystemTime` values may be unrepresentable on another host; network database paths/WAL and actual power-loss durability are not supported claims. Permission denial/read-only behavior, owned-file cleanup, subprocess termination, and native bundled compilation were demonstrated on Windows; additional platforms remain unvalidated. The conceptual architecture guidance permits these choices, but no approved ADR is rewritten by this plan.

**Milestone 03 is implemented within the approved scope.** Local validation evidence follows; no hosted CI result, database-backed UI workflow, or later milestone completion is claimed.

## Implementation and validation record

### Implemented public APIs and files

Core adds validated `AliasSnapshot` and `CanonicalFileSnapshot`, `CanonicalFile::restore`, `AliasName::comparison_key`, and the `AliasOwnershipMismatch` error. All entity fields remain protected. The canonical snapshot constructor groups already validated canonical facts with explicit stored status/time and an ordered alias collection, avoiding a large constructor with positional primitive arguments. Restoration validates the complete collection before returning an aggregate and does not replay observations.

Ports adds `CanonicalFileRepository`, `StoredCanonicalFile`, `PersistenceRevision`, and `PersistenceError`. Insert returns revision zero; update requires the expected revision and increments it on each successful write, including an unchanged snapshot. Load returns None only for a missing parent record. List returns IDs in binary order. Revisions are nonnegative signed storage counters, independent of file identity and timestamps.

Infra exposes `SqliteRepository::create_new`, `open_existing`, and their `_with_timeout` variants. Only the bounded wait is configurable; journal/durability/foreign-key policy is fixed. Raw connections and SQL execution are private. SQL is confined to the initial migration and adapter implementation; native path conversion is confined to infra.

Created core rehydration source/tests, the ports persistence module, five SQLite source modules, one initial SQL migration, two adapter integration test files, and the shared owned-directory/process test support. Modified core exports/alias-name/canonical/error modules, ports/infra exports, infra's dependency manifest, and `Cargo.lock`. The existing `alias.rs` needed no change; restoration uses its protected internal construction/mutation. This document records the approved design and actual results. No app, desktop, frontend, ADR, CI, root manifest, or frontend lockfile change was made.

### Schema, migration, and failure behavior

Format version 1 uses two STRICT tables, per-owner alias ID/name-key/position uniqueness, restricted owner foreign keys, full-size BLOB encoding, timestamp pairs, status/reason discriminators, opaque optional evidence, tagged paths, and storage revisions. The private application ID is `0x464e594d`. Exact initial table DDL, allowed schema objects, index count, SQLite integrity, and foreign keys are checked before accepting an existing database. Migration SQL is immutable after release; changes, including constraint changes, require a future version.

Exclusive creation refuses any existing target. Existing-file open never enables CREATE and refuses absent, unmarked, foreign, newer, or incompatible databases. Initial schema/marker/version changes share one immediate transaction. Failed creation retains its owned artifact rather than deleting/adopting it automatically. Production currently has only version 0-to-1 initialization; the version-2 rollback/retry case is a test fixture, not a shipped migration.

Aggregate writes use immediate transactions and bound values. Parent changes and full alias replacement commit together. Registration/evidence safeguards and optimistic revisions prevent invalid replacement or stale overwrites. Reads use one transaction for parent and aliases. Deterministic statement and deferred-constraint commit failures roll back; failed rollback or other ambiguous commit errors return `UncertainOutcome`, retire the adapter, and require a fresh reopen/reload.

Implementation refinements preserving the approved rules:

- Added the explicit `UncertainOutcome` port category because the approved safety requirements distinguish uncertain durability from deterministic failure. A test-only disrupted transaction checks retirement; it does not claim to reproduce hardware I/O failure.
- Checked SQLite's read-only fallback explicitly instead of assuming a read/write open request guarantees write access. The owned Windows read-only fixture demonstrated this behavior; original permissions are restored during cleanup.
- Re-encoded decoded timestamps to reject host rounding. Windows `SystemTime` resolution is 100 ns; compatible values round trip exactly, finer raw stored values fail safely. The storage format itself retains seconds/nanoseconds rather than floating point.
- Used BLOB byte length for identity scheme nonempty checks. Existing core accepts embedded NUL in opaque schemes; ordinary SQLite text length would incorrectly reject a valid domain value. The new regression proves exact insert/load/reopen without changing core validation.
- Canonicalized the owned temporary root and rechecked the resolved child directory before recursive cleanup. No test cleans up an arbitrary caller path or real user database.

### Automated validation results

Local validation date: 2026-10-15. Base revision: `f88234a`, plus these uncommitted changes. Validation used Windows, Rust/Cargo 1.99.0, Node 22.21.1, npm 10.9.4, rusqlite 0.40.2, libsqlite3-sys 0.38.2, and bundled SQLite 3.53.2. Only `bundled` was enabled directly on rusqlite with default features disabled; its required bundled/modern-binding dependencies were verified through the feature tree. No upstream minimum Rust version was assumed from an undeclared manifest field.

| Check                                                                         | Recorded result                                                         |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Rust formatting and `cargo fmt --all -- --check`                              | PASS                                                                    |
| `cargo check --workspace --locked`                                            | PASS                                                                    |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`              | PASS; no warnings                                                       |
| `cargo test --workspace --locked`                                             | PASS; 54 tests including the existing compile-fail doctest              |
| Focused core/ports/infra validation during development                        | PASS after corrections; all members also passed the final workspace run |
| `npm.cmd run format:check`                                                    | PASS                                                                    |
| `npm.cmd run lint`                                                            | PASS                                                                    |
| `npm.cmd run typecheck`                                                       | PASS                                                                    |
| `npm.cmd run test`                                                            | PASS; existing frontend test, 1/1                                       |
| `npm.cmd run build`                                                           | PASS                                                                    |
| `npm.cmd run tauri:build -- --no-bundle`                                      | PASS; final executable rebuilt after the storage correction             |
| Existing Prettier check of this Markdown document                             | PASS                                                                    |
| Relative links, referenced source paths, script names, and Markdown structure | PASS                                                                    |
| `git diff --check` and final scope audit                                      | PASS                                                                    |

The final Rust total is 22 core tests (including the doctest), 2 ports tests, and 30 infra tests. Milestone 03 added 35 counted tests: 3 core, 2 ports, and 30 infra. One infra entry is the subprocess worker's test-harness entry; its real write/read/uncommitted modes are exercised by the parent restart test. The remaining 34 new tests contain substantive assertions, often with table-driven case loops. No tests are ignored to obtain a pass.

Coverage includes every lifecycle status/reason, exact snapshot/order restoration, alias edits/uniqueness/Unicode/byte limits, evidence/size/time/path codecs, schema ownership/constraints, malformed data, migrations/rollback/retry, expected-revision conflicts/overflow, locking/WAL snapshots, read-only/inaccessible paths, uncertain-outcome retirement, exclusive concurrent creation, fresh-process commit/reopen, pre-commit process termination, and physical sentinel preservation. The sentinel test verifies contents, filename existence, length, and modification time; static inspection confirms persistence never reads the canonical file. Tests use only exclusively created owned directories and databases. Initial failures were corrected and the final suites passed without weakening domain policies or CI checks.

### Desktop regression and visual inspection

| Existing behavior                         | Recorded result                                                                                                                                   |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Development launch and real version IPC   | PASS; FileNym title, logos/branding/tagline, and visible `v0.1.0`                                                                                 |
| Default/minimum size and maximize/restore | PASS; configured 900x640/640x440 logical client sizes checked at 125% DPI, with readable unclipped content                                        |
| Development clean close/reopen            | PASS after rendering settled; app closed cleanly                                                                                                  |
| Production executable without Vite        | PASS; embedded frontend and real version IPC, sizing/maximize/restore, clean close/reopen; final rebuilt executable launch/IPC/close also checked |
| Frontend-only browser fallback            | PASS; headless installed browser rendered branding/tagline without native version                                                                 |
| Cleanup                                   | PASS; no test FileNym app or Vite listener remained                                                                                               |

These were tool-driven window actions with screenshot-based visual review, separately from automated Rust/frontend suites. The accessibility tree intermittently omitted WebView content and first-frame screenshots could be blank; bounded rendering waits and settled captures were used before accepting a result. Preliminary blank captures were excluded. Temporary helpers, profiles, screenshots, build outputs, and databases are not repository documentation artifacts or files to commit. No separate human acceptance test or committed desktop automation suite is claimed.

No database-backed UI, registration/relink workflow, watcher, filesystem identity detection, or startup filesystem reconciliation exists in this milestone. Desktop checks prove regression behavior of the existing shell; repository tests supply persistence proof.

### Limits and final verdict

**PASS within the approved Windows Milestone 03 scope.** No required Windows test is blocked or knowingly skipped. Unix codec branches exist but were not compiled/executed on a Unix host; that additional platform evidence remains pending. No claim is made about arbitrary hardware power loss, injected real disk I/O failures, network-hosted databases, installer bundles, global file-type registration, or GitHub Actions execution. Native paths are not reinterpreted across platforms, and timestamps unsupported by the decoding host are rejected rather than rounded.

Milestone 04/application wiring, production app-data location resolution, explicit registration removal, filesystem observation/reconciliation, recovery UX, and external filename presentation remain deferred. The architecture and previously approved alias/lifecycle rules are preserved. This document was reviewed against actual APIs/schema/files and finalized for public inclusion with these limitations, without publishing or committing the implementation.
