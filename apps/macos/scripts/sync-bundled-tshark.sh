#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_APP="${ICESNIFF_WIRESHARK_APP:-/Applications/Wireshark.app}"
DEST_ROOT="$APP_ROOT/Sources/IceSniffMac/Resources/BundledTShark"
DEST_APP="$DEST_ROOT/Wireshark.app"
VERSION_FILE="$DEST_ROOT/WIRESHARK_VERSION.txt"

if [[ ! -d "$SOURCE_APP" ]]; then
  echo "Wireshark.app not found at $SOURCE_APP" >&2
  echo "Install Wireshark.app or set ICESNIFF_WIRESHARK_APP to a valid bundle before packaging." >&2
  exit 1
fi

TSHARK_BIN="$SOURCE_APP/Contents/MacOS/tshark"
if [[ ! -x "$TSHARK_BIN" ]]; then
  echo "Expected tshark executable not found at $TSHARK_BIN" >&2
  exit 1
fi

mkdir -p "$DEST_ROOT"
rm -rf "$DEST_APP"
ditto "$SOURCE_APP" "$DEST_APP"

"$TSHARK_BIN" --version | head -n 1 > "$VERSION_FILE"

echo "==> Bundled tshark runtime refreshed at $DEST_APP"
echo "==> Recorded bundled tshark version in $VERSION_FILE"
