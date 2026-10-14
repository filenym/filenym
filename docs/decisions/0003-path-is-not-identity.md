# ADR 0003: Path Is Not Canonical File Identity

## Status

Accepted

## Decision

FileNym must not treat a filesystem path as the identity of a canonical file.

A canonical file receives a stable FileNym internal identity.

Filesystem identity evidence and current path are stored separately.

## Rationale

Files can be renamed or moved.

A different file can later appear at the same path.

Therefore:

same canonical file != permanently same path

and:

same path != necessarily same canonical file

## Consequences

FileNym requires a platform abstraction for filesystem identity and reconciliation.

The exact Windows identity mechanism will be selected only after controlled experiments.
