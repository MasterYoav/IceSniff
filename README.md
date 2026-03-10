# IceSniff

IceSniff is a modern, open-source network packet analyzer built for clarity, speed, and cross-platform usability.

The project is designed to provide a cleaner and more approachable experience than traditional packet analysis tools, without sacrificing technical depth. IceSniff will offer both a desktop application and a command-line interface, with the same core capabilities and the same underlying engine.

## Vision

IceSniff exists to make packet analysis easier to use, easier to explain, and easier to extend.

Many existing tools are extremely powerful, but they can feel visually outdated, difficult to approach, or fragmented across different interfaces. IceSniff is intended to close that gap by combining:

- a modern desktop experience
- a powerful CLI
- a shared Rust core
- strong documentation
- a fully open-source development model

## Core Principles

### One engine, two interfaces
IceSniff is built around a single shared core written in Rust.

That shared core powers:
- the desktop application
- the CLI

This means feature parity is a fundamental project requirement. The CLI is not an afterthought, and the desktop app is not a separate implementation. Both are different interfaces to the same engine.

### Documentation-first development
IceSniff is intended to be easy to understand for users, contributors, and maintainers.

Documentation is treated as part of the product, not as optional cleanup work. The codebase should be readable, modular, and easy to explain. Public APIs, architectural boundaries, protocol support, and contributor workflows should all be clearly documented.

### Built for humans and tools
The project should be easy to work on using modern AI-assisted development tools such as Claude Code and Codex.

To support that, the repository will prioritize:
- clear module boundaries
- predictable naming
- small focused crates
- strong inline documentation
- explicit architecture documents
- examples and task-oriented guides

## Technology Stack

IceSniff is planned with the following stack:

- **Rust** for the shared core, parsing, capture, filtering, and CLI
- **Tauri 2** for the desktop shell
- **Svelte** for the desktop UI

### Why this stack

**Rust** provides performance, memory safety, and strong cross-platform support. It is a natural fit for packet capture, parsing, protocol decoding, and a portable CLI.

**Tauri 2** provides a lightweight and modern desktop shell that integrates well with Rust and supports desktop builds on macOS, Windows, and Linux.

**Svelte** provides a clean and maintainable way to build the desktop interface without unnecessary frontend complexity.

## Cross-Platform Goals

IceSniff is intended to support:

- **macOS**
- **Windows**
- **Linux**

This applies to both the desktop app and the CLI.

The desktop application should be buildable and usable across all major platforms supported by Tauri.

The CLI should work naturally across:
- Linux shells
- macOS shells
- Windows PowerShell
- Windows Command Prompt

Platform-specific capture requirements may differ depending on the operating system, but cross-platform support is a core project goal from the start.

## What IceSniff Will Be

IceSniff is planned as:

- a desktop packet analysis application
- a CLI packet analysis tool
- a local-first tool
- a fully open-source project
- a documented and contributor-friendly codebase

## What IceSniff Will Not Be

IceSniff is not intended to become:

- a subscription service
- an open-core product with locked features
- a cloud-first monitoring platform
- an enterprise observability suite
- a rushed clone that tries to match every Wireshark feature immediately

The goal is to build a focused, high-quality foundation first, then expand carefully.

## Initial Scope

The first milestones are expected to focus on a strong core foundation, including capabilities such as:

- opening capture files
- saving capture files
- live capture on selected interfaces
- packet listing
- packet inspection
- raw byte and hex views
- protocol filtering
- basic protocol support
- shared services used by both the desktop app and the CLI

## Architecture Direction

The project will follow a layered design:

- shared Rust crates for capture, parsing, filtering, analysis, and services
- a thin desktop shell on top of the shared services
- a thin CLI shell on top of the same shared services

This structure is meant to keep behavior consistent, reduce duplication, improve testing, and make the codebase easier to maintain.

## Open Source Direction

IceSniff is being built as a public open-source project.

The long-term goal is to put the project on GitHub in a form that is clean, understandable, well-documented, and inviting for contributors. The project should be approachable for developers who want to inspect the codebase, contribute improvements, add protocols, improve UX, or help with cross-platform support.

## Development Standard

The codebase should remain:

- readable
- modular
- well documented
- easy to test
- easy to explain
- easy to extend

Any design or implementation choice that makes the project harder to understand without a strong technical reason should be avoided.

## Status

IceSniff has entered the initial implementation phase.

The repository now includes:

- a Rust workspace
- a CLI application scaffold
- shared crates for service, file IO, formatting, and session models
- continuity documentation for future sessions

The first implemented vertical slices are:

- `inspect <capture-file>` for shared capture metadata inspection
- `list <capture-file> [limit] [--filter <expr>]` for shared PCAP and PCAPNG packet enumeration
- `show-packet <capture-file> <packet-index>` for shared PCAP and PCAPNG packet detail inspection
- `stats <capture-file> [--filter <expr>]` for shared capture and protocol summary statistics
- `conversations <capture-file> [--filter <expr>]` for shared bidirectional flow summaries
- `--json` output mode for machine-readable CLI automation

Packet detail now includes:

- decoded layer summaries
- raw bytes
- a basic field tree for Ethernet, ARP, IPv4, TCP, UDP, and ICMP
- byte ranges for field-tree nodes so decoded fields can be traced back to raw bytes
- application metadata for DNS, HTTP/1.1, and TLS handshake packets

Packet listing now includes analyst-oriented derived columns:

- source
- destination
- protocol
- info

Current shared filter expressions include:

- `protocol=dns|http|tls|tcp|udp|icmp|ipv4|arp`
- `port=<number>`
- `host=<name-or-address>`

## Current CLI

```bash
cargo run -p icesniff-cli -- help
```

Current commands:

- `inspect <capture-file>`
- `list <capture-file> [limit] [--filter <expr>]`
- `show-packet <capture-file> <packet-index>`
- `stats <capture-file> [--filter <expr>]`
- `conversations <capture-file> [--filter <expr>]`

All commands support `--json`.

## Repository Guide

Key docs:

- `instructions.md`
- `docs/repo-map.md`
- `docs/architecture/overview.md`
- `docs/feature-parity-matrix.md`
- `docs/task-recipes.md`
- `docs/continuity-log.md`
