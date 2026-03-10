# Desktop Placeholder

`apps/desktop` is reserved for the future Tauri 2 + Svelte application.

The desktop shell is intentionally not bootstrapped yet. The current phase is CLI-first so the shared Rust engine and service boundaries are defined before UI code exists.

When this app is initialized, it should consume shared capabilities from the Rust workspace rather than implementing packet logic in frontend code.

