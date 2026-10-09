# ADR 0002: Phase 1 Aliases Are Metadata

## Status

Accepted

## Decision

During Phase 1, a FileNym alias is a logical metadata record associated with a canonical physical file.

Creating an alias does not create another physical file.

## Rationale

The concerns:

1. what contextual names belong to a file
2. how an external application eventually receives a selected contextual name

are separate problems.

Phase 1 solves the first problem independently.

## Consequences

Alias management requires no hard links, symbolic links, virtual filesystem, browser extension, or copying mechanism.

Future external presentation mechanisms may consume the alias model without changing its meaning.
