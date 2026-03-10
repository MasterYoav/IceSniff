# IceSniff Instructions

This document is intended to preserve the current project direction and working assumptions so development can continue in another environment without losing context.

## Project Name

**IceSniff**

The name was chosen after rejecting alternatives with obvious software-name collisions. The project identity should remain centered around a modern, open-source, cross-platform packet analyzer with a cleaner user experience than traditional tools.

## Project Goal

Build a modern, open-source alternative to Wireshark with:

- a polished desktop application
- a fully capable CLI
- strict feature parity between desktop and CLI
- strong cross-platform support
- a documentation-first engineering style
- a codebase that is easy to understand, explain, and extend

The goal is not to clone every Wireshark feature immediately. The goal is to create a strong, modern foundation that is pleasant to use and pleasant to contribute to.

## Product Positioning

IceSniff should aim to occupy the gap between:

- extremely powerful but intimidating / visually dated tools
- more modern-looking tools that are not truly open-source or rely on subscription models

IceSniff should be:

- local-first
- fully open-source
- approachable
- technically serious
- contributor-friendly

## Hard Requirements

### 1. Shared engine
There must be one shared Rust core.

That core should implement the real product logic:
- capture
- parsing
- filtering
- packet inspection
- analysis
- file IO
- stream / conversation logic
- export behavior

The desktop app and CLI must both sit on top of this shared engine.

### 2. Desktop and CLI feature parity
This is a hard requirement.

The CLI must not be a reduced helper tool, and the desktop app must not contain hidden behavior that only exists in UI code.

Whenever a new feature is added, it should first be thought of as a shared capability. The GUI and CLI should then expose that capability in their own presentation layer.

### 3. Cross-platform support
The project should target:
- macOS
- Windows
- Linux

This applies to both:
- the desktop application
- the CLI

The CLI should work naturally in:
- macOS shells
- Linux shells
- Windows PowerShell
- Windows Command Prompt

### 4. Documentation-first standard
Documentation is part of the product.

The project should be heavily documented from the beginning so it is:
- easy to onboard into
- easy to maintain
- easy to explain
- easy for open-source contributors to work on
- easy for AI coding tools to navigate safely

### 5. AI-tool-friendly structure
The repository should be intentionally organized so tools like Claude Code and Codex can understand and modify it safely.

This means:
- explicit names
- small focused modules / crates
- clear architectural boundaries
- predictable folder structure
- public APIs documented with doc comments
- task recipes and repo maps included in docs

## Confirmed Technology Choices

### Language and engine
- **Rust**

Rust was chosen because it is an excellent fit for:
- performance-sensitive packet work
- protocol parsing
- memory safety
- cross-platform binaries
- a shared core used by both desktop and CLI

### Desktop shell
- **Tauri 2**

Tauri was chosen because it provides:
- good Rust integration
- lightweight desktop packaging
- cross-platform support for macOS, Windows, and Linux

### Desktop UI
- **Svelte**

Svelte was chosen because the project does not require extreme frontend complexity and should remain approachable. It should provide enough structure for building a polished interface with basic modern UX elements such as:
- buttons
- toolbars
- sidebars
- panes
- filters
- simple transitions
- polished layout work

The main engineering effort should go into the packet analysis engine rather than frontend framework complexity.

## Recommended High-Level Architecture

The architecture should be centered around one shared core and two thin shells.

### Shared core crates
Recommended crate breakdown:

- `capture-engine`
  - live capture
  - interface enumeration
  - capture session lifecycle
  - pcap / pcapng read-write integration

- `parser-core`
  - decoding packets into a normalized internal model
  - field tree generation
  - byte-range mapping for hex highlighting

- `protocol-dissectors`
  - protocol-specific parsing and display logic
  - organized by protocol

- `filter-engine`
  - shared filtering language
  - protocol presets
  - text / structured search behavior

- `analysis-core`
  - streams
  - conversations
  - endpoints
  - packet statistics

- `app-services`
  - high-level use cases used by both interfaces
  - open file
  - save file
  - start capture
  - stop capture
  - inspect packet
  - follow stream
  - export packets
  - list endpoints / conversations

- `session-model`
  - stable data model shared across clients

- `file-io`
  - file import/export helpers where separation makes sense

- `output-formatters`
  - CLI textual and JSON output formatting

### Client shells
- `apps/desktop`
  - Tauri 2 + Svelte
  - visual shell and UI state

- `apps/cli`
  - Rust CLI using `clap`
  - command surface on top of shared services

## Core Architectural Rules

### Rule 1: shared capability first
When a feature is added, decide whether it is:
- a shared capability
- a presentation detail

Examples:
- packet filtering semantics = shared capability
- follow TCP stream = shared capability
- desktop timeline visualization = desktop presentation
- CLI JSON output = CLI presentation

### Rule 2: no duplicate business logic
Business logic must not diverge between desktop and CLI.

### Rule 3: clear module boundaries
Each crate should have a narrow, obvious responsibility.

### Rule 4: boring names are better than clever names
Clarity beats novelty in code organization.

## Initial Product Scope

The first meaningful version should support a thin but real vertical slice.

### Minimum viable functionality
- choose a network interface
- start live capture
- stop live capture
- open capture files
- save capture files
- display packet list
- inspect one packet in detail
- show parsed field tree
- show raw bytes / hex view
- basic packet filtering
- simple stats

### MVP protocol support
Recommended initial protocol support:
- Ethernet
- IPv4
- IPv6
- TCP
- UDP
- ICMP
- ARP
- DNS
- HTTP/1.1
- TLS handshake metadata

This is enough to make the project genuinely useful without trying to decode every protocol on earth immediately.

## Differentiation Goals

IceSniff should not just be “Wireshark with a prettier window.”

It should try to improve the experience of common tasks.

### Areas to win on
- cleaner onboarding
- better defaults
- more understandable packet inspection flow
- faster path from packet list to insight
- easier filtering
- more approachable UX for students and developers
- easier contributor story

### Strong future differentiators
- follow stream UX that feels much clearer
- protocol timelines
- request-response linking for common protocols
- issue highlighting (e.g. resets, DNS failures, suspicious handshakes)
- beginner mode vs advanced mode
- rule-based “explain this packet” assistance

## Suggested Repository Structure

```text
icesniff/
  apps/
    desktop/
    cli/
  crates/
    capture-engine/
    parser-core/
    protocol-dissectors/
    filter-engine/
    analysis-core/
    app-services/
    session-model/
    file-io/
    output-formatters/
  docs/
    architecture/
    protocols/
    cli/
    desktop/
    contributing/
    decisions/
    tutorials/
  examples/
  fixtures/
  scripts/
  assets/
```

## Required Documentation Set

The repo should eventually include at least:

- `README.md`
- `CONTRIBUTING.md`
- `docs/repo-map.md`
- `docs/architecture/overview.md`
- `docs/feature-parity-matrix.md`
- `docs/task-recipes.md`
- `docs/protocols/`
- `docs/cli/`
- `docs/desktop/`
- `docs/decisions/` for ADRs

### Especially important docs
- **repo map**: explains the entire codebase layout
- **task recipes**: how to add a protocol, command, packet column, etc.
- **feature parity matrix**: ensures GUI and CLI stay aligned
- **ADRs**: explains why major architectural decisions were made

## Suggested Development Order

The safest implementation path is:

### Phase 1: shared core + CLI first
Build the shared engine and expose it through a usable CLI.

Why first?
Because a CLI forces the core to remain honest and prevents business logic from being buried in UI code.

Recommended first CLI milestones:
- open a pcap file
- list packets
- inspect a selected packet
- filter packets
- show basic stats

### Phase 2: live capture support
Add live capture through the shared core and expose it through CLI commands.

### Phase 3: desktop application
Build the Tauri + Svelte desktop UI on top of the already-established shared services.

### Phase 4: parity hardening
As desktop features grow, continuously ensure they are also represented in CLI form where applicable.

## UX Direction for Desktop

The desktop app should feel modern, clean, and fast without unnecessary visual complexity.

Important UX values:
- not intimidating on first launch
- sane empty states
- smooth packet browsing
- readable typography
- resizable panes
- clear filter controls
- protocol color rules
- keyboard shortcuts
- command palette later if useful

The UI should be polished but not overdesigned.
This is a professional network tool, not a motion-design experiment.

## CLI Direction

The CLI should be a real first-class interface.

It should support:
- direct human use
- scripting
- machine-readable output
- parity with shared engine capabilities where practical

It should eventually provide:
- concise table output
- detailed packet inspection output
- JSON output mode
- filtering options
- stats output
- capture commands
- file operations

## Principles for Contributions and Maintenance

The codebase should remain:
- readable
- modular
- testable
- well documented
- predictable
- easy to explain

Avoid designs that are overly clever or difficult to trace.

When in doubt:
- choose the more explicit API
- choose the more maintainable structure
- choose the design that a new contributor can understand fastest

## Summary of Confirmed Decisions

The following decisions are currently locked unless there is a strong reason to revisit them:

- project name: **IceSniff**
- fully open-source direction
- desktop app + CLI
- strict feature parity goal between both interfaces
- shared Rust core
- desktop stack: **Tauri 2 + Svelte**
- cross-platform goal: macOS, Windows, Linux
- documentation-first workflow
- AI-tool-friendly repo structure and documentation

## Immediate Next Step Recommendation

When resuming the project in a coding environment, the first concrete step should be:

1. initialize a monorepo
2. create the shared Rust workspace
3. add the CLI app
4. add placeholder crates for the core architecture
5. write the initial repo docs
6. implement the first vertical slice through CLI first

That is the cleanest starting point for turning the project from planning into a real codebase.
