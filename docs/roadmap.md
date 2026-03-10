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
- CLI integration smoke tests added

## Next implementation steps

1. Replace manual CLI parsing with `clap` when dependency management is in place.
2. Extend shared filter semantics beyond `protocol`, `port`, and `host` into compound analyst workflows.
3. Extend PCAPNG support beyond the current section/interface/enhanced-packet flow into broader block coverage and richer timestamp option handling.
4. Expand the current byte-range-aware field tree and early DNS/HTTP/TLS support into richer protocol-specific inspection and deeper protocol coverage.
5. Replace inline sample generation with committed fixtures and broader CLI snapshot-style tests.
6. Add the remaining planned crates: capture, analysis.
7. Bootstrap `apps/desktop` once shared services can drive a real workflow.
