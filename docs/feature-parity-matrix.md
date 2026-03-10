# Feature Parity Matrix

This matrix tracks whether a capability exists in the shared engine and how each interface exposes it.

| Capability | Shared service | CLI | Desktop | Notes |
| --- | --- | --- | --- | --- |
| Inspect capture file metadata | Yes | Yes | Planned | Implemented as `inspect <capture-file>` |
| Open capture file | Partial | Partial | Planned | Shared file loading exists for metadata and PCAP/PCAPNG packet records |
| List packets | Yes | Yes | Planned | Implemented as `list <capture-file> [limit] [--filter <expr>]` for PCAP and common PCAPNG enhanced-packet blocks with derived source/destination/protocol/info columns |
| Inspect packet detail | Partial | Partial | Planned | Implemented as `show-packet <capture-file> <packet-index>` for PCAP and common PCAPNG enhanced-packet blocks with minimal Ethernet/ARP/IPv4/TCP/UDP/ICMP decoding, a byte-range-aware field tree, and DNS/HTTP/TLS application metadata |
| Capture stats | Partial | Partial | Planned | Implemented as `stats <capture-file> [--filter <expr>]` for PCAP and common PCAPNG enhanced-packet blocks with summary counts by link/network/transport |
| Basic filtering | Yes | Yes | Planned | Shared semantics currently support `protocol=...`, `port=...`, and `host=...` |
| Conversations | Partial | Partial | Planned | Implemented as `conversations <capture-file> [--filter <expr>]` with bidirectional endpoint normalization, service guessing, directional packet counts, and request/response totals for recognized protocols |
| Streams and transactions | Partial | Partial | Planned | Implemented as `streams <capture-file> [--filter <expr>]` with client/server roles, directional counts, and basic matched transaction counters; HTTP remains packet-based without TCP reassembly |
| Live capture | Planned | Planned | Planned | Requires capture-engine crate |
| Save/export capture | Planned | Planned | Planned | Shared service required |
