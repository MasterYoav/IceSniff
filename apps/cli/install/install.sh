#!/bin/sh
set -eu

REPO="${ICESNIFF_INSTALL_REPO:-MasterYoav/IceSniff}"
INSTALL_ROOT="${ICESNIFF_INSTALL_ROOT:-$HOME/.local/share/icesniff-cli}"
BIN_ROOT="${ICESNIFF_INSTALL_BIN:-$HOME/.local/bin}"
VERSION="${ICESNIFF_INSTALL_VERSION:-latest}"

append_path_line() {
  profile_path="$1"
  path_line="$2"

  if [ -f "$profile_path" ] && grep -F "$path_line" "$profile_path" >/dev/null 2>&1; then
    return
  fi

  if [ ! -f "$profile_path" ]; then
    mkdir -p "$(dirname "$profile_path")"
    : > "$profile_path"
  fi

  {
    printf '\n# Added by IceSniff CLI installer\n'
    printf '%s\n' "$path_line"
  } >> "$profile_path"
}

ensure_shell_path() {
  path_line="export PATH=\"$BIN_ROOT:\$PATH\""

  append_path_line "$HOME/.profile" "$path_line"

  if [ -n "${BASH_VERSION:-}" ] || [ -f "$HOME/.bashrc" ] || [ "${SHELL:-}" = "/bin/bash" ]; then
    append_path_line "$HOME/.bashrc" "$path_line"
  fi

  if [ -n "${ZSH_VERSION:-}" ] || [ -f "$HOME/.zshrc" ] || [ "${SHELL:-}" = "/bin/zsh" ]; then
    append_path_line "$HOME/.zshrc" "$path_line"
    append_path_line "$HOME/.zprofile" "$path_line"
  fi
}

configure_linux_capture_permissions() {
  dumpcap_path="$1"

  if [ ! -f "$dumpcap_path" ]; then
    return
  fi

  if ! command -v setcap >/dev/null 2>&1; then
    printf '\nWarning: setcap is not available, so live capture may still require manual setup.\n'
    return
  fi

  printf '\nConfiguring Linux capture permissions for bundled dumpcap...\n'
  if [ "$(id -u)" -eq 0 ]; then
    if setcap cap_net_admin,cap_net_raw=eip "$dumpcap_path" 2>/dev/null; then
      printf 'Capture permissions configured.\n'
    else
      printf 'Warning: failed to set capture capabilities on bundled dumpcap.\n'
    fi
    return
  fi

  if command -v sudo >/dev/null 2>&1; then
    if sudo setcap cap_net_admin,cap_net_raw=eip "$dumpcap_path"; then
      printf 'Capture permissions configured.\n'
    else
      printf 'Warning: failed to set capture capabilities on bundled dumpcap.\n'
    fi
  else
    printf 'Warning: sudo is not available, so live capture may still require manual setup.\n'
  fi
}

write_launcher() {
  launcher_path="$1"
  target_dir="$2"
  entrypoint="$3"
  install_root="$4"
  bin_root="$5"

  cat > "$launcher_path" <<EOF
#!/bin/sh
set -eu

TARGET_DIR="$target_dir"
RUNTIME_ROOT="\$TARGET_DIR/runtime"
INSTALL_ROOT="$install_root"
BIN_ROOT="$bin_root"

export ICESNIFF_RUNTIME_ROOT="\$RUNTIME_ROOT"
export ICESNIFF_INSTALL_ROOT="\$INSTALL_ROOT"
export ICESNIFF_INSTALL_BIN="\$BIN_ROOT"

if [ -d "\$RUNTIME_ROOT/wireshark/bin" ]; then
  export PATH="\$RUNTIME_ROOT/wireshark/bin:\$PATH"
fi

if [ -d "\$RUNTIME_ROOT/Wireshark.app/Contents/MacOS" ]; then
  export PATH="\$RUNTIME_ROOT/Wireshark.app/Contents/MacOS:\$PATH"
fi

if [ -d "\$RUNTIME_ROOT/wireshark/lib" ]; then
  if [ -n "\${LD_LIBRARY_PATH:-}" ]; then
    export LD_LIBRARY_PATH="\$RUNTIME_ROOT/wireshark/lib:\$LD_LIBRARY_PATH"
  else
    export LD_LIBRARY_PATH="\$RUNTIME_ROOT/wireshark/lib"
  fi
fi

exec "\$TARGET_DIR/libexec/icesniff-cli" $entrypoint "\$@"
EOF

  chmod +x "$launcher_path"
}

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

  curl -fsSL "https://api.github.com/repos/$REPO/releases?per_page=100" \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
    | grep '^cli-v' \
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

if [ "$platform" = "linux" ]; then
  configure_linux_capture_permissions "$target_dir/runtime/wireshark/bin/dumpcap"
fi

write_launcher "$BIN_ROOT/icesniff-cli" "$target_dir" "" "$INSTALL_ROOT" "$BIN_ROOT"
write_launcher "$BIN_ROOT/icesniff" "$target_dir" "launcher" "$INSTALL_ROOT" "$BIN_ROOT"

ensure_shell_path

printf '\nInstalled IceSniff CLI %s to %s\n' "$tag" "$target_dir"
printf 'Binaries: %s\n' "$BIN_ROOT"
printf '\nRun one of these commands:\n'
printf '  icesniff      Start the IceSniff terminal menu\n'
printf '  icesniff-cli  Start the IceSniff TUI directly (skip the menu)\n'

case ":$PATH:" in
  *":$BIN_ROOT:"*) ;;
  *)
    printf '\nPATH was added to your shell startup files. Open a new terminal window if needed.\n'
    ;;
esac
