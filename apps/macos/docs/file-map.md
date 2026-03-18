# IceSniffMac File Map

This file explains the purpose of every important file and folder in `apps/macos`.

## Root

### `README.md`

Entry-point documentation for the mac app track:

- how to run it
- how the backend is resolved
- how packaging works
- what is bundled

### `Package.swift`

Swift Package manifest for the native mac app.

It defines:

- the executable target
- packaged runtime resources
- the test target

### `docs/`

App-local documentation for the mac app track.

Important current references:

- `docs/profile-cloud-sync-plan.md` (historical design note)
- `docs/appwrite-integration-outline.md`
- `docs/supabase-auth-setup.md`

## Swift App

### `Sources/IceSniffMac/IceSniffMacApp.swift`

App entry point and top-level scene wiring.

This file owns:

- app startup
- top-level scene creation
- top-level environment and toolbar wiring
- native macOS window behavior

### `Sources/IceSniffMac/AppModel.swift`

Operational brain of the app.

This file owns:

- section and theme models
- filter normalization
- capture save scope logic
- backend capability parsing
- live-capture runtime resolution
- CLI process launching and JSON/text bridging
- app state for opened captures and live sessions
- selected-packet context exported for the AI runtime

If the app “does something” and it is not just view layout, it is probably rooted here.

### `Sources/IceSniffMac/AIChatRuntime.swift`

App-local AI runtime and provider integration layer.

This file owns:

- AI provider and model catalog
- Keychain-backed API key storage
- direct provider HTTP requests
- local `codex` CLI execution
- local `claude` CLI execution
- provider-specific output sanitization and failure mapping

### `Sources/IceSniffMac/Views.swift`

Primary UI composition file.

This file owns:

- sidebar layout
- section containers
- packet list and detail panes
- AI panel and chat composer UI
- settings UI
- themes and font presentation helpers
- reusable cards and controls

It is intentionally large because the app currently keeps most UI composition in one place.

### `Sources/IceSniffMac/ProfileCloudSync.swift`

Cloud auth runtime.

This file owns:

- Supabase configuration parsing
- Keychain-backed session storage
- Google and GitHub OAuth runtime
- session restoration
- public-build cloud-sync disabled messaging

## Runtime Resources

### `Sources/IceSniffMac/Resources/icon.icon`

Primary icon bundle used by the app when available.

### `Sources/IceSniffMac/Resources/icon-light.png`

Official light icon fallback, also used explicitly in the sidebar.

### `Sources/IceSniffMac/Resources/icon-dark.png`

Dark icon fallback kept as an alternate packaged asset.

### `Sources/IceSniffMac/Resources/BundledCLI/icesniff-cli`

Bundled Rust analysis backend used by the app at runtime.

### `Sources/IceSniffMac/Resources/BundledCLI/icesniff-capture-helper`

Bundled Rust live-capture helper used by the app at runtime.

Only runtime binaries belong in `Resources/BundledCLI`.

## Scripts

### `scripts/sync-bundled-cli.sh`

Builds the mac-local Rust workspace and refreshes the bundled backend binaries.

Use this when:

- backend code changed
- the bundled binaries need to be refreshed before packaging

### `scripts/release-macos.sh`

Builds and packages the mac app for release use.

It coordinates:

- backend refresh
- Xcode app build
- optional signing
- optional notarization

## Rust Engine

### `rust-engine/Cargo.toml`

Workspace manifest for the mac-local Rust backend.

### `rust-engine/apps/cli`

Rust CLI backend used by the app for analysis commands.

### `rust-engine/apps/capture-helper`

Rust live-capture helper used by the app on macOS.

### `rust-engine/crates/app-services`

High-level use cases exposed by the engine.

### `rust-engine/crates/capture-engine`

Live-capture orchestration and capture helper behavior.

### `rust-engine/crates/file-io`

PCAP and PCAPNG loading and writing.

### `rust-engine/crates/filter-engine`

Packet filter semantics.

### `rust-engine/crates/output-formatters`

Text and JSON formatting for the CLI backend.

### `rust-engine/crates/parser-core`

Packet report assembly and analysis orchestration.

### `rust-engine/crates/protocol-dissectors`

Protocol decoding logic.

### `rust-engine/crates/session-model`

Stable shared backend domain model.

## Tests

### `Tests/IceSniffMacTests/AppModelTests.swift`

Swift-side regression tests for:

- filter normalization
- save scope behavior
- engine capability parsing
- bundled CLI behavior
- live capture command and runtime resolution behavior

### `Tests/Fixtures/sample.pcap.hex`

Hex-encoded sample capture fixture used to generate a temporary `.pcap` during tests.

It is intentionally kept out of app runtime resources.
