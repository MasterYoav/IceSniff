# Architecture Overview

IceSniff is moving toward an app-per-platform architecture.

The repository is no longer optimized around one root-level shared engine feeding every interface. Instead, each shipping app is expected to own the code and packaging it needs in order to build and ship independently.

## Product Surfaces

The intended long-term product surfaces are:

- `apps/cli`
- `apps/live`
- `apps/macos`
- `apps/windows`

Each of those app folders should become independently understandable and independently releasable.

## Repository Rules

1. Prefer app-local code over root-level shared implementation when independence matters.
2. Keep the repository root thin: `apps/`, `docs/`, and top-level project metadata.
3. Treat packaging, runtime resources, and tests as app-local by default.
4. Duplication across apps is acceptable when it reduces coupling or simplifies platform-specific evolution.

## App Shape

Each app may own its own:

- source code
- backend or engine code
- runtime assets
- packaging scripts
- tests
- release automation

This means the native macOS app can keep a local Rust engine workspace under `apps/macos/rust-engine`, while future Windows work can do the same under its own app folder if needed.

## Documentation Role

The shared concern that remains at the repository level is documentation and project metadata:

- architecture direction
- contributor guidance
- roadmap
- repository map
- major decisions

Those documents should explain how the apps relate to one another, but the code ownership should stay local to each app whenever possible.
