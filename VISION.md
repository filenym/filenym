# FileNym Vision

## Vision

FileNym aims to separate a file's underlying identity from the filename used to present it.

A single canonical file should be capable of having multiple contextual names without requiring the user to repeatedly rename, reorganize, or duplicate the original file.

**One file. Many names.**

## Why FileNym Exists

Filenames often serve two different purposes.

On a user's own computer, a filename may be descriptive and useful for organization.

When that same file is shared, uploaded, attached, submitted, or used by another application, a different filename may be more appropriate.

Today, users often solve this by renaming the original file or creating additional copies.

FileNym explores whether this can be handled as a separate file identity layer.

## Core Principles

### One Canonical File

The original file remains the source of truth.

### Multiple Contextual Names

A canonical file may have different names for different uses or contexts.

### Avoid Content Duplication

FileNym should avoid duplicating the underlying file contents when presenting a file under another name wherever technically possible.

### Preserve the Original

Using an alternative name should not rename, move, or modify the canonical file.

### Local First

Core functionality should work locally without depending on a remote service.

### Privacy by Design

File contents should not need to leave the user's device for FileNym's core functionality.

### No Mandatory Account

Core functionality should not require registration or a user account.

### Open Source

The architecture, research, implementation, and technical decisions should be developed openly with the community.

### Cross Platform Direction

FileNym should ultimately consider Windows, macOS, and Linux, while allowing early experiments to focus on one platform where necessary.

## What FileNym Is Not

FileNym is not intended to be a cloud storage service, document editor, file conversion service, backup system, or traditional document management platform.

Its central concern is simpler:

**How can one canonical file have multiple useful names depending on context?**

## Current Stage

FileNym is currently an early stage research project.

The technical mechanism has intentionally not been finalized.

Possible approaches may involve filesystem capabilities, operating system APIs, virtual filesystems, application level presentation, browser integration, or approaches not yet considered.

The project will evaluate these possibilities before committing to a long term architecture.

## Long Term Direction

If the core concept proves technically viable, FileNym may eventually support desktop applications, command line tools, browser workflows, operating system integrations, contextual naming rules, developer APIs, and other integrations.

FileNym may also explore optional intelligent capabilities where they provide meaningful value. These could include contextual filename suggestions, document aware naming assistance, destination aware recommendations, naming convention automation, and other AI assisted workflows.

Any intelligent capabilities should complement rather than replace FileNym's core functionality. The fundamental file identity system should remain useful without requiring AI, cloud services, user accounts, or paid infrastructure.

These are future possibilities, not current commitments.

The immediate priority is to validate the fundamental idea.

## Guiding Question

> How can one physical file be reliably presented under multiple contextual filenames while preserving the canonical file and avoiding unnecessary duplication of its contents?
