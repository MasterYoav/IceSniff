# Desktop Overview

The desktop application is built with Tauri 2 and Svelte.

## Current state

A speed-first prototype now exists under `apps/desktop` with:

- Tauri command bridge in `apps/desktop/src-tauri`
- Svelte frontend in `apps/desktop/src`
- end-to-end flow for `inspect`, `stats`, `list`, `show-packet`, `conversations`, `streams`, `transactions`, `save`, and desktop analysis export commands
- desktop live-capture controls (`interfaces`, `start`, `status`, `stop`) backed by shared capture orchestration
- live packet/stats preview refresh during active desktop capture polling
- optional follow-latest behavior so packet detail/hex panes can track newest captured packets
- packet table + detail + decoded fields + byte-range-aware hex pane
- conversation, stream, and transaction table row selection with in-panel drill-down details
- one-click focus actions that convert selected conversation/stream/transaction rows into active capture filters
- cross-panel preselection: selecting conversation/stream/transaction rows auto-selects related rows in the other analysis panels when present
- filter input, packet-row limit controls, stream/transaction analysis filters, and filtered PCAP save action
- analysis-row export actions for conversations/streams/transactions (`json` or `csv`)
- native source/output file-picker actions through Tauri dialog plugin
- recent capture path list and last-used filter/path state persisted locally
- stop-live-capture handoff that auto-loads the generated capture file into packet/analysis panels

## Prototype rules

- move fast over architecture polish
- keep desktop thin and service-driven
- avoid desktop-side packet parsing/filter logic

## Current limitations

- desktop bridge now returns structured JSON values but still reuses `output-formatters` JSON schema instead of dedicated typed DTOs
- no production packaging or signing setup yet
- no long-run stability tuning yet

## Design rule

Desktop code should focus on presentation, layout, state orchestration, and user interaction. Parsing, filtering, and capture behavior remain in shared Rust crates.
