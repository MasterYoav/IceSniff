# Desktop Overview

The desktop application will be built with Tauri 2 and Svelte.

## Current state

The desktop app is intentionally deferred while shared Rust services are established through the CLI.

## Initial desktop goals

- packet list
- packet detail panes
- field tree
- raw bytes and hex view
- filter bar
- simple stats and capture controls

## Design rule

Desktop code should focus on presentation, layout, state orchestration, and user interaction. Parsing, filtering, and capture behavior must remain in shared Rust crates.

