# Phase 1 Milestone 05: Desktop UI and Tauri Integration

## Status and objective

**Implemented and accepted within the approved Windows Milestone 05 scope.** Validation completed on 2026-10-16–17. Automated suites, real development/production desktop journeys, and user-reported Narrator/Windows scaling acceptance passed. Environment-dependent evidence and test-method limits remain explicit below. The refined plan and fail-closed cloud-placeholder fallback were authorized on 2026-10-16.

Make FileNym's existing registry and alias services usable from a small Windows desktop interface. Users select an existing file, view its saved information, and manage multiple logical names for it. The original file stays where it is. Deliver a complete UI-to-command-to-service-to-SQLite workflow, with conservative selection checks, persistent startup, explicit errors, optimistic concurrency, accessibility, and polished light/dark themes.

**One file. Many names.** Aliases are metadata. FileNym does not upload, copy, rename, move, delete, or write the selected physical file, and does not create alias files.

## Verified pre-implementation repository baseline

Inspection began on `main` with a clean working tree at `ceadb09`, the merge of PR #14. That merge includes `f5c5c07`, which implements Milestone 04. The required merged-services prerequisite is satisfied. No branch change, commit, push, or PR is part of this planning task.

Reviewed [AGENTS.md](../../AGENTS.md), [problem statement](../../docs/problem-statement.md), accepted architecture documents ([scope](../../docs/architecture/phase-1-scope.md), [domain](../../docs/architecture/domain-model.md), [persistence](../../docs/architecture/persistence.md), [lifecycle](../../docs/architecture/file-lifecycle.md)), ADRs [0001](../../docs/decisions/0001-local-first-desktop.md), [0002](../../docs/decisions/0002-aliases-are-metadata.md), [0003](../../docs/decisions/0003-path-is-not-identity.md), and milestone records [01](01-project-foundation.md), [02](02-domain-core.md), [03](03-sqlite-persistence.md), [04](04-application-services.md). Source inspection covered workspace/crate manifests, core rules and restoration, service requests/errors/methods, repository/connection/codecs/migrations, Tauri configuration/capabilities/commands, React/styles/client, existing tests and [CI](../../.github/workflows/foundation.yml).

| Area                | Actual baseline and implication                                                                                                                                                                                                                                                                                                      |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Services            | `FileRegistryService` borrows `&mut R: CanonicalFileRepository + ?Sized`. Six methods: register, add/rename/delete alias, get canonical, list registered IDs. Requests accept supplied IDs/times; alias mutations require `PersistenceRevision`. No changes to these contracts are proposed.                                         |
| Core                | Protected aggregate, ordered aliases, distinct record IDs and optional opaque evidence. Alias names are 1–255 UTF-8 bytes, ASCII case-insensitive per owner, without Unicode normalization; extensions remain exact. Identical-name rename preserves domain fields but service still conditionally writes and advances revision.     |
| Lifecycle           | Available, Modified, three Unavailable reasons, PossibleReplacement persist. Rename/move are change results. Alias metadata remains editable in every stored status. Available is not a fresh filesystem check.                                                                                                                      |
| Persistence         | Explicit `SqliteRepository::create_new` versus `open_existing`; version 1 STRICT schema; native path BLOBs, full u64 sizes, second/nanosecond pairs, revision checks. WAL/FULL, foreign keys and default 5-second timeout enforced. UncertainOutcome retires the adapter. No automatic recovery or production startup wiring exists. |
| Desktop             | Only a `version` command; desktop depends directly on app/infra, not yet core/ports. Tauri locked version is 2.12.1. Default window 900x640, minimum 640x440 logical pixels; main capability currently grants `core:default`; CSP is currently null.                                                                                 |
| Frontend            | React 19/TypeScript/Vite shell, branding/version, one rendering test; no registry state, picker, themes, CRUD or persistence client. No large UI framework.                                                                                                                                                                          |
| Regression evidence | M04 records 76 Rust tests, including two worker entries, and one frontend test. These are historical results, not new test results from this planning task. CI is Windows-only workspace/frontend validation; no existing GUI E2E framework or hosted result is assumed.                                                             |

Documentation discrepancies: M02–04 retain pre-merge completion wording while Git confirms merges; overview research language is older than the accepted Phase 1 scope. M04's appended coverage ranges contain literal question marks in some places; use the original matrix IDs and test source, not those damaged separators, as evidence. Conceptual persistence fields/repositories are not mandates for new columns or an alias repository. Native paths can be non-Unicode while the current database **location** adapter requires a Unicode absolute path. Existing core permits relative locations, but this new physical-selection adapter will accept only its explicitly supported absolute Windows paths. That narrower adapter policy does not change core rules.

No accepted ADR conflicts with this design. Production filesystem checks are new adapter behavior requiring the tests below; they are not evidence that Milestone 06's identity questions have been answered.

## Scope, terminology, and non-goals

Use **Add file**, **Select existing file**, **My Files**, **Aliases**, **Add alias**, **Rename**, and **Delete alias**. Internal registration/canonical terminology may remain in Rust. Do not describe adding a file as upload, synchronization, or import/copy.

Main files come exclusively from registered records. The native picker is the only route to selecting an unregistered physical file. Never scan drives, enumerate user folders, build a file explorer, read file contents, or infer identity from location/size/times/equal opaque bytes.

Included: production database composition, bounded blocking command execution, native selection/metadata inspection, Rust-generated IDs/times, typed IPC, saved registry inventory/details, alias CRUD, explicit refresh/error recovery, themes, accessibility, and complete acceptance coverage.

Excluded: physical aliases, links as presentation, virtual filesystems, external filename presentation, upload integration, browser extensions, shell actions/open-with/drag-and-drop, previews/thumbnails/content indexing, watchers, rename/move tracking, startup filesystem reconciliation, identity experiments, observation/relink/replacement workflows, registration removal, backups/automatic repair, cloud/accounts/telemetry/AI, advanced settings, pagination/search frameworks, and installer distribution. Later milestones require separate plans.

## Architecture and responsibilities

```text
React components + useRegistry + typed IPC client
                    |
             main-window IPC ACL
                    |
src-tauri commands (validate transport, dispatch, map DTO/error)
                    |
desktop registry coordinator + synchronized storage session
          |                              |
FileRegistryService                 native dialog (Rust)
          |                              |
core + ports                  infra registration inspector
          ^                              |
SqliteRepository (infra)              core facts

core -> std only; ports -> core; app -> core/ports
infra -> core/ports; desktop composes app/infra/core/ports
```

SQL and existing codecs stay in infra. New Windows metadata/reparse checks belong in a small infra registration module without Tauri dependencies. Dialog ownership, startup directories, clocks/ID generation, transport and synchronization belong in desktop composition. Coordinator code prepares inputs and calls services; it does not reimplement alias rules or perform arbitrary aggregate saves. React never receives a filesystem plugin or decides identity.

Compare connection approaches: a dedicated worker thread would also serialize SQLite but needs a job protocol/shutdown machinery. Use one `SqliteRepository` inside a desktop-owned `Arc<Mutex<Session>>`, accessed only within `tauri::async_runtime::spawn_blocking`. The installed Tauri source provides that API; rusqlite's Connection is Send and serialized ownership avoids shared concurrent access. Do not hold a mutex guard across an await or while a picker is open. No pool, async SQL library, new runtime, or command bus is needed.

## Screens and textual wireframes

### Normal layout

```text
+--------------------------------------------------------------+
| [logo] FileNym   One file. Many names.     Theme [System v]    |
+-----------------------+--------------------------------------+
| My Files  [+ Add file] | Resume.pdf                           |
|                       | Saved location                       |
| [file] Resume.pdf   3 | C:\Documents\Resume.pdf               |
| [file] Proposal.pdf 1 | Recorded: Available                  |
|                       | Size: 120 KB    Added: ...            |
|                       | [More saved details]                 |
|                       |                                      |
|                       | Aliases                       [+ Add]|
|                       | Resume_LinkedIn.pdf            [...] |
|                       | Resume_Indeed.pdf              [...] |
|                       | Resume_CompanyA.pdf            [...] |
+-----------------------+--------------------------------------+
| Saved data. File availability is not checked automatically.   |
+--------------------------------------------------------------+
```

Header 48 logical pixels; brand logo 24px, version unobtrusive in footer or brand title. Tagline can hide at narrow widths. Left panel approximately 264px at default size; right panel takes remaining width. Both panels have independent vertical scrolling and minimum width zero on flexible children. No file cards, oversized hero, dashboard metrics, or per-row full paths.

At widths below 720 CSS pixels, use a single-panel master/detail view: show My Files, then selected details with **Back to My Files**. Preserve selected ID and list scroll; back returns focus to that row. Do not reduce the existing 640x440 minimum or default dimensions. At 640x440 keep section controls, form errors and confirmations reachable by vertical scrolling, without page-level horizontal overflow. Test actual logical viewport and DPI, not screenshot outer dimensions alone.

File rows: simple 40–44px lines with decorative file icon, filename, optional alias count and selected indication. Use a semantic list of buttons with `aria-current` for the selected row, avoiding a partially implemented ARIA listbox. Tab/Enter/Space work; do not claim Explorer keyboard behavior. Rows with unreadable stored data display an ID-based placeholder and an error action instead of vanishing.

Detail heading shows the filename derived from saved location. Label path **Saved location** and status **Recorded status**. A wrapping/selectable location field and native `<details>` for size/timestamps avoid clutter. At most the selected file's full path is shown. Render paths/names as text, never executable URLs or HTML. Use bidi isolation; full names remain in accessible labels and a keyboard-reachable expansion when truncated. No path links that launch external programs.

### First launch, no selection, and unreadable storage

```text
My Files                     [+ Add file]

One file. Many names.
Choose an existing file and give it names for different contexts.
Your original file stays unchanged.
```

Keep the compact heading action; do not duplicate it with a full-width CTA. A nonempty registry with no selected file shows **Select a file to view its aliases**. On each launch select the first readable binary-ordered record after inventory loads; no persisted selection preference. If all rows are unreadable, keep them visible and show the data error. An initial loading state or failed database opening is **never** the empty registry state.

On database startup failure, retain header/theme controls and a concise error panel with the appropriate retry action. Do not show a blank working registry, fabricate sample files, or suggest deleting database files.

## Design system and interaction sizes

Use local system fonts (`"Segoe UI", system-ui, sans-serif`); no font download. Body 14px/1.45, secondary 12–13px, section headings 14px/600, filename heading 18px/600. Use 4/8/12/16/24px spacing, 1px borders, 4–6px control radii, 16px icons. Main content padding 20px, 12–16px in narrow mode. No heavy gradients, card shadows, glass effects or decorative animation.

| Token                  | Light starting value  | Dark starting value   |
| ---------------------- | --------------------- | --------------------- |
| Background / panel     | `#F6F7F8` / `#FFFFFF` | `#15181C` / `#1C2026` |
| Text / secondary       | `#17202A` / `#52606D` | `#F0F2F5` / `#ADB7C3` |
| Separator              | `#D7DCE2`             | `#353D48`             |
| Accent / selected tint | `#2457C5` / `#EAF0FF` | `#93B4FF` / `#25334D` |
| Error / success        | `#B42318` / `#176B46` | `#FDA29B` / `#75D5A2` |

These are design-target tokens; actual neutral theme variables are in src/styles.css and do not constitute verified contrast results. Measure all rendered text, focus/control boundaries, states and themes before acceptance; normal text must meet 4.5:1. Secondary text must remain readable rather than faint. [WCAG contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).

**Add file is content-width**: inline-flex, approximately 30–32px height, 8–10px horizontal padding, 13px text, 14px plus icon; `flex: none`, no width:100% or flex growth. It sits beside My Files. Aliases has the similarly compact **+ Add** action. Provide at least a 28px square target for contextual actions and a clearly visible 2px focus outline with offset. Do not require hover to discover the only keyboard-accessible action.

Theme control is a compact native select, labelled **Theme**, with System/Light/Dark. Default System follows `prefers-color-scheme` and reacts to OS changes; explicit override wins. Store only the enum at `localStorage["filenym-theme"]`; validate it on load and fall back to System for malformed/unavailable storage. Preference failure leaves the chosen in-memory theme usable. No SQLite migration/settings screen. Use CSS variables and `color-scheme`; native picker chrome remains OS-controlled. Keep transitions to color/opacity of about 100–150ms and disable them under reduced motion. Validate forced colors and 200% text zoom.

## Detailed use-case interactions

### Add an existing file

1. Compact Add file invokes `select_existing_file`, opening the single-file native picker owned by main. Cancellation leaves registry/list/selection unchanged.
2. Rust inspects supported input and checks exact saved-path hints. With no hint, it generates ID/time and calls registration directly; React shows Adding file until this same command returns committed success.
3. Only a duplicate hint opens a small View existing / Cancel / Add separately decision. The pending token stays backend-bound to that candidate; the continuation reinspects before a separate insert.
4. Missing/inaccessible/changed/unsupported selection fails without inserting a row. No speculative file row, copied file or extra normal confirmation.
5. On committed success select the returned record, announce File added, focus details and update inventory from its returned revision. A subsequent inventory refresh failure does not negate a successful commit. Unknown delivery/commit outcome blocks writes and requires explicit storage reopen/reload/review, never resending the picker/registration automatically.

### Add alias

Select a file, click compact + Add, and reveal a small inline labelled Filename form above the alias rows. Start empty; offer no automatically rewritten extension. Helper text: **Use the complete name, including an extension if you want one. Aliases do not create files.** Enter submits, Escape/Cancel closes before dispatch, and successful commit closes the editor and focuses the new row. Keep exact input on errors. Disable this record's other mutations while pending; show the alias only from the committed response. A UTF-8 byte counter uses TextEncoder; do not use HTML maxlength=255 as a byte validator. Reject unpaired JavaScript surrogates explicitly rather than having TextEncoder replace them silently.

Frontend validation may provide early feedback for the documented name rules, but Rust/core is authoritative. Use shared transport fixtures to keep reason codes/examples aligned; do not substitute JavaScript Unicode lowercase for the core's ASCII-only comparison. Do not silently trim names. Extensions may differ from the original and must be preserved as entered.

### Rename alias

The compact ellipsis opens an accessible small popover containing ordinary Rename/Delete alias buttons (not a custom ARIA menu unless its full key model is implemented). Rename turns that row into an inline form prefilled with the exact current name; preserve alias ID and input selection. Submit with the revision captured for this edit. An identical name still submits and advances storage revision on success; no-op UI optimization is allowed. Case-only changes update exact spelling. Escape cancels before submission. On success focus that row's actions; on failure keep the draft and error.

### Delete alias

Open a small native HTML dialog, max-width approximately 360px: **Delete this alias?** Display the alias name and **Only this alias will be removed. Your original file stays unchanged.** Buttons Cancel and Delete alias, initial focus Cancel. Enter activates only the focused control; opening the dialog must not make a global Enter handler submit deletion. On confirmed success remove only from the returned aggregate. Focus the next surviving alias action, previous row, or Add if none remain. No physical-file deletion warning, undo claim, or registration removal is offered.

### View, refresh, restart, and navigation during work

Show saved filename/path/size, registration time, recorded modification time if present, last recorded observation time and status, plus alias count. Use **Not recorded** for missing modification time. Footer/help: **Saved information only. File availability is not checked automatically.** Modified means metadata differed in the last accepted observation, not proof that bytes changed.

Refresh is **Refresh saved data** and only reloads SQLite through services. Never stat registered files on selection, refresh, startup or alias edits. Closing/reopening restores IDs, names, order, revisions and recorded facts. No selection persistence is necessary. Drafts are in-memory only; if an unsaved editor is open, changing file/back navigation offers a compact discard choice. Once a command has been sent it cannot be cancelled by Escape; show pending status. Window close during a dispatched write does not imply rollback: next launch reads persisted state without replaying the command.

## Native file-selection and filesystem policy

Use the Rust `tauri-plugin-dialog` builder, main-window parent and `blocking_pick_file` inside spawn_blocking, never the GUI thread. This keeps picker results in Rust and avoids installing a JavaScript dialog or filesystem plugin. The official plugin supports Rust-only usage and returns cancellation separately. No save/multiple/folder picker capability is exposed. [Tauri dialog guide](https://v2.tauri.app/plugin/dialog/), [builder API](https://docs.rs/tauri-plugin-dialog/2.8.3/tauri_plugin_dialog/struct.FileDialogBuilder.html).

### Reparse policy comparison and implemented support

Reparse points are not synonymous with symbolic links. Windows uses them for mounted folders, redirects and cloud placeholders. Microsoft documents offline/recall attributes separately from PINNED user intent; a pinned flag alone does not prove all bytes are local. Rejecting every reparse ancestor can exclude otherwise ordinary Documents directories. [Reparse tags](https://learn.microsoft.com/en-us/windows/win32/fileio/reparse-point-tags), [Windows attribute definitions](https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants).

| Approach                                                  | Tradeoff                                                                                                                                                                      | Decision                                       |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| Reject all reparse ancestors/files                        | Small implementation, but excludes junction-based folders and many OneDrive layouts, including resident files.                                                                | Replace the original blanket rule.             |
| Let the OS follow everything                              | Convenient, but accepts unknown redirects, remote destinations, file symlinks and recall-sensitive placeholders without a clear policy.                                       | Reject.                                        |
| Classify redirects and placeholders; metadata-only access | Supports ordinary local directory redirection and known resident cloud files while rejecting unsupported final objects. Needs explicit tag/attribute checks and native tests. | Implemented with cloud classes disabled below. |
| Add file-ID tracking or provider hydration management     | Could address other concerns but expands into identity/provider orchestration.                                                                                                | Defer; no M06 work or hydration API.           |

Final supported selection policy:

| Selection                                                                 | Decision                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Regular file on local fixed/removable drive                               | Accept after metadata/path checks; availability may change afterward.                                                                                                                                                                                                                                                           |
| Junction/mounted-directory ancestor or directory symlink                  | Follow ordinary local directory redirects recognized by Windows/Rust read_link. Validate their targets before opening and the final handle destination afterward. Nested reparse targets, broken redirects, loops, cloud/unknown ancestors and remote destinations fail closed. This is path validation, not identity tracking. |
| Recognized Windows cloud-placeholder directory ancestor                   | Reject this class in the implemented fallback. Ordinary local ancestors without reparse/recall attributes remain eligible; no OneDrive/provider compatibility is claimed.                                                                                                                                                       |
| Final recognized cloud-tag file already fully resident                    | Reject all final cloud reparse objects, including resident/pinned placeholders. Safe provider-specific support has not been established; there is no data-open fallback or CloudFilters dependency.                                                                                                                             |
| Final cloud-only, partially resident, offline or recall-sensitive file    | Reject before any normal data open, preview or read. Explain that cloud placeholders are unsupported and an ordinary local file is required; making a placeholder resident does not guarantee acceptance. FileNym does not download, pin, change attributes or offer implicit hydration consent.                                |
| Final file symlink, unknown reparse tag or unrecognized redirect/provider | Reject; directory navigation support does not imply accepting linked final files or arbitrary filter behavior.                                                                                                                                                                                                                  |
| Directory/device/ADS/relative/drive-relative/root-only/traversal          | Reject without rewriting input.                                                                                                                                                                                                                                                                                                 |
| UNC/VerbatimUNC, mapped remote drive or resolved remote target            | Reject for M05, including local-looking paths redirected to a network destination.                                                                                                                                                                                                                                              |
| Unicode or extended drive path                                            | Preserve native code units and require current domain lexical validity; no lossy path conversion.                                                                                                                                                                                                                               |
| Missing/inaccessible/sharing-denied/changing unsupported object           | Typed selection error, no insertion or automatic status change. Recognized unsupported modification-time capability maps to None; other metadata errors fail.                                                                                                                                                                   |

Implement `inspect_registration_candidate(&Path) -> Result<RegistrationFacts, RegistrationError>` in infra, independent of Tauri, clocks, IDs and SQLite. Facts retain the picker-returned absolute path and recorded metadata; optional evidence remains None. Keep the resolved destination transient for support checks; do not replace the saved path with it or use either spelling to identify a physical file.

Use an exact-name Windows metadata query (not a directory scan) to obtain attributes/tag before a potentially recall-sensitive open; interpret RECALL_ON_OPEN only in APIs where that flag is defined. Open for FILE_READ_ATTRIBUTES with existing-file semantics, OPEN_REPARSE_POINT for the final object and ordinary share-read/write/delete. Use directory handles with appropriate backup-semantics flags only for directory resolution/metadata, not backup privileges. Query final tag/attributes on the handle, classify only documented allow-listed directory redirects/cloud tags, resolve/check the destination drive for directory redirects, and recheck detectable changes before insertion. GetFileInformationByHandleEx/FileAttributeTagInfo and GetFinalPathNameByHandleW are metadata/path facilities; do not collect file IDs, volume serials, hashes or link counts. Final-file symlinks remain rejected. Extend the existing windows-sys proposal with only Win32_Storage_CloudFilters for CfGetPlaceholderStateFromAttributeTag: this is a pure classification of attributes/tag, not a hydrate, pin, provider or identity operation. Reject INVALID, PARTIAL or PARTIALLY_ON_DISK results and unrecognized tags, in addition to offline/recall flags; do not use IN_SYNC or PINNED as residency proof. No separate Cloud Files SDK crate, provider client or network service is added. [Handle tag information](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_attribute_tag_info), [path resolution](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew), [attribute/tag placeholder classification](https://learn.microsoft.com/en-us/windows/win32/api/cfapi/nf-cfapi-cfgetplaceholderstatefromattributetag), [Rust Windows open options](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html).

The implementation must prove that opening a recognized final cloud reparse object for attributes does not become an ordinary data open; never follow it as a fallback after a failed no-follow metadata query. No FILE_FLAG_OPEN_NO_RECALL flag, PINNED flag, matching metadata or provider name is treated as a universal non-hydration guarantee. Native picker/Shell/provider background activity is outside FileNym's control; FileNym makes no content request, and real-provider tests must distinguish background activity from its own operations.

Reparse tags/attributes classify support, not authenticity or physical continuity. Ancestors/targets may change between checks, the selected file may be replaced with identical metadata, and rename/deletion can occur immediately afterward. There is no atomic filesystem/SQLite transaction or race-free sandbox guarantee. Do not describe GetFinalPathNameByHandleW as canonical identity, persist its result as evidence or silently merge records. The policy applies to the path returned by the picker; Shell may resolve shortcuts before returning it. Revalidate the returned candidate and do not claim to detect every shortcut used during navigation.

Directory-redirect support requires mandatory real local junction fixtures plus deterministic broken/loop/remote-target classification tests. Resident-cloud support requires mandatory automated tag/recall/metadata-only tests; real OneDrive/provider evidence is separately classified below. If the required safe metadata path cannot be implemented or verified, disable/reject that class explicitly and report the narrower support; never fall back to a content open. No supported-class claim may exceed its recorded evidence.

### Registration coordination: one command, pending duplicate only

Normal flow is **Add file -> native selection -> inspected facts -> service insert -> committed registry entry**, within `select_existing_file`. No normal selection ticket, expiry timer, extra frontend registration call or confirmation exists. The backend retains the native path locally through this call; React cannot supply arbitrary paths or facts. Capture storage generation before opening the picker, recheck it after the picker returns before inspection, and recheck again under the repository lock before insertion. Retirement/reopen during the picker invalidates the operation without a write.

When exact-path candidates justify a user decision, return duplicateDecision with an opaque pending-selection token and candidate IDs. Keep only one pending duplicate per main window; bind it to that window and the current storage generation. A new picker, explicit Cancel/View existing decision, terminal registration success/uncertainty, storage retirement or app close invalidates it. No wall-clock expiry is necessary: the token authorizes only the already chosen candidate, and confirmation always reinspects it. A forged/stale/other-window token fails without filesystem access or writes.

`register_selected_file` is now the duplicate-decision continuation: decision is cancel, viewExisting or addSeparately. Cancel discards pending state; View existing discards it and performs only a saved-record retrieval for a candidate ID; Add separately atomically reserves it, reinspects and rechecks candidates, then inserts a new record. Do not hold the repository lock across the native picker or user decision. Retain only minimal pending duplicate state after deterministic pre-write errors so the user can explicitly retry or cancel; consume it after any dispatched uncertain outcome. Changed metadata/type or unsupported state requires selecting again. Matching metadata never proves continuity.

Keep storage generation/epoch protection because queued commands must not cross retirement/reopen. Use the same generation for duplicate binding and response guards rather than a second selection epoch or timer subsystem. Revisions still protect record edits; generation protects the storage session, so these are distinct necessary checks. Maintain one picker/pending mutation guard to prevent double activation, not a command queue or generic capability framework.

## Duplicate registration UX and limits

Compare exact native saved-path spelling against validated registered aggregates. This is a hint, not path normalization or physical deduplication. No case folding, resolve/canonicalize, inode/token synthesis or silent merge. If any stored candidate cannot be loaded, report a saved-data error instead of asserting that no record matches.

Dialog: **This location is already recorded.** Explain **It may be the same file or a different file at the saved location. Existing aliases will stay unchanged.** Offer **View existing**, Cancel, and secondary **Add separately**. For multiple candidates show a small list identified by filename/recorded status/alias count. View existing only selects/reloads a record; it does not claim the new file is that record or change status/evidence/metadata. Add separately generates a new canonical ID and preserves every previous record/alias.

Check exact-path hints under the local storage lock just before normal insertion. If candidates exist, return duplicateDecision and retain only the pending duplicate; no write occurs. Explicit addSeparately permits a distinct record even for a recorded Unavailable/PossibleReplacement candidate. A candidateId is required only for viewExisting, must belong to the pending candidate set, and is null for cancel/addSeparately. Across independent processes, warning lookup plus insert is not an atomic path constraint; simultaneous duplicate records remain possible and safe. Different spellings/paths, moved files and hard links may not trigger a hint. Unsupported reparse selections are rejected before this stage; supported local directory redirects/resident-cloud metadata follow the policy above. Do not deduplicate by name/size/timestamp or silently attach aliases to an old record.

## Production database initialization, ownership, and shutdown

Resolve the OS local per-user directory through `app.path().app_local_data_dir()` using identifier `app.filenym.desktop`; use its `registry/filenym.sqlite3` child to keep metadata separate from WebView files. Create only this application-owned directory. Never use repository/build/temp/documents paths, a user-selected file directory, a roaming fallback, or network storage for production SQLite. Directory resolution is a desktop concern. [Tauri path resolver](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html#method.app_local_data_dir).

### Bootstrap alternatives and chosen decision

| Option                                            | First run / missing database                                                                                             | Interrupted initialization / concurrency                                                                                                          | Assessment                                                                                                                                                                          |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A: durable attempted-initialization sentinel      | Enables automatic first-run creation; sentinel plus missing DB distinguishes a known registry loss.                      | Sentinel reserved before DB; failures leave artifacts. Exclusive creation arbitrates concurrent starters; no repair/adoption of an incomplete DB. | Retain, simplified to one immutable fixed-format sentinel.                                                                                                                          |
| B: create when absent, open when present          | Smallest code, but a previously used DB removed between launches looks exactly like first run and is silently recreated. | Exclusive DB create prevents overwrite, not this ambiguity; partial existing DB still must fail.                                                  | Reject because it loses the approved missing-known-database guarantee.                                                                                                              |
| C: explicitly ask before every absent-DB creation | No durable sentinel needed; always block absent DB until the user chooses Create empty registry, including first run.    | Existing/incomplete DB must still fail; exclusive create handles races.                                                                           | Safe alternative if the product accepts a first-launch setup decision and weaker automatic missing-history diagnosis. Not chosen for the approved immediate empty-state experience. |

Choose A with `registry/registry-initialization-v1`, containing only fixed ASCII `FileNym registry initialization v1` plus newline. Reserve with create_new, write/flush/sync before create_new(database); no JSON manifest, mutable ready flag, timestamps or profile ID. It records an initialization attempt, not successful migration. Keep existing SQLite format/version checks as the authority. serde_json remains useful in tests, but is not needed in production for this sentinel.

| Disk/session state                                                  | Action                                                                                                                                                                    |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Neither sentinel nor DB exists and first-run eligible               | Create app-owned directory, exclusively reserve/write/sync sentinel, then create DB once. No create after sentinel-write failure.                                         |
| DB exists                                                           | Only open_existing/verify. If sentinel absent, establish it after successful verification; do not reset data.                                                             |
| Sentinel exists, DB missing                                         | Block with missing-known-database error on startup/retry; no empty-registry fallback.                                                                                     |
| Sentinel malformed/partial/unsupported, or DB unowned/corrupt/newer | Preserve all artifacts, report opening failure; no automatic repair, adoption, overwrite or deletion.                                                                     |
| Concurrent initialization                                           | One exclusive reservation wins. Other process may explicitly reopen only a fully valid existing DB; intermediate states fail safely and permit user-initiated open retry. |
| Early directory/sentinel permission failure before reservation      | Explicit retry can repeat first-run setup if neither artifact exists and no DB has been observed. Once reservation/DB attempt occurred, no automatic create fallback.     |

If initialization fails after reserving the sentinel, an explicit later recovery decision is needed; do not implement a reset feature to hide this error. A crash before the sentinel is durably written can leave a partial sentinel, which blocks creation. Filesystem power loss can lose directory entries despite flush; this is not a transactional or hardware-durability guarantee. Deleting the entire app-data directory, or both sentinel and DB, remains indistinguishable from a new installation. Retaining this one-bit durable history is the smallest recommended way to preserve automatic first run plus missing-known detection without a schema change.

In Tauri setup, resolve configuration/manage Initializing state, then perform directory/SQLite work in a blocking task. `registry_status` exposes Initializing/Ready/error; frontend waits/polls status at a bounded interval while initialization runs, stopping on completion/unmount or the visible ten-second client timeout. This timeout invalidates late read responses, does not cancel a native operation and never retries a write. Retry is an explicit reopening operation obeying the table, never a repeated creation loop. Only successful initialization exposes usable registry operations.

Retain existing format-1 migration/verification unchanged. There are no new migrations or arbitrary SQL commands. Use existing WAL/FULL/foreign-key checks and five-second Busy limit; a blocked DB call must not freeze the UI. Preserve `-wal`/`-shm` sidecars; never manually clean/checkpoint/delete them. SQLite owns them. Database-location non-Unicode paths and redirected/network local-data directories may fail safely under current adapter limits; no lossy fallback.

Storage session phases: Initializing, Ready(repo, epoch), Unavailable/Invalid/Corrupt/Incompatible, NeedsReopen, Reopening, NeedsReload(repo, newEpoch, affectedRecordIds), Closing. Epochs are opaque random tokens, not file identities/revisions. Commands carry the current epoch and recheck it **after acquiring the storage mutex**, so commands queued before retirement cannot run after reopening. A poisoned mutex/panicked blocking operation fails closed and retires the connection; do not unwrap and resume a possibly inconsistent session.

UncertainOutcome drops the adapter and blocks **all writes**. Explicit Reopen storage opens existing only, gives a new epoch, then reloads affected records/inventory. Definitive get success or CanonicalNotFound resolves the corresponding read requirement; absence is data, not permission to recreate a record. Writes stay blocked until the required reloads complete, and users must review before resubmission. Reopen failure leaves writes blocked. Busy alone does not change lifecycle state or imply an uncertain commit. Known typed outcome receipts may include attempted canonical/alias IDs to help reload; no blind retry.

During graceful exit, block new writes, let the bounded in-flight DB operation finish in a background shutdown task, drop the repository/pending duplicate, then exit. Do not block the main event loop with a mutex wait. If a command had been dispatched, show **Finishing save…** briefly; forced process termination cannot be reported as rollback. The five-second Busy timeout bounds lock contention, not arbitrary OS I/O or total shutdown duration. A slow disk can outlive that wait; keep the UI responsive, and do not pretend cancelling a spawn_blocking future stops a write. If a separate explicit Quit without waiting action is offered after a long wait, label the outstanding result unknown and do not resend it next launch. No custom flush promise exceeds SQLite's guarantees. Reopen/restart reads persisted state; it does not reconcile physical files.

## Tauri command surface and IPC contracts

Commands remain thin and use a desktop coordinator. `DesktopState` contains one synchronized repository/epoch/pending-duplicate session, with a separate atomic picker guard. Keep a small separately locked status snapshot so registry_status can report Initializing/NeedsReopen without waiting for a SQLite operation. Define a consistent lock order; never hold selection and repository locks while opening a picker. The existing version command remains available. Database/picker commands are async; registry_status reads only the separately locked small snapshot synchronously. Errors are structured and never contain SQLite/OS diagnostics. Tauri may reject malformed argument deserialization before a typed handler runs; the client replaces unstructured framework rejection text with a fixed safe transportUnknown message, conservatively requiring review rather than parsing diagnostic strings and must never display/log the raw rejection. A lost or undecodable response after dispatch remains transportUnknown, not an assumed rejected write. Command contracts (implemented in the thin command modules):

```rust
type CommandResult<T> = Result<Reply<T>, CommandErrorDto>;

async fn registry_status(state: State<'_, DesktopState>)
    -> CommandResult<RegistryStatusDto>;
async fn select_existing_file(app: AppHandle, window: WebviewWindow,
    state: State<'_, DesktopState>, request: SessionRequestDto)
    -> CommandResult<SelectionOutcomeDto>;
async fn register_selected_file(state: State<'_, DesktopState>,
    window: WebviewWindow, request: RegisterSelectionRequestDto)
    -> CommandResult<DuplicateDecisionOutcomeDto>;
async fn add_alias(state: State<'_, DesktopState>, request: AddAliasRequestDto)
    -> CommandResult<CanonicalFileDto>;
async fn rename_alias(state: State<'_, DesktopState>, request: RenameAliasRequestDto)
    -> CommandResult<CanonicalFileDto>;
async fn delete_alias(state: State<'_, DesktopState>, request: DeleteAliasRequestDto)
    -> CommandResult<CanonicalFileDto>;
async fn get_canonical_file(state: State<'_, DesktopState>, request: GetFileRequestDto)
    -> CommandResult<CanonicalFileDto>;
async fn list_registered_files(state: State<'_, DesktopState>, request: SessionRequestDto)
    -> CommandResult<RegistryDto>;
async fn reopen_registry(state: State<'_, DesktopState>)
    -> CommandResult<RegistryStatusDto>;
```

Normal selection and explicit Add separately map inspected facts to `RegisterCanonicalFile`; add generates AliasId/time and maps to `AddAlias`; rename supplies current Rust time; delete maps IDs/revision unchanged. Get forwards the validated ID. List calls existing `list_registered_file_ids`, then `get_canonical_file` for each ID within one local worker operation; no new port/app summary method or schema change. This is an initial small-registry N+1 read strategy, not a global snapshot; external processes can write between reads. Return full DTOs as small-registry entries to avoid a second summary model.

If one aggregate has InvalidStoredData/UnsupportedPathEncoding, preserve its ID as an unreadable entry with a safe code; never skip it and pretend the inventory is complete. CorruptDatabase, inventory-invalid-ID or global storage failure returns an overall error. A record removed externally between list/get gets a missing entry, not silently selected as another file. Refresh is explicit, with no filesystem scan.

### Transport definitions

All Rust DTO fields use serde camelCase and deny unknown fields on requests. Tagged enums use explicit lower-camel discriminators. TypeScript mirrors the following schema; optional fields are represented consistently as null, not inconsistently omitted. `Reply<T>` contains `storageEpoch: string | null` and `value: T`; null epoch is valid only before usable storage. Commands receive the nested argument `{ request: ... }` where declared. No domain entity is serialized directly.

```typescript
type Decimal = string; // canonical decimal spelling, validated in Rust
// Same names are explicit Rust serde enum renames, not Debug formatting.
type CommandCode =
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
type AliasReason =
  | "empty"
  | "whitespace"
  | "traversalOrSeparator"
  | "forbiddenCharacter"
  | "deviceName"
  | "trailingPeriod"
  | "tooLong";
type SelectionReason =
  | "directory"
  | "remoteLocation"
  | "reparsePoint"
  | "offlineFile"
  | "invalidPath"
  | "unsupportedDrive";
type ErrorReason = AliasReason | SelectionReason;
type SessionRequestDto = { storageEpoch: string };
type RegisterSelectionRequestDto = SessionRequestDto & {
  selectionToken: string;
  decision: "cancel" | "viewExisting" | "addSeparately";
  candidateId: string | null;
};
type AddAliasRequestDto = SessionRequestDto & {
  canonicalId: string;
  expectedRevision: Decimal;
  name: string;
};
type RenameAliasRequestDto = AddAliasRequestDto & { aliasId: string };
type DeleteAliasRequestDto = SessionRequestDto & {
  canonicalId: string;
  expectedRevision: Decimal;
  aliasId: string;
};
type GetFileRequestDto = SessionRequestDto & { canonicalId: string };
type TimestampDto = { seconds: Decimal; nanoseconds: number };
type NativeTextDto = {
  encoding: "windowsUtf16";
  units: number[];
  display: string;
  escaped: boolean;
};
type StatusDto =
  | { kind: "available" }
  | { kind: "modified" }
  | { kind: "unavailable"; reason: "notFound" | "permissionDenied" | "unknown" }
  | { kind: "possibleReplacement" };
type AliasDto = {
  id: string;
  canonicalFileId: string;
  name: string;
  createdAt: TimestampDto;
  updatedAt: TimestampDto;
};
type CanonicalFileDto = {
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
type RegistryEntryDto =
  | { kind: "file"; file: CanonicalFileDto }
  | { kind: "unreadable"; id: string; code: CommandCode }
  | { kind: "missing"; id: string };
type RegistryDto = { entries: RegistryEntryDto[] };
type SelectionOutcomeDto =
  | { kind: "cancelled" }
  | { kind: "added"; file: CanonicalFileDto }
  | {
      kind: "duplicateDecision";
      selectionToken: string;
      filename: NativeTextDto;
      duplicateCandidateIds: string[];
    };
type DuplicateDecisionOutcomeDto =
  | { kind: "cancelled" }
  | { kind: "viewExisting"; file: CanonicalFileDto }
  | { kind: "added"; file: CanonicalFileDto };
type RegistryStatusDto = {
  phase:
    "initializing" | "ready" | "blocked" | "needsReopen" | "reopening" | "needsReload" | "closing";
  code: CommandCode | null;
  affectedCanonicalIds: string[];
};
type CommandErrorDto = {
  code: CommandCode;
  field: "canonicalId" | "aliasId" | "name" | "expectedRevision" | null;
  reason: ErrorReason | null;
  candidateIds: string[];
  attempted: { canonicalId: string | null; aliasId: string | null } | null;
  storageEpoch: string | null;
};
```

Code/reason strings are closed enums in Rust and TypeScript; this implementation produces alias reasons and null selection reasons. Unknown wire values fail client validation and enter a safe fallback; transportUnknown is a client-generated delivery category, not an assertion that the database rejected a request. AliasNameError reasons use the exact explicit spellings above. Remaining unexpected domain errors (such as ownership/evidence errors that these commands should never produce) map to domainFailure with no raw diagnostic or identity payload. Selection errors use a reason only when the field is relevant; otherwise null. Validate native units as integers 0 through 65535, timestamp nanos as integers in range, and every discriminated enum variant before accepting a response.

Size is unsigned decimal 0 through 18446744073709551615; revision is decimal 0 through 9223372036854775807. Request revision parsing accepts only `0` or `[1-9][0-9]*`, checks i64 range, then uses PersistenceRevision. Do not turn values into JS numbers, exponential notation or floating-point comparisons. Use BigInt only within display calculations; JSON contains strings. IDs remain opaque existing domain IDs, not a UUID-format filter, so older records remain addressable.

Timestamps preserve normalized signed decimal Unix seconds plus nanos 0–999999999. Convert with checked arithmetic, including pre-epoch values; do not serialize SystemTime directly or collapse to floating-point milliseconds. Human Date formatting is permitted only after range/precision checks; exact timestamp fields remain available in saved details. Beyond supported Date range show exact seconds/nanos. Missing modified time stays null. Existing host precision limits remain: Windows SystemTime represents 100 ns, and stored values finer than a decoding host supports fail explicitly.

Native paths are never sent back as authority. UTF-16 unit arrays preserve even unpaired surrogates; display text escapes unpaired units/control characters and sets escaped=true rather than pretending a lossy replacement is the actual filename. Ordinary Unicode displays normally. Show a small **Some characters are displayed as escapes** explanation when needed. No to_str unwrap, lossy identity comparison, decode of Windows paths on other platforms, or string concatenation into SQL/URLs. Identity schemes/bytes are deliberately omitted from IPC. Full saved paths are returned only as necessary registry data, never in ordinary errors or logs.

Rust serialization fixtures live under `tests/contracts/` and are consumed by Vitest runtime guards. Assert camelCase, tags, nullable fields, unsafe integer boundaries, invalid requests and all error reasons. `invoke<T>` typing alone is insufficient validation. No code-generation dependency is necessary at this size; contract changes must update both sides and fixtures in the same change.

### Permissions and CSP

Tauri app commands are otherwise available to app windows by default; explicitly enumerate them with `tauri_build::AppManifest::commands` and grant only generated allow-* permissions to the local main window. Add backend main-window checks for selection tokens. Do not enable dialog:default, dialog:allow-open, fs, SQL, shell/opener or remote-origin capabilities for React: native dialog use stays in trusted Rust. The installed plugin's Rust path and generated app ACL must be tested on the selected version rather than granting broad permissions to fix failures. [Tauri capability rules](https://tauri.app/security/capabilities/).

Declare all ten app commands, including the retained version command, in the build manifest. The main capability's implemented permission list is `core:default`, `allow-version`, `allow-registry-status`, `allow-select-existing-file`, `allow-register-selected-file`, `allow-add-alias`, `allow-rename-alias`, `allow-delete-alias`, `allow-get-canonical-file`, `allow-list-registered-files`, and `allow-reopen-registry`. Use the generated names verified by the selected Tauri build tooling; no wildcard command grant or remote URL entry. These app permissions replace reliance on the default app-command exposure and are distinct from plugin permissions.

Keep current window sizes, explicitly label main, and set `dragDropEnabled: false` to disable WebView file-drop handling because this milestone supports picker selection only. Replace null production CSP with a tested restrictive policy starting from `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'self'; frame-src 'none'`. Use `devCsp` to allow only the configured local Vite/HMR origins in development, while retaining the production policy. Retain Tauri's required generated hashes/nonces. No production unsafe-inline, unsafe-eval, remote script/content or asset-file scopes. Development CSP may allow inline styles solely for Vite CSS/HMR injection; this does not relax production CSP or permit remote scripts. Verify both compiled IPC and styling before accepting the policy. [Tauri CSP guidance](https://v2.tauri.app/security/csp/).

## ID and timestamp composition

Use a direct desktop dependency on **getrandom 0.4.3**, already present transitively, to fill 16 OS-random bytes and encode a lowercase 32-character hex opaque ID. The domain requires no UUID format, so a UUID dependency adds no needed semantics. Generate canonical IDs, alias IDs, pending-duplicate tokens and session epochs in Rust, independently of paths/names/metadata and each other. Entropy failure returns idGenerationFailed before writes. The documented fill API is fallible. [getrandom fill](https://docs.rs/getrandom/0.4.3/getrandom/fn.fill.html).

Uniqueness is probabilistic until checked by persistence/domain. A detected duplicate canonical/alias ID is a typed failure, never overwrite or silent regeneration/retry. User may explicitly retry after refreshing; that new command generates a new ID. Tests inject repeated bytes/collisions and entropy failure without weakening production randomness. No clock/ID provider is added to core/app; desktop helpers can use small test-local injected functions.

Use Rust SystemTime::now after successful metadata inspection for registration, and when assembling each alias add/rename request. Never accept JS timestamps or new record IDs. Do not fake monotonic wall-clock ordering: a clock rollback that precedes registration/alias creation yields the existing InvalidTimestamp, with friendly correction guidance. Do not clamp/rewrite supplied persisted times or filesystem modification time. Request IDs/expected revisions still identify the intended existing record.

## React component and state design

Use built-in hooks and one small useReducer-based `useRegistry`, not a global store/query framework. Keep native `isDesktop` guarding: browser mode shows a readable shell with **Open the desktop app to manage files**, theme usable and native actions disabled. Do not fake persistence or let a browser upload input substitute for the native picker.

| Component/module                          | Responsibility                                                                                                 |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| App / AppShell                            | Desktop bootstrap/version, compact header/theme and responsive panel shell; no SQL/filesystem policy.          |
| FileList / FileListItem                   | My Files, compact Add file, readable rows/selected state/loading/unreadable entries.                           |
| FileDetails                               | Saved facts, recorded status explanation, optional metadata details, Aliases heading.                          |
| AliasList / AliasRow                      | Ordered exact names, compact actions, stable ID keys, focus recovery.                                          |
| AliasNameForm                             | Reusable inline add/rename editor, byte feedback, authoritative errors, pending state.                         |
| DeleteAliasDialog / DuplicateFileDialog   | Small accessible decisions with precise safe copy.                                                             |
| Feedback / EmptyState                     | Local field errors, global storage notice, polite success announcement, real empty/loading distinction.        |
| ThemeControl / useTheme                   | System/Light/Dark enum, persisted preference and OS listener.                                                  |
| useRegistry / registry client / transport | Async state transitions, token/revision/epoch propagation, runtime DTO checking; no domain identity inference. |

State contains storage phase/epoch, ordered inventory, `byId` snapshots, selectedId, per-resource request counters, active editor bound to canonicalId/aliasId/baseRevision/draft, pending mutation receipt, conflict/reload requirements and feedback. Keep cached snapshots display-only; writing is always through a command using an explicitly reviewed revision.

Initial status then inventory; selection triggers get for a current snapshot. All reads carry a request sequence and captured epoch/record ID. Accept a response only if its sequence remains current for that resource, its epoch matches the active session and it is not older than an already accepted mutation revision. Compare revision decimals with BigInt. Never compare revisions across canonical IDs. Starting a mutation invalidates outstanding reads for that record; successful mutation response wins over an earlier read/list. A list response cannot replace a newer byId snapshot for the same ID. Do not auto-select a row because a late response for another selection arrived.

Allow at most one pending mutation per window and disable conflicting editor/selection actions during dispatch; read-only browsing elsewhere may be allowed only if the pending result stays keyed to its original ID and cannot steal focus. Prefer the simpler initial policy of temporarily disabling file switching until the bounded mutation resolves. Pre-dispatch drafts can be discarded explicitly. React StrictMode setup/cleanup must not duplicate picker opening or mutations; event handlers alone dispatch writes.

Conflicts preserve draft and original base revision. **Reload saved data** fetches current state and presents the draft alongside the new value; it does not silently retarget the draft to a new revision. A distinct **Apply my edit** after user review submits the refreshed revision. Rename/delete revalidate alias existence; if another writer removed it, explain and offer close/discard, not recreate it. Busy leaves drafts intact; an explicit retry first reloads/reviews state. No background mutation retries.

Multiple windows are not offered in M05; main-only permissions and one picker are enforced. Independent processes still exist: service revisions are authoritative and no shared in-memory mutex is claimed across processes. Refresh saved data on user request/window focus may refresh reads only, preserving any editor draft/base revision and marking it stale if necessary.

Transport disconnection after dispatch is not proof of failure. Enter transportUnknown recovery, block further writes, reopen/reload inventory and affected record if known, and ask review. Known UncertainOutcome returns attempted IDs; actual state must be inspected after reopen. Session-invalidating errors (uncertainOutcome, staleSession and transportUnknown) from an active dispatch must update the global write gate even if the selected record changed. A response carrying an unexpected epoch cannot merely be ignored while writes remain enabled; require status/reload. Old responses from a fully superseded session cannot restore readiness. If the process/response is lost, the receipt may also be lost; no exactly-once guarantee or automatic identification by same alias name/path is possible. Never blindly resend a registration/add command after reconnect.

## Error and recorded-status UX

Error codes are closed lower-camel Rust/TypeScript enums. UI owns friendly fixed copy; structured errors contain no SQL/OS message/path/name/evidence payload. Field reason permits precise explanation without interpolating sensitive input. Unexpected code uses a generic failure and safe reload/review action.

| Code/state                                                                                          | Copy and action                                                                                                                                                           |
| --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| invalidAliasName + reason                                                                           | Field-specific empty/whitespace/forbidden/device/trailing-period/byte-limit message; keep exact draft and focus input.                                                    |
| duplicateAliasName                                                                                  | **This file already has that alias.** Keep draft; do not auto-add a suffix.                                                                                               |
| duplicateAliasId / duplicateCanonicalId                                                             | **A unique record ID could not be saved. Refresh and try again.** No overwrite or silent retry.                                                                           |
| invalidCanonicalId / invalidAliasId / invalidRequest                                                | **This request could not be used. Reload saved data.** No repository write on invalid transport.                                                                          |
| aliasNotFound                                                                                       | **This alias is no longer in the saved record.** Reload; never recreate on rename/delete.                                                                                 |
| canonicalNotFound                                                                                   | **This file is no longer in the registry.** Retain draft for review; refresh inventory, no physical deletion inference.                                                   |
| conflict                                                                                            | **This record changed elsewhere. Reload it before applying your edit.** Explicit review/resubmit only.                                                                    |
| busy                                                                                                | **The registry is busy. Try again after reloading saved data.** No success or retry loop.                                                                                 |
| databaseUnavailable / permissionDenied                                                              | **The local registry could not be opened/accessed.** Block data mutation; Retry opening storage; no privilege escalation or reset.                                        |
| corruptDatabase / invalidStoredData / incompatibleDatabase                                          | **Saved data could not be read** / **This registry needs a compatible FileNym version.** Preserve files; distinguish per-record unreadable row from global blocked store. |
| unsupportedPathEncoding / unrepresentableValue                                                      | **This saved value cannot be represented on this system.** Do not replace or round it.                                                                                    |
| uncertainOutcome                                                                                    | **The save result is unknown. Reopen storage and reload before making changes.** No success toast; global write gate.                                                     |
| storageFailure / invalidAggregateChange / migrationFailed / internalFailure                         | **The registry operation could not be completed.** Safe category, preserve draft, review before retry.                                                                    |
| selectionNotFound / selectionPermissionDenied / selectionUnsupported / selectionMetadataUnavailable | Explain missing/access/unsupported regular-local-file/metadata restriction; choose another file; no registration write.                                                   |
| Recorded Available                                                                                  | **Recorded: Available**; last saved observation, not a live existence guarantee.                                                                                          |
| Recorded Modified                                                                                   | **Recorded: Metadata changed**; explain changed recorded metadata, not proven content bytes.                                                                              |
| Recorded Unavailable + reason                                                                       | **Recorded: Unavailable**, optional last-recorded not-found/access/unknown explanation; aliases remain editable.                                                          |
| Recorded PossibleReplacement                                                                        | **Recorded: Possible replacement**; last accepted facts and aliases retained; no relink/recovery control.                                                                 |

Also distinguish databaseAlreadyExists (creation race; safe explicit open/retry), staleSession (reload session), selectionInvalid/selectionChanged (choose again), pickerBusy (finish/cancel current picker), duplicatePath (decision dialog), invalidTimestamp (check system clock), unsupportedPlatform, idGenerationFailed and transportUnknown. None changes a persisted lifecycle status in M05. A Retry opening action reopens **storage**, not a physical canonical file.

## Accessibility acceptance

Use native buttons/forms/select/details and small HTML dialogs where possible. Every icon-only action has a label including its alias; decorative icons have empty alternative text. Tab order follows visual order; keyboard opening the actions popover moves focus to its first action, Escape closes/returns focus and outside-click closes it. Inline editors have a visible label, associated hint/error via aria-describedby and aria-invalid when rejected; Enter submits except during IME composition or pending work.

Dialogs need labelled headings, inert background, contained Tab/Shift+Tab navigation, Escape cancellation before dispatch, visible Cancel, and return focus. Delete defaults to the less destructive action. Pending commands cannot be undone by dismissing the dialog; retain accessible pending information until their result. Screen-reader tests must verify behavior, not just presence of aria attributes. [WAI dialog guidance](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).

Announce failures with role=alert and successes/loading completion with a polite live region without repeatedly reading the full path or every row. Keep focus outlines visible in both themes/forced colors. Long filenames use visual ellipsis without shortening accessible text; details allow wrapped full filename/path selection. Do not rely only on color for selected/error/status. Verify 100%, 125%, 150% and 200% Windows display scaling, minimum window, text zoom, keyboard-only workflows and Windows Narrator. No remote fonts/assets or decorative animations are required.

## Detailed automated test matrix

Acceptance is divided into three explicit categories. R = Rust desktop/coordinator, I = real local infra/SQLite tests, F = React/Vitest, G = real Windows GUI. Each applicable matrix row retains its safety assertions, but environment-specific branches are identified separately rather than silently counted as passed.

| Category                                     | Required evidence                                                                                                                                                                                                                                                                                                                                                                   | What blocks release                                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A: Mandatory automated acceptance            | Every R/I/F assertion in the matrix, existing M01-04 regression suites, deterministic attribute/tag/provider-error fixtures, real local junction/regular-file/SQLite tests, DTO/ACL/concurrency/error tests and all build/lint checks.                                                                                                                                              | Any failure or unexecuted mandatory assertion blocks milestone release. A simulated API/provider result must be labelled simulation; it cannot establish real OneDrive/device behavior.                                                                                                                                                                   |
| B: Mandatory real Windows GUI acceptance     | G1-G9 below in isolated app data, real native picker/IPC/services/SQLite, local file and junction workflow, CRUD/restart/sentinel integrity, conflict handling, compact minimum layout, keyboard/dialog/theme/DPI behavior.                                                                                                                                                         | Missing isolated profile, failed/blocked native picker or required GUI journey blocks release. Mocked tests do not substitute. All configured supported layout/keyboard checks remain mandatory; no visual pass without inspection.                                                                                                                       |
| E: Environment-dependent additional evidence | E1 real OneDrive/other provider with resident/partial/cloud-only files and non-hydration observation; E2 physical removable drive/disconnection and actual remote share/mapped-drive rejection; E3 privilege-dependent directory/file symlink setups beyond the mandatory junction/classifier cases; E4 other OSes, unusual display hardware, real disk-I/O/power-loss experiments. | Unavailable E checks alone do not block the bounded M05 release after A/B pass. Record them as NOT RUN or BLOCKED ADDITIONAL EVIDENCE, never PASS. No provider/device/platform-specific support or durability claim may rely on them. Observed unsafe behavior does block the affected support class/release until fixed or explicitly disabled/reviewed. |

A missing E1 environment does not justify claiming tested OneDrive compatibility or universal non-hydration. The metadata-only known-tag path can be implemented against documented APIs plus A tests; publish the real-provider limitation explicitly. If A cannot establish safe class handling without a content open, reject that class and require review of the narrower product support before release. E3 being unavailable never excuses removing final-link/unknown-tag rejection logic or its automated coverage. No safety test is discarded merely for being hard to run.

Completion report must give separate A/B verdicts and E evidence status, naming each unexecuted case. An overall PASS is limited to the approved A/B scope and must include additional-evidence limitations; it must never say every Windows/provider/device case passed. **No tests are claimed passed during planning.**

| ID       | Layer       | Required cases and assertions                                                                                                                                                                                                                                                                                                                                                          |
| -------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| DB1      | R/I/G       | First-ever startup creates only app-owned directory, sentinel, DB/sidecars; same state reopens across app/process restart. Missing-known DB never becomes empty registry; sentinel missing/corrupt/partial/unsupported states and initialization failure preserve artifacts.                                                                                                           |
| DB2      | R/I         | Existing v1 opens, wrong sentinel/schema/newer/corrupt/read-only DB rejects; no new migration; rollback/open failures remain typed. Concurrent first initialization has one creation and no adoption of half-initialized DB. No production temp/repo/network/roaming fallback.                                                                                                         |
| DB3      | R/I/F       | SQLite work and mutex waits outside UI thread; UI remains responsive through five-second Busy; no await with guard; panic/poison retires state; shutdown drains bounded in-flight work and releases handles.                                                                                                                                                                           |
| FS1      | R/I/G       | Actual native picker chooses one fixture, cancellation leaves DB/selection unchanged, repeated activation blocked, directory/multiple selection unsupported; dialog belongs to main window. No unrestricted frontend path/metadata command.                                                                                                                                            |
| FS2      | I           | Real regular/zero-size/read-only/Unicode/extended-drive fixtures; nonexistent/access-denied/sharing-denied/disappearing file fail without record; modification-time unsupported path yields None only for that recognized category. No content reads.                                                                                                                                  |
| FS3      | I           | Mandatory local junction redirect/target, broken redirect, loop and final-link rejection tests; known tag/attribute classification, cloud-only/partial/recall rejection, metadata-only access and simulated remote target rejection. Real providers/devices and privileged symlink fixtures belong to E1 below; no blanket ancestor rejection.                                         |
| FS4      | I           | Candidate changes type/size/time between select/confirm, reparse substitution at deterministic stage, disappears before final inspection; reject detectable invalidity without insertion (not a continuity guarantee). Native UTF-16 including unpaired surrogate persists and displays honestly. Residual undetectable replacement/ancestor races are documented, not claimed solved. |
| TK1      | R/F         | Normal pick/save uses no token/expiry/second call; duplicate-only token replacement/window/generation binding/forgery/concurrent reservation, cancel/view invalidation, terminal consumption and deterministic retry. No timer subsystem.                                                                                                                                              |
| DUP1     | R/I/F/G     | Repeated exact saved path decision; View existing makes no write; Cancel unchanged; Add separately preserves existing IDs/aliases/status; multiple candidates; Unavailable/PossibleReplacement candidates; corrupt candidate errors; race introduces hint before insertion.                                                                                                            |
| DUP2     | I           | Different spellings/paths/equal metadata do not silently merge; no uniqueness-by-path constraint; concurrent separate processes may both insert distinct records; no physical continuity inference.                                                                                                                                                                                    |
| IPC1     | R/F         | Actual command adapters invoke six services; malformed/unknown fields, invalid IDs/names/revisions and forbidden caller fields rejected before mutation; serialize camelCase/tags/nulls and typed closed errors. No raw SQL/path/evidence in error output.                                                                                                                             |
| IPC2     | R/F         | Size 0/u64::MAX, revisions 0/i64::MAX, beyond-JS-safe boundaries, negative/leading-zero/exponent/overflow revisions; epoch/pre-epoch/fractional/range-limit timestamps, null modified time; no precision loss; unsupported host values fail.                                                                                                                                           |
| IPC3     | R/F         | Native unit arrays preserve Unicode/unpaired units, display escape flag correct, full path used as text only; XSS/markup/quotes/bidi input not executed; transport fixtures checked by Rust and TypeScript runtime validators.                                                                                                                                                         |
| SEC1     | R/G         | Main-window/local-origin ACL only; another window/origin denied; no JS fs/dialog-save/SQL/shell access; CSP permits intended IPC/assets in dev and production and blocks remote resources. No bypass to make a test pass.                                                                                                                                                              |
| ID1      | R/I         | Independent 128-bit generated IDs/tokens/epochs, generated in Rust, bounded fixture collision and entropy failure before write; never derive from path/time; distinct service business duplicate errors with no silent regeneration/retry.                                                                                                                                             |
| TIME1    | R/I/F       | Trusted clock supplied once per operation; backward clock yielding invalid alias creation/update produces friendly error; no clamping; filesystem modified time may be older/null.                                                                                                                                                                                                     |
| CRUD1    | R/I/F/G     | Add file then zero/many aliases; exact name/extension, owner/order/ID/times and canonical metadata preserved; success only after commit; get/list reflect persisted state, sibling record unchanged.                                                                                                                                                                                   |
| CRUD2    | R/I/F/G     | Rename ordinary/case-only/identical, identity stable; identical rename conditional write increments revision; duplicate/missing alias and invalid time reject; delete first/middle/last/only alias, parent/original file survive, repeated delete error after reload.                                                                                                                  |
| VAL1     | R/F/G       | Empty/control/edge Unicode whitespace, dot/dotdot/separators, forbidden punctuation/device stems including superscript digits, trailing dot, 255/256 UTF-8 ASCII/multibyte boundaries; non-ASCII case/composed/decomposed distinctions; unpaired JS surrogate invalid. Test add and rename.                                                                                            |
| REV1     | R/I/F/G     | Caller stale/future revision, intervening external writer between load/update, repeat submit while pending; Conflict/no retry/no overwrite; explicit reload plus user-reviewed resubmit succeeds. Missing alias after conflict does not become add.                                                                                                                                    |
| ASYNC1   | F           | Rapid A/B selection, out-of-order get/list, older read after mutation, stale epoch/late mutation error, list overwriting newer selected snapshot, unmount/StrictMode, pending close/navigation and IME Enter. Full resource/epoch identity guards verified with deferred promises.                                                                                                     |
| ERR1     | R/I/F       | Busy/unavailable/permission/corrupt/invalid/incompatible/unrepresentable errors keep drafts, no false empty/success; per-record unreadable inventory vs overall failure; no automatic deletion/repair/recreation, no sensitive diagnostics.                                                                                                                                            |
| UNC1     | R/I/F       | Inject UncertainOutcome after dispatch, adapter dropped/global writes blocked, queued prior-epoch commands fail, explicit reopen existing + targeted success/definitive missing reload unblocks review, failed reopen stays blocked. Transport-lost receipt case reloads without blind retry. No claim of real disk-I/O durability.                                                    |
| LIFE1    | R/I/F/G     | Seed all six stored status/reason values, render as recorded; add/rename/delete/restart preserve accepted facts/status/evidence/observation times. No stat/watch/observe/recover on list/selection/refresh/startup/alias edits.                                                                                                                                                        |
| SAFE1    | I/G         | Own sentinel bytes, existence/name, size, modification time and sibling files unchanged after every success/error/restart; no alias files created; fixture test code may hash/read solely to assert integrity, production may not.                                                                                                                                                     |
| UI1      | F/G         | Empty/loading/no selection/nonempty/unreadable/blocked/browser-only states distinct; compact content-width Add file/alias controls, long names/counts/scrolled lists, responsive master/detail and no minimum-size clipping.                                                                                                                                                           |
| UX1      | F/G         | Inline add/rename keep exact draft and authoritative field reason; safe delete/duplicate confirmation cancellation; success focus/announcements only after persistence; no physical deletion/upload copy; no stale dirty draft silently rebound.                                                                                                                                       |
| THEME1   | F/G         | System default and live OS change, explicit light/dark override, persisted choice/restart, invalid/denied localStorage, reduced motion/forced colors; measured contrast and readable status/focus in both themes.                                                                                                                                                                      |
| A11Y1    | F/G         | Keyboard-only picker/selection/CRUD/menu/dialog, Escape/Enter/IME, focus trap/return/least-destructive default, live errors, complete truncated-name labels, Narrator and 200% zoom. No inaccessible icon-only action.                                                                                                                                                                 |
| RESTART1 | I/G         | Full committed GUI journey closes/reopens development and release apps with no Vite; exact stable IDs/order/revisions/recorded data retained; forced-process restart reads committed state without replaying intent. M03 crash tests remain regression evidence, not proof of power-loss safety.                                                                                       |
| REG1     | Build/tests | Preserve M01–04 domain/ports/persistence/service tests, adapt obsolete shell rendering test without dropping branding/version guarantees; full lint/format/typecheck/build; core/ports/app production boundaries unchanged.                                                                                                                                                            |

Tests use exclusively owned temporary directories, databases, fixture files, process IDs and cleanup guards. Close handles before deletion; verify resolved cleanup targets remain within owned roots. Deterministic permission/reparse/remote classification assertions remain mandatory; real provider/device/privileged fixture evidence is reported separately under E. Unexecuted mandatory A/B cases block release; unavailable E branches are reported as limitations under the category rules, never silently skipped or counted as a pass.

### Windows end-to-end acceptance protocol

Run against a dedicated Windows test account/profile so the unmodified application resolves isolated per-user local data. Use test-owned regular files outside that DB directory. Do not redirect normal release data through a React-supplied path or ship a testing-only arbitrary-filesystem IPC. If the profile cannot be isolated safely, GUI acceptance is blocked rather than using a real user registry.

| Journey                  | Real UI actions and proof                                                                                                                                                                                                                                                                    |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1 First launch          | Fresh isolated profile -> loading -> empty state; compact Add file; cancel picker; inventory remains empty. Original fixture unchanged.                                                                                                                                                      |
| G2 Complete workflow     | Native picker fixture and ordinary local junction-directory fixture -> saved row/detail; add three aliases -> rename/case-only/identical -> delete one with Cancel then confirm; verify exact returned revision semantics via scoped test evidence, stable IDs/order and no speculative row. |
| G3 Restart               | Close cleanly -> reopen -> same aliases/recorded data; add another alias/delete final alias while retaining parent; original bytes/path/mtime and sibling unchanged.                                                                                                                         |
| G4 Invalid and duplicate | Invalid names and byte boundaries on add/rename, duplicate ASCII name, Unicode distinctions, explicit extension changes; field focus/draft preserved; exact-path hint View existing/Cancel/Add separately all exercised.                                                                     |
| G5 Conflict/storage      | Controlled second process changes selected revision; submit stale edit -> review/reload -> explicit resubmit. File-backed Busy and storage error routing; use fault injection in tests for uncertainty, label simulation separately. No production fault-injection command.                  |
| G6 Recorded lifecycle    | Seed isolated DB through existing repository APIs before launch; all statuses displayed truthfully, aliases editable, restart preserves facts; no physical revalidation or recovery controls.                                                                                                |
| G7 Layout/accessibility  | Default/minimum/maximize/restore at 100/125/150/200% DPI; narrow fallback; long Unicode names/path, keyboard-only CRUD, Narrator, text zoom and dialogs; no giant/full-width sidebar CTA.                                                                                                    |
| G8 Themes/browser        | System/light/dark, restart preference and reduced motion; inspect contrast/readability. Frontend-only browser fallback disables native operations without pretending to persist.                                                                                                             |
| G9 Release/cleanup       | Repeat G1–G4 and core error/focus/layout/theme checks on rebuilt production executable without Vite. Confirm real IPC and close/reopen. Reap only test-owned processes/listeners/fixtures, preserve production profile.                                                                      |

Use tool-driven UI Automation/keyboard plus screenshot inspection where reliable, with manual completion when WebView accessibility omits content. Record which method executed each journey; do not call a mocked React test real E2E or claim a separate human test without one. No WebDriver framework is required for M05; compare adding one later only if it materially improves repeatability of established workflows. Never use a mocked picker as the sole native selection acceptance evidence.

## Implemented files and dependencies

The actual changed/new files are listed below. Small presentation pieces are composed in AppShell, AliasNameForm, Confirmation and ThemeControl rather than creating every anticipated one-line component. Backend tests stay private in src-tauri/src/tests.rs plus ids.rs; no production test IPC or public raw-connection API was added. Generated app-command permission files are included for review.

```text
Cargo.lock
crates/filenym-infra/Cargo.toml
crates/filenym-infra/src/lib.rs
eslint.config.js
src-tauri/Cargo.toml
src-tauri/build.rs
src-tauri/capabilities/default.json
src-tauri/src/commands/mod.rs
src-tauri/src/lib.rs
src-tauri/tauri.conf.json
src/components/AppShell.test.tsx
src/components/AppShell.tsx
src/styles.css
crates/filenym-infra/src/registration.rs
crates/filenym-infra/tests/registration.rs
plans/phase-1/05-desktop-ui.md
src-tauri/permissions/autogenerated/add_alias.toml
src-tauri/permissions/autogenerated/delete_alias.toml
src-tauri/permissions/autogenerated/get_canonical_file.toml
src-tauri/permissions/autogenerated/list_registered_files.toml
src-tauri/permissions/autogenerated/register_selected_file.toml
src-tauri/permissions/autogenerated/registry_status.toml
src-tauri/permissions/autogenerated/rename_alias.toml
src-tauri/permissions/autogenerated/reopen_registry.toml
src-tauri/permissions/autogenerated/select_existing_file.toml
src-tauri/permissions/autogenerated/version.toml
src-tauri/src/commands/registry.rs
src-tauri/src/commands/selection.rs
src-tauri/src/ids.rs
src-tauri/src/registry.rs
src-tauri/src/selection.rs
src-tauri/src/state.rs
src-tauri/src/storage.rs
src-tauri/src/tests.rs
src-tauri/src/transport.rs
src/components/AliasNameForm.test.tsx
src/components/AliasNameForm.tsx
src/components/Confirmation.tsx
src/components/ThemeControl.tsx
src/hooks/useRegistry.test.tsx
src/hooks/useRegistry.ts
src/hooks/useTheme.test.tsx
src/hooks/useTheme.ts
src/lib/aliasValidation.ts
src/lib/format.ts
src/lib/registry.test.ts
src/lib/registry.ts
src/lib/transport.ts
tests/contracts/registry.json
tests/desktop/README.md
tests/desktop/smoke.ps1
tests/desktop/native-controls.cs
tests/desktop/journey.ps1
tests/desktop/webview.mjs
tests/desktop/controls.mjs
```

ESLint also recognizes the Node-only desktop test helpers and excludes ignored temporary evidence; no production lint rule is disabled. The core/ports/application production crates and SQLite adapter/schema/migrations are unchanged. src/App.tsx and its version facade were reused without changes. No npm dependency or lockfile change was needed.

| Actual dependency                                                                                 | Placement and necessity                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tauri-plugin-dialog` 2.8.3 compatible candidate                                                  | Desktop only, Rust native picker. New dependency; check maintained tagged release/MSRV and locked Tauri compatibility before adding. Current docs list Rust 1.90 minimum; repository's recorded 1.99 toolchain is newer, but build proof is still required. No npm plugin.          |
| `serde` 1 with derive; `serde_json` 1 (dev-only)                                                  | serde is a direct desktop dependency for typed IPC; serde_json is a desktop dev-dependency for contracts; both already transitively resolved. No core/app serde. serde is needed for IPC; serde_json is needed only for transport fixtures/tests, not production sentinel handling. |
| `getrandom` 0.4.3                                                                                 | Direct desktop use of already locked package for fallible entropy; no uuid, random framework or ID port.                                                                                                                                                                            |
| `windows-sys` 0.61.2, target cfg(windows), FileSystem/Foundation/WindowsProgramming features only | Infra only for attribute/path/drive APIs; all cloud reparse objects are rejected, with no CloudFilters, hydration or identity calls. Already present transitively; native calls remain small and narrowly reviewed. No new file identity API.                                       |

Use existing Tauri blocking runtime, React/hooks/CSS, Vitest/Testing Library and PowerShell capabilities. No UI/icon/animation/state library, ORM, fs/sql plugin, HTTP dependency or E2E framework. Small icons can be local inline SVG components; no generated bitmap assets or image-generation work is needed. The dialog plugin uses rfd 0.16.0 and brings tauri-plugin-fs 2.7.1 transitively. The filesystem plugin is not initialized and grants no JS capability. Desktop tests additionally depend directly on already locked rusqlite 0.40.2 to exercise real external writers, Busy and corrupted owned fixtures. The production dependency direction is unchanged. Cargo.lock adds 14 packages, removes none, and upgrades no existing package version; feature resolution changes two existing dependency edges (cssparser-macros syn and objc2-foundation libc).

## Incremental implementation sequence

Each stage depends on the preceding contract/safety stage; UX components can proceed against approved transport fixtures once stage 3 is settled, but cannot be reported as integrated before real command acceptance.

| Stage                       | Files/responsibility                                        | Acceptance and edge cases                                                                                                                                                               |
| --------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 Baseline and approval     | This plan/manifests/source review                           | Confirm merge/current graph; approve support/sentinel/IPC/UX decisions; no identity assumptions.                                                                                        |
| 2 Storage composition       | state/storage/lib + storage tests                           | Isolated first-run/open-existing/sentinel matrix, off-thread work, no recreation, permission/schema/Busy/poison/shutdown cases.                                                         |
| 3 DTO/ACL contracts         | transport/build/capability/config + contract fixtures/tests | Lossless numeric/time/native text, closed errors, main-only permissions, tested dev/production CSP; no arbitrary path/SQL.                                                              |
| 4 IDs/times                 | ids/registry + deterministic tests                          | Entropy/collision failures explicit, trusted supplied times, backward-clock failures; no service changes.                                                                               |
| 5 Safe selection            | infra registration + desktop selection/commands/tests       | One-command picker/save, pending duplicate only, local redirect/resident-cloud classification, metadata-only/no-recall checks, identity None. Block if adapter tests contradict policy. |
| 6 Command integration       | registry/commands + desktop tests                           | Six existing use cases mapped exactly, inventory loading/error entries, duplicate hint/recheck, revision propagation; no new repositories.                                              |
| 7 Frontend state/client     | transport/client/useRegistry + tests                        | Initial/loading/error/empty states, all async identity/epoch guards, no optimistic writes or hidden retries.                                                                            |
| 8 Compact registry          | AppShell/FileList/Details/CSS + tests                       | Default and minimum layout, master/detail fallback, content-width controls, no path list clutter, saved-data labels.                                                                    |
| 9 Alias interactions        | Alias rows/form/dialogs/Feedback + tests                    | Exact names/extensions, bytes/Unicode validation, conditional no-op rename, safe delete, focus/drafts/IME/cancellation.                                                                 |
| 10 Conflict/reopen UX       | useRegistry/state/storage + fault tests                     | Stale edits require explicit review; uncertainty retires/reopens/reloads with new epoch; queued commands cannot cross recovery.                                                         |
| 11 Themes/accessibility     | useTheme/control/tokens/all controls                        | System preference, both palettes/contrast, storage denial, reduced motion, forced colors, keyboard/Narrator/DPI.                                                                        |
| 12 Real journeys/regression | infra/desktop/React suites + GUI protocol                   | Every mandatory A/B case named and executed; E cases separately recorded; development/release/no-Vite/native picker/sentinel proof, existing suites preserved.                          |
| 13 Final documentation      | This plan and diff audit                                    | Actual API/dependency/results/limits replace proposals where appropriate; links/Markdown/scope checked; no invented CI or manual evidence.                                              |

## Acceptance criteria and validation

Milestone completion requires architectural/UX approval before implementation, all six use cases accessible through real IPC/services, isolated persisted restart proof, unchanged original fixture and no alias files, accurate recorded statuses, exact transport values, safe startup/recovery and explicit conflict handling. Every matrix row must map to named A/B evidence and explicitly labelled E status; blocked mandatory checks prevent release, while unavailable additional checks remain declared limitations. Actual passing counts and incomplete acceptance are recorded below; passing automated suites alone do not complete the milestone.

Retain window constraints and verify the compact button requirement visually. Require all theme/accessibility/minimum-DPI flows and both dev/release native workflows; mocked tests alone are insufficient. Core/ports/app and SQLite invariants remain unchanged. Any necessary broader architectural rewrite or stronger identity guarantee requires a separate approved change.

After approved dependency resolution, run commands separately from repository root and inspect each result:

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p filenym-infra --test registration --locked
cargo test -p filenym-desktop --locked
cargo test --workspace --locked
cargo tree -p filenym-app --edges normal
cargo tree -p filenym-infra --edges normal
npm.cmd run format:check
npm.cmd run lint
npm.cmd run typecheck
npm.cmd run test
npm.cmd run build
npm.cmd run tauri:dev
npm.cmd run tauri:build -- --no-bundle
.\node_modules\.bin\prettier.cmd --check plans/phase-1/05-desktop-ui.md
git diff --check
git status --short --untracked-files=all
```

Run native GUI journeys separately in the isolated profile; record executable path, actual Tauri versions, OS/DPI/theme and each method/result. Use `npm.cmd ci` only if locked frontend dependencies are absent; do not change npm dependencies merely to validate. Existing format script excludes Markdown, so check this plan explicitly. Verify new contract fixture formatting with installed Prettier too. Inspect actual hosted CI before claiming its result. Additional platforms remain separate evidence; this is Windows-first.

Implementation validation and the actual GUI limitations are recorded below. The isolated acceptance protocol is in [tests/desktop/README.md](../../tests/desktop/README.md).

## Risks, limitations, and approval decisions

The original architecture/UX approval and subsequent explicit implementation authorization are preserved. These choices govern the implementation:

1. **Selection support:** accept ordinary local directory redirects with ordinary local targets; reject all cloud reparse/recall classes under the explicitly authorized fallback; reject final file links, cloud-only/partial/recall-sensitive/unknown types and remote destinations. Preserve the picker path, metadata-only access and no identity evidence. New mandatory redirect/tag/race tests and separately reported provider evidence replace the blanket ancestor ban; no claim of TOCTOU elimination.
2. **Registration trust:** last inspected facts initialize Available via existing service; UI always says recorded. Deletion/replacement immediately after selection can still occur and is not detected automatically. No path/metadata continuity promise or M06 identity algorithm is introduced.
3. **Duplicate UX:** exact-path hints with View existing/Cancel/Add separately, never merge/overwrite; distinct IDs and concurrent duplicate records permitted. No canonical deletion feature to clean them up is added here.
4. **Storage bootstrap:** choose compared option A: one immutable fixed-format attempted-initialization sentinel in local app data; missing-known DB is a blocking error, failed artifacts preserved, no automatic reset/recreation. Entire-profile deletion cannot be distinguished from a new installation. Sentinel crash/race tests are required.
5. **Coordination/recovery:** one-command normal picker/save, pending duplicate only with no expiry, one synchronized adapter via blocking runtime, epochs, global uncertainty gate and explicit reopen/reload/review; no mutation retries, cross-process lock guarantee or exactly-once delivery. Release shutdown cannot guarantee a result after forced termination/hardware failure.
6. **Transport and IDs:** decimal integers, exact native unit arrays plus escaped display, normalized timestamp pairs, Rust fallible random IDs/times, manually maintained fixture-verified Rust/TS contract. No identity bytes or native paths in errors.
7. **UX:** compact two-panel layout with narrow master/detail, inline add/rename, small safe confirmations, System/Light/Dark preference only, no auto extension rewriting or optimistic persisted rows. Default/minimum window sizes unchanged.
8. **Dependencies/security/test delivery:** Rust dialog plugin, narrow existing-package direct dependencies/Windows bindings, main-only ACL/CSP, no large frontend framework; full real GUI acceptance in an isolated Windows profile plus automated suites. Exact plugin compatibility is an implementation check, not an assumed result.

Other known limits: full aggregate inventory reads scale linearly and are not a database-wide snapshot; unusual legacy IDs/native paths/times must fail or display honestly, not be silently rewritten; privacy-sensitive paths remain local UI data; Unicode visual similarity is not prevented by ASCII uniqueness; unavailable mandatory A/B fixtures block release, while environment-dependent E evidence is a reported limitation and cannot be claimed passed. No uninstall/backup/repair/multiwindow product policy, network registry durability, non-Windows runtime support, hardware power-loss proof, or filesystem reconciliation is claimed.

Implementation and Windows acceptance are complete in the working tree, with the bounded support and evidence limits recorded below. No branch change, commit, push, merge or publication was performed.

## Implementation and validation record (2026-10-16–17)

### Actual architecture and behavior

The working tree implements React -> typed Tauri adapter -> private desktop coordinator -> the six unchanged application service operations -> the existing SQLite repository. Nine new IPC commands are registry_status, select_existing_file, register_selected_file, add_alias, rename_alias, delete_alias, get_canonical_file, list_registered_files and reopen_registry; version is retained. Commands contain no SQL or filesystem policy. Blocking inspection/SQLite work and lock waits run on Tauri's blocking runtime. One repository/epoch/pending-duplicate mutex, one picker atomic and a separate status snapshot replace a separate selection-state subsystem. The closing event is a notification only; it blocks UI writes and displays Finishing save while the background shutdown task drains the repository lock. It is not a new application service operation.

Rust generates 128-bit random hexadecimal IDs, duplicate tokens and storage epochs through fallible getrandom. It supplies each operation's timestamp. The inspector always supplies identity=None; neither paths nor IDs are physical identity. Registration metadata and every lifecycle status remain recorded facts. Existing unavailable/replacement facts survive alias operations; there is no startup physical reconciliation, watcher, relink or observation endpoint.

The UI now provides the compact two-panel registry, native-selection action, saved details, counts/status, inline alias creation/renaming, small contextual actions, safe delete/duplicate dialogs, narrow master/detail Back navigation and System/Light/Dark themes. Preferences contain only the theme enum. Names/extensions are not trimmed, normalized or rewritten. Rust enforces the 255 UTF-8-byte, ASCII-only per-owner uniqueness policy. Frontend validation includes unpaired-surrogate rejection and Rust-compatible Unicode edge whitespace, excluding BOM. Non-ASCII case and normalization forms remain distinct. Mutations use their captured revision; rejected operations keep the draft, and conflicts require an explicit reload/review before submission. No row is optimistically treated as saved.

DTOs preserve u64 sizes/i64 revisions as canonical decimal strings, timestamps as signed floor seconds plus nanoseconds, and native Windows UTF-16 units with escaped display. Saved timestamp formatting retains all nine fractional digits, with an exact fallback outside Date range. No non-Windows lossy decoding fallback is used; unsupported platforms fail explicitly. Request DTOs reject unknown fields. Runtime client guards reject malformed/unknown response values and normalize framework errors without showing diagnostics. An unexpected active epoch closes the write gate rather than silently accepting further writes. A late superseded inventory cannot overwrite a mutation.

### Final Windows support and registration coordination

Supported and demonstrated automatically: ordinary local fixed-drive files, empty/read-only/Unicode/extended-drive paths, unpaired UTF-16 filenames, and ordinary directory junction navigation to a local ordinary target. Directory symlinks use the same recognized read_link checks but have no privileged real fixture acceptance. Removable media remain eligible by drive-class policy but have no physical-device evidence. The inspector uses exact-name metadata queries, FILE_READ_ATTRIBUTES with OPEN_REPARSE_POINT, handle metadata and a local resolved-destination check; it never requests file data, hashes, previews, writes, identity tokens or link counts. The attribute handle stays alive through the insert without claiming continuity or denying file sharing.

All final reparse files and cloud reparse ancestors, offline/recall-sensitive attributes, unknown redirects, nested reparse targets, remote/device/ADS/relative/traversal/directory candidates and failed inspection are rejected. This is the explicitly authorized cloud fallback. Resident/pinned placeholders are not an exception. No OneDrive compatibility or universal non-hydration claim is made; Shell/provider behavior while browsing the native picker is not established by attribute-only adapter tests. There is no ordinary data-open fallback, CloudFilters dependency or provider-management code.

Normal registration is one select_existing_file call and no token. Only exact native-path matches create one backend-held duplicate decision. The pending record is bound to main/session, is reserved under the session lock to prevent concurrent consumption, and is revalidated outside/inside the lock before insert. No expiry timer or second selection epoch exists. Cancel/View existing/new picker/success/session replacement invalidate it. Deterministic storage/entropy errors release the reservation for explicit retry; metadata/type changes and unsupported/disappeared candidates require selecting again. Duplicate candidate sets are rechecked before Add separately. Identical paths may still represent separate records; no merge, identity inference or physical uniqueness guarantee is introduced.

An UncertainOutcome retires the repository, retains attempted IDs where available and gates all writes. Panic/poison or an unknown receipt requires inventory review. Reopen is explicit open-existing only, rotates the epoch and requires successful/definitively missing reads. An unreadable potentially affected row keeps the gate closed. Client-side transportUnknown and stale-session cases likewise require explicit reopen/reload/review. No mutation is replayed automatically.

### Database startup and dependencies

Production composition resolves only app_local_data_dir()/registry/filenym.sqlite3. A missing path resolution, remote/redirected registry path, invalid marker, inaccessible database or failed format check blocks startup. There is no repository-directory, roaming, temporary or network fallback.

The immutable 35-byte registry-initialization-v1 sentinel is reserved with create_new and synced before first database creation. Existing DBs use open_existing and format validation; a missing sentinel is created only after a valid existing database is opened. A sentinel with a missing DB blocks creation. Partial/invalid markers and failed initialization artifacts are preserved. Concurrent initialization may fail safely and require explicit reopening; it does not adopt an unfinished schema or silently reset state. Explicit reopen cannot create a replacement registry. Whole-profile deletion remains indistinguishable from a new installation, and the sentinel plus SQLite is not a claim of an atomic power-loss-safe multi-file transaction.

No application service, core/ports production contract, SQLite adapter production code, schema or migration changed. Desktop direct dependencies add the already locked core/ports/getrandom/serde packages and tauri-plugin-dialog 2.8.3; serde_json and already locked rusqlite are test dependencies. Infra adds target-Windows windows-sys 0.61.2 with three narrow feature groups. Tauri remains 2.12.1. Fourteen newly required plugin/native-target packages are added to Cargo.lock; no existing package version was removed or upgraded. The dialog plugin brings tauri-plugin-fs transitively, but no filesystem plugin is initialized or exposed to JavaScript. The generated main-only app allow-list contains precisely the nine new commands plus version, alongside core:default. Production CSP excludes unsafe-inline and remote sources; development CSP permits the intended local Vite/HMR connection and style injection. Actual production-origin/security GUI probing is still unaccepted below.

### Executed automated coverage

The tests below are real Rust/SQLite/owned-filesystem tests unless labelled simulation. Frontend tests use mocked IPC; the dialog unit test substitutes showModal and establishes component behavior only, not native focus trapping or desktop E2E.

| Source                                     | Executed evidence and matrix coverage                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| src-tauri/src/tests.rs                     | Bootstrap missing/invalid/partial sentinel, valid unmarked database, concurrent creation, corruption, read-only/newer format, explicit reopen and shutdown gate (DB1–DB3); complete registration/alias journey, identical rename/revisions, exact-path view/cancel/separate decisions, changed/reserved/forged/stale selections, exact Unicode byte boundaries, backward-clock rejection, all six recorded status/reason values and physical sentinel preservation (TK1, CRUD1–2, VAL1, TIME1, LIFE1, SAFE1); real external SQLite revision change and five-second Busy with explicit retry (REV1, ERR1); serde request rejection, native units, decimal boundaries, shared JSON, all 18 storage errors and static ACL/CSP checks (IPC1–3, SEC1). |
| src-tauri/src/tests.rs recovery cases      | uncertainty_retires_adapter_and_requires_new_epoch_and_inventory, poison_blocks_work_and_reopen_requires_inventory, background_worker_panic_without_mutex_poison_still_retires_session and unreadable_inventory_keeps_uncertain_session_gated_until_verified_reload use controlled uncertainty/panic/corruption fixtures. These establish the retirement/gating contract (SIMULATED uncertain engine result), not an actual disk-I/O or hardware failure (UNC1).                                                                                                                                                                                                                                                                                  |
| src-tauri/src/tests.rs process cases       | desktop_composition_survives_fresh_writer_reader_and_next_writer_processes launches bounded writer, reader/next-writer and reader test processes. Each process uses the actual private coordinator, bootstrap, application services and SQLite; exact snapshots/revisions and unchanged physical sentinel survive. desktop_restart_worker normally returns immediately without the test environment. This is process integration, not a GUI restart (RESTART1).                                                                                                                                                                                                                                                                                   |
| src-tauri/src/ids.rs                       | entropy_failure_is_explicit_and_bytes_encode_without_fallback uses a deterministic private entropy callback; generated_record_identifiers_are_native_random_128_bit_hex also exercises real OS entropy. Existing application-service duplicate-ID regression tests plus complete transport mapping preserve collision errors without silent regeneration (ID1).                                                                                                                                                                                                                                                                                                                                                                                   |
| crates/filenym-infra/tests/registration.rs | Six real isolated fixture tests: ordinary/zero/Unicode/read-only/extended paths; unsupported input and independent data-sharing restrictions; local/broken junctions; unpaired native filename; redirect loop; real denied attributes plus parent enumeration permission with restored ACLs (FS2–FS3, SAFE1).                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| crates/filenym-infra/src/registration.rs   | Four units: reject reparse/offline/recall classes; reject remote/device destination prefixes; deterministic disappearance/directory/junction substitution after opening the attribute handle; only Unsupported modified-time errors become None. The last error categories are supplied API-result simulations; the substitution test uses real owned filesystem mutations with the same final-inspection function (FS3–FS4).                                                                                                                                                                                                                                                                                                                     |
| src/components/AppShell.test.tsx           | Nine component cases: preserved branding/version and empty registry; normal one-command selection plus persisted alias receipt; conflict/Busy draft preservation and mandatory review; disabled browser fallback; literal markup/bidi rendering; all three duplicate decisions; inline rename and delete Cancel/confirm; unknown duplicate-save receipt with reachable recovery (CRUD1, REV1, UI1, UX1, IPC3, REG1).                                                                                                                                                                                                                                                                                                                              |
| src/components/AliasNameForm.test.tsx      | Four tests: exact rename/revision with IME/Escape; invalid-name no-dispatch/focus; modal opening/accessible title/Cancel default/Tab wrap through a showModal substitute; connected-trigger focus return after unmount (VAL1, UX1, A11Y1 component evidence).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| src/hooks/useRegistry.test.tsx             | Seven deferred-response tests: late initial inventory; repeated action guard and unexpected epoch gating; unknown transport outcome requiring reopen/reload; completion after unmount; root StrictMode replay; pending resource navigation guard; explicit startup timeout with late-response rejection (ASYNC1, UNC1).                                                                                                                                                                                                                                                                                                                                                                                                                           |
| src/hooks/useTheme.test.tsx                | Two tests for enum preference/denied storage and system listener/explicit override cleanup (THEME1).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| src/lib/registry.test.ts                   | Four boundary/contract tests: alias forbidden categories and byte/Unicode cases; exact numeric/time/native values and invalid payload rejection; shared Rust/TS fixture; Rust-compatible whitespace/BOM policy (IPC1–3, VAL1).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Existing M01–M04 suites                    | All prior Rust tests remain, including crash/restart workers, rollback/commit failures, revision races, Busy, domain/lifecycle/identity invariants and service error categories. The obsolete hero test was adapted while retaining branding/version assertions (REG1).                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

Final suite accounting is **112 Rust tests** including the compile-fail doctest and three subprocess-worker entries, and **26 frontend tests in five files**. Of the Rust entries, 36 are new: 26 desktop (including one worker), four infra classifier/final-inspection units and six native-input integration tests. Table-driven branches are not separate counted tests. No test is ignored to obtain these counts. The earlier M01–M04 baseline was 76 Rust tests, including two workers.

The final `generated_runtime_acl_denies_other_windows_remote_origins_and_ungranted_commands` test executes Tauri's generated runtime authority for every app command: local main/main is allowed, other/other and remote origins are denied, and ungranted dialog/filesystem/SQL/window-creation commands are denied. This uses the real compiled ACL resolver with supplied labels/origins, not a static JSON assertion or a second physical native window. A live production WebView navigated to an owned loopback HTTP page additionally received **Command registry_status not allowed by ACL**, then returned to the app. No production testing IPC or extra window capability was added. This resolves the window-scope acceptance assertion at its actual authorization boundary; no unsupported secondary-window UI is claimed.

Real SQLite contention additionally established responsive rendering and independent status IPC during the five-second Busy wait. Native close while a write waited on an external lock drained after release (about 2.9 seconds), exited cleanly, and retained the committed alias at revision 14. The component and live dialog checks establish safe default focus, Tab containment, Escape and focus return. **Category A: PASS**, using these explicitly stated methods in addition to the named suites.

### Real Windows GUI results and human acceptance

Development and production used the temporary identifier `app.filenym.m05-validation-20261016`, with isolated registry/WebView data. The normal production profile was not used. The production executable was rebuilt with that config and tested with no port 5173 listener. The final normal release build restores `app.filenym.desktop`; it was not launched against user data. Temporary evidence remains under ignored `temp/m05-evidence/`; fixture registries are preserved beneath the isolated identifier, including `registry-production-evidence`. The development registry was preserved/restored as a whole while a separate fresh production registry exercised first-run behavior; no production data or schema was reset.

UI Automation drives accessible controls. Native picker controls without patterns use bounded, PID/class-checked WM_SETTEXT/BM_CLICK messages; no mouse fallback is required. Loopback CDP was enabled only through the environment of the isolated test process, with no product debug port/config/IPC change. It exercised the actual WebView, real commands/services and SQLite. CDP-generated input, viewport/media emulation, native window actions and human checks are distinguished below. The helpers/protocol are in [tests/desktop](../../tests/desktop/README.md).

| Journey | Final result and evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| G1      | PASS in development and fresh production: empty state, branding/version, compact controls, real native picker Cancel, unchanged zero-record inventory.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| G2      | PASS: native regular-file selection, real local directory-junction selection in development and final production, three aliases, case-only and identical rename, delete Cancel/confirm. Scoped SQLite evidence established revision 6 and ordered `context.PDF`, `Third.txt` after initial CRUD. Fresh production repeated native picker/CRUD through `journey.ps1`.                                                                                                                                                                                                                                                                                                                                                                                                                                |
| G3      | PASS: clean close/reopen and persisted records, aliases, exact metadata/revisions; subsequent writes succeed. Lifecycle fixtures exercise deletion of the final alias while retaining the parent. Hash/path/size/mtime of original, junction target and sibling remained unchanged; no physical alias files exist.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| G4      | PASS in actual development/production UI: invalid path/device/whitespace/character/trailing-period/over-byte names (path and byte rejection also exercised during rename), 256-byte multibyte rejection, exact 255-byte acceptance, ASCII duplicate rejection with draft/focus retained, distinct Unicode case/normalization names. All exact-path Cancel/View existing/Add separately decisions passed through native selection. Extensions remain exactly entered.                                                                                                                                                                                                                                                                                                                                |
| G5      | PASS: real external SQLite revision update produces conflict, disables Save, retains draft, and requires explicit reload/review/resubmit. A real external write lock produces Busy without a false success or automatic retry; final production requires reload/review before retry. Status IPC and animation frames remain responsive. Missing DB with existing marker produces blocked storage, disabled writes and no recreated file. Uncertain-outcome/transport-loss/poison cases remain deliberately SIMULATED in Rust/React tests, not claims about hardware failure.                                                                                                                                                                                                                        |
| G6      | PASS: six isolated schema-valid fixture records (available, modified, possible replacement and three unavailable reasons) display honestly in production. Each allows alias add/final delete; before/after receipts preserve path, size, modification/registration/observation timestamps and status. Restart/repeat advances revisions from 2 to 4 without reclassification or physical resolution. Fixtures were seeded by scoped test-only SQLite writes using the existing schema; no production seed endpoint or migration exists.                                                                                                                                                                                                                                                             |
| G7      | PASS with methods distinguished: native 125% DPI default/minimum/maximize/restore measured 1125x800 and 800x550 client pixels (900x640 and 640x440 logical); captures inspected. Actual WebView viewport/DPR emulation also covered 100/125/150/200% rendering and narrow detail layout. Doubled computed text sizes at minimum logical viewport retained scrolling/editor reachability without horizontal overflow (test-only text resizing, not native browser zoom). Keyboard menus/dialogs, Cancel default, Tab/Shift+Tab containment, Escape/focus return passed. **The user separately confirmed Narrator/keyboard/dialogs and actual Windows 100/125/150/200% layout checks all passed, with usual scaling restored.** This is human-reported acceptance, not automated speech verification. |
| G8      | PASS: real theme select keyboard operation, explicit Light/Dark, persisted Dark after clean production restart; System responds to emulated OS media changes. User's confirmation covers the listed theme review; no independent OS-setting trace is asserted. Actual rendered palettes meet text 4.5:1 and focus/control boundaries 3:1 across tested surfaces. Forced-colors/reduced-motion media emulation and captures passed. Installed Edge headless, isolated profile and owned loopback static server returned exit 0 with disabled native operations, browser disclaimer and no native version; screenshot inspected.                                                                                                                                                                      |
| G9      | PASS: rebuilt production executable without Vite repeated fresh G1–G4, error/focus/layout/theme checks, real IPC, close/reopen and persisted lifecycle operations. Test windows and temporary listeners were closed; owned evidence was retained. No hosted CI run, installer acceptance or other-platform run is asserted.                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |

The initial native Cancel limitation was resolved with targeted native messages, without the previously declined mouse fallback. Failed prototype harness assumptions (picker labels, Enter character dispatch, fixture selection, dev/production origin-specific theme storage, browser startup/exit handling) were corrected before accepting results. The attempted Tauri mock-runtime feature test could not start on this host; that experimental feature/test was removed and replaced by the passing generated-authority test above. No ignored test or runtime dependency was retained to bypass failure.

Live tests found and fixed real dialog defects: React autofocus ran before `showModal`, selecting the destructive action and obscuring the original focus target; focus now moves explicitly to Cancel after opening and returns after unmount. Explicit Tab wrapping keeps focus inside the production modal. Busy now uses the same explicit review gate as conflicts. Input/select boundaries and selected-row borders were strengthened for contrast. Regression assertions and final real-production checks passed after those changes.

**Category B: PASS within Windows M05 scope**, combining tool-driven real GUI evidence and the explicitly attributed human checks. **Category E: NOT RUN additional evidence** for real OneDrive/providers, physical removable/network devices, privileged symlink fixtures, other platforms/unusual hardware and power-loss experiments. Unsupported cloud reparse classes remain disabled; optional evidence cannot imply provider/device/identity support.

### Validation commands and scope audit

| Command/check                                                                           | Actual result                                                                                                                         |
| --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| cargo fmt --all; cargo fmt --all -- --check                                             | PASS                                                                                                                                  |
| cargo check --workspace --locked                                                        | PASS                                                                                                                                  |
| cargo clippy --workspace --all-targets --locked -- -D warnings                          | PASS, no warnings                                                                                                                     |
| cargo test -p filenym-infra --test registration --locked                                | PASS, six tests in the final workspace run; focused runs also executed                                                                |
| cargo test -p filenym-desktop --locked                                                  | PASS in the workspace run; focused desktop runs also executed                                                                         |
| cargo test --workspace --locked                                                         | PASS, 112 tests including doctest/workers                                                                                             |
| cargo tree -p filenym-app --edges normal; cargo tree -p filenym-infra --edges normal    | PASS; approved inward dependency direction                                                                                            |
| npm.cmd run format:check                                                                | PASS                                                                                                                                  |
| npm.cmd run lint                                                                        | PASS                                                                                                                                  |
| npm.cmd run typecheck                                                                   | PASS                                                                                                                                  |
| npm.cmd run test                                                                        | PASS, 26 tests                                                                                                                        |
| npm.cmd run build                                                                       | PASS, also executed by the Tauri production build                                                                                     |
| npm.cmd run tauri:dev -- --config temp/m05.config.json                                  | PASS; real isolated native selection/CRUD and acceptance above                                                                        |
| npm.cmd run tauri:build -- --no-bundle --config temp/m05.config.json                    | PASS; final isolated production executable rebuilt and GUI accepted without Vite                                                      |
| npm.cmd run tauri:build -- --no-bundle                                                  | PASS; normal production identifier restored in final executable; real GUI used the isolated identifier build of the same final source |
| Installed Prettier checks for this plan, tests/desktop/README.md and shared JSON        | PASS after final formatting                                                                                                           |
| Relative links/source paths, PowerShell parser, git diff --check and working-tree scope | PASS after final audit                                                                                                                |

Early expectation failures were corrected with evidence: Windows SystemTime retains 100 ns rather than 1 ns; attribute-only opens can succeed independently of data-sharing denial; file-only deny-attributes did not establish an inaccessible-metadata fixture here, so the final owned fixture denies parent enumeration too and restores the directory before the file. One temporary fixture cleanup failed while the directory was still denied; its ACLs were restored, its exact owned path/content were verified, and it was removed. These were test-harness corrections, not production data-access broadening. No arbitrary user file or registry was reset, repaired or deleted.

### Remaining limitations and verdict

Filesystem/SQLite operations are not atomic together. Same-metadata replacement and ancestor changes can remain undetectable; a held attribute handle or resolved path is not identity evidence. No physical availability check happens during list/refresh/startup/alias editing. Unsupported cloud/unknown/nested redirects and non-Windows runtime support remain explicit. Inventory is linear and not a database-wide snapshot. Local marker/profile deletion, forced termination and hardware failure exceed demonstrated guarantees. Retired/unknown outcomes require user review and never imply rollback or exactly-once registration.

Production core/ports/application contracts and SQLite schema/behavior remain unchanged. No M06 or later identity detection, watcher, reconciliation, relink, physical aliasing, file presentation, browser/upload/cloud-service integration was implemented. Git remains on the existing main branch; no commit, push, merge or publication was performed.

**Verdict: PASS within the approved Windows Milestone 05 scope.** Mandatory automated checks and real desktop acceptance passed using the methods above; Narrator and actual four-scale Windows checks are user-reported. Environment-dependent checks remain unclaimed limitations. Implementation, regressions and this evidence record are ready for review; nothing was committed, pushed, merged or published.
