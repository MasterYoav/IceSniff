#!/bin/sh
set -eu

SCRIPT_URL="https://raw.githubusercontent.com/MasterYoav/IceSniff/main/apps/cli/install/install.sh"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$SCRIPT_URL" | sh
  exit 0
fi

if command -v wget >/dev/null 2>&1; then
  wget -qO- "$SCRIPT_URL" | sh
  exit 0
fi

echo "IceSniff installer requires curl or wget." >&2
exit 1
