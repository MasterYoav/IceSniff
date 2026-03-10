# Roadmap

## Completed foundation

- initial Rust workspace created
- CLI app scaffolded
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
- CLI integration smoke tests added

## Next implementation steps

1. Replace manual CLI parsing with `clap` when dependency management is in place.
2. Extend shared filter semantics beyond `protocol`, `port`, and `host` into compound analyst workflows.
3. Replace the current packet-based HTTP and TLS stream heuristics with true reassembled stream and transaction analysis.
4. Extend PCAPNG support beyond the current section/interface/enhanced-packet flow into broader block coverage and richer timestamp option handling.
5. Expand the current byte-range-aware field tree and early DNS/HTTP/TLS support into richer protocol-specific inspection and deeper protocol coverage.
6. Replace inline sample generation with committed fixtures and broader CLI snapshot-style tests.
7. Add the remaining planned crates: capture, analysis.
8. Bootstrap `apps/desktop` once shared services can drive a real workflow.
