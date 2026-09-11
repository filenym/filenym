# FileNym Problem Statement

## The Problem

A filename often serves more than one purpose.

On a user's own computer, a filename may be detailed and descriptive because it helps with organization, searching, version identification, or understanding the file later.

When that same file is uploaded, attached, submitted, shared, or opened through another workflow, the internal filename may no longer be appropriate.

For example, a user may keep:

`project-proposal-client-work-final.pdf`

but want the same file to be presented in different situations as:

`Project-Proposal.pdf`

`Client-Proposal.pdf`

`Software-Development-Proposal.pdf`

The file contents have not changed. Only the appropriate name has changed.

## How Users Handle This Today

Users commonly solve this problem by:

1. Renaming the original file.
2. Uploading or sharing it.
3. Renaming it back afterward.

Alternatively, they create additional copies with different filenames.

Both approaches create unnecessary friction.

Repeated renaming can disrupt personal organization, while duplicate files can create confusion about which copy is current or authoritative.

## The FileNym Idea

FileNym proposes separating the canonical file from the name used to present that file in a particular context.

Conceptually:

    Canonical File
          |
          +-- Project-Proposal.pdf
          |
          +-- Client-Proposal.pdf
          |
          +-- Software-Development-Proposal.pdf

The aliases represent different contextual identities of the same underlying file.

The canonical file remains the source of truth.

## Core Requirement

FileNym should make it possible for one canonical file to have multiple contextual filenames without requiring the user to repeatedly rename the original file.

Where technically possible, this should also be achieved without duplicating the underlying file contents.

Using a contextual name should not modify, move, or rename the canonical file.

## The Technical Challenge

The concept is simple from the user's perspective, but the implementation may not be.

Different operating systems, filesystems, browsers, upload interfaces, desktop applications, security models, and sandboxing rules may treat filenames and file references differently.

The central technical question is:

> How can one physical file be reliably presented to another application under a different filename while preserving the canonical file and avoiding unnecessary duplication of its contents?

FileNym has intentionally not selected an implementation mechanism yet.

Potential areas of investigation include:

* Hard links
* Symbolic links
* Virtual filesystems
* Filesystem overlays
* Operating system specific APIs
* Application level file presentation
* Browser APIs and browser integration
* Native application integration
* Other mechanisms proposed through research and community discussion

## Success Criteria

A successful FileNym architecture should aim for the following:

* The canonical file remains unchanged.
* Multiple contextual filenames can refer to the same underlying content.
* File content duplication is avoided wherever technically possible.
* The workflow is simple for ordinary users.
* Security and file permissions are respected.
* The design does not require a mandatory cloud service.
* Core functionality can operate locally.
* The architecture can evolve toward multiple operating systems.
* External applications receive the expected file content and contextual filename reliably.

## Current Scope

FileNym is currently a research and architecture project.

The immediate objective is not to build a feature rich desktop application.

The first objective is to determine which technical mechanism, or combination of mechanisms, can reliably support the FileNym concept.

Once that question has been investigated and validated through practical experiments, the project can move toward implementation.

## Broader Opportunity

Although a simple example may involve uploading a document under a cleaner filename, the underlying problem is broader.

The same concept may apply to professional documents, legal filings, proposals, reports, portfolios, media files, educational submissions, business documents, software artifacts, and many other workflows.

FileNym therefore treats contextual naming as a general file identity problem rather than a feature for one particular type of document.
