# Phase 1 Milestone 01: Project Foundation

## Status

**Completed and merged into `main`.** The foundation was implemented in commit `ba80810` and merged through pull request #11 in commit `7d65d48`. The line-ending follow-up is recorded in `c13ebfb`.

This document preserves the milestone's scope, design rationale, implementation sequence, and acceptance criteria. Forward-looking instructions describe the original foundation work, not additional work to perform after completion. Milestone 02's domain implementation is documented separately in [Domain Core](02-domain-core.md).

## Objective

Establish the FileNym application foundation for Phase 1: File Identity & Alias Registry.

This milestone creates the repository structure, workspace boundaries, development tooling, and CI checks needed for future implementation. It must not implement user-facing FileNym behavior yet.

The foundation separates the frontend transport boundary from the Rust dependency graph:

```text
React UI
  -> Tauri invoke boundary

filenym-desktop (src-tauri)
  -> filenym-app   -> filenym-core, filenym-ports
  -> filenym-infra -> filenym-core, filenym-ports

filenym-ports -> filenym-core
filenym-core -> Rust standard library
```

The immediate goal is a buildable, testable Tauri 2 + React + TypeScript + Rust workspace with clear boundaries that future milestones can fill in.

## Scope

Included:

- Create the initial desktop application skeleton.
- Create a Rust workspace with separate crates for the desktop adapter, application layer, domain core, port traits, and infrastructure adapters.
- Create a React + TypeScript frontend skeleton.
- Add formatting, linting, and test commands.
- Add CI for the foundation-level checks.
- Add minimal placeholder modules only where needed to make the workspace compile.
- Document the intended dependency direction in crate manifests and module comments.

Not included:

- Registering files.
- Creating, renaming, or deleting aliases.
- SQLite schema or migrations.
- Filesystem identity design.
- File watching.
- Startup reconciliation.
- Relink/recovery behavior.
- Any external alias presentation mechanism.
- Any hard link, symbolic link, virtual filesystem, WinFsp, FUSE, browser extension, Native Messaging, cloud, account, telemetry, or AI functionality.

## Repository Structure

The foundation should use this structure:

```text
.
|-- .github/
|   |-- workflows/
|   |   `-- foundation.yml
|   |-- ISSUE_TEMPLATE/
|   |-- pull_request_template.md
|-- crates/
|   |-- filenym-core/
|   |   |-- Cargo.toml
|   |   `-- src/
|   |       `-- lib.rs
|   |-- filenym-ports/
|   |   |-- Cargo.toml
|   |   `-- src/
|   |       `-- lib.rs
|   |-- filenym-app/
|   |   |-- Cargo.toml
|   |   `-- src/
|   |       `-- lib.rs
|   `-- filenym-infra/
|       |-- Cargo.toml
|       `-- src/
|           `-- lib.rs
|-- src-tauri/
|   |-- Cargo.toml
|   |-- build.rs
|   |-- capabilities/
|   |   `-- default.json
|   |-- icons/
|   |-- src/
|   |   |-- lib.rs
|   |   |-- main.rs
|   |   `-- commands/
|   |       `-- mod.rs
|   `-- tauri.conf.json
|-- src/
|   |-- App.tsx
|   |-- main.tsx
|   |-- styles.css
|   |-- components/
|   |   `-- AppShell.tsx
|   |-- lib/
|   |   `-- tauri.ts
|   `-- test/
|       `-- setup.ts
|-- public/
|-- plans/
|   `-- phase-1/
|       `-- 01-project-foundation.md
|-- Cargo.toml
|-- package.json
|-- package-lock.json
|-- tsconfig.json
|-- tsconfig.node.json
|-- vite.config.ts
|-- eslint.config.js
|-- prettier.config.cjs
|-- .gitignore
|-- rustfmt.toml
|-- README.md
|-- VISION.md
|-- ROADMAP.md
|-- CONTRIBUTING.md
|-- SECURITY.md
|-- docs/
`-- rfcs/
```

`src/` is the React frontend source. `src-tauri/` is only the Tauri shell and command adapter. Rust domain and service code lives under `crates/`.

## Rust Crate Boundaries

### Root Workspace

Create a root `Cargo.toml` that declares these workspace members:

```toml
[workspace]
members = [
  "crates/filenym-core",
  "crates/filenym-ports",
  "crates/filenym-app",
  "crates/filenym-infra",
  "src-tauri",
]
resolver = "2"
```

The implemented workspace uses resolver 2. Each crate declares its own package metadata; the foundation workflow applies shared formatting and lint checks. No centralized workspace package or lint table was introduced.

### `filenym-core`

Purpose:

- Domain concepts and rules.
- Future homes for `CanonicalFile`, `Alias`, `AliasName`, `FileIdentity`, and `FileStatus`.

Allowed dependencies:

- Rust standard library.
- Small, domain-safe dependencies only when justified in a later plan.

Must not depend on:

- Tauri.
- React or frontend concepts.
- SQLite libraries.
- Filesystem watcher libraries.
- Platform-specific filesystem APIs.

Initial milestone content:

- A minimal `lib.rs` with module-level documentation describing the crate's role.
- No production domain behavior yet.

### `filenym-ports`

Purpose:

- Trait boundaries for persistence, filesystem observation, time/ID generation, and similar external capabilities.
- Ports should be expressed in terms of domain/application concepts without leaking adapter implementation details.

Dependency direction:

- May depend on `filenym-core`.
- Must not depend on `filenym-app`, `filenym-infra`, or `src-tauri`.

Initial milestone content:

- A minimal `lib.rs` with module-level documentation.
- No finalized repository or filesystem identity traits yet, because those belong to later implementation plans.

### `filenym-app`

Purpose:

- Application services and use-case orchestration.
- Future homes for register file, create alias, rename alias, delete alias, list files, reconcile, and relink use cases.

Dependency direction:

- May depend on `filenym-core` and `filenym-ports`.
- Must not depend on `filenym-infra`, Tauri, React, or SQLite directly.

Initial milestone content:

- A minimal `lib.rs` with module-level documentation.
- No use-case implementation yet.

### `filenym-infra`

Purpose:

- Infrastructure adapters for persistence, filesystem access, and platform integration.
- Future homes for SQLite repositories and filesystem adapters.

Dependency direction:

- May depend on `filenym-core` and `filenym-ports`.
- Must not be depended on by `filenym-core`, `filenym-ports`, or `filenym-app`.
- Tauri may compose this crate with `filenym-app`.

Initial milestone content:

- A minimal `lib.rs` with module-level documentation.
- Do not add SQLite dependencies in this milestone unless the approved implementation explicitly starts persistence setup. SQLite is approved for Phase 1, but this milestone should avoid schema decisions.

### `src-tauri`

Purpose:

- Tauri 2 desktop shell.
- Thin command adapter.
- Application composition root for desktop runtime wiring.

Dependency direction:

- May depend on `filenym-app`, `filenym-core`, `filenym-ports`, and `filenym-infra`.
- Must not contain domain rules, filesystem policy, or SQL.

Initial milestone content:

- Standard Tauri 2 app entry points.
- A `commands` module reserved for future Tauri commands.
- At most a simple health/version command if needed to validate frontend-to-Tauri wiring. If added, it must not touch filesystem or persistence behavior.

## Frontend Structure

React source should live in root `src/`.

Suggested files:

- `src/main.tsx`: React bootstrap only.
- `src/App.tsx`: top-level application composition.
- `src/components/AppShell.tsx`: basic application frame.
- `src/lib/tauri.ts`: typed wrapper around Tauri `invoke` calls.
- `src/styles.css`: global foundation styles.
- `src/test/setup.ts`: frontend test setup.

Frontend rules for this milestone:

- The UI may show a simple placeholder shell for FileNym Phase 1.
- The UI must not encode filesystem identity rules.
- The UI must not access SQLite.
- The UI must not perform direct filesystem operations.
- All future native behavior must go through typed Tauri adapter functions.

## Tauri Adapter Location

Tauri commands should be located under:

```text
src-tauri/src/commands/
```

The module should remain thin:

1. Validate transport-level input.
2. Call an application service from `filenym-app`.
3. Translate typed Rust results into frontend-safe DTOs.

No substantial domain logic, filesystem policy, or persistence queries should be placed in Tauri command functions.

## Dependency Direction

The implemented direct dependencies are:

```text
filenym-desktop (src-tauri) -> filenym-app, filenym-infra
filenym-app                -> filenym-core, filenym-ports
filenym-infra              -> filenym-core, filenym-ports
filenym-ports              -> filenym-core
filenym-core               -> Rust standard library

React frontend
  -> Tauri invoke boundary
```

Additional notes:

- `filenym-core` is the inward-most Rust crate.
- `filenym-app` coordinates use cases and depends on abstractions.
- `filenym-infra` implements external adapters later.
- `src-tauri` is the desktop composition and transport adapter.
- TypeScript code communicates with Rust only through Tauri commands.

## Testing Structure

### Rust Tests

Use Rust's built-in test structure:

```text
crates/filenym-core/src/
crates/filenym-core/tests/
crates/filenym-ports/src/
crates/filenym-app/src/
crates/filenym-app/tests/
crates/filenym-infra/src/
crates/filenym-infra/tests/
src-tauri/src/
```

Foundation milestone checks:

- `cargo test --workspace` should pass.
- Placeholder crates may contain trivial compile tests only if needed.

Future test allocation:

- Domain invariant tests belong in `filenym-core`.
- Application service tests belong in `filenym-app`.
- Port contract tests may belong near `filenym-ports` or adapter test suites.
- SQLite repository tests belong in `filenym-infra`.
- Controlled filesystem lifecycle tests belong in `filenym-infra` or a dedicated later test crate, using isolated temporary directories only.

### Frontend Tests

Use Vitest and React Testing Library for frontend behavior.

Suggested structure:

```text
src/**/*.test.ts
src/**/*.test.tsx
src/test/setup.ts
```

Foundation milestone checks:

- `npm run test` should run frontend tests.
- The initial test can verify that the app shell renders.

### End-to-End Tests

The foundation did not add an E2E framework. Full workflow E2E tests remain deferred until user workflows exist. Milestone 02 includes regression checks of the existing desktop shell, without introducing alias UI workflows.

## Formatting and Linting

### Rust

Add `rustfmt.toml` with conservative defaults. Avoid unusual formatting choices.

Required commands:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### TypeScript and React

Use:

- TypeScript strict mode.
- ESLint for TypeScript/React linting.
- Prettier for formatting.

Required commands:

```text
npm run format:check
npm run lint
npm run typecheck
npm run test
npm run build
```

`npm run build` should include Vite frontend build. Tauri bundle/build can be left out of routine CI if platform package dependencies make it too heavy, but `cargo check` for `src-tauri` should be included.

## Development Commands

The implemented scripts are defined in [package.json](../../package.json):

| Command                | Responsibility                                                                                      |
| ---------------------- | --------------------------------------------------------------------------------------------------- |
| `npm run dev`          | Start Vite for frontend-only development                                                            |
| `npm run tauri:dev`    | Start the Tauri desktop application and Vite                                                        |
| `npm run build`        | Build the frontend with Vite                                                                        |
| `npm run tauri:build`  | Build the desktop application with Tauri                                                            |
| `npm run typecheck`    | Check both `tsconfig.json` and `tsconfig.node.json` without emitting JavaScript                     |
| `npm run lint`         | Run ESLint                                                                                          |
| `npm run format`       | Format the explicitly listed frontend/configuration files with Prettier, then run Rust formatting   |
| `npm run format:check` | Check the same frontend/configuration file set with Prettier; Rust formatting is checked separately |
| `npm run test`         | Run Vitest once                                                                                     |
| `npm run test:watch`   | Run Vitest in watch mode                                                                            |

The format scripts do not include Markdown documents or lockfiles. On Windows PowerShell, use `npm.cmd` if script execution policy prevents invoking `npm` directly. Install the locked frontend dependencies with `npm ci`; commit neither dependency updates nor lockfile changes merely to run validation.

Rust commands:

```text
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Local setup command:

```text
npm ci
```

Run the frontend only:

```text
npm run dev
```

Run the desktop app:

```text
npm run tauri:dev
```

## CI Checks

The committed workflow is [foundation.yml](../../.github/workflows/foundation.yml).

Implemented CI configuration:

- Trigger on pull requests and pushes to the default branch.
- Run on `windows-latest` because early FileNym filesystem research and developer environment are Windows-oriented.
- Only `windows-latest` is configured. Additional platform jobs remain deferred until their Tauri system prerequisites are validated.

Steps in the single foundation job:

1. Checkout repository.
2. Install stable Rust.
3. Install Node.js 22.
4. Run `npm ci`.
5. Run `npm run format:check`.
6. Run `npm run lint`.
7. Run `npm run typecheck`.
8. Run `npm run test`.
9. Run `npm run build`.
10. Run `cargo check --workspace`.
11. Run `cargo fmt --all -- --check`.
12. Run `cargo clippy --workspace --all-targets -- -D warnings`.
13. Run `cargo test --workspace`.

Do not require a Tauri installer bundle in CI for this milestone unless package dependencies are proven stable and inexpensive.

These are configured checks, not evidence of a successful GitHub Actions run. No hosted run status is asserted in this document.

## Foundation files

Create:

- `plans/phase-1/01-project-foundation.md`
- `Cargo.toml`
- `Cargo.lock`
- `crates/filenym-core/Cargo.toml`
- `crates/filenym-core/src/lib.rs`
- `crates/filenym-ports/Cargo.toml`
- `crates/filenym-ports/src/lib.rs`
- `crates/filenym-app/Cargo.toml`
- `crates/filenym-app/src/lib.rs`
- `crates/filenym-infra/Cargo.toml`
- `crates/filenym-infra/src/lib.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`
- `src-tauri/src/main.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/mod.rs`
- `src/App.tsx`
- `src/main.tsx`
- `src/styles.css`
- `src/components/AppShell.tsx`
- `src/components/AppShell.test.tsx`
- `src/lib/tauri.ts`
- `src/test/setup.ts`
- `public/`
- `package.json`
- `package-lock.json`
- `tsconfig.json`
- `tsconfig.node.json`
- `vite.config.ts`
- `eslint.config.js`
- `prettier.config.cjs`
- `rustfmt.toml`
- `.gitignore`
- `.github/workflows/foundation.yml`

The foundation also committed the application logo in `public/filenym-logo.png`, platform icons in `src-tauri/icons/`, and the initial `AGENTS.md`, Phase 1 architecture documents, and accepted ADRs. The follow-up added `.gitattributes` for consistent line endings. These are recorded outcomes, rather than authorization to change existing architecture documents in later milestones.

Change:

- `README.md`: only if needed to add foundation development commands after implementation.

Do not change:

- Existing architecture documents, unless a concrete contradiction is approved for correction.
- Existing ADRs.
- Existing RFCs.

## Implementation Steps

1. Confirm the approved plan.
2. Scaffold the Tauri 2 + React + TypeScript project in place without overwriting documentation.
3. Add the Rust workspace and foundation crates.
4. Move Tauri Rust code under `src-tauri` and wire it as the adapter crate.
5. Add minimal module documentation to each Rust crate explaining its architectural role.
6. Add frontend app shell and typed Tauri invocation wrapper.
7. Add formatting, linting, typechecking, and test configuration.
8. Add CI workflow for foundation checks.
9. Run all foundation validation commands locally.
10. Update `README.md` with development commands only if the commands have been verified.

## Validation Criteria

This milestone is complete when:

- The repository has a buildable Tauri 2 + React + TypeScript foundation.
- The Rust workspace compiles.
- The crate boundaries match the dependency direction in this plan.
- The frontend builds.
- Formatting, linting, typechecking, and tests pass.
- CI contains the same checks expected locally.
- No Phase 1 domain behavior has been implemented prematurely.
- No out-of-scope mechanism has been introduced.

## Risks

- Tauri scaffolding may introduce default code that blurs adapter boundaries. Review generated code before accepting it.
- CI Tauri builds can require platform packages. Prefer frontend build plus Rust workspace checks at this milestone.
- Adding dependencies too early may accidentally commit the project to implementation details. Keep dependencies minimal.
- The current documentation still contains research-stage language, while Phase 1 instructions now approve a technology baseline. Treat this as context to document, not a reason to rewrite docs in this milestone.

## Unresolved Questions

These questions are intentionally deferred:

1. Exact filesystem identity evidence to persist.
2. Windows identity behavior across rename, move, volume boundary, network path, and removable storage.
3. Alias name normalization and cross-platform validation policy (subsequently resolved for logical aliases in [Milestone 02](02-domain-core.md#approved-validation-policies); external filename compatibility remains deferred).
4. SQLite schema and migration contents.
5. Database location strategy.
6. Filesystem watcher choice.
7. Startup reconciliation algorithm.
8. Relink and replacement decision UX.
9. Whether to add a dedicated filesystem lifecycle test crate later.

## Documentation context and implementation evidence

The initial plan recorded naming discrepancies for the domain-model document and ADRs. The implemented foundation uses [domain-model.md](../../docs/architecture/domain-model.md) and four-digit ADR filenames: [0001](../../docs/decisions/0001-local-first-desktop.md), [0002](../../docs/decisions/0002-aliases-are-metadata.md), and [0003](../../docs/decisions/0003-path-is-not-identity.md). Those naming discrepancies are resolved; they are not current missing-file issues.

Some project overview and research documents retain research-stage language and future external filename presentation goals. The accepted ADRs and [Phase 1 scope](../../docs/architecture/phase-1-scope.md) establish the approved local-first implementation baseline. Logical alias metadata and external filename presentation remain separate concerns; the latter is excluded from Phase 1.

The merged foundation contains the five-member Rust workspace, independent placeholder crates, the Tauri version command, React branding shell, one frontend rendering test, development scripts, formatting/lint/typecheck configuration, lockfiles, and the Windows CI workflow. At the Milestone 01 merge, the four non-desktop crates contained role documentation only; the later core implementation does not change that historical scope.

The merge and files are verifiable in local Git history. A standalone Milestone 01 command-by-command validation log is not retained here, so historical test results and GitHub Actions success are not reconstructed. The subsequent locally recorded [Milestone 02 validation](02-domain-core.md#implementation-and-validation-record) includes regression checks of this foundation. Those results belong to Milestone 02 and are not backdated to the foundation merge.
