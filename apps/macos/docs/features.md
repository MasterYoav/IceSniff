# IceSniffMac Feature Guide

This document lists the current user-facing features of the mac app in plain language.

## Navigation

The app currently includes these sections:

- Packets
- Stats
- Conversations
- Streams
- Transactions
- Profile
- Settings

## Capture Workflows

### Open Capture

Users can open an existing capture file and inspect it inside the native app.

### Live Capture

Users can:

- select a capture interface
- start sniffing
- stop sniffing
- see packets appear live

### Save Capture

Users can save:

- the whole capture
- only the filtered packets when a filter is active

## Packet Analysis

### Packet List

The app shows:

- packet number
- endpoints
- protocol
- timing/info summaries

### Packet Detail

The app supports:

- packet JSON/detail inspection
- decoded field inspection
- packet context actions

### Context Detail Window

Right-click packet detail includes:

- packet number
- timestamp
- size
- link/network/transport/application layers
- decoded fields list
- bytes/hex metadata view for selected fields

## Filter Experience

The app supports comfort-first filters.

Examples that normalize automatically:

- `http`
- `HTTP`
- `tcp`
- `443`
- `udp and 443`
- `udp & 443`
- `udp && 443`

Explicit expressions still work too:

- `protocol=http`
- `port=443`

## Analysis Sections

The app exposes backend analysis for:

- stats
- conversations
- streams
- transactions

These are powered by the Rust backend, not reimplemented in Swift.

## Personalization

Users can change:

- app theme
- app font family
- app font size

Current theme set:

- Default Dark
- Default Light
- Ocean
- Ember
- Forest

Current font set:

- System
- Rounded
- Serif
- Monospaced

## Visual Design

The mac app currently uses:

- a native SwiftUI layout
- a molded sidebar
- a section-title header integrated with the main content area
- theme-aware gradients and frosted cards

## Engineering Notes

The UI is native, but the feature logic is backend-driven.

That means:

- protocol additions should usually happen in Rust first
- new analysis reports should usually happen in Rust first
- Swift should mostly own presentation, state wiring, and macOS-native workflows
