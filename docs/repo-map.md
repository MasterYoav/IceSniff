# Repository Map

This document explains the current repository layout and the intended responsibility of each top-level area.

## Top Level

- `apps/cli`: first-class command-line interface on top of shared Rust services
- `apps/desktop`: placeholder for the future Tauri 2 + Svelte shell
- `crates/app-services`: shared use-case layer consumed by CLI and later the desktop app
- `crates/filter-engine`: shared packet-filter semantics used by parser and future interfaces
- `crates/file-io`: capture container loading for PCAP and PCAPNG into shared raw packet models
- `crates/output-formatters`: CLI-facing renderers for shared model data
- `crates/parser-core`: shared report assembly, packet inspection orchestration, and capture stats
- `crates/protocol-dissectors`: protocol and link-layer decoding logic
- `crates/session-model`: stable domain model shared across interfaces
- `docs/`: architecture, process, roadmap, and continuity documentation
- `examples/`: future runnable examples
- `fixtures/`: future sample packet captures and protocol fixtures
- `scripts/`: future helper scripts for contributors and automation
- `assets/`: reserved for project assets beyond branding media
- `media/`: existing project branding files

## Current Vertical Slice

The first implemented slice is intentionally narrow:

1. `apps/cli` accepts `inspect <capture-file>`.
2. `apps/cli` accepts `list <capture-file> [limit]`.
3. `apps/cli` accepts `show-packet <capture-file> <packet-index>`.
4. `apps/cli` accepts `stats <capture-file>` and supports `--json`.
5. `apps/cli` accepts shared `--filter <expr>` semantics for `list`, `stats`, and `conversations`.
6. `crates/app-services` exposes the use cases as shared services.
7. `crates/file-io` loads capture files into shared packet records.
8. `crates/filter-engine` applies shared packet-filter semantics.
9. `crates/parser-core` turns loaded packets into list/detail/stats/conversation reports.
10. `crates/protocol-dissectors` performs minimal Ethernet/ARP/IPv4/TCP/UDP/ICMP decoding plus early DNS/HTTP/TLS inspection.
11. `crates/session-model` carries the report structures.
12. `crates/output-formatters` renders text and JSON CLI output.

This keeps business logic out of the CLI binary and establishes the layering needed for future desktop reuse.

## Planned Expansion

The following crates from the architecture plan still need to be added:

- `capture-engine`
- `analysis-core`

`parser-core`, `protocol-dissectors`, and `filter-engine` now exist. The remaining crates are intentionally deferred until the next implementation step so the current slice stays testable.
