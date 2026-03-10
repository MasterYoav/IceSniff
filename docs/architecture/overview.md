# Architecture Overview

IceSniff is structured around one shared Rust engine with two thin interfaces:

- CLI
- Desktop

## Layers

### Interface layer

This layer is responsible for argument parsing, UI state, and presentation only.

- `apps/cli`
- `apps/desktop`

### Service layer

This layer exposes application use cases in a client-agnostic form.

- `crates/app-services`

Examples:

- inspect capture file
- open capture
- start capture
- stop capture
- export packets

### Domain/model layer

This layer holds stable data structures that can be shared by both interfaces.

- `crates/session-model`

### Infrastructure layer

This layer contains file access and capture-container loading.

- `crates/file-io`
- `crates/capture-engine`

### Parser/dissector layer

This layer turns loaded packet bytes into decoded packet views and shared analysis reports.

- `crates/parser-core`
- `crates/protocol-dissectors`

The parser layer depends on shared models and loaded packet records, not on CLI formatting.

### Presentation formatting layer

This layer contains CLI-specific formatting, not business logic.

- `crates/output-formatters`

## Rules

1. Shared capability first.
2. No duplicate business logic between CLI and desktop.
3. Keep crates narrow and explicit.
4. Prefer stable shared models over ad hoc interface-specific payloads.

## Current State

The current implementation proves the initial dependency direction:

- CLI -> app-services -> file-io/session-model
- CLI -> app-services -> parser-core -> protocol-dissectors/session-model
- CLI -> app-services -> capture-engine
- CLI -> output-formatters -> session-model

That is the baseline shape the rest of the project should preserve.
