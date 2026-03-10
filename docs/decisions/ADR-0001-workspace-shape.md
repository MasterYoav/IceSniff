# ADR-0001: Establish CLI-First Shared Rust Workspace

## Status

Accepted on 2026-03-10.

## Context

IceSniff started as a planning-only repository. The project charter requires:

- one shared Rust engine
- desktop and CLI parity
- documentation-first development
- a structure that AI coding tools can navigate safely

The first implementation step needed to create a real codebase without prematurely burying business logic inside a desktop shell.

## Decision

Create a Rust workspace now and implement the first thin vertical slice through the CLI first.

The initial members are:

- `apps/cli`
- `crates/app-services`
- `crates/file-io`
- `crates/output-formatters`
- `crates/session-model`

The desktop app remains a documented placeholder until the shared services support more than metadata inspection.

## Consequences

- The repository now has an executable path that demonstrates the intended dependency direction.
- Shared service boundaries are established before UI complexity arrives.
- The first slice is intentionally narrow and will need follow-up work for real packet parsing and listing.

