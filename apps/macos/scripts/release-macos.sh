#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DERIVED_DATA="${ICESNIFF_DERIVED_DATA:-/tmp/icesniff-macos-derived-data}"
BUILD_DIR="$DERIVED_DATA/Build/Products/Release"
APP_NAME="${ICESNIFF_APP_NAME:-IceSniffMac}"
ARCHIVE_DIR="${ICESNIFF_RELEASE_DIR:-$APP_ROOT/build/release}"
APP_PATH="$ARCHIVE_DIR/$APP_NAME.app"
ZIP_PATH="$ARCHIVE_DIR/$APP_NAME.zip"
GPL_ARCHIVE_DIR="$ARCHIVE_DIR/gpl-compliance"
ICON_SOURCE="$APP_ROOT/Sources/IceSniffMac/Resources/icon-light.png"
EXECUTABLE_PATH="$BUILD_DIR/$APP_NAME"
RESOURCE_BUNDLE_NAME="${APP_NAME}_${APP_NAME}.bundle"
RESOURCE_BUNDLE_PATH="$BUILD_DIR/$RESOURCE_BUNDLE_NAME"
BUNDLED_TSHARK_PATH="$APP_ROOT/Sources/IceSniffMac/Resources/BundledTShark/Wireshark.app"
THIRD_PARTY_NOTICES_PATH="$APP_ROOT/Sources/IceSniffMac/Resources/ThirdPartyNotices"
CONTENTS_PATH="$APP_PATH/Contents"
MACOS_PATH="$CONTENTS_PATH/MacOS"
RESOURCES_PATH="$CONTENTS_PATH/Resources"
INFO_PLIST_PATH="$CONTENTS_PATH/Info.plist"
PKGINFO_PATH="$CONTENTS_PATH/PkgInfo"
BUNDLE_IDENTIFIER="${ICESNIFF_BUNDLE_IDENTIFIER:-io.icesniff.mac}"
MINIMUM_SYSTEM_VERSION="${ICESNIFF_MINIMUM_SYSTEM_VERSION:-13.0}"
APP_VERSION="${ICESNIFF_APP_VERSION:-1.0.0}"
BUILD_VERSION="${ICESNIFF_BUILD_VERSION:-1}"
WIRESHARK_SOURCE_ARCHIVE="${ICESNIFF_WIRESHARK_SOURCE_ARCHIVE:-}"
ALLOW_MISSING_SUPABASE_CONFIG="${ICESNIFF_ALLOW_MISSING_SUPABASE_CONFIG:-0}"
ENV_FILES=(
  "$APP_ROOT/.env.release.local"
  "$APP_ROOT/.env.release"
  "$APP_ROOT/.env.local"
  "$APP_ROOT/.env"
)

env_file_value() {
  local key="$1"
  local file="$2"

  (
    set -a
    source "$file" >/dev/null 2>&1
    print -r -- "${(P)key-}"
  )
}

resolve_config_value() {
  local key="$1"

  if [[ -n "${(P)key-}" ]]; then
    print -r -- "${(P)key}"
    return
  fi

  local file value
  for file in "${ENV_FILES[@]}"; do
    [[ -f "$file" ]] || continue
    value="$(env_file_value "$key" "$file")"
    if [[ -n "$value" ]]; then
      print -r -- "$value"
      return
    fi
  done
}

announce_loaded_env_files() {
  local file
  for file in "${ENV_FILES[@]}"; do
    [[ -f "$file" ]] || continue
    echo "==> Loading release config from $file"
  done
}

SUPABASE_URL="$(resolve_config_value ICESNIFF_SUPABASE_URL)"
SUPABASE_PUBLISHABLE_KEY="$(resolve_config_value ICESNIFF_SUPABASE_PUBLISHABLE_KEY)"
SUPABASE_REDIRECT_URL="$(resolve_config_value ICESNIFF_SUPABASE_REDIRECT_URL)"

validate_release_configuration() {
  if [[ "$ALLOW_MISSING_SUPABASE_CONFIG" == "1" ]]; then
    return
  fi

  local missing_keys=()
  [[ -z "$SUPABASE_URL" ]] && missing_keys+=("ICESNIFF_SUPABASE_URL")
  [[ -z "$SUPABASE_PUBLISHABLE_KEY" ]] && missing_keys+=("ICESNIFF_SUPABASE_PUBLISHABLE_KEY")

  if (( ${#missing_keys[@]} > 0 )); then
    echo "Missing required Supabase auth config: ${missing_keys[*]}" >&2
    echo "Create macos/.env.release or export the missing values before running ./scripts/release-macos.sh" >&2
    echo "If you intentionally want a build without sign-in, set ICESNIFF_ALLOW_MISSING_SUPABASE_CONFIG=1." >&2
    exit 1
  fi
}

assemble_app_bundle() {
  rm -rf "$APP_PATH"
  mkdir -p "$MACOS_PATH" "$RESOURCES_PATH"

  ditto "$EXECUTABLE_PATH" "$MACOS_PATH/$APP_NAME"
  chmod +x "$MACOS_PATH/$APP_NAME"
  ditto "$RESOURCE_BUNDLE_PATH" "$RESOURCES_PATH/$RESOURCE_BUNDLE_NAME"

  cat > "$INFO_PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_IDENTIFIER</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$APP_VERSION</string>
  <key>CFBundleVersion</key>
  <string>$BUILD_VERSION</string>
  <key>LSMinimumSystemVersion</key>
  <string>$MINIMUM_SYSTEM_VERSION</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

  printf 'APPL????' > "$PKGINFO_PATH"

  if [[ -n "$SUPABASE_URL" ]]; then
    /usr/libexec/PlistBuddy -c "Add :ICESNIFF_SUPABASE_URL string $SUPABASE_URL" "$INFO_PLIST_PATH"
  fi

  if [[ -n "$SUPABASE_PUBLISHABLE_KEY" ]]; then
    /usr/libexec/PlistBuddy -c "Add :ICESNIFF_SUPABASE_PUBLISHABLE_KEY string $SUPABASE_PUBLISHABLE_KEY" "$INFO_PLIST_PATH"
  fi

  if [[ -n "$SUPABASE_REDIRECT_URL" ]]; then
    /usr/libexec/PlistBuddy -c "Add :ICESNIFF_SUPABASE_REDIRECT_URL string $SUPABASE_REDIRECT_URL" "$INFO_PLIST_PATH"
  fi
}

prepare_gpl_compliance_bundle() {
  rm -rf "$GPL_ARCHIVE_DIR"
  mkdir -p "$GPL_ARCHIVE_DIR"

  if [[ ! -d "$BUNDLED_TSHARK_PATH" ]]; then
    echo "Expected bundled Wireshark runtime not found at $BUNDLED_TSHARK_PATH" >&2
    exit 1
  fi

  if [[ -z "$WIRESHARK_SOURCE_ARCHIVE" || ! -f "$WIRESHARK_SOURCE_ARCHIVE" ]]; then
    echo "Bundled tshark requires the matching Wireshark source archive." >&2
    echo "Set ICESNIFF_WIRESHARK_SOURCE_ARCHIVE to the exact source tarball for the bundled Wireshark build." >&2
    exit 1
  fi

  ditto "$THIRD_PARTY_NOTICES_PATH" "$GPL_ARCHIVE_DIR/notices"
  cp "$WIRESHARK_SOURCE_ARCHIVE" "$GPL_ARCHIVE_DIR/"
}

"$SCRIPT_DIR/sync-bundled-cli.sh"
zsh "$SCRIPT_DIR/sync-bundled-tshark.sh"

mkdir -p "$ARCHIVE_DIR"
announce_loaded_env_files
validate_release_configuration

echo "==> Building macOS release app"
cd "$APP_ROOT"
xcodebuild \
  -scheme "$APP_NAME" \
  -configuration Release \
  -derivedDataPath "$DERIVED_DATA" \
  -destination "platform=macOS" \
  build

if [[ ! -f "$EXECUTABLE_PATH" ]]; then
  echo "Expected built executable not found at $EXECUTABLE_PATH" >&2
  exit 1
fi

if [[ ! -d "$RESOURCE_BUNDLE_PATH" ]]; then
  echo "Expected resource bundle not found at $RESOURCE_BUNDLE_PATH" >&2
  exit 1
fi

echo "==> Assembling app bundle"
assemble_app_bundle

echo "==> Preparing GPL compliance materials"
prepare_gpl_compliance_bundle

echo "==> Injecting bundle icon"
zsh "$SCRIPT_DIR/inject-bundle-icon.sh" "$ICON_SOURCE" "$APP_PATH"

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
echo "==> GPL compliance materials ready at $GPL_ARCHIVE_DIR"
