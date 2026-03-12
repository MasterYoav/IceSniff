#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DERIVED_DATA="${ICESNIFF_DERIVED_DATA:-/tmp/icesniff-macos-derived-data}"
BUILD_DIR="$DERIVED_DATA/Build/Products/Release"
APP_NAME="${ICESNIFF_APP_NAME:-IceSniffMac}"
APP_PATH="$BUILD_DIR/$APP_NAME.app"
ARCHIVE_DIR="${ICESNIFF_RELEASE_DIR:-$APP_ROOT/build/release}"
ZIP_PATH="$ARCHIVE_DIR/$APP_NAME.zip"

"$SCRIPT_DIR/sync-bundled-cli.sh"

mkdir -p "$ARCHIVE_DIR"

echo "==> Building macOS release app"
cd "$APP_ROOT"
xcodebuild \
  -scheme "$APP_NAME" \
  -configuration Release \
  -derivedDataPath "$DERIVED_DATA" \
  -destination "platform=macOS" \
  build

if [[ ! -d "$APP_PATH" ]]; then
  echo "Expected app bundle not found at $APP_PATH" >&2
  exit 1
fi

if [[ -n "${ICESNIFF_SIGNING_IDENTITY:-}" ]]; then
  echo "==> Signing app bundle"
  codesign \
    --force \
    --deep \
    --options runtime \
    --timestamp \
    --sign "$ICESNIFF_SIGNING_IDENTITY" \
    "$APP_PATH"
fi

echo "==> Creating notarization zip"
ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"

if [[ -n "${ICESNIFF_NOTARY_KEYCHAIN_PROFILE:-}" ]]; then
  echo "==> Submitting for notarization"
  xcrun notarytool submit "$ZIP_PATH" \
    --keychain-profile "$ICESNIFF_NOTARY_KEYCHAIN_PROFILE" \
    --wait
  echo "==> Stapling notarization ticket"
  xcrun stapler staple "$APP_PATH"
fi

echo "==> Release app ready at $APP_PATH"
echo "==> Zip ready at $ZIP_PATH"
