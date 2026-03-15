#!/bin/zsh
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <source-png> <app-bundle>" >&2
  exit 1
fi

SOURCE_PNG="$1"
APP_PATH="$2"
TEMP_DIR="$(mktemp -d /tmp/icesniff-appicon.XXXXXX)"
TEMP_SOURCE_PNG="$TEMP_DIR/source.png"
PADDED_SOURCE_PNG="$TEMP_DIR/source-padded.png"
RESOURCE_FILE="$TEMP_DIR/icon.rsrc"
ICON_FILE="${APP_PATH}/Icon"$'\r'

cleanup() {
  rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

if [[ ! -f "$SOURCE_PNG" ]]; then
  echo "Source icon not found at $SOURCE_PNG" >&2
  exit 1
fi

if [[ ! -d "$APP_PATH" ]]; then
  echo "App bundle not found at $APP_PATH" >&2
  exit 1
fi

cp "$SOURCE_PNG" "$TEMP_SOURCE_PNG"

swift - "$TEMP_SOURCE_PNG" "$PADDED_SOURCE_PNG" <<'EOF'
import AppKit
import Foundation

let sourceURL = URL(fileURLWithPath: CommandLine.arguments[1])
let destinationURL = URL(fileURLWithPath: CommandLine.arguments[2])

guard let sourceImage = NSImage(contentsOf: sourceURL) else {
    fputs("Unable to load icon source image.\n", stderr)
    exit(1)
}

let canvasSize = NSSize(width: 1024, height: 1024)
let insetRatio: CGFloat = 0.16
let inset = canvasSize.width * insetRatio
let destinationRect = NSRect(
    x: inset,
    y: inset,
    width: canvasSize.width - (inset * 2),
    height: canvasSize.height - (inset * 2)
)

let paddedImage = NSImage(size: canvasSize)
paddedImage.lockFocus()
NSColor.clear.set()
NSRect(origin: .zero, size: canvasSize).fill()
sourceImage.draw(
    in: destinationRect,
    from: NSRect(origin: .zero, size: sourceImage.size),
    operation: .sourceOver,
    fraction: 1
)
paddedImage.unlockFocus()

guard
    let tiffData = paddedImage.tiffRepresentation,
    let imageRep = NSBitmapImageRep(data: tiffData),
    let pngData = imageRep.representation(using: .png, properties: [:])
else {
    fputs("Unable to encode padded icon image.\n", stderr)
    exit(1)
}

try pngData.write(to: destinationURL)
EOF

cp "$PADDED_SOURCE_PNG" "$ICON_FILE"

sips -i "$PADDED_SOURCE_PNG" >/dev/null
DeRez -only icns "$PADDED_SOURCE_PNG" > "$RESOURCE_FILE"
Rez -append "$RESOURCE_FILE" -o "$ICON_FILE"

SetFile -a C "$APP_PATH"
SetFile -a V "$ICON_FILE"
