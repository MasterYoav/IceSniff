# IceSniffMac Overview

This document explains what the native macOS app is, what code owns which responsibility, and how the app is expected to behave.

## Purpose

`apps/macos` is the native SwiftUI macOS distribution of IceSniff.

It is designed to be:

- independently buildable
- independently testable
- independently packageable
- understandable without needing the rest of the repository

The app is intentionally self-contained:

- SwiftUI app code lives in `Sources/IceSniffMac`
- the mac-specific Rust backend lives in `rust-engine`
- release and bundling scripts live in `scripts`
- regression tests live in `Tests`

## High-Level Architecture

The mac app has three runtime layers:

1. SwiftUI shell
   - window setup
   - sidebar and section navigation
   - cards, tables, detail panes, and settings UI

2. Swift-side app model and process bridge
   - persistent UI state
   - capture state
   - filter normalization
   - save/open flows
   - launching backend processes

3. Rust backend
   - packet parsing
   - protocol dissection
   - filtering
   - stats, conversations, streams, transactions
   - saved capture export
   - live capture helper

## Runtime Model

The app uses two bundled backend binaries:

- `icesniff-cli`
- `icesniff-capture-helper`

`icesniff-cli` is the analysis backend.
It is responsible for reading captures and producing structured packet/report output.

`icesniff-capture-helper` is the macOS live-capture helper.
It is responsible for capture acquisition on macOS.

The Swift app does not reimplement packet parsing or protocol decoding.
It delegates that work to the Rust backend.

## Main User Surfaces

The app currently exposes these sections:

- `Packets`
- `Stats`
- `Conversations`
- `Streams`
- `Transactions`
- `Profile`
- `Settings`

The `Profile` section is no longer placeholder UI.

It now includes:

- real GitHub sign-in
- real Google sign-in
- Keychain-backed session persistence
- remote avatar display
- local-only preference storage in the public build

The `Packets` section is the operational center for:

- opening captures
- live sniffing
- filtering
- packet inspection
- saving filtered or whole captures

## Design Direction

The app uses a native SwiftUI visual language with:

- a molded sidebar
- theme selection
- font selection
- font size controls
- native macOS window behavior

The visual system is intentionally app-owned and not shared with other distributions.

## Contributor Guidance

If you are changing behavior:

- UI-only changes should usually stay in `Views.swift`
- app workflow/state changes should usually stay in `AppModel.swift`
- app/window lifecycle changes should usually stay in `IceSniffMacApp.swift`
- parsing, capture, and analysis changes should usually happen in `rust-engine`

If you are adding a feature, prefer this order:

1. backend capability in `rust-engine`
2. Swift-side bridge in `AppModel.swift`
3. UI in `Views.swift`

That keeps the app thin and the engine authoritative.

## Current Status Snapshot

At the end of the current implementation phase, the mac app has:

- native packet browsing and live capture
- bundled Rust analysis and capture helpers
- Google and GitHub auth
- local-only theme/font preference persistence
- GitHub Actions CI for CLI and mac app validation

Open follow-up areas still worth improving later:

- bundle-level macOS app icon metadata beyond runtime Dock icon assignment
- production signing/notarization polish
- additional profile fields beyond UI preferences
