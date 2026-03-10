# CLI Overview

The CLI is the first production interface for IceSniff.

## Current command surface

- `inspect <capture-file>`: reads a file through shared services and prints basic metadata
- `list <capture-file> [limit] [--filter <expr>]`: enumerates PCAP and PCAPNG packet records through shared services
- `show-packet <capture-file> <packet-index>`: decodes one PCAP or PCAPNG packet through shared services
- `stats <capture-file> [--filter <expr>]`: reports packet counts, byte totals, and protocol-family summaries
- `conversations <capture-file> [--filter <expr>]`: summarizes bidirectional flows across shared decoded packets

All commands support `--json` for machine-readable output.

`list` currently includes derived analyst-facing columns:

- source
- destination
- protocol
- info

The shared filter engine currently supports:

- `protocol=dns|http|tls|tcp|udp|icmp|ipv4|arp`
- `port=<number>`
- `host=<name-or-address>`

`conversations` currently groups traffic into bidirectional rows using:

- application or transport protocol
- service guesses built from application and well-known port metadata
- normalized endpoint pairs
- packet counts
- directional packet counts
- request and response counts for recognized DNS, HTTP, and TLS handshake traffic
- captured-byte totals
- first and last packet indexes

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
