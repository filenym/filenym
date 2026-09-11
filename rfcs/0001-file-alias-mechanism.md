# RFC 0001: Contextual File Alias Mechanism

## Status

Open for discussion

## Summary

FileNym aims to allow one canonical file to be presented under multiple contextual filenames without repeatedly renaming the original file.

Where technically possible, FileNym should achieve this without duplicating the underlying file contents.

This RFC exists to investigate and compare the technical mechanisms that could make this possible.

No implementation approach has been selected yet.

## Example

Suppose a user has one canonical file:

`C:\Documents\project-proposal-client-work-final.pdf`

Depending on the context, the user may want the same file to be presented as:

`Project-Proposal.pdf`

`Client-Proposal.pdf`

`Software-Development-Proposal.pdf`

The underlying document remains the same.

Only the filename presented to another application or workflow changes.

## Core Requirement

A FileNym implementation should aim to satisfy the following:

1. The canonical file remains unchanged.
2. The canonical file remains the source of truth.
3. A user can associate multiple contextual filenames with that file.
4. Using an alias should not require manually renaming the canonical file.
5. File contents should not be duplicated where technically avoidable.
6. External applications should receive the correct file contents.
7. External applications should see the intended contextual filename.
8. Core functionality should not require a remote server or cloud service.

## Key Technical Question

> How can one physical file be reliably presented to another application under a different filename while preserving the canonical file and avoiding unnecessary duplication of its contents?

This question should be answered through research and practical experiments rather than assumptions.

## Candidate Approaches

The following approaches are starting points for investigation. This list is intentionally not exhaustive.

### Hard Links

Investigate whether multiple filesystem names can reference the same underlying file data.

Questions include:

* Which filesystems support the required behavior?
* Must the aliases exist on the same volume?
* How do browsers and desktop applications treat hard linked files?
* What happens when the canonical file is moved, renamed, modified, or deleted?
* Can this provide a reliable user experience across platforms?

### Symbolic Links

Investigate whether symbolic links can provide contextual filenames while referencing the canonical file.

Questions include:

* How consistently do applications follow symbolic links?
* How do browser upload dialogs behave?
* What permissions are required?
* What happens when the target is moved?
* Are there security or usability concerns?

### Virtual Filesystem

Investigate whether FileNym could expose a virtual view containing contextual filenames while reading content from canonical files stored elsewhere.

Conceptually:

    FileNym
        |
        +-- Project-Proposal.pdf
        +-- Client-Proposal.pdf
        +-- Software-Development-Proposal.pdf

All three entries could potentially reference the same canonical content.

Questions include:

* What operating system APIs would be required?
* Would elevated privileges or drivers be necessary?
* Can this be implemented safely?
* What is the cross platform complexity?
* How would browsers and ordinary applications interact with it?

### Filesystem Overlays

Investigate whether an overlay or similar filesystem abstraction could provide contextual names without modifying the canonical file.

The project should determine whether this offers meaningful advantages over a virtual filesystem or other mechanisms.

### Operating System APIs

Investigate platform specific capabilities that may provide a cleaner solution.

Initial research may focus on Windows because it is practical for early experimentation.

Any Windows specific approach should document whether equivalent mechanisms exist on macOS and Linux.

### Application Level Presentation

Investigate whether FileNym can provide the original file contents to another application while supplying a different filename at the application boundary.

This may avoid representing the alias as a traditional physical filesystem entry.

Questions include:

* Which application APIs permit this?
* Can this work with drag and drop?
* Can it work with file selection dialogs?
* Can it work with email clients?
* Can it work with browsers?
* Does it require application specific integrations?

### Browser Integration

Investigate what browsers permit when selecting, uploading, or programmatically providing local files.

Potential areas include:

* Browser extension APIs
* Web file APIs
* Drag and drop behavior
* Native application integration
* Native Messaging
* Browser security restrictions

Browser integration should be considered separately from the fundamental filesystem mechanism because browser security models may introduce additional constraints.

### Other Approaches

Contributors are encouraged to propose mechanisms not listed in this RFC.

The project should not commit to one approach until the alternatives have been investigated sufficiently.

## Evaluation Criteria

Each proposed mechanism should be evaluated against the same criteria.

### Content Duplication

Does the approach create another physical copy of the file contents?

If temporary duplication is unavoidable for a particular workflow, why is it necessary?

### File Integrity

Can the canonical file remain unchanged?

### Reliability

Will common applications consistently receive the expected content and filename?

### Windows Support

How well does the approach work on modern Windows systems and commonly used filesystems?

### macOS Support

Can the approach be implemented or adapted for macOS?

### Linux Support

Can the approach be implemented or adapted for Linux?

### Browser Compatibility

How does the mechanism behave with major browsers?

### Security

What new security risks could the mechanism introduce?

### Permissions

Does the mechanism require administrator privileges, elevated permissions, special filesystem settings, or drivers?

### Performance

Does the mechanism introduce meaningful latency, memory usage, disk activity, or other overhead?

### Portability

How dependent is the implementation on one operating system or filesystem?

### User Experience

Can ordinary users understand and use the mechanism without needing filesystem knowledge?

### Implementation Complexity

How difficult will the approach be to implement, test, maintain, and secure?

## Important Distinction

FileNym should distinguish between:

**The product requirement**

One canonical file can have multiple contextual names.

and:

**The implementation mechanism**

How the operating system, filesystem, browser, or application makes that possible.

The product requirement should remain stable while the implementation mechanism is researched openly.

## Initial Research Direction

The first experiments should focus on determining what is technically possible on Windows without building a complete desktop application.

Small proof of concept programs are preferred over large implementations.

Initial experiments may examine:

1. Hard link behavior.
2. Symbolic link behavior.
3. Browser upload behavior.
4. Application level filename presentation.
5. Virtual filesystem feasibility.

Results should be documented so that architectural decisions can be based on reproducible evidence.

## Non Goals for This RFC

This RFC does not attempt to decide:

* The final desktop user interface.
* The final frontend framework.
* Cloud synchronization.
* User accounts.
* AI functionality.
* Team collaboration.
* Monetization.
* Advanced naming rules.

Those questions can be addressed later.

The immediate issue is the core file identity mechanism.

## Open Questions

1. Can FileNym satisfy its core requirement without duplicating file contents at all?

2. Is one mechanism sufficient across operating systems and applications, or will FileNym require multiple platform specific strategies?

3. Should aliases exist as filesystem objects, or should they exist only at the application presentation layer?

4. How should FileNym behave when the canonical file is moved, renamed, modified, or deleted?

5. What mechanism provides the best balance between reliability, security, portability, and simplicity?

6. How should browsers and sandboxed applications interact with FileNym?

7. Are there existing filesystem technologies or operating system capabilities that already provide an appropriate primitive?

## Decision

No decision has been made.

This RFC remains open for research, experimentation, technical proposals, and community discussion.

A final architectural decision should be made only after the leading approaches have been tested and their tradeoffs documented.
