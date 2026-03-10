# Task Recipes

This document records the preferred implementation path for common tasks so contributors and AI tools can resume safely.

## Add a new shared capability

1. Define or extend the domain types in `crates/session-model`.
2. Add the use case in `crates/app-services`.
3. Implement infrastructure support in a dedicated crate.
4. Expose the capability in `apps/cli`.
5. Add or update CLI formatting in `crates/output-formatters`.
6. Update the parity matrix and architecture docs.

## Extend capture-file support

1. Add or refine low-level parsing in `crates/file-io`.
2. Return shared models from `crates/session-model`, not file-io-specific structs.
3. Expose the flow through `crates/app-services`.
4. Keep CLI commands focused on presentation and argument handling only.
5. Record format support and limitations in `docs/continuity-log.md`.

## Extend packet inspection

1. Decide whether the new detail belongs in link, network, transport, or future field-tree models.
2. Add the shared structure in `crates/session-model`.
3. Decode it inside `crates/file-io` for the currently supported capture formats.
4. Render it in `crates/output-formatters` without adding parsing logic there.
5. Add fixture-backed tests for both happy-path and truncated packets.

## Add a new protocol parser later

1. Add packet/domain types in shared model crates only if truly cross-protocol.
2. Implement the protocol dissector in the future `protocol-dissectors` crate.
3. Surface the parsed result through shared services.
4. Add fixtures covering valid and malformed packets.
5. Document the protocol behavior in `docs/protocols/`.

## Add a desktop feature later

1. Decide whether the change is a shared capability or desktop presentation.
2. If shared, implement it below `apps/desktop` first.
3. Keep UI state and rendering inside the desktop app.
4. Update the parity matrix if the feature has a CLI equivalent.
