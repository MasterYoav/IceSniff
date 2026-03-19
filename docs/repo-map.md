# Repository Map

This document explains the current repository layout and the intended responsibility of each top-level area.

## Top Level

The repository is moving toward a thin-root shape.

- `apps/cli`: first-class command-line interface with its own Rust workspace, fixtures, and tests
- `apps/live`: local web app track with a browser UI and local process-backed Rust runtime bridge
- `apps/macos`: native SwiftUI macOS app track with its own local Rust workspace under `apps/macos/rust-engine`, release scripts, tests, and runtime resources
- `apps/windows`: intended future native Windows app track
- `docs/`: architecture, process, roadmap, and continuity documentation
- `docs/media/`: branding files used by shared documentation and the root README

## Direction

The documented target is to keep the root focused on:

- `apps/`
- `docs/`
- top-level project metadata

Root-level implementation folders should be removed or moved under the owning app as the migration continues.

## Native macOS App Track

`apps/macos` is intentionally self-contained so the native macOS distribution can evolve and ship without depending on other app tracks at runtime.

Inside that app track:

- `apps/macos/Sources/IceSniffMac`: SwiftUI app source
- `apps/macos/Sources/IceSniffMac/Resources`: shipping icons and bundled runtime binaries only
- `apps/macos/rust-engine`: mac-local Rust engine workspace used by the native app
- `apps/macos/scripts`: app-specific build and packaging helpers
- `apps/macos/Tests`: native app regression coverage

Generated SwiftPM state under `apps/macos/.swiftpm` is not part of the repository layout.

## CLI App Track

`apps/cli` is now its own Rust workspace and owns:

- the CLI entry point
- CLI-owned Rust crates under `apps/cli/crates`
- CLI fixtures under `apps/cli/fixtures`
- CLI tests

## Current Vertical Slice

The first implemented slice is intentionally narrow:

1. `apps/cli` accepts `inspect <capture-file>`.
2. `apps/cli` accepts `save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`.
3. `apps/cli` accepts `list <capture-file> [limit]`.
4. `apps/cli` accepts `show-packet <capture-file> <packet-index>`.
5. `apps/cli` accepts `stats <capture-file>` and supports `--json`.
6. `apps/cli` accepts shared `--filter <expr>` semantics for `list`, `stats`, `conversations`, and `streams`, and `--stream-filter <expr>` semantics for `streams` and `save`.
7. `apps/cli/crates/app-services` exposes the use cases as shared services for the CLI app track.
8. `apps/cli/crates/file-io` loads capture files into shared packet records and writes PCAP output for save/export workflows.
9. `apps/cli/crates/filter-engine` applies shared packet-filter semantics.
10. `apps/cli/crates/parser-core` turns loaded packets into list/detail/stats/conversation/stream reports and stream packet-index selection.
11. `apps/cli/crates/protocol-dissectors` performs minimal Ethernet/ARP/IPv4/TCP/UDP/ICMP decoding plus early DNS/HTTP/TLS inspection.
12. `apps/cli/crates/session-model` carries the report structures.
13. `apps/cli/crates/output-formatters` renders text and JSON CLI output.
14. `apps/macos` consumes its app-local Rust engine through CLI/helper boundaries and renders packet/detail/field/hex panes plus conversation/stream/transaction analysis tables, filtered export controls, and native live-capture controls.
15. `apps/live` ships a browser-local shell with an overlay section rail, toggleable AI side panel, browser-local appearance preferences, and packets-view live-capture controls while talking to the same Rust analysis/capture backend through a local HTTP server that launches `icesniff-cli` and `icesniff-capture-helper`.

This reflects the current transition state. The intended long-term shape is app-local ownership, not continued growth of root-level shared code by default.

The remaining migration work is mainly repository cleanup and future Windows implementation, not preserving the old root-level workspace shape.
