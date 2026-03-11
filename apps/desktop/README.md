# IceSniff Desktop (Prototype)

This directory now contains a speed-first desktop vertical slice using Tauri 2 + Svelte.

## Current scope

- open a capture file by absolute path
- apply shared packet filter expression
- load packet table (`list` shared service)
- load capture summary and stats (`inspect` + `stats` shared services)
- select a packet row and inspect decoded detail (`show-packet` shared service)
- view decoded field tree and byte-range-aware hex highlights
- load conversation rows (`conversations` shared service)
- load stream rows (`streams` shared service)
- load transaction rows (`transactions` shared service)
- inspect selected conversation rows in detail panes
- inspect selected stream and transaction rows in detail panes
- jump from selected conversation/stream/transaction rows into a focused capture filter view
- run live capture from desktop (interface list + start/status/stop) through shared capture services
- live-refresh packet table and capture stats while desktop capture is running
- optional follow-latest mode to keep packet detail focused on newest packets during live capture
- auto-load the generated temporary capture when stopping live capture
- save filtered captures to PCAP (`save` shared service)
- export conversations/streams/transactions to JSON or CSV files
- keep recent capture paths and last-used filters in local desktop state
- linked row-selection behavior across conversation/stream/transaction panels to speed drill-down

## Important constraints

- This is intentionally a rapid prototype for velocity, not stability.
- UI state and API contracts can change quickly.
- No desktop-specific business logic: all packet behavior is still served by shared Rust crates.

## Run (from this folder)

```bash
npm install
npm run tauri -- dev
```

If macOS live capture permissions are still flaky in your shell, force `tcpdump` explicitly:

```bash
npm run tauri:dev:tcpdump
```

## Build frontend only

```bash
npm run build
```

## Build desktop binary (debug)

```bash
npm run tauri -- build --debug
```

## Notes

- The app includes quick sample buttons when fixture files are present under `fixtures/golden`.
- The app includes native file-picker buttons for source and export paths.
- Desktop command responses are now returned to the UI as structured JSON values.
