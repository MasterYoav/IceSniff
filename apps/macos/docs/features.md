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

## AI Chat

The mac app includes a collapsible AI chat panel on the right side of the window.

Current AI chat capabilities:

- ask general capture-analysis questions
- ask questions about the currently selected packet
- send on Enter
- insert a newline with Shift+Enter or Option+Enter
- keep provider configuration inside the main `Settings` screen

When a packet is selected, the assistant receives:

- the selected packet index
- packet summary metadata from the packet list
- the full selected packet JSON from the detail pane

Current provider options:

- `OpenAI · GPT-4.1` through an OpenAI API key
- `Anthropic · Claude Sonnet 4` through an Anthropic API key
- `Google · Gemini 2.5 Pro` through a Google API key
- `OpenAI · Codex` through the local `codex` CLI session
- `Anthropic · Claude Code` through the local `claude` CLI session

Current AI settings live in:

- the main `Settings` section

Not currently supported:

- hosted shared free AI models in the public build
- direct ChatGPT subscription login as API access
- cloud-synced AI conversations

## Profile Cloud Sync

The `Profile` section is now live.

Users can:

- sign in with GitHub
- sign in with Google
- sign out
- see provider identity and avatar
- keep theme, font family, and font size saved locally on the current Mac

Current provider scope:

- Google
- GitHub

Not currently supported:

- Apple sign-in
- cloud-backed preference sync in the public build

## Visual Design

The mac app currently uses:

- a native SwiftUI layout
- a molded sidebar
- a collapsible right-side AI rail
- a section-title header integrated with the main content area
- theme-aware gradients and frosted cards

## Engineering Notes

The UI is native, but the feature logic is backend-driven.

That means:

- protocol additions should usually happen in Rust first
- new analysis reports should usually happen in Rust first
- Swift should mostly own presentation, state wiring, and macOS-native workflows
