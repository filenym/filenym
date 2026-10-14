# ADR 0001: Local-First Desktop Architecture

## Status

Accepted

## Decision

FileNym will begin as a local-first desktop application using:

- Tauri 2
- React
- TypeScript
- Rust
- SQLite

Phase 1 requires no application server, remote database, user account, or cloud service.

## Rationale

FileNym's core problem concerns local files and operating-system behavior.

Keeping the initial architecture local:

- minimizes infrastructure
- preserves privacy
- avoids unnecessary network dependencies
- enables direct native filesystem integration
- keeps the project usable without an account or internet connection

## Consequences

Native platform behavior must be handled carefully.

Cross-platform abstractions should prevent Windows-specific implementation details from becoming core domain concepts.
