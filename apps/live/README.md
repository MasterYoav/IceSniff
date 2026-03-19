# IceSniff Live

`apps/live` is the local web app track for IceSniff.

It mirrors the launch-ready macOS app look and uses the same backend runtime model:

- `icesniff-cli` for analysis JSON
- `icesniff-capture-helper` for live capture
- `tshark` for packet metadata and decoding inside the Rust engine

## Run

```bash
cd apps/live
node server.mjs
```

Then open:

```text
http://127.0.0.1:4318
```

## Runtime Resolution

The web app resolves backend binaries in this order:

1. `ICESNIFF_CLI_BIN`
2. `apps/macos/Sources/IceSniffMac/Resources/BundledCLI/icesniff-cli`
3. `apps/macos/rust-engine/target/.../icesniff-cli`
4. `cargo run -q -p icesniff-cli -- ...`

For live capture:

1. `ICESNIFF_CAPTURE_HELPER_BIN`
2. `apps/macos/Sources/IceSniffMac/Resources/BundledCLI/icesniff-capture-helper`
3. `apps/macos/rust-engine/target/.../icesniff-capture-helper`
4. `cargo run -q -p icesniff-capture-helper -- ...`

For tshark:

1. `ICESNIFF_TSHARK_BIN`
2. bundled Wireshark runtime under `apps/macos/Sources/IceSniffMac/Resources/BundledTShark`
3. common system install paths

## Current Scope

- browser UI styled as a local web shell with:
  - a collapsible overlay section rail
  - a toggleable AI side panel
  - browser-local theme, font, and panel-background preferences
- packet/live-capture surface
- stats, conversations, streams, and transactions sections
- upload existing capture files into a local temporary workspace
- start and stop live capture through the Rust capture helper with a packets-view toggle control
- save/export the current capture from the packets view
- analysis powered by the same Rust+tshark engine used by the macOS app

## UI Notes

- the section rail opens as an overlay inside the main workspace instead of resizing the primary content area
- the section rail auto-closes after a section is selected
- the main header owns the view title, the section-rail toggle, the shared open-capture action, and the AI-panel toggle
- packets view keeps the filter and capture utilities in the top row and uses a toggle control for starting or stopping live capture
- packets view saves the current capture from the capture panel, while file import stays in the shared header action
- theme, font, panel-background density, and panel visibility preferences are stored in browser-local state
