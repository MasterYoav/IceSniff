<p align="center">
  <img src="docs/media/banner.png" alt="IceSniff app banner" width="1000">
</p>

# IceSniff

IceSniff is an open-source packet analysis project focused on a modern native UI, a scriptable CLI, and a shared Rust analysis engine.

The repository currently contains:

- a Rust CLI in `apps/cli`
- a native SwiftUI macOS app in `apps/macos`
- a placeholder `apps/windows` track for future work

IceSniff is released under the MIT License. See `LICENSE`.
![CI](https://github.com/MasterYoav/MLOps_project/actions/workflows/ci.yml/badge.svg)

[![Rust](https://img.shields.io/badge/Rust-%23000000.svg?e&logo=rust&logoColor=white)](#)
[![Swift](https://img.shields.io/badge/Swift-F54A2A?logo=swift&logoColor=white)](#)
[![Supabase](https://img.shields.io/badge/Supabase-3FCF8E?logo=supabase&logoColor=fff)](#)
[![GitHub Actions](https://img.shields.io/badge/GitHub_Actions-2088FF?logo=github-actions&logoColor=white)](#)

## What Works Today

### CLI

The CLI can:

- open and inspect `.pcap` and `.pcapng` captures
- list packets and inspect packet details
- calculate capture stats and conversation summaries
- analyze streams and transactions
- save filtered captures to a new PCAP file
- emit text or stable `--json` output
- run an interactive shell workflow
- perform live capture through external packet capture tools

Current protocol coverage includes:

- Ethernet, ARP, IPv4, TCP, UDP, ICMP
- DNS
- HTTP/1.1
- TLS handshake metadata and stream/transaction summaries

### macOS App

The native macOS app currently supports:

- opening existing capture files
- starting and stopping live capture
- packet, stats, conversations, streams, and transactions views
- packet detail inspection driven by the shared Rust backend
- local UI preferences for theme and typography
- optional Google and GitHub sign-in through Supabase

## What Is Not Done Yet

IceSniff is still early-stage software. Important gaps include:

- no Windows app yet
- no Linux desktop app yet
- protocol coverage is still limited compared with mature analyzers
- live capture depends on external system capture tooling and platform permissions
- cloud-backed profile sync is disabled in the public macOS build
- contributor-facing packaging and release workflows are still evolving

## Getting Started

### CLI

```bash
cd apps/cli
cargo run -p icesniff-cli -- help
```

Useful commands:

```bash
cargo run -p icesniff-cli -- inspect path/to/capture.pcap
cargo run -p icesniff-cli -- list path/to/capture.pcap
cargo run -p icesniff-cli -- stats path/to/capture.pcap
cargo run -p icesniff-cli -- conversations path/to/capture.pcap
cargo run -p icesniff-cli -- streams path/to/capture.pcap
cargo run -p icesniff-cli -- transactions path/to/capture.pcap
```

### macOS App

```bash
cd apps/macos
./scripts/sync-bundled-cli.sh
swift run IceSniffMac
```

## Repository Guide

- `CONTRIBUTING.md` for contribution rules and local workflows
- `docs/architecture/overview.md` for architecture notes
- `docs/feature-parity-matrix.md` for cross-surface tracking
- `docs/repo-map.md` for repository structure
- `apps/macos/README.md` for macOS-specific setup

## Contributing

If you want to help, areas with clear value right now include:

- protocol support and parser hardening
- live capture reliability across platforms
- filtering and analysis UX
- tests, fixtures, and regression coverage
- packaging and release automation
- future Windows app work
