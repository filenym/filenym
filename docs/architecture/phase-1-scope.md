# FileNym Phase 1: File Identity & Alias Registry

## Objective

Phase 1 establishes FileNym's local domain foundation.

The objective is to prove that FileNym can maintain a durable logical relationship between one canonical physical file and multiple contextual names while safely understanding changes to the canonical file.

Phase 1 does not attempt to present those aliases to external applications.

## Product Statement

A user can register an existing file with FileNym.

Example canonical file:

`D:\Documents\Resume.pdf`

The user may then define logical aliases such as:

- `CureMD-Resume.pdf`
- `Senior-Software-Engineer.pdf`
- `Full-Stack-Engineer.pdf`

Only the canonical physical file exists on disk.

The aliases are FileNym metadata stored locally.

## User Capabilities

During Phase 1 a user must eventually be able to:

1. Register an existing file.
2. See registered canonical files.
3. View a canonical file's current known location and status.
4. Create an alias.
5. View aliases.
6. Rename an alias.
7. Delete an alias.
8. Observe when the canonical file has changed.
9. Continue using FileNym metadata after a canonical file rename where identity can be established reliably.
10. Continue using FileNym metadata after a canonical file move where identity can be established reliably.
11. See that a canonical file is unavailable when it has been deleted or cannot be found.
12. Recover/relink an unavailable canonical file.
13. Be protected from FileNym silently attaching aliases to an unrelated replacement file.

## Core Invariant

Creating or managing an alias must not modify the canonical physical file.

An alias is metadata.

## Canonical File States

The domain should support at least:

### Available

The expected canonical file is available and its identity is consistent with FileNym's stored information.

### Modified

The canonical file appears to be the same file but its content-related metadata changed.

Modification does not invalidate aliases.

### Renamed

The same canonical file has a different filename.

FileNym should update its current known path/name when this can be established reliably.

### Moved

The same canonical file has a different location.

FileNym should update its current known path when this can be established reliably.

### Unavailable

The canonical file cannot currently be located or accessed.

Aliases and FileNym metadata remain intact.

### Possible Replacement

A file exists at the stored path, but evidence indicates that it may not be the canonical file originally registered.

FileNym must not silently accept the replacement.

## Application Restart

Real-time filesystem monitoring cannot guarantee complete history because FileNym may not be running when a change occurs.

Therefore FileNym must reconcile persisted state against current filesystem state when the application starts.

## Non-Goals

Phase 1 does not include:

- presenting aliases to external applications
- uploading aliases through browsers
- hard links
- symbolic links
- virtual filesystems
- WinFsp
- FUSE
- browser extensions
- Native Messaging
- shell extensions
- cloud synchronization
- accounts
- AI naming

## Completion Criteria

Phase 1 is complete when:

1. Canonical files and aliases persist across application restarts.
2. Alias operations never modify the canonical file.
3. Canonical file lifecycle states are represented explicitly.
4. Supported rename/move behavior is demonstrated through tests.
5. Deleted/unavailable canonical files do not cause alias metadata loss.
6. replacement scenarios are handled conservatively.
7. startup reconciliation is implemented.
8. database migrations are reproducible.
9. core behavior is covered by automated tests.
10. filesystem assumptions relied upon by FileNym are documented and experimentally validated.
