# FileNym Phase 1 Domain Model

## CanonicalFile

A `CanonicalFile` represents the physical file that the user registered with FileNym.

It is a FileNym domain entity.

It must not be identified solely by its path.

Conceptual fields:

- FileNym internal ID
- current known path
- current filename
- filesystem identity information where available
- size
- modification metadata
- registration timestamp
- last observation timestamp
- status

The exact filesystem identity representation is intentionally not finalized until platform experiments establish what is reliable.

## Alias

An `Alias` is a logical contextual name associated with a `CanonicalFile`.

An alias does not represent a second physical file during Phase 1.

Conceptual fields:

- FileNym alias ID
- canonical file ID
- alias name
- creation timestamp
- update timestamp

## AliasName

`AliasName` should be represented as a validated domain value rather than an arbitrary unchecked string.

It should enforce FileNym's naming rules.

Examples of invalid aliases may include:

- empty names
- path traversal
- path separators where only a filename is expected
- invalid platform filename forms

The exact cross-platform validation policy should be documented before implementation.

## FileIdentity

`FileIdentity` represents filesystem-level evidence that may help FileNym determine whether an observed file is the same physical file previously registered.

It is distinct from:

- filename
- path
- FileNym database ID

The representation may differ by operating system.

Phase 1 begins with Windows research but the domain must not expose Windows-specific identity structures directly.

## FileStatus

The domain must represent canonical file state explicitly.

Initial conceptual statuses:

- Available
- Modified
- Unavailable
- PossibleReplacement

Rename and move may be represented as lifecycle events rather than permanent statuses if implementation research shows that is cleaner.

The distinction should be decided during the file lifecycle implementation plan.

## Relationships

A canonical file may have zero or more aliases.

An alias belongs to exactly one canonical file.

Conceptually:

CanonicalFile
|
+-- Alias
+-- Alias
+-- Alias

## Domain Invariants

1. An alias cannot exist without a canonical file record.
2. Alias creation does not modify the canonical physical file.
3. Alias deletion does not delete the canonical physical file.
4. Alias rename does not rename the canonical physical file.
5. Duplicate aliases for the same canonical file should not be allowed according to the defined normalization policy.
6. A canonical file path is mutable.
7. A FileNym canonical file ID is stable.
8. Same path does not necessarily mean same canonical file.
9. Loss of the canonical physical file does not automatically delete aliases.
10. Persistence implementation details must not leak into domain entities.

## Open Questions Requiring Experimentation

The following are intentionally unresolved:

1. Which Windows filesystem identity information should FileNym persist?
2. Which identity properties survive rename?
3. Which survive moves within a volume?
4. What happens across volumes?
5. How should identity behave for network or removable storage?
6. Which metadata is sufficiently reliable to detect possible replacement?
7. How should unsupported filesystems degrade gracefully?

These questions must be answered through controlled experiments rather than assumptions.
