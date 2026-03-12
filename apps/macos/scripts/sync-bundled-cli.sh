#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_WORKSPACE_ROOT="${ICESNIFF_RUST_WORKSPACE_ROOT:-$APP_ROOT/rust-engine}"
TARGET_DIR="${ICESNIFF_CARGO_TARGET_DIR:-/tmp/icesniff-macos-release-target}"
PROFILE="${ICESNIFF_CLI_PROFILE:-release}"

if [[ ! -f "$RUST_WORKSPACE_ROOT/Cargo.toml" || ! -f "$RUST_WORKSPACE_ROOT/apps/cli/Cargo.toml" ]]; then
  echo "Local mac Rust workspace not found at $RUST_WORKSPACE_ROOT" >&2
  exit 1
fi

if [[ "$PROFILE" == "release" ]]; then
  BUILD_ARGS=(build --release -p icesniff-cli -p icesniff-capture-helper)
  CLI_PATH="$TARGET_DIR/release/icesniff-cli"
  HELPER_PATH="$TARGET_DIR/release/icesniff-capture-helper"
else
  BUILD_ARGS=(build -p icesniff-cli -p icesniff-capture-helper)
  CLI_PATH="$TARGET_DIR/debug/icesniff-cli"
  HELPER_PATH="$TARGET_DIR/debug/icesniff-capture-helper"
fi

echo "==> Building bundled CLI and capture helper ($PROFILE)"
cd "$RUST_WORKSPACE_ROOT"
CARGO_TARGET_DIR="$TARGET_DIR" cargo "${BUILD_ARGS[@]}"

if [[ ! -x "$CLI_PATH" ]]; then
  echo "Bundled CLI build did not produce $CLI_PATH" >&2
  exit 1
fi

if [[ ! -x "$HELPER_PATH" ]]; then
  echo "Bundled capture helper build did not produce $HELPER_PATH" >&2
  exit 1
fi

DEST_DIR="$APP_ROOT/Sources/IceSniffMac/Resources/BundledCLI"
CLI_DEST_PATH="$DEST_DIR/icesniff-cli"
HELPER_DEST_PATH="$DEST_DIR/icesniff-capture-helper"
mkdir -p "$DEST_DIR"
cp "$CLI_PATH" "$CLI_DEST_PATH"
chmod +x "$CLI_DEST_PATH"
cp "$HELPER_PATH" "$HELPER_DEST_PATH"
chmod +x "$HELPER_DEST_PATH"

echo "==> Bundled CLI refreshed at $CLI_DEST_PATH"
echo "==> Bundled capture helper refreshed at $HELPER_DEST_PATH"
