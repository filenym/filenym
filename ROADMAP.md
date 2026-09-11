# FileNym Roadmap

FileNym is currently in an early research and validation stage.

The roadmap intentionally begins with the core technical problem before committing to a desktop framework, user interface, browser extension, or larger product architecture.

**One file. Many names.**

## v0.0.1: Core Mechanism Research

### Goal

Determine how one canonical file can be reliably presented under another filename without modifying the original file and without duplicating its contents wherever technically possible.

### Research Areas

* Hard links
* Symbolic links
* Application level file presentation
* Virtual filesystem approaches
* Operating system specific APIs
* Browser upload behavior
* Security and permission implications

### Initial Platform

Early experiments may focus on Windows for practical testing.

The research should document implications for macOS and Linux wherever possible.

### Deliverable

A documented comparison of the leading approaches supported by reproducible technical experiments.

The project should identify which mechanism, or combination of mechanisms, is suitable for the first implementation.

## v0.0.2: Minimal Proof of Concept

### Goal

Implement the selected mechanism in the smallest practical form.

This may initially be a command line tool or another minimal technical prototype.

The proof of concept should demonstrate:

1. Selection of one canonical file.
2. Assignment of at least one alternative filename.
3. Use or presentation of that file under the alternative filename.
4. Preservation of the original file.
5. Verification of whether underlying file contents were duplicated.

The objective is technical validation, not user interface polish.

## v0.1.0: Minimal Desktop Experience

After the core mechanism has been validated, FileNym can introduce a simple desktop interface.

The first interface should remain intentionally small.

A user should be able to:

1. Select an existing file.
2. Associate multiple contextual names with it.
3. Select a contextual name.
4. Use the file under that name.
5. Keep the canonical file unchanged.

The implementation stack should be selected after the core mechanism is sufficiently understood.

## Future Exploration

Once the fundamental FileNym workflow is reliable, future versions may explore:

* Improved desktop workflows
* Contextual naming rules
* Operating system integrations
* Drag and drop workflows
* Command line tooling
* Browser integration
* Browser extensions
* Email and attachment workflows
* Developer APIs
* Plugin architecture
* Cross platform support
* Optional intelligent filename suggestions
* AI assisted contextual naming

These items are directions for exploration rather than commitments.

## AI and Intelligent Features

AI may eventually provide meaningful assistance with contextual naming.

Potential examples include:

* Suggesting professional filenames based on document context
* Recommending names appropriate for a destination or workflow
* Understanding document purpose
* Applying user preferred naming conventions
* Identifying filenames that may expose unnecessary information

AI should remain an optional enhancement.

FileNym's fundamental file identity functionality should remain useful without requiring AI, cloud infrastructure, an internet connection, or a user account.

## What We Are Not Building Yet

The following are intentionally outside the immediate roadmap:

* Cloud synchronization
* User accounts
* Team management
* Enterprise administration
* Large desktop dashboards
* Mandatory hosted infrastructure
* Complex automation systems
* AI as a dependency for core functionality

## How Roadmap Decisions Are Made

FileNym is developed openly.

Significant technical decisions should be informed by:

* Reproducible experiments
* Technical research
* GitHub Discussions
* RFCs
* Contributor feedback
* Security considerations
* Real world usability

The roadmap may evolve as research reveals better approaches.

## Current Priority

**v0.0.1: Core Mechanism Research**

Before building the larger application, we need to answer the project's fundamental engineering question:

> How can one physical file be reliably presented under multiple contextual filenames while preserving the canonical file and avoiding unnecessary duplication of its contents?
