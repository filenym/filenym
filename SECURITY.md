# Security Policy

Security is an important consideration for FileNym because the project may interact with filesystems, file paths, operating system APIs, browsers, symbolic links, hard links, virtual filesystems, and other application boundaries.

FileNym is currently in an early research and architecture stage. Security considerations should influence technical decisions from the beginning.

## Reporting a Security Vulnerability

Please do not publicly disclose a suspected security vulnerability through a GitHub Issue, Discussion, Pull Request, or other public channel before maintainers have had an opportunity to investigate it.

If GitHub Private Vulnerability Reporting is enabled for this repository, please use it to report security vulnerabilities privately.

If private vulnerability reporting is not yet available, please avoid publishing exploit details publicly and contact the project maintainers through an available private contact method.

## What to Include

When reporting a vulnerability, please provide as much relevant information as possible, including:

* A description of the vulnerability
* The affected FileNym component or mechanism
* Steps to reproduce the issue
* Operating system and version
* Filesystem, where relevant
* Browser or application involved, where relevant
* Required permissions or privileges
* Potential security impact
* Proof of concept information, if appropriate
* Any suggested mitigation

Please avoid including sensitive personal information or unrelated confidential data.

## Areas of Particular Interest

Security research related to FileNym may include:

* Symbolic link attacks
* Hard link behavior
* Path traversal
* Path manipulation
* Race conditions
* Canonical file replacement
* Permission boundary violations
* Privilege escalation
* Malicious filenames
* Filename extension mismatches
* Executable file handling
* Virtual filesystem security
* Browser integration boundaries
* Native application communication
* Untrusted files
* Network paths
* Removable storage
* Metadata exposure

This list is not exhaustive.

## Supported Versions

FileNym has not yet reached a stable release.

During the current research and experimental stage, security findings should generally relate to the latest code and experiments available in the repository.

A formal supported versions policy will be introduced when stable releases exist.

## Security by Design

FileNym aims to follow several security principles:

* Request the minimum privileges necessary.
* Avoid requiring administrator access where possible.
* Preserve the integrity of canonical files.
* Treat filenames, paths, aliases, and external input as untrusted.
* Avoid silently changing file types or extensions.
* Respect operating system permission boundaries.
* Minimize unnecessary access to file contents.
* Keep core functionality local first.
* Avoid unnecessary transmission of file contents to remote services.
* Evaluate security implications before adopting new filesystem or integration mechanisms.

## Experimental Software

FileNym is currently experimental.

Research code and proof of concept implementations should not be assumed to be production ready or security hardened.

Users and contributors should exercise appropriate caution when testing experimental functionality, particularly with important or sensitive files.

## Responsible Disclosure

We appreciate responsible security research and good faith vulnerability reporting.

The project will aim to investigate credible reports carefully and address confirmed security issues transparently while avoiding unnecessary exposure of users before appropriate mitigations are available.
