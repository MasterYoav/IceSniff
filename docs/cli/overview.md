# CLI Overview

The CLI is the first production interface for IceSniff.

## Current command surface

- `shell [capture-file]`: starts a persistent interactive session with a current capture context
- `save <source-capture-file> <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`: writes a new PCAP using shared save/export services
- `inspect <capture-file>`: reads a file through shared services and prints basic metadata
- `list <capture-file> [limit] [--filter <expr>]`: enumerates PCAP and PCAPNG packet records through shared services
- `show-packet <capture-file> <packet-index>`: decodes one PCAP or PCAPNG packet through shared services
- `stats <capture-file> [--filter <expr>]`: reports packet counts, byte totals, and protocol-family summaries
- `conversations <capture-file> [--filter <expr>]`: summarizes bidirectional flows across shared decoded packets
- `streams <capture-file> [--filter <expr>] [--stream-filter <expr>]`: summarizes client/server streams and basic transaction counts
- `transactions <capture-file> [--filter <expr>] [--transaction-filter <expr>]`: enumerates parsed HTTP and TLS transactions from shared stream analysis

All commands support `--json` for machine-readable output, with a stable top-level `schema_version` field (`v1`).
CLI stderr now uses stable error-code prefixes for scripting: `[ISCLI_USAGE]` (exit `2`) for argument/usage failures and `[ISCLI_RUNTIME]` (exit `1`) for runtime/service failures.

When started with no command, the CLI now drops into the interactive shell by default.

The shell supports:

- `open <capture-file>`
- `save <output-capture-file> [--filter <expr>] [--stream-filter <expr>]`
- `capture interfaces`
- `capture start [interface]`
- `capture stop`
- `capture status`
- `close`
- `status`
- `mode <text|json>`
- `inspect`
- `list [limit] [--filter <expr>]`
- `show-packet <packet-index>`
- `stats [--filter <expr>]`
- `conversations [--filter <expr>]`
- `streams [--filter <expr>] [--stream-filter <expr>]`
- `transactions [--filter <expr>] [--transaction-filter <expr>]`
- `quit`

Live capture now routes through shared `app-services` orchestration into the `capture-engine` crate, which supports `tcpdump`-style and `dumpcap` backends, writes to a temporary `.pcap`, prints a live packet table with `Id | Time | Source | Destination | Protocol | Info` while capture is active, and switches the shell's current capture to that file when recording stops.

Backend/tool overrides:

- `ICESNIFF_CAPTURE_TOOL`: explicit path or executable name for the capture tool.
- `ICESNIFF_CAPTURE_BACKEND`: optional backend selector (`tcpdump` or `dumpcap`) when tool-name inference is not enough.

`capture status` now prints the effective backend and tool path/name in addition to live state, interface, and capture path.
`save` uses the same shared service path and can apply packet filters (`--filter`) and stream-row filters (`--stream-filter`) before writing output (`--filter` runs first, then `--stream-filter` is evaluated on resulting streams).

`list` currently includes derived analyst-facing columns:

- source
- destination
- protocol
- info

The shared filter engine currently supports:

- `protocol=dns|http|tls|tcp|udp|icmp|ipv4|arp`
- `service=dns|http|tls|tcp|udp|icmp|ipv4|arp`
- `port=<number>`
- `ip=<address>`
- `endpoint=<address:port>`
- `host=<name-or-address>`
- `http.method=<verb>`
- `http.path=<path>`
- `http.kind=<request|response>`
- `http.status=<code>`
- `http.reason=<phrase>`
- `http.host=<host>`
- `dns.question=<name>`
- `dns.question_count=<count>`
- `dns.answer_count=<count>`
- `dns.is_response=<true|false>`
- `tls.handshake_type=<type>`
- `tls.server_name=<name>`
- `tls.record_version=<major.minor>`
- `tls.handshake_length=<bytes>`

Those clauses can now be combined with:

- `&&` or `and`
- `||` or `or`
- `!` or `not`
- parentheses for grouping
- commas as backward-compatible AND separators

Supported clause operators now include:

- `=` exact match
- `!=` inequality
- `>` `>=` `<` `<=` for numeric comparisons
- `~=` for substring contains matching on text fields

Text comparisons are case-insensitive for exact and contains matching.

For `protocol=dns`, `protocol=http`, and `protocol=tls`, the shared filter engine now falls back to well-known ports so fragmented application streams are still included before full packet-local decoding succeeds. `host` also matches packet addresses now, not just application-layer names.

`streams` now also supports row-level analysis filters after packet selection. Current keys include:

- `stream.service`
- `stream.protocol`
- `stream.client`
- `stream.server`
- `stream.state`
- `stream.tls_state`
- `stream.packets`
- `stream.syn`
- `stream.fin`
- `stream.rst`
- `stream.requests`
- `stream.responses`
- `stream.matched`
- `stream.unmatched_requests`
- `stream.unmatched_responses`
- `stream.tls_alert_count`
- `stream.tls_alert`
- `stream.tls_client_hellos`
- `stream.tls_server_hellos`
- `stream.tls_certificates`
- `stream.tls_finished`
- `stream.tls_handshake_cycles`
- `stream.tls_incomplete_handshakes`
- `stream.has_alerts`
- `stream.has_timeline`
- `stream.has_notes`
- `stream.has_reassembly_issues`
- `stream.is_pipelined`
- `stream.client_packets`
- `stream.server_packets`
- `stream.total_bytes`
- `stream.first_packet`
- `stream.last_packet`
- `stream.timeline`
- `stream.note`

`transactions` also supports row-level analysis filters after packet selection. Current keys include:

- `tx.service`
- `tx.protocol`
- `tx.client`
- `tx.server`
- `tx.sequence`
- `tx.state`
- `tx.request_summary`
- `tx.response_summary`
- `tx.note`
- `tx.request.method`
- `tx.request.path`
- `tx.request.host`
- `tx.response.status_code`
- `tx.response.reason_phrase`
- `tx.response.body_bytes`
- `tx.has_request`
- `tx.has_response`
- `tx.complete`
- `tx.has_alerts`
- `tx.http.status_class`
- `tx.http.method`
- `tx.http.path`
- `tx.http.host`
- `tx.http.status`
- `tx.http.reason`
- `tx.http.transfer_semantics`
- `tx.http.transfer_encoding`
- `tx.http.content_type`
- `tx.http.body_bytes`
- `tx.http.header_count`
- `tx.tls.record_version`
- `tx.tls.server_name`
- `tx.tls.alpn`
- `tx.tls.handshake_messages`
- `tx.tls.alerts`
- `tx.tls.certificate_messages`
- `tx.request.record_version`
- `tx.request.server_name`
- `tx.request.alpn`
- `tx.response.certificate_messages`
- `tx.response.alerts`

`conversations` currently groups traffic into bidirectional rows using:

- application or transport protocol
- service guesses built from application and well-known port metadata
- normalized endpoint pairs
- packet counts
- directional packet counts
- request and response counts for recognized DNS, HTTP, and TLS handshake traffic
- captured-byte totals
- first and last packet indexes

`streams` currently builds on top of the same shared decoded packets and adds:

- client and server endpoint roles
- derived TCP session state plus SYN/FIN/RST counts
- directional packet counts
- request and response counts for DNS, reassembled HTTP traffic, and reassembled TLS handshake traffic
- matched and unmatched transaction counters
- TLS handshake progression state plus client hello, server hello, certificate, and finished counts
- TLS alert counts and alert labels across the connection
- repeated TLS handshake cycle counts and incomplete-handshake counts for long-lived connections
- multi-message HTTP counting within one reassembled stream
- ordered session-event timeline entries for HTTP and TLS, preserving reconstructed direction/order semantics
- explicit HTTP pipelining notes when the stream shows multiple outstanding requests
- explicit notes where reassembly detects out-of-order delivery, retransmissions, overlapping segments, sequence gaps, or incomplete protocol records

`transactions` currently builds on top of the same reassembled stream layer and exposes:

- ordered HTTP request/response rows from reconstructed TCP byte streams
- ordered TLS handshake rows from reconstructed client-hello, server-hello, certificate, and finished records
- transaction states for matched and unmatched HTTP rows
- transaction states for TLS progression such as server hello seen, certificate seen, finished seen, or incomplete tail
- structured HTTP details such as method, path, host, header count, status, reason phrase, transfer semantics, and body size
- structured TLS details such as record version, SNI/ALPN when available, certificate-message counts, alert records, and per-side handshake message lists
- propagated reassembly notes so transaction output carries stream-quality warnings

`show-packet` currently includes:

- packet metadata
- layer summaries
- raw bytes
- a basic hierarchical field tree
- byte ranges on field nodes for future hex highlighting
- application-layer metadata for DNS, HTTP/1.1, and TLS handshake packets

## Direction

The CLI should grow into a full interface for:

- opening capture files
- listing packets
- inspecting packet detail
- summarizing capture statistics
- filtering traffic
- showing stats
- starting and stopping live capture
- machine-readable JSON output

## Design rule

The CLI should remain a thin shell over shared capabilities. Command parsing and text formatting belong here; packet logic does not.
