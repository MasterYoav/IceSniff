# Roadmap

## Completed foundation

- initial Rust workspace created
- CLI app scaffolded
- persistent interactive CLI shell added with a current-capture session model
- shell-based live capture added through the system `tcpdump` tool with temp-PCAP handoff back into the current session
- shared `capture-engine` crate added and wired into CLI live-capture lifecycle
- shared crates for service, model, file IO, and output formatting added
- documentation set started
- first vertical slice implemented for capture metadata inspection
- shared PCAP packet listing implemented through the CLI
- shared PCAP packet detail inspection implemented through the CLI
- shared PCAP capture stats and JSON CLI output implemented
- shared PCAPNG packet listing, packet detail, and stats implemented for section/interface/enhanced-packet flows
- parser and protocol crates introduced so `file-io` no longer owns packet decoding
- basic field-tree packet inspection added on top of the parser/dissector layer
- byte-range-aware field nodes added to packet inspection output
- application-layer inspection added for DNS, HTTP/1.1, and TLS handshake metadata
- shared filtering added for packet listing and capture stats
- packet-list derived columns added for source, destination, protocol, and info
- shared bidirectional conversation summaries added through the CLI
- shared conversation summaries extended with service guessing and request/response counts
- shared stream summaries added with client/server roles and basic transaction matching
- shared stream summaries extended with TCP reassembly for HTTP transactions and TLS handshake records
- shared stream summaries extended with retransmission/overlap/gap notes and multi-message HTTP counting
- shared stream summaries extended with explicit out-of-order segment handling and notes
- shared stream summaries extended with TCP session-state reporting from SYN/FIN/RST flags
- shared stream summaries extended with TLS handshake progression state and reset-aware interpretation
- shared stream summaries extended with repeated TLS handshake cycle tracking on long-lived connections
- shared transaction enumeration added for reassembled HTTP request/response rows and TLS handshake rows
- shared transaction enumeration extended with TLS certificate/finished progression per handshake row
- shared transaction enumeration extended with structured HTTP header/body metadata and TLS record-version/SNI/message-list details
- shared transaction enumeration extended with chunked HTTP body handling and TLS ALPN/alert/certificate-count details
- shared stream summaries extended with session-level transaction timelines and TLS alert summaries
- shared stream summaries extended with ordered session-event timelines and HTTP pipelining notes
- shared filter semantics extended from flat clauses to boolean expressions with grouping and negation
- shared filter semantics extended with field-aware HTTP, DNS, and TLS predicates
- shared filter semantics extended with comparison and contains operators for analyst-facing predicates
- shared filter semantics extended with additional HTTP/DNS/TLS fields and case-insensitive text matching
- shared filter semantics extended with service, ip, and endpoint matching plus address-aware host matching
- shared filter semantics extended into stream- and transaction-level row filters for analysis outputs
- shared row filters extended with derived session flags, richer TLS counters, and HTTP/TLS alias predicates for analysis outputs
- shared save/export service added for writing filtered capture output to PCAP
- CLI integration parity tests added to verify text and JSON totals stay aligned across core shared commands
- stable JSON schema tagging added across CLI `--json` reports (`schema_version: v1`)
- stable CLI error-code prefixes and exit-status mapping added for scripting ergonomics
- committed golden fixtures added for PCAP, PCAPNG, and malformed capture containers with fixture-backed parsing tests
- CLI integration smoke tests added

## Public alpha gap map (priority order)

1. Capture engine (live traffic) - critical
   - Cross-platform interface enumeration and start/stop capture.
   - Permission/error handling UX including Npcap/libpcap differences.
   - Consistent live-session model shared by CLI and desktop.
2. Filter engine v2 - critical
   - Composable expression language (`and`/`or`/`not`, parentheses).
   - Deterministic parser and evaluator with tests.
   - Shared semantics across list/stats/conversations/streams.
3. Protocol depth (practical coverage) - critical
   - Harden DNS/HTTP/TLS for common analyst workflows.
   - Improve TCP reassembly edge cases (out-of-order, retransmits, gaps).
   - Improve malformed and truncated packet handling.
4. Desktop vertical slice (Tauri + Svelte) - high
   - Packet table, details pane, and hex/bytes pane.
   - Selection synchronization between packet rows, decoded fields, and byte ranges.
   - Keep desktop thin: no business logic in UI.
5. Save/export workflows - high
   - Save filtered captures.
   - Export selected packets and streams.
   - Reuse shared output behavior via services.
6. Test/fixtures expansion - high
   - Golden fixtures for PCAP, PCAPNG, and malformed captures.
   - Integration tests comparing CLI text and JSON outputs.
   - Regression tests for filter semantics and stream reconstruction.
7. Performance baseline - medium-high
   - Profile parse/list/stats on medium and large captures.
   - Add low-cost indexes/caching where benchmarks justify it.
8. CLI ergonomics polish - medium
   - Move to `clap`.
   - Standardize error codes/messages for scripting.
   - Add stable JSON schema versioning.
9. Observability inside engine - medium
   - Structured internal logging/events for debug mode.
   - Better diagnostics for parser/filter failures.
10. Security hardening posture - medium
    - Treat capture files as untrusted input.
    - Add parser/dissector fuzzing.
    - Keep panic-safe boundaries and defensive decoding.

## Current execution status

- Started item #1 by adding a shared `crates/capture-engine` crate and moving CLI live-capture process management into that shared layer.
- Added platform-aware default capture-tool resolution, explicit backend abstraction (`tcpdump`-style + `dumpcap`), and driver-aware capture error mapping (permission vs backend/runtime issues), plus stricter capture-stop validation and runtime-aware shell status reporting.
- Moved capture session orchestration behind `crates/app-services` so CLI capture commands no longer call `capture-engine` directly.
- Current backend still relies on external capture tooling and remains partial while deeper provider coverage and richer permission UX are implemented.
- Started item #5 with shared save/export plumbing: `save` now writes packet-filtered or stream-filter-selected output to PCAP through shared services and shared stream semantics.
- Extended item #5 into the desktop prototype by exposing filtered PCAP save through Tauri commands/UI controls.
- Extended item #5 again with desktop analysis-row export actions for conversations/streams/transactions in JSON and CSV formats.
- Started item #4 with a speed-first `apps/desktop` Tauri 2 + Svelte prototype wired to shared `inspect`, `stats`, `list`, and `show-packet` services, including packet-table selection, field-tree display, and byte-range-aware hex highlighting.
- Extended item #4 with desktop stream and transaction analysis panels driven by shared `streams` and `transactions` services plus analysis-filter inputs.
- Extended item #4 again with desktop stream/transaction drill-down details, native file-picker actions, and local recent-capture/last-state persistence.
- Extended item #4 further with desktop conversations panel parity, including selected-row drill-down details.
- Extended item #4 with focus-navigation controls from selected analysis rows back into capture-level filtered views.
- Extended item #4 with cross-panel preselection (conversation -> stream/transaction) and structured desktop JSON command responses.
- Extended item #1 into the desktop prototype with shared-service live-capture controls (interfaces/start/status/stop), live packet/stats polling, and stop-to-auto-load capture handoff.
- Started item #8 schema stabilization by adding top-level JSON schema version tags across CLI report output.
- Started item #8 scripting ergonomics by adding stable CLI error-code prefixes and consistent exit-status categories.

## 4-week execution sequence

1. Week 1: finish capture-engine v1
   - Add provider abstraction and OS-specific defaults (libpcap/tcpdump and Windows Npcap-compatible tool pathing).
   - Improve permission/tool-missing guidance in CLI shell output.
2. Week 2: protocol depth + fixtures
   - Harden DNS/HTTP/TLS malformed/truncated paths.
   - Add committed golden fixtures (PCAP + PCAPNG + malformed).
3. Week 3: save/export + performance baseline
   - Expand shared save/export beyond the initial `save` PCAP path to cover additional export targets and workflows.
   - Capture baseline benchmarks and apply only benchmark-backed optimizations.
4. Week 4: desktop vertical slice bootstrapping
   - Implement packet table + detail + hex pane with byte-range selection sync using shared services.
