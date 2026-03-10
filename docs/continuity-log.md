# Continuity Log

This file is the running project documentary for future sessions.

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

### Current limitations

- CLI argument parsing uses the standard library, not `clap`, to avoid adding dependency friction in the first pass.
- PCAPNG support currently covers section header, interface description, and enhanced packet blocks; other block types are still limited.
- Packet detail decoding now includes a byte-range-aware basic field tree plus early DNS, HTTP/1.1, and TLS handshake support, but protocol coverage is still limited.
- Capture stats are summary-only.
- Conversation summaries now track basic request/response state, but they still do not reassemble streams or correlate full transactions.
- Shared filtering is still intentionally narrow and does not yet include Wireshark-style boolean expressions.
- No desktop hex-highlighting or byte-range-driven UI exists yet.
- Packet timestamps are surfaced raw from the file and are not yet normalized into wall-clock formatting helpers.
- Desktop app is still a documented placeholder.

### Recommended next move

Build reassembled stream and transaction analysis on top of the current conversation summaries, then deepen filter semantics and protocol coverage while keeping the CLI as the reference interface.
