# Repository Map

This document explains the current repository layout and the intended responsibility of each top-level area.

## Top Level

- `apps/cli`: first-class command-line interface on top of shared Rust services
- `apps/desktop`: placeholder for the future Tauri 2 + Svelte shell
- `crates/app-services`: shared use-case layer consumed by CLI and later the desktop app, including shared live-capture orchestration
- `crates/capture-engine`: shared live-capture interface/session orchestration and provider-facing process control
- `crates/filter-engine`: shared packet-filter semantics used by parser and future interfaces
- `crates/file-io`: capture container loading for PCAP and PCAPNG into shared raw packet models
- `crates/output-formatters`: CLI-facing renderers for shared model data
- `crates/parser-core`: shared report assembly, packet inspection orchestration, and capture stats
- `crates/protocol-dissectors`: protocol and link-layer decoding logic
- `crates/session-model`: stable domain model shared across interfaces
- `docs/`: architecture, process, roadmap, and continuity documentation
- `examples/`: future runnable examples
- `fixtures/`: committed golden capture fixtures used by shared parser and file-io tests (PCAP, PCAPNG, malformed)
- `scripts/`: future helper scripts for contributors and automation
- `assets/`: reserved for project assets beyond branding media
- `media/`: existing project branding files

## Current Vertical Slice

The first implemented slice is intentionally narrow:

1. `apps/cli` accepts `inspect <capture-file>`.
2. `apps/cli` accepts `save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`.
3. `apps/cli` accepts `list <capture-file> [limit]`.
4. `apps/cli` accepts `show-packet <capture-file> <packet-index>`.
5. `apps/cli` accepts `stats <capture-file>` and supports `--json`.
6. `apps/cli` accepts shared `--filter <expr>` semantics for `list`, `stats`, `conversations`, and `streams`, and `--stream-filter <expr>` semantics for `streams` and `save`.
7. `crates/app-services` exposes the use cases as shared services.
8. `crates/file-io` loads capture files into shared packet records and writes PCAP output for save/export workflows.
9. `crates/filter-engine` applies shared packet-filter semantics.
10. `crates/parser-core` turns loaded packets into list/detail/stats/conversation/stream reports and stream packet-index selection.
11. `crates/protocol-dissectors` performs minimal Ethernet/ARP/IPv4/TCP/UDP/ICMP decoding plus early DNS/HTTP/TLS inspection.
12. `crates/session-model` carries the report structures.
13. `crates/output-formatters` renders text and JSON CLI output.

This keeps business logic out of the CLI binary and establishes the layering needed for future desktop reuse.

## Planned Expansion

The following crate from the architecture plan still needs to be added:

- `analysis-core`

`capture-engine`, `parser-core`, `protocol-dissectors`, and `filter-engine` now exist. The remaining crate is intentionally deferred until the next implementation step so the current slice stays testable.
