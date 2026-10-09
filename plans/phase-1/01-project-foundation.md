# Phase 1 Milestone 01: Project Foundation

## Objective

Establish the FileNym application foundation for Phase 1: File Identity & Alias Registry.

This milestone creates the repository structure, workspace boundaries, development tooling, and CI checks needed for future implementation. It must not implement user-facing FileNym behavior yet.

The foundation must support the approved architecture:

```text
React UI
  -> thin Tauri adapter
  -> application layer
  -> FileNym core
  -> ports
  -> infrastructure adapters
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

The workspace should centralize package metadata and shared lint expectations where practical.

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

The allowed dependency graph for this milestone is:

```text
src-tauri
  -> filenym-app
  -> filenym-ports
  -> filenym-core

src-tauri
  -> filenym-infra
  -> filenym-ports
  -> filenym-core

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

Do not add full E2E coverage in this milestone unless the Tauri skeleton generated by tooling already includes a lightweight pattern. E2E tests are more valuable once user workflows exist.

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

Add these scripts to `package.json`:

```json
{
  "scripts": {
    "dev": "vite",
    "tauri:dev": "tauri dev",
    "build": "vite build",
    "tauri:build": "tauri build",
    "typecheck": "tsc --noEmit",
    "lint": "eslint .",
    "format": "prettier --write .",
    "format:check": "prettier --check .",
    "test": "vitest run",
    "test:watch": "vitest"
  }
}
```

Rust commands:

```text
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Local setup command:

```text
npm install
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

Create `.github/workflows/foundation.yml`.

Recommended initial CI:

- Trigger on pull requests and pushes to the default branch.
- Run on `windows-latest` because early FileNym filesystem research and developer environment are Windows-oriented.
- Optionally add `ubuntu-latest` for frontend and Rust compile checks once Tauri system dependencies are confirmed.

Required jobs for this milestone:

1. Checkout repository.
2. Install stable Rust.
3. Install Node.js LTS.
4. Run `npm ci`.
5. Run `npm run format:check`.
6. Run `npm run lint`.
7. Run `npm run typecheck`.
8. Run `npm run test`.
9. Run `npm run build`.
10. Run `cargo fmt --all -- --check`.
11. Run `cargo clippy --workspace --all-targets -- -D warnings`.
12. Run `cargo test --workspace`.

Do not require a Tauri installer bundle in CI for this milestone unless package dependencies are proven stable and inexpensive.

## Files to Create or Change

Create:

- `plans/phase-1/01-project-foundation.md`
- `Cargo.toml`
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
3. Alias name normalization and cross-platform validation policy.
4. SQLite schema and migration contents.
5. Database location strategy.
6. Filesystem watcher choice.
7. Startup reconciliation algorithm.
8. Relink and replacement decision UX.
9. Whether to add a dedicated filesystem lifecycle test crate later.

## Known Repository Conflicts or Mismatches

- The user-requested path `docs/architecture/domain-model.md` does not currently exist. The repository contains `docs/architecture/domain-modal.md`, which appears to be the intended domain model document.
- The user-requested decision paths use four-digit names like `0001-local-first-desktop.md`, but the repository currently contains three-digit names: `001-local-first-desktop.md`, `002-aliases-are-metadata.md`, and `003-path-is-not-identity.md`.
- `README.md`, `VISION.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `SECURITY.md`, and `rfcs/0001-file-alias-mechanism.md` still describe FileNym as research-stage or as not having selected an implementation. `AGENTS.md`, accepted ADRs, and the Phase 1 scope document now establish Tauri 2, React, TypeScript, Rust, and SQLite as the Phase 1 baseline.
- `.github/ISSUE_TEMPLATE/research-contribution.md` has `title: 'Reseaerch: '`, which appears to be a typo. It is unrelated to this milestone.
