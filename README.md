<p align="center">
  <img src="media/icesniff_dark.png" alt="IceSniff app icon" width="140">
</p>

# IceSniff
IceSniff is a modern, open-source network packet analyzer built for clarity, speed, and cross-platform usability.

License: MIT. See `LICENSE`.

[![Svelte](https://img.shields.io/badge/Svelte-%23f1413d.svg?logo=svelte&logoColor=white)](#)
[![Tauri](https://img.shields.io/badge/Tauri-24C8D8?logo=tauri&logoColor=fff)](#)
[![Rust](https://img.shields.io/badge/Rust-%23000000.svg?e&logo=rust&logoColor=white)](#)

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

- `shell [capture-file]` for a persistent interactive CLI session
- `save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]` for shared filtered capture export to PCAP
- `inspect <capture-file>` for shared capture metadata inspection
- `list <capture-file> [limit] [--filter <expr>]` for shared PCAP and PCAPNG packet enumeration
- `show-packet <capture-file> <packet-index>` for shared PCAP and PCAPNG packet detail inspection
- `stats <capture-file> [--filter <expr>]` for shared capture and protocol summary statistics
- `conversations <capture-file> [--filter <expr>]` for shared bidirectional flow summaries
- `streams <capture-file> [--filter <expr>] [--stream-filter <expr>]` for shared client/server stream and transaction summaries
- `transactions <capture-file> [--filter <expr>] [--transaction-filter <expr>]` for shared HTTP and TLS transaction enumeration
- `--json` output mode for machine-readable CLI automation with stable `schema_version` tagging

Conversation analysis now includes:

- bidirectional packet counts
- request and response counts for recognized application protocols
- service guessing on top of transport and application metadata

Stream analysis now includes:

- client/server endpoint orientation
- derived TCP session state for open, closed, reset, or midstream flows
- SYN, FIN, and RST packet counts
- matched and unmatched transaction counts
- directional packet counts per stream
- reassembled HTTP transaction counting across fragmented TCP payloads
- reassembled TLS handshake counting across TCP segments
- explicit TLS handshake progression state and message counts
- repeated TLS handshakes on a single connection, with cycle and incomplete-handshake counts
- ordered session-event timelines derived from reassembled HTTP and TLS activity
- explicit HTTP pipelining notes when multiple requests are in flight before responses
- TLS alert counts and alert labels summarized at stream level
- multiple HTTP messages on the same stream
- explicit notes for out-of-order segments, retransmissions, overlaps, sequence gaps, and partial records

Transaction analysis now includes:

- HTTP request/response transaction rows derived from reassembled TCP payloads
- TLS handshake transaction rows derived from reassembled client hello, server hello, certificate, and finished records
- per-transaction state for matched, request-only, response-only, and partial or progressed TLS handshake rows
- structured HTTP transaction details including method, path, host, status, header count, and body size
- HTTP transfer-semantic parsing for `content-length`, header-only messages, and chunked bodies
- structured TLS transaction details including record version, SNI/ALPN when present, certificate-message counts, alerts, and per-side handshake message lists
- propagated stream-level reassembly notes so transaction output exposes gaps, retransmissions, overlap trimming, and incomplete protocol records

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
- `service=dns|http|tls|tcp|udp|icmp|ipv4|arp`
- `port=<number>`
- `ip=<address>`
- `endpoint=<address:port>`
- `host=<name-or-address>`
- `http.method=<verb>`
- `http.path=<path>`
- `http.kind=<request|response>`
- `http.status=<code>`
- `http.reason=<phrase>`
- `http.host=<host>`
- `dns.question=<name>`
- `dns.question_count=<count>`
- `dns.answer_count=<count>`
- `dns.is_response=<true|false>`
- `tls.handshake_type=<type>`
- `tls.server_name=<name>`
- `tls.record_version=<major.minor>`
- `tls.handshake_length=<bytes>`

These clauses can now be combined with boolean filter expressions using `&&`, `||`, `!`, parentheses, and the word forms `and`, `or`, `not`. Commas are still accepted as AND separators for backward compatibility.

Supported comparison operators now include:

- `=` exact match
- `!=` inequality
- `>` `>=` `<` `<=` for numeric fields such as `port` and `http.status`
- `~=` substring contains matching for text fields such as `host` and `tls.server_name`

Text comparisons are now case-insensitive for exact and contains matching.

For `protocol=dns`, `protocol=http`, and `protocol=tls`, filtering now falls back to well-known ports when packet-local application metadata is not yet available, which keeps fragmented streams visible to the shared analysis layer. `host` now matches both application-layer names and packet IP addresses.

Stream-level filtering is also available on `streams` with keys such as:

- `stream.service`
- `stream.protocol`
- `stream.client`
- `stream.server`
- `stream.state`
- `stream.tls_state`
- `stream.packets`
- `stream.syn`
- `stream.fin`
- `stream.rst`
- `stream.requests`
- `stream.responses`
- `stream.matched`
- `stream.unmatched_requests`
- `stream.unmatched_responses`
- `stream.tls_alert_count`
- `stream.tls_alert`
- `stream.tls_client_hellos`
- `stream.tls_server_hellos`
- `stream.tls_certificates`
- `stream.tls_finished`
- `stream.tls_handshake_cycles`
- `stream.tls_incomplete_handshakes`
- `stream.has_alerts`
- `stream.has_timeline`
- `stream.has_notes`
- `stream.has_reassembly_issues`
- `stream.is_pipelined`
- `stream.client_packets`
- `stream.server_packets`
- `stream.total_bytes`
- `stream.first_packet`
- `stream.last_packet`
- `stream.timeline`
- `stream.note`

Transaction-level filtering is also available on `transactions` with keys such as:

- `tx.service`
- `tx.protocol`
- `tx.client`
- `tx.server`
- `tx.sequence`
- `tx.state`
- `tx.request_summary`
- `tx.response_summary`
- `tx.note`
- `tx.request.method`
- `tx.request.path`
- `tx.request.host`
- `tx.response.status_code`
- `tx.response.reason_phrase`
- `tx.response.body_bytes`
- `tx.has_request`
- `tx.has_response`
- `tx.complete`
- `tx.has_alerts`
- `tx.http.status_class`
- `tx.http.method`
- `tx.http.path`
- `tx.http.host`
- `tx.http.status`
- `tx.http.reason`
- `tx.http.transfer_semantics`
- `tx.http.transfer_encoding`
- `tx.http.content_type`
- `tx.http.body_bytes`
- `tx.http.header_count`
- `tx.tls.record_version`
- `tx.tls.server_name`
- `tx.tls.alpn`
- `tx.tls.handshake_messages`
- `tx.tls.alerts`
- `tx.tls.certificate_messages`
- `tx.request.record_version`
- `tx.request.server_name`
- `tx.request.alpn`
- `tx.response.certificate_messages`
- `tx.response.alerts`

## Current CLI

```bash
cargo run -p icesniff-cli -- help
```

Interactive shell:

```bash
cargo run -p icesniff-cli --
# or
cargo run -p icesniff-cli -- shell path/to/capture.pcap
```

Inside the shell:

- `open <capture-file>`
- `save <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`
- `capture interfaces`
- `capture start [interface]`
- `capture stop`
- `capture status`
- `status`
- `mode <text|json>`
- `inspect`
- `list [limit] [--filter <expr>]`
- `show-packet <packet-index>`
- `stats [--filter <expr>]`
- `conversations [--filter <expr>]`
- `streams [--filter <expr>] [--stream-filter <expr>]`
- `transactions [--filter <expr>] [--transaction-filter <expr>]`
- `quit`

Live capture now routes through shared `app-services` orchestration backed by the `capture-engine` crate with `tcpdump`-style and `dumpcap` backend support, writing to a temporary `.pcap` that becomes the current open capture when you stop recording.
While capture is active, the shell now prints a live packet table with `Id | Time | Source | Destination | Protocol | Info`.
Set `ICESNIFF_CAPTURE_TOOL` to override the capture executable path/name, and set `ICESNIFF_CAPTURE_BACKEND` (`tcpdump` or `dumpcap`) to override backend inference when needed.
`capture status` also reports the effective backend and tool for easier troubleshooting.
`save` now writes a new PCAP through shared services with optional packet filtering (`--filter`) and stream-row selection (`--stream-filter`) using the same stream filter semantics as `streams` (`--filter` is applied first, then stream rows are selected).

Current commands:

- `shell [capture-file]`
- `save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`
- `inspect <capture-file>`
- `list <capture-file> [limit] [--filter <expr>]`
- `show-packet <capture-file> <packet-index>`
- `stats <capture-file> [--filter <expr>]`
- `conversations <capture-file> [--filter <expr>]`
- `streams <capture-file> [--filter <expr>] [--stream-filter <expr>]`
- `transactions <capture-file> [--filter <expr>] [--transaction-filter <expr>]`

All commands support `--json` and include a top-level `schema_version` field (`v1`).
CLI errors are script-friendly: usage errors are prefixed with `[ISCLI_USAGE]` and exit with status `2`, while runtime/service errors are prefixed with `[ISCLI_RUNTIME]` and exit with status `1`.

## Repository Guide

Key docs:

- `instructions.md`
- `docs/repo-map.md`
- `docs/architecture/overview.md`
- `docs/feature-parity-matrix.md`
- `docs/task-recipes.md`
- `docs/continuity-log.md`
