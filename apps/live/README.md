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

- browser UI styled to match the macOS app shell
- packet/live-capture surface
- stats, conversations, streams, and transactions sections
- upload existing capture files into a local temporary workspace
- start and stop live capture through the Rust capture helper
- analysis powered by the same Rust+tshark engine used by the macOS app
