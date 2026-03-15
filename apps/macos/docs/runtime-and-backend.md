# IceSniffMac Runtime And Backend Guide

This document explains exactly how the mac app talks to its backend and how live capture works.

## Backend Binaries

The app uses two backend binaries:

- `icesniff-cli`
- `icesniff-capture-helper`

## `icesniff-cli`

`icesniff-cli` is the structured analysis backend.

It is used for:

- `inspect`
- `list`
- `show-packet`
- `stats`
- `conversations`
- `streams`
- `transactions`
- `save`
- `engine-info`

The app expects JSON output for machine-readable operations.

## `icesniff-capture-helper`

`icesniff-capture-helper` is the mac-specific capture acquisition helper.

It is used for:

- interface discovery
- live capture start
- live capture stop
- writing capture data that the analysis backend can read

## Resolution Order

The app resolves the analysis backend in this order:

1. `ICESNIFF_CLI_BIN`
2. bundled `icesniff-cli`
3. local mac Rust workspace build output
4. local `cargo run` fallback

The app resolves the capture helper similarly, preferring:

1. `ICESNIFF_CAPTURE_HELPER_BIN`
2. bundled helper
3. local mac Rust workspace build output

## Why The App Uses Process Boundaries

The mac app intentionally launches backend binaries instead of embedding all backend logic inside Swift because:

- Rust owns packet parsing and protocol logic
- CLI/helper boundaries make backend behavior easier to test directly
- packaging can bundle backend artifacts without rewriting core logic

## Live Capture Model

The live-capture path is:

1. resolve capture helper
2. enumerate interfaces
3. start capture helper on selected interface
4. write capture data incrementally
5. ask `icesniff-cli` to analyze the growing capture
6. update the UI with packet/report results

## Capture Permissions

The app uses a one-time privileged setup model for macOS capture instead of prompting on every start.

That setup is required because passive packet capture on macOS is permission-sensitive.

The app model contains the Swift-side setup orchestration and error mapping for:

- setup install
- permission-denied states
- helper resolution failures
- user-facing capture backend errors

## Failure Surfaces

If the app cannot capture live traffic, the failure is usually in one of these areas:

1. capture helper resolution
2. one-time capture privilege setup
3. interface discovery
4. capture file readability while capture is active
5. analysis backend compatibility

The Swift app should expose these as plain user-facing status messages, not raw backend dumps, unless debugging is needed.

## Packaging Rule

The runtime app bundle must contain:

- `icesniff-cli`
- `icesniff-capture-helper`
- icon resources

The app should not rely on:

- Cargo being installed
- the monorepo existing on disk
- Wireshark being installed

Those are development conveniences only, not release requirements.

## Profile Runtime

That runtime lives in `ProfileCloudSync.swift` and is responsible for:

- Supabase configuration parsing from environment variables
- Keychain-backed auth session storage
- browser-based OAuth sign-in
- provider identity normalization
- session restoration across launches

Current supported providers:

- Google
- GitHub

Apple sign-in is intentionally not part of the current runtime.

Cloud preference sync is currently disabled in the public build. Theme and font preferences are stored locally via `PreferencesStore`.

## Dock Icon Behavior

The app uses the bundled `icon.icon` asset as the preferred source for the running application icon.

At launch, the app sets `NSApp.applicationIconImage` from the bundled icon so the running app uses the official IceSniff icon in the Dock.

For packaged release builds, `scripts/release-macos.sh` also stamps the `.app` bundle with a custom Finder icon before signing. That gives Finder and the Dock a packaged app icon even before the app launches.
