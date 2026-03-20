#!/bin/sh
set -eu

REPO="${ICESNIFF_INSTALL_REPO:-MasterYoav/IceSniff}"
INSTALL_ROOT="${ICESNIFF_INSTALL_ROOT:-$HOME/.local/share/icesniff-cli}"
BIN_ROOT="${ICESNIFF_INSTALL_BIN:-$HOME/.local/bin}"
VERSION="${ICESNIFF_INSTALL_VERSION:-latest}"

detect_os() {
  case "$(uname -s)" in
    Darwin) printf '%s' "macos" ;;
    Linux) printf '%s' "linux" ;;
    *) echo "Unsupported OS" >&2; exit 1 ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf '%s' "x86_64" ;;
    arm64|aarch64) printf '%s' "aarch64" ;;
    *) echo "Unsupported architecture" >&2; exit 1 ;;
  esac
}

resolve_tag() {
  if [ "$VERSION" != "latest" ]; then
    printf '%s' "$VERSION"
    return
  fi

  curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1
}

platform="$(detect_os)"
arch="$(detect_arch)"
tag="$(resolve_tag)"

if [ -z "$tag" ]; then
  echo "Failed to resolve release tag for $REPO" >&2
  exit 1
fi

asset="icesniff-cli-${platform}-${arch}.tar.gz"
url="https://github.com/$REPO/releases/download/$tag/$asset"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT INT TERM

archive="$tmpdir/$asset"
target_dir="$INSTALL_ROOT/$tag"

mkdir -p "$INSTALL_ROOT" "$BIN_ROOT"
if ! curl -fsSL "$url" -o "$archive"; then
  echo "Failed to download $asset from release $tag." >&2
  echo "This usually means the current platform/architecture is not published yet." >&2
  exit 1
fi
rm -rf "$target_dir"
mkdir -p "$target_dir"
tar -xzf "$archive" -C "$target_dir" --strip-components=1

ln -sfn "$target_dir/bin/icesniff-cli" "$BIN_ROOT/icesniff-cli"
ln -sfn "$target_dir/bin/icesniff-cli" "$BIN_ROOT/icesniff"

printf '\nInstalled IceSniff CLI %s to %s\n' "$tag" "$target_dir"
printf 'Launcher: %s/icesniff-cli\n' "$BIN_ROOT"

case ":$PATH:" in
  *":$BIN_ROOT:"*) ;;
  *)
    printf '\nAdd this to your shell profile if needed:\n'
    printf '  export PATH="%s:$PATH"\n' "$BIN_ROOT"
    ;;
esac
