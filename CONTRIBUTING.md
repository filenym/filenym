# Contributing to FileNym

Thank you for your interest in contributing to FileNym.

FileNym is currently in an early research and architecture stage. The project is intentionally being developed in public so that important technical decisions can benefit from community knowledge, experimentation, and review.

## Current Priority

The immediate priority is validating FileNym's core concept:

**One file. Many names.**

We are investigating how one canonical file can be presented under multiple contextual filenames while preserving the original file and avoiding duplication of its contents wherever technically possible.

At this stage, research and small technical experiments are more valuable than building a large application.

## Ways to Contribute

Contributions are welcome in areas including:

* Technical research
* Architecture proposals
* Proof of concept implementations
* Filesystem experiments
* Windows compatibility testing
* macOS compatibility research
* Linux compatibility research
* Browser behavior testing
* Security analysis
* Documentation
* User experience ideas
* Reproducible technical findings

## Before Writing Major Code

Please do not begin a large implementation or introduce a major architectural dependency without discussing it first.

For significant architectural proposals:

1. Read the project README and Vision.
2. Read the current RFCs.
3. Join the relevant GitHub Discussion.
4. Explain the proposed approach and its tradeoffs.
5. Where useful, provide a small proof of concept or reproducible experiment.

This helps prevent contributors from spending significant time on an approach that has not yet been evaluated by the project.

## Research Contributions

Research findings should be reproducible whenever possible.

Useful research contributions may include:

* Operating system documentation
* Filesystem behavior
* Browser behavior
* Small test programs
* Compatibility results
* Performance observations
* Security implications
* Known limitations
* Comparisons between possible approaches

Please clearly state the operating system, filesystem, browser, application, or environment used for testing.

## Issues and Discussions

Use **GitHub Discussions** for:

* Architecture questions
* Open ended ideas
* Design proposals
* Technical debates
* Questions about project direction

Use **GitHub Issues** for:

* Specific research tasks
* Confirmed bugs
* Documentation work
* Reproducible experiments
* Clearly scoped implementation tasks

## Pull Requests

Keep pull requests focused on one clear purpose whenever possible.

A good pull request should explain:

* What problem it addresses
* What was changed
* Why the approach was chosen
* How the change was tested
* Any known limitations

Major architectural changes should reference the relevant Discussion, Issue, or RFC.

## Design Principles

Contributions should respect FileNym's current core principles:

* One canonical file remains the source of truth.
* Contextual naming should not unnecessarily modify the original file.
* File content duplication should be avoided wherever technically possible.
* Core functionality should remain local first.
* A mandatory cloud service should not be required.
* A mandatory user account should not be required.
* Privacy and security should be considered from the beginning.
* The architecture should remain open to cross platform support.

## Scope

FileNym is deliberately small at this stage.

Features such as cloud synchronization, accounts, team collaboration, AI assistance, advanced automation, and extensive user interfaces may be explored later.

Please keep current contributions focused primarily on validating and implementing the fundamental file identity concept.

## Respectful Collaboration

FileNym welcomes contributors with different levels of experience and different technical perspectives.

Disagreement about architecture is expected and can be valuable.

Please focus technical discussions on evidence, tradeoffs, reproducible results, and the quality of the resulting software.

## Getting Started

A good first contribution is to read:

* `README.md`
* `VISION.md`
* `docs/problem-statement.md`
* `rfcs/0001-file-alias-mechanism.md`

Then join the project's main architecture Discussion and share any relevant technical experience, research, or proposed experiments.

Thank you for helping explore FileNym.
