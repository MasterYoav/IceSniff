# ADR-0002: Move to an App-Per-Platform Repository Shape

## Status

Accepted on 2026-03-12.

## Context

IceSniff originally started from a cross-platform Tauri + Svelte direction with one shared Rust workspace at the repository root.

That shape no longer matches the current product direction:

- `IceSniffMac` is a native SwiftUI app
- future `IceSniffWindows` work is expected to be platform-specific
- `IceSniffCLI` should remain independently buildable and shippable

The previous repository shape keeps too much product logic and packaging logic at the root, which creates confusion about ownership and makes each app feel less self-contained than the product direction requires.

## Decision

The repository should move toward a thin-root, app-per-platform structure.

The root should primarily contain:

- `apps/`
- `docs/`
- top-level project metadata such as `README.md`, `LICENSE`, `CONTRIBUTING.md`, and `.gitignore`

Each app should be independently understandable and independently shippable.

That means each app may own its own:

- source code
- platform assets
- packaging scripts
- tests
- runtime resources
- backend or engine code

Duplication between apps is acceptable when it improves app independence, packaging clarity, and platform-specific evolution.

## Target Repository Shape

The intended long-term layout is:

- `apps/cli`
- `apps/macos`
- `apps/windows`
- `docs`

Within each app:

- all runtime dependencies should live with that app
- all app-specific build and packaging logic should live with that app
- all app-specific tests should live with that app

Shared root-level engine crates are no longer the target architecture for the product.

## Consequences

- The repository becomes easier to understand by product surface instead of by implementation-layer theory.
- Packaging and release ownership becomes clearer for each distribution.
- App-specific evolution no longer has to preserve an artificial shared-engine abstraction when the product no longer wants that coupling.
- Some code and assets may be duplicated across apps, and that duplication is considered an acceptable cost.

## Migration Rules

1. Prefer moving app-owned code into the owning app folder instead of creating new shared top-level crates.
2. Keep only cross-project documentation and contributor metadata at the root.
3. Remove stale cross-platform placeholders once a platform has its own real app track.
4. Treat runtime assets and packaging scripts as app-local by default.
