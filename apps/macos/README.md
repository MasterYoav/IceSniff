# IceSniff macOS Native App (Option 2)

This is a native SwiftUI macOS app track (no embedded web app).
It uses a mac-local Rust engine workspace under `rust-engine/` and talks to the bundled/local `icesniff-cli --json`.

For the full app-specific documentation set, start here:

- `docs/overview.md`
- `docs/file-map.md`
- `docs/runtime-and-backend.md`
- `docs/features.md`
- `docs/profile-cloud-sync-plan.md` (historical note; sync is currently disabled)
- `docs/appwrite-integration-outline.md`
- `docs/supabase-auth-setup.md`

Repository hygiene for this app track:

- Shipping resources live under `Sources/IceSniffMac/Resources/` and should stay limited to runtime assets and bundled binaries.
- Example captures used for testing should live under repository fixtures, not inside the shipping app bundle.
- Generated SwiftPM state lives under `.swiftpm/` and is ignored.

## Run

From the mac app folder:

```bash
./scripts/sync-bundled-cli.sh
swift run IceSniffMac
```

## How it talks to backend

The app resolves the CLI in this order:

1. `ICESNIFF_CLI_BIN` env var (explicit path)
2. Bundled `icesniff-cli` in app resources
3. Local mac workspace CLI under `rust-engine/target/...`
4. `cargo run -q -p icesniff-cli -- --json ...` in the local mac Rust workspace

If the local Rust workspace is not auto-detected (for example when launching from Xcode), set:

```bash
export ICESNIFF_RUST_WORKSPACE_ROOT=/absolute/path/to/IceSniff/apps/macos/rust-engine
```

## Packaging

The mac app now includes a bundled copy of `icesniff-cli` under `Sources/IceSniffMac/Resources/BundledCLI/icesniff-cli`, and the source for that backend lives in the local mac workspace at `rust-engine/`.
Only runtime binaries belong in `Resources/BundledCLI/`.

To refresh that bundled binary during development:

```bash
cd /path/to/IceSniff/apps/macos
./scripts/sync-bundled-cli.sh
```

By default that script builds a release CLI into `/tmp/icesniff-macos-release-target` and copies it into app resources. For a debug refresh:

```bash
ICESNIFF_CLI_PROFILE=debug ./scripts/sync-bundled-cli.sh
```

## Release Packaging, Signing, and Notarization

Use the release script to rebuild the bundled CLI, build the macOS app, sign it, and optionally notarize it:

```bash
cd /path/to/IceSniff/apps/macos
ICESNIFF_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)" \
ICESNIFF_NOTARY_KEYCHAIN_PROFILE="notary-profile" \
./scripts/release-macos.sh
```

The release script also stamps the built `.app` bundle with a Finder-level custom icon before signing so Finder and the Dock use the packaged app icon without relying only on the runtime Dock override.

Environment variables:

1. `ICESNIFF_SIGNING_IDENTITY`
Required for code signing. If omitted, the app is built but not signed.

2. `ICESNIFF_NOTARY_KEYCHAIN_PROFILE`
Optional. If provided, `xcrun notarytool submit --wait` runs and the resulting app is stapled.

3. `ICESNIFF_RUST_WORKSPACE_ROOT`
Optional explicit local Rust workspace root. Defaults to `apps/macos/rust-engine`.

4. `ICESNIFF_CARGO_TARGET_DIR`
Optional Cargo target directory for the bundled CLI build.

5. `ICESNIFF_DERIVED_DATA`
Optional Xcode derived data directory for the release app build.

Output:

1. Release `.app` under `build/release` via the Xcode release build products path.
2. Notarization zip at `build/release/IceSniffMac.zip`.

## Regression Tests

Run the mac package regression suite with:

```bash
cd /path/to/IceSniff/apps/macos
swift test
```

Current coverage includes:

1. Opening a bundled fixture capture through the bundled CLI.
2. Comfort-first filter normalization.
3. Save filtered vs whole capture scope selection logic.
4. Engine info / capability payload compatibility.
5. Privileged live-capture command generation and error mapping.

## Supabase Auth Setup

The public build keeps Google/GitHub sign-in, but cloud preference sync is currently disabled.
Supabase is only used for browser-based auth and session restoration when these environment variables are present:

1. `ICESNIFF_SUPABASE_URL`
2. `ICESNIFF_SUPABASE_PUBLISHABLE_KEY`
3. `ICESNIFF_SUPABASE_REDIRECT_URL` (optional, defaults to `icesniff://auth/callback`)

For local Xcode testing:

1. Open the app scheme in Xcode.
2. Edit the `Run` action environment variables.
3. Add the `ICESNIFF_SUPABASE_*` values.
4. Relaunch the app.

App-side behavior:

1. If the variables are missing, sign-in is unavailable and the `Profile` screen explains what is missing.
2. If the variables are present, the app uses real Supabase auth for Google and GitHub.
3. Theme, font family, and font size remain local-only preferences in the public build.

## Current scope

- Native frosted, collapsible sidebar
- Separate section views (Capture, Packets, Stats, Conversations, Streams, Transactions, Profile, Settings)
- Uses the official light app icon in the sidebar
- Uses the official bundled icon as the running Dock icon
- Release packaging stamps the `.app` bundle with a packaged Finder/Dock icon
- Loads capture data through CLI JSON commands
- One-time privileged live-capture setup on macOS
- Bundled analysis backend and bundled capture helper
- Real Google and GitHub sign-in through Supabase auth
- Local-only preferences in the public build
