#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
CLI_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(CDPATH= cd -- "$CLI_ROOT/../.." && pwd)"
DIST_ROOT="${ICESNIFF_DIST_ROOT:-$CLI_ROOT/dist}"
RUNTIME_ROOT="${ICESNIFF_WIRESHARK_RUNTIME_ROOT:-}"
WIRESHARK_APP="${ICESNIFF_WIRESHARK_APP:-}"
PROFILE="${ICESNIFF_CLI_PROFILE:-release}"
STAGED_RUNTIME_ROOT=""

cleanup() {
  if [ -n "$STAGED_RUNTIME_ROOT" ] && [ -d "$STAGED_RUNTIME_ROOT" ]; then
    rm -rf "$STAGED_RUNTIME_ROOT"
  fi
}

trap cleanup EXIT INT TERM

detect_os() {
  case "$(uname -s)" in
    Darwin) printf '%s' "macos" ;;
    Linux) printf '%s' "linux" ;;
    *) echo "Unsupported host OS for package-cli-bundle.sh" >&2; exit 1 ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf '%s' "x86_64" ;;
    arm64|aarch64) printf '%s' "aarch64" ;;
    *) echo "Unsupported host architecture for package-cli-bundle.sh" >&2; exit 1 ;;
  esac
}

command_path_or_die() {
  if command -v "$1" >/dev/null 2>&1; then
    command -v "$1"
    return
  fi

  echo "Required runtime tool \`$1\` was not found in PATH." >&2
  exit 1
}

copy_binary_with_deps() {
  binary_path="$1"
  target_root="$2"
  target_bin="$target_root/bin/$(basename "$binary_path")"

  mkdir -p "$target_root/bin" "$target_root/lib"
  cp -L "$binary_path" "$target_bin"
  chmod +x "$target_bin"

  ldd "$binary_path" \
    | awk '
        /=> \// { print $3 }
        /^\// { print $1 }
      ' \
    | while IFS= read -r dependency; do
        if [ -n "$dependency" ] && [ -f "$dependency" ]; then
          cp -L "$dependency" "$target_root/lib/"
        fi
      done
}

copy_directory_contents() {
  source_dir="$1"
  target_dir="$2"

  if [ -d "$source_dir" ]; then
    mkdir -p "$target_dir"
    cp -R "$source_dir"/. "$target_dir/"
  fi
}

stage_linux_runtime() {
  tshark_bin="${ICESNIFF_TSHARK_BIN:-}"
  dumpcap_bin="${ICESNIFF_DUMPCAP_BIN:-}"

  if [ -z "$tshark_bin" ]; then
    tshark_bin="$(command_path_or_die tshark)"
  fi
  if [ -z "$dumpcap_bin" ]; then
    dumpcap_bin="$(command_path_or_die dumpcap)"
  fi

  stage_root="$(mktemp -d "${TMPDIR:-/tmp}/icesniff-linux-runtime.XXXXXX")"
  copy_binary_with_deps "$tshark_bin" "$stage_root"
  copy_binary_with_deps "$dumpcap_bin" "$stage_root"

  folders_output="$("$tshark_bin" -G folders 2>/dev/null || true)"
  share_dir="$(printf '%s\n' "$folders_output" | sed -n 's/^Global configuration:[[:space:]]*//p' | head -n 1)"
  plugin_dir="$(printf '%s\n' "$folders_output" | sed -n 's/^Global Plugins:[[:space:]]*//p' | head -n 1)"
  lua_plugin_dir="$(printf '%s\n' "$folders_output" | sed -n 's/^Global Lua Plugins:[[:space:]]*//p' | head -n 1)"
  extcap_dir="$(printf '%s\n' "$folders_output" | sed -n 's/^Global Extcap path:[[:space:]]*//p' | head -n 1)"

  copy_directory_contents "$share_dir" "$stage_root/share/wireshark"

  if [ -n "$plugin_dir" ] && [ -d "$plugin_dir" ]; then
    plugin_parent="$(basename "$(dirname "$plugin_dir")")"
    plugin_leaf="$(basename "$plugin_dir")"
    if [ "$plugin_parent" = "plugins" ]; then
      copy_directory_contents "$plugin_dir" "$stage_root/lib/wireshark/plugins/$plugin_leaf"
    else
      copy_directory_contents "$plugin_dir" "$stage_root/lib/wireshark/plugins"
    fi
  fi

  if [ -n "$lua_plugin_dir" ] && [ -d "$lua_plugin_dir" ]; then
    copy_directory_contents "$lua_plugin_dir" "$stage_root/lib/wireshark"
  fi

  if [ -n "$extcap_dir" ] && [ -d "$extcap_dir" ]; then
    copy_directory_contents "$extcap_dir" "$stage_root/extcap"
  fi

  printf '%s' "$stage_root"
}

runtime_source() {
  if [ -n "$RUNTIME_ROOT" ] && [ -d "$RUNTIME_ROOT" ]; then
    printf '%s' "$RUNTIME_ROOT"
    return
  fi

  if [ -n "$WIRESHARK_APP" ] && [ -d "$WIRESHARK_APP" ]; then
    printf '%s' "$WIRESHARK_APP"
    return
  fi

  if [ "$(detect_os)" = "linux" ]; then
    STAGED_RUNTIME_ROOT="$(stage_linux_runtime)"
    printf '%s' "$STAGED_RUNTIME_ROOT"
    return
  fi

  default_app="$REPO_ROOT/apps/macos/Sources/IceSniffMac/Resources/BundledTShark/Wireshark.app"
  if [ -d "$default_app" ]; then
    printf '%s' "$default_app"
    return
  fi

  echo "No Wireshark runtime source found. Set ICESNIFF_WIRESHARK_RUNTIME_ROOT or ICESNIFF_WIRESHARK_APP." >&2
  exit 1
}

copy_runtime() {
  source_path="$1"
  target_path="$2"

  mkdir -p "$target_path"
  if [ -d "$source_path/Contents/MacOS" ]; then
    cp -R "$source_path" "$target_path/Wireshark.app"
  else
    cp -R "$source_path"/. "$target_path/wireshark"
  fi
}

write_unix_launchers() {
  target_root="$1"
  launcher_path="$target_root/bin/icesniff-cli"

  cat > "$launcher_path" <<'EOF'
#!/bin/sh
set -eu

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname "$0")" && pwd)"
BUNDLE_ROOT="$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)"
RUNTIME_ROOT="$BUNDLE_ROOT/runtime"

export ICESNIFF_RUNTIME_ROOT="$RUNTIME_ROOT"

if [ -d "$RUNTIME_ROOT/wireshark/bin" ]; then
  export PATH="$RUNTIME_ROOT/wireshark/bin:$PATH"
fi

if [ -d "$RUNTIME_ROOT/Wireshark.app/Contents/MacOS" ]; then
  export PATH="$RUNTIME_ROOT/Wireshark.app/Contents/MacOS:$PATH"
fi

if [ -d "$RUNTIME_ROOT/wireshark/lib" ]; then
  if [ -n "${LD_LIBRARY_PATH:-}" ]; then
    export LD_LIBRARY_PATH="$RUNTIME_ROOT/wireshark/lib:$LD_LIBRARY_PATH"
  else
    export LD_LIBRARY_PATH="$RUNTIME_ROOT/wireshark/lib"
  fi
fi

exec "$BUNDLE_ROOT/libexec/icesniff-cli" "$@"
EOF

  chmod +x "$launcher_path"
  ln -sf "icesniff-cli" "$target_root/bin/icesniff"
}

platform="$(detect_os)"
arch="$(detect_arch)"
bundle_name="icesniff-cli-${platform}-${arch}"
bundle_root="$DIST_ROOT/$bundle_name"
archive_path="$DIST_ROOT/${bundle_name}.tar.gz"
runtime_path="$(runtime_source)"

mkdir -p "$DIST_ROOT"
rm -rf "$bundle_root" "$archive_path"
mkdir -p "$bundle_root/bin" "$bundle_root/libexec" "$bundle_root/runtime"

cd "$CLI_ROOT"
if [ "$PROFILE" = "release" ]; then
  cargo build --locked --release
  cli_binary="$CLI_ROOT/target/release/icesniff-cli"
else
  cargo build --locked
  cli_binary="$CLI_ROOT/target/debug/icesniff-cli"
fi

cp "$cli_binary" "$bundle_root/libexec/icesniff-cli"
chmod +x "$bundle_root/libexec/icesniff-cli"
write_unix_launchers "$bundle_root"

copy_runtime "$runtime_path" "$bundle_root/runtime"

cat > "$bundle_root/README.txt" <<EOF
IceSniff CLI bundle

This bundle contains:
- bin/icesniff-cli launcher
- libexec/icesniff-cli
- a bundled Wireshark runtime for dumpcap/tshark-backed packet operations

Install with:
  curl -fsSL https://raw.githubusercontent.com/MasterYoav/IceSniff/main/apps/cli/install/install.sh | bash
EOF

cd "$DIST_ROOT"
tar -czf "$archive_path" "$bundle_name"
echo "Created $archive_path"
