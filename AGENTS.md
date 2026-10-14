# FileNym Engineering Instructions

## Project

FileNym is an open-source, local-first desktop application.

Its central product concept is:

> One file. Many names.

A canonical physical file may have multiple logical contextual names without requiring multiple maintained copies of that file.

Read the existing repository documentation before making architectural or implementation decisions.

## Current Development Scope

The current implementation phase is Phase 1: File Identity & Alias Registry.

Phase 1 includes only:

1. Register an existing physical file with FileNym.
2. Persist metadata describing the canonical file.
3. Create multiple logical aliases for the canonical file.
4. Persist aliases locally.
5. Rename and delete logical aliases.
6. Detect relevant changes to the canonical file.
7. Handle canonical file modification.
8. Handle canonical file rename.
9. Handle canonical file movement where reliably detectable.
10. Handle canonical file deletion or unavailability.
11. Detect or safely handle canonical file replacement.
12. Reconcile stored state when FileNym starts after filesystem changes occurred while the application was not running.
13. Provide recovery/relink behavior for unavailable canonical files.

## Explicitly Out of Scope

Do not implement or introduce any of the following during Phase 1 unless the user explicitly changes the scope:

- hard links
- symbolic links
- virtual filesystems
- WinFsp
- FUSE
- browser extensions
- Native Messaging
- browser upload interception
- external filename presentation
- virtual drag and drop
- shell extensions
- cloud synchronization
- user accounts
- authentication
- remote databases
- telemetry
- AI filename generation
- automatic file copying as an alias mechanism

Research documents may mention these future mechanisms, but Phase 1 production code must not implement them.

## Technology Baseline

The approved application technology baseline is:

- Tauri 2 for the desktop application shell
- React for the desktop user interface
- TypeScript for frontend code
- Rust for the FileNym core, application services, filesystem interaction, and native logic
- SQLite for local persistence

No server is required.

No HTTP backend is required.

No remote database is required.

## Architectural Direction

Use clear dependency boundaries.

Conceptually:

React UI
    |
Tauri Adapter
    |
Application Services
    |
FileNym Core
    |
Ports / Interfaces
    |
SQLite and Filesystem Adapters

Dependencies should point inward toward the domain rather than outward from the domain.

### React

React is responsible for presentation and user interaction.

React must not contain filesystem business rules.

React must not access SQLite directly.

React must not determine canonical file identity.

### Tauri

Tauri is an adapter between the frontend and the Rust application.

Tauri commands must remain thin.

Do not place substantial domain logic, filesystem policy, or SQL directly inside Tauri commands.

A Tauri command should validate transport-level input, call an application service, and translate the result for the frontend.

### Application Layer

Application services coordinate use cases such as:

- register canonical file
- create alias
- rename alias
- delete alias
- list registered files
- reconcile canonical file state
- relink unavailable file

Application services may coordinate domain objects and ports.

They should not contain UI concerns.

### Domain/Core

The FileNym core represents concepts and rules such as:

- CanonicalFile
- Alias
- AliasName
- FileIdentity
- FileStatus

The core must not depend on:

- React
- Tauri UI concepts
- browser APIs
- WinFsp
- FUSE
- browser extensions

Avoid coupling the domain to a specific future presentation mechanism.

### Persistence

SQLite stores FileNym metadata.

SQLite does not store copies of registered user files.

The actual canonical file remains where the user placed it.

Persistence code must be behind repository abstractions rather than scattered through application services.

## Fundamental Domain Rules

### One canonical physical file

FileNym does not create a physical copy merely because the user creates an alias.

### Aliases are metadata

During Phase 1 an alias is a logical FileNym record.

Creating an alias must not rename, move, modify, copy, hard-link, or symlink the canonical file.

### Alias deletion is safe

Deleting a FileNym alias must never delete the canonical physical file.

### Path is not identity

Never assume that a filesystem path uniquely defines the canonical file.

A canonical file has:

- a FileNym internal identity
- filesystem identity information when available
- a current known path
- metadata
- availability/status

A path is the current known location of the canonical file, not its FileNym identity.

### Same path does not necessarily mean same file

If a canonical file disappears and another file later appears at the same path, FileNym must not silently assume that it is the same canonical file.

### Preserve user intent

When a canonical file becomes unavailable, retain FileNym metadata and aliases unless the user explicitly removes the FileNym record.

## Canonical File Lifecycle

Phase 1 must distinguish at least:

- available
- modified
- renamed
- moved
- unavailable/deleted
- potentially replaced

Exact filesystem mechanisms for detecting these states must be validated experimentally.

Do not make unsupported assumptions about Windows filesystem identity behavior.

## File Watching vs File Identity

Treat these as separate concerns.

File watching answers:

> Something changed.

File identity answers:

> Is this still the same canonical file?

Do not conflate them.

## Startup Reconciliation

FileNym cannot assume it observed every filesystem event.

The application may have been closed while a file was changed.

Therefore persistent state must be reconciled with filesystem state at application startup.

Correctness must not depend exclusively on a real-time watcher.

## Errors

Prefer typed Rust errors over arbitrary strings.

Errors should distinguish meaningful categories where appropriate, including:

- invalid input
- invalid alias
- duplicate alias
- file not found
- file unavailable
- permission denied
- persistence failure
- filesystem failure
- identity mismatch
- possible replacement

Do not expose sensitive internal details unnecessarily to the UI.

## Security

Treat all paths and filenames as untrusted input.

Validate alias names.

Prevent path traversal.

Do not allow an alias to escape an intended naming context.

Do not silently change file extensions as a side effect of alias creation.

Filesystem operations must respect operating-system permissions.

Do not weaken OS security boundaries for convenience.

## Testing

New domain behavior requires tests.

Prefer:

- unit tests for domain rules
- repository tests for persistence
- integration tests for SQLite adapters
- controlled filesystem tests for file lifecycle behavior

Filesystem behavior that depends on Windows semantics must be demonstrated through reproducible tests before becoming a relied-upon architectural assumption.

Tests must not modify arbitrary user files.

Use isolated temporary test directories.

## Database Changes

Use migrations.

Do not manually mutate production schema assumptions throughout the codebase.

Schema changes should be explicit and reproducible.

## Dependencies

Do not add dependencies merely for convenience.

Before introducing a dependency:

1. determine whether the standard library or existing dependency already solves the problem
2. verify that the dependency is actively maintained
3. understand why FileNym needs it
4. avoid introducing a large framework for a small task

## Code Quality

Prefer:

- small cohesive modules
- explicit types
- clear ownership
- dependency inversion where it provides real value
- composition over unnecessary inheritance-style abstractions
- readable code over clever code
- deterministic behavior
- typed boundaries

Avoid:

- giant service objects
- giant Tauri command modules
- speculative abstractions
- unnecessary generic frameworks
- premature optimization
- hidden filesystem side effects
- silent fallback between semantically different mechanisms

## Architecture Changes

Do not silently change established architecture.

If implementation reveals that an architectural decision is incorrect:

1. document the evidence
2. explain the proposed change
3. update or create an ADR/RFC where appropriate
4. obtain approval before performing a large architectural rewrite

## Research Claims

Existing GitHub issue comments and research responses are hypotheses unless backed by reliable documentation or reproducible experiments.

Do not convert an unsupported issue comment directly into architecture.

## Execution Plans

For substantial features, architecture changes, migrations, or multi-step implementation work, create an execution plan under:

`plans/`

The plan must describe:

- objective
- scope
- non-goals
- affected architecture
- implementation steps
- tests
- validation criteria
- risks
- unresolved questions

Do not begin a large implementation until the plan has been reviewed.

## Working Method

Before implementing a requested milestone:

1. read relevant repository documentation
2. inspect the current codebase
3. identify affected architectural boundaries
4. write or update the execution plan when required
5. implement the smallest coherent change
6. run formatting
7. run linting/static analysis
8. run relevant tests
9. report what changed
10. report assumptions or unresolved questions

Do not expand scope simply because an adjacent feature appears useful.