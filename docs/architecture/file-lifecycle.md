# FileNym Canonical File Lifecycle

## Purpose

FileNym must preserve the logical identity of a registered canonical file while recognizing that filesystem state may change.

## Registration

When a user registers a file:

1. Validate that the path refers to a supported existing file.
2. Gather available metadata.
3. Gather filesystem identity evidence using the platform adapter.
4. Create a stable FileNym internal ID.
5. Persist the canonical file record.
6. Record its current observed state.

Registration must not modify the file.

## Modification

If the canonical file is determined to be the same physical file but its content-related metadata changes:

1. Keep the canonical FileNym ID.
2. Keep all aliases.
3. Update observed metadata.
4. Record the latest observation.

## Rename

If FileNym can reliably establish that the canonical file has been renamed:

1. Keep the canonical FileNym ID.
2. Keep all aliases.
3. Update the current known path and filename.
4. Update observation metadata.

## Move

If FileNym can reliably establish that the same canonical file moved:

1. Keep the canonical FileNym ID.
2. Keep aliases.
3. Update current known location.

Move behavior must not be assumed to work identically across filesystem or volume boundaries.

## Unavailable or Deleted

If the canonical file cannot be found:

1. Do not delete the FileNym canonical record.
2. Do not delete aliases.
3. Mark the canonical file unavailable.
4. Preserve the last known location and identity evidence.
5. Allow future recovery/relinking.

FileNym should not claim permanent deletion when it only knows that a file is currently unavailable.

Examples include:

- deleted file
- disconnected external drive
- unavailable network path
- permission change

## Possible Replacement

If a file exists at the stored path but does not match sufficient identity evidence:

1. Do not silently accept it as the canonical file.
2. Preserve the existing FileNym record.
3. mark or report a possible replacement/identity mismatch.
4. require a safe reconciliation or explicit user decision.

## Relink

A relink operation allows a user to tell FileNym where an unavailable canonical file now exists.

Relinking must verify as much identity evidence as reasonably possible.

If identity cannot be established, FileNym should clearly distinguish between:

- locating the same file
- intentionally replacing the registered canonical file

Those operations must not be silently conflated.

## Real-Time Observation

A filesystem watcher may improve responsiveness while FileNym is running.

However, watcher events are signals rather than the source of domain truth.

Watcher behavior may be:

- duplicated
- coalesced
- delayed
- platform-specific
- missed while FileNym is not running

Therefore watcher events must feed reconciliation logic rather than directly defining canonical identity.

## Startup Reconciliation

When FileNym starts:

1. Load persisted canonical file records.
2. inspect each current known location.
3. compare current filesystem observations with stored identity evidence.
4. classify each canonical file conservatively.
5. update current observations where safe.
6. preserve unresolved states for user recovery.

## Safety Principle

When FileNym is uncertain whether an observed file is the registered canonical file, prefer an explicit unresolved state over silently attaching aliases to the wrong physical file.
