# Continuity Log

This file is the running project documentary for future sessions.

## 2026-03-11

### Starting point

- Shared Rust engine/services were already in place via CLI-first work.
- `apps/desktop` was still a placeholder with no runnable Tauri/Svelte app.

### Decisions preserved

- Prioritized speed-first desktop delivery over architecture hardening.
- Kept desktop business logic thin by calling shared `app-services` directly.
- Reused existing JSON rendering from `output-formatters` to avoid DTO rewrite overhead.

### Work completed

- Bootstrapped `apps/desktop` as a Tauri 2 + Svelte prototype app.
- Added `src-tauri` backend command bridge for:
  - `inspect_capture`
  - `list_packets`
  - `inspect_packet`
  - `capture_stats`
  - `list_conversations`
  - `list_streams`
  - `list_transactions`
  - `save_capture`
  - `capture_runtime_info`
  - `capture_interfaces`
  - `capture_start`
  - `capture_status`
  - `capture_stop`
  - `export_conversations`
  - `export_streams`
  - `export_transactions`
  - `sample_capture_paths`
- Wired backend commands to shared Rust services (`app-services`) and shared JSON output (`output-formatters`).
- Built a desktop UI vertical slice with:
  - capture path + filter + row-limit controls
  - native source/output file pickers via Tauri dialog plugin
  - recent capture list and last-used analysis state persistence in local desktop storage
  - stream-filter and transaction-filter controls with analysis refresh
  - packet table with selection
  - packet detail summary
  - decoded layer previews
  - decoded field tree
  - byte-range-aware hex pane highlighting
  - conversation rows with selected-row drill-down details
  - stream rows and transaction rows
  - stream/transaction selected-row drill-down details
  - focus actions from selected conversation/stream/transaction rows into active capture filters
  - cross-panel preselection so selected conversation/stream/transaction rows can auto-select matching rows in adjacent analysis panels
  - filtered PCAP save/export action
  - live-capture controls for interface selection, start/status/stop, and backend/tool runtime visibility
  - live packet/stats polling from the active temporary capture file while live capture is running
  - optional follow-latest packet mode so detail/hex panes can track newest packets during live capture
  - stop-live-capture handoff that auto-loads the generated capture into packet/analysis views
  - analysis-row export actions for conversations/streams/transactions as JSON or CSV
- Shifted desktop command responses from raw JSON strings to structured JSON values at the Tauri boundary.
- Verified desktop bootstrap build path with `npm run tauri -- build --debug`.
- Updated `docs/desktop/overview.md` to reflect the new prototype status.
- Replaced desktop placeholder README with runnable prototype instructions.

### Current limitations

- Desktop is currently a rapid prototype and not stability-tuned.
- Desktop still relies on shared output-formatter JSON schemas rather than dedicated typed desktop DTOs.
- Desktop app is not included in the Rust workspace members yet, so root `cargo test` does not validate `src-tauri`.
- Recent/session persistence is local-state only and not yet a formal cross-device session model.

### Recommended next move

Keep momentum on desktop workflow depth:

- add explicit typed desktop DTO contracts instead of formatter-schema coupling
- add persistent cross-panel selection linking (conversation -> stream -> transaction preselection, not only filter focus)
- unify desktop analysis export schema/contracts with future CLI export surfaces

## 2026-03-10

### Starting point

- Repository contained only planning docs and branding assets.
- No Rust workspace, no apps, no shared crates, and no executable code existed yet.

### Decisions preserved

- Treated `README.md` and `instructions.md` as the authoritative project charter.
- Followed the recommended implementation order: workspace plus CLI-first slice before desktop bootstrap.
- Kept the first implementation intentionally small so the architecture is visible and testable.

### Work completed

- Created a Rust workspace rooted at `Cargo.toml`.
- Added `apps/cli` with a working `inspect <capture-file>` command.
- Added a persistent `shell [capture-file]` workflow and made no-argument startup drop into the interactive CLI session by default.
- Added shell-based live capture commands so the CLI can start sniffing without an already-open capture file, using the system `tcpdump` tool, a live `Id | Time | Source | Destination | Protocol | Info` packet table, and a temporary PCAP handoff.
- Added `crates/capture-engine` and moved live-capture interface enumeration/start/stop process management out of `apps/cli` into the shared engine layer so future desktop reuse can follow the same capture-session lifecycle.
- Extended `crates/capture-engine` with platform-aware default capture-tool discovery (`tcpdump` and `/usr/sbin/tcpdump` on Unix-like systems, `windump`/`tcpdump` candidates on Windows), backend-aware error classification for permission versus missing driver/runtime (`libpcap`/Npcap), and stricter stop-path validation so failed captures do not report success without a readable output file.
- Added explicit capture backend abstraction in `crates/capture-engine` with `tcpdump`-style and `dumpcap` providers, env override support (`ICESNIFF_CAPTURE_BACKEND`), and automatic backend inference from the selected tool name.
- Moved live-capture orchestration into `crates/app-services` via a shared `LiveCaptureCoordinator`/`LiveCaptureSession` API so interface enumeration, start, status polling, and stop lifecycle can be consumed uniformly by CLI and future desktop flows.
- Extended the shared capture orchestration API with runtime backend/tool metadata so interfaces can surface the active capture provider configuration without directly importing capture-engine internals.
- Updated CLI capture UX to surface capture-engine error hints for missing tools, missing backend drivers, and permission failures, and made `status`/`capture status` check live process state before reporting `running`.
- Added CLI smoke coverage for capture processes that exit naturally, so shell `status`/`capture status` can report `exited` instead of always `running`.
- Added shared save/export support in `crates/app-services` with PCAP writer plumbing in `crates/file-io`, exposed as CLI `save` command for filtered capture output.
- Extended save/export selection with stream-aware filtering (`--stream-filter`) so saved output can be selected from shared stream rows, not only packet-level predicates.
- Added CLI integration parity tests that compare text and JSON totals for core shared commands (`save`, `list`, `stats`, `conversations`, `streams`, and `transactions`) against the same captures.
- Added stable JSON schema tagging (`schema_version: v1`) across all CLI `--json` report outputs for scripting/version negotiation.
- Added stable CLI error-code prefixes and exit-status mapping for scripting (`[ISCLI_USAGE]` -> `2`, `[ISCLI_RUNTIME]` -> `1`).
- Added committed golden capture fixtures under `fixtures/golden` for PCAP, PCAPNG, and malformed containers, and wired `crates/file-io` tests to validate both successful parsing and expected error paths from those fixtures.
- Added `apps/cli` packet listing with `list <capture-file> [limit]`.
- Added `apps/cli` packet detail inspection with `show-packet <capture-file> <packet-index>`.
- Added `apps/cli` capture statistics with `stats <capture-file>`.
- Added `--json` output mode across CLI commands.
- Added shared crates:
  - `app-services`
  - `filter-engine`
  - `file-io`
  - `output-formatters`
  - `parser-core`
  - `protocol-dissectors`
  - `session-model`
- Implemented basic capture container detection for PCAP and PCAPNG by file magic number.
- Implemented a real shared PCAP reader that enumerates packet records and timestamps.
- Implemented a shared PCAPNG reader for section header, interface description, and enhanced packet blocks.
- Moved packet decoding out of `file-io` into `parser-core` and `protocol-dissectors`.
- Implemented minimal shared packet decoding for Ethernet, ARP, IPv4, TCP, UDP, and ICMP in the new parser/dissector layer.
- Added a basic field-tree representation to packet detail output.
- Added byte ranges to field-tree nodes so decoded fields can be mapped back to raw bytes.
- Added application-layer inspection for DNS, HTTP/1.1, and TLS handshake metadata.
- Added shared filtering for packet listing and capture stats with `protocol`, `port`, and `host` expressions.
- Added analyst-oriented packet-list columns for source, destination, protocol, and info.
- Added bidirectional conversation summaries as a shared report and CLI command.
- Extended conversation summaries with service guessing, directional packet counts, and request/response counts.
- Added a top-level MIT `LICENSE` file and aligned contributor terms to MIT so the public repository can accept outside help cleanly.
- Added a dedicated shared stream report and CLI command with client/server role selection and basic matched transaction counters.
- Extended the shared stream report with TCP reassembly for fragmented HTTP transactions and TLS handshake records.
- Updated shared protocol filtering so DNS/HTTP/TLS filters fall back to well-known ports when packet-local application metadata is absent.
- Extended the stream reassembler to surface retransmissions, overlaps, and sequence gaps as explicit notes.
- Added stream-level coverage for multiple HTTP messages in a single reassembled flow.
- Extended the stream reassembler to surface out-of-order delivery explicitly and validate reordered HTTP fragment handling.
- Added TCP session-state reporting to stream summaries using SYN/FIN/RST counts.
- Added TLS handshake progression reporting to stream summaries, including reset-aware handshake states and per-message counts.
- Added repeated TLS handshake cycle tracking so long-lived connections can report multiple handshakes and incomplete tails.
- Added transaction enumeration as a separate shared report and CLI command for ordered HTTP and TLS transaction rows.
- Extended TLS transaction rows so individual handshake rows now reflect certificate and finished progression, not only client-hello/server-hello pairing.
- Extended shared filtering with `service`, `ip`, and `endpoint` clauses, and aligned `host` matching so it also matches packet addresses.
- Extended transaction rows with structured HTTP metadata fields and richer TLS detail fields so automation and future UI work do not need to scrape summary strings.
- Extended HTTP transaction parsing to handle chunked bodies and expose transfer semantics explicitly.
- Extended TLS transaction parsing to expose ALPN, certificate-message counts, and alert records alongside existing SNI and handshake progression details.
- Extended stream summaries with session-level transaction timelines and TLS alert summaries so long-lived connections expose more than aggregate counters.
- Changed stream timelines from paired transaction summaries to ordered session-event timelines, and added explicit HTTP pipelining notes when multiple requests are outstanding.
- Replaced the flat shared filter parser with a boolean expression parser supporting `&&`, `||`, `!`, parentheses, word-form operators, and backward-compatible comma-AND syntax.
- Extended shared filtering with field-aware HTTP, DNS, and TLS predicates so the CLI can filter on packet-decoded application metadata instead of only coarse protocol/host keys.
- Extended shared filtering again with comparison operators (`!=`, `>`, `>=`, `<`, `<=`) and substring matching (`~=`) so field-aware predicates can express ranges and contains-style analyst workflows.
- Extended shared filtering further with additional HTTP/DNS/TLS fields and case-insensitive text matching for exact and contains comparisons.
- Extended shared filtering into stream- and transaction-level row predicates so `streams` and `transactions` can be filtered after packet selection with analysis-aware keys.
- Extended row-level filtering again with derived stream/session predicates such as pipelining, timeline presence, reassembly-issue detection, TLS counters and alert labels, plus HTTP/TLS transaction aliases and completeness/alert predicates.
- Extended the shared packet decoding and stats flow to common PCAPNG packets.
- Added CLI integration smoke tests that execute the compiled binary against sample PCAP and PCAPNG captures.
- Added continuity docs:
  - repo map
  - architecture overview
  - feature parity matrix
  - task recipes
  - roadmap
  - CLI overview
  - desktop overview
  - ADR-0001 for the workspace decision
- Captured a public-alpha technical gap map in `docs/roadmap.md` with priority ordering and a first 4-week execution sequence.

### Current limitations

- CLI argument parsing and the interactive shell both use the standard library, not `clap` or a TUI framework, to avoid adding dependency friction in the first pass.
- Live capture now routes through a shared `capture-engine` crate, but still relies on a `tcpdump`-style backend and does not yet provide full cross-platform provider coverage or first-class permission troubleshooting UX.
- PCAPNG support currently covers section header, interface description, and enhanced packet blocks; other block types are still limited.
- Packet detail decoding now includes a byte-range-aware basic field tree plus early DNS, HTTP/1.1, and TLS handshake support, but protocol coverage is still limited.
- Capture stats are summary-only.
- Conversation summaries now track basic request/response state, stream summaries now perform TCP payload reassembly for HTTP transactions and TLS handshake records with explicit session-state, repeated TLS handshake cycles, out-of-order, retransmission, gap reporting, ordered session-event timelines, HTTP pipelining notes, and TLS alert summaries, and transaction reports now expose ordered HTTP/TLS rows plus structured HTTP/TLS metadata including chunked HTTP handling and TLS ALPN/alerts, but deeper TLS correlation is still limited.
- Shared filtering now covers packet-level protocol/service, port, IP, endpoint, host, and a broader set of HTTP/DNS/TLS field predicates, plus row-level stream and transaction predicates including derived session and protocol aliases, all with boolean grouping, exact/contains/range operators, case-insensitive text matching, and negation, but is still intentionally narrow and does not yet include full display-filter parity.
- Save/export currently writes PCAP output only and does not yet include dedicated stream or transaction export formats.
- No desktop hex-highlighting or byte-range-driven UI exists yet.
- Packet timestamps are surfaced raw from the file and are not yet normalized into wall-clock formatting helpers.
- Desktop app is still a documented placeholder.

### Recommended next move

Finish capture-engine v1 parity and protocol-depth hardening (DNS/HTTP/TLS malformed paths plus TCP reassembly edge cases), then complete save/export breadth beyond filtered PCAP output and move into benchmark-driven performance work and the desktop vertical slice.

### Handoff checkpoint

- Current test baseline is green with `cargo test` from workspace root.
- CLI scripting contracts are now stable for automation:
  - JSON reports include top-level `schema_version: "v1"`.
  - stderr error codes are stable: `[ISCLI_USAGE]` (exit `2`) and `[ISCLI_RUNTIME]` (exit `1`).
- Suggested first task for next session:
  - Continue roadmap item #1 by extending capture-engine provider coverage and permission UX (especially Windows/Npcap edge handling) through shared `capture-engine` + `app-services`.
- Suggested second task:
  - Continue roadmap item #5 by expanding save/export beyond PCAP-only output and adding explicit stream/transaction export targets.
