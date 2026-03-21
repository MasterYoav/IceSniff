# CLI Install And Bundling

This document defines the packaged CLI layout and the one-line installer entrypoints.

## End-User Install

### macOS and Linux

```bash
curl -fsSL https://raw.githubusercontent.com/MasterYoav/IceSniff/main/install.sh | sh
```

Current published targets:

- macOS Apple Silicon (`icesniff-cli-macos-aarch64.tar.gz`)
- Linux x86_64 (`icesniff-cli-linux-x86_64.tar.gz`)
- Linux ARM64 (`icesniff-cli-linux-aarch64.tar.gz`)

### Windows

```powershell
irm https://raw.githubusercontent.com/MasterYoav/IceSniff/main/install.ps1 | iex
```

Current published targets:

- Windows x86_64 (`icesniff-cli-windows-x86_64.zip`)
- Windows ARM64 (`icesniff-cli-windows-aarch64.zip`)

Both installers expect GitHub release assets with these names:

- `icesniff-cli-macos-aarch64.tar.gz`
- `icesniff-cli-linux-x86_64.tar.gz`
- `icesniff-cli-windows-x86_64.zip`

Install behavior:

- extracts the release bundle into a user-local directory
- installs `icesniff-cli` as a direct CLI launcher wrapper
- installs `icesniff` as the IceSniff launcher menu
- keeps the bundled Wireshark runtime next to the CLI bundle
- keeps the bundled live web app files next to the CLI bundle
- does not require users to install a separate Wireshark app for product builds

## Bundle Layout

Each bundle is expected to look like this:

```text
icesniff-cli-<platform>-<arch>/
  bin/
    icesniff-cli
    icesniff
  libexec/
    icesniff-cli(.exe)
  live-app/
    server.mjs
    public/...
  runtime/
    wireshark/bin/...
    wireshark/lib/...
    wireshark/share/wireshark/...
    or
    Wireshark.app/Contents/MacOS/...
```

The `bin/` launchers export `ICESNIFF_RUNTIME_ROOT` before they exec the real binary in `libexec/`, so packaged builds can always resolve the private capture runtime without depending on repo paths or a preinstalled Wireshark app.

## Maintainer Packaging

### macOS and Linux

```bash
cd apps/cli
./scripts/package-cli-bundle.sh
```

Optional inputs:

- `ICESNIFF_WIRESHARK_RUNTIME_ROOT=/path/to/runtime-dir`
- `ICESNIFF_WIRESHARK_APP=/path/to/Wireshark.app`
- `ICESNIFF_CLI_PROFILE=debug`
- `ICESNIFF_DIST_ROOT=/custom/output/dir`
- `ICESNIFF_TSHARK_BIN=/custom/path/to/tshark` on Linux staging
- `ICESNIFF_DUMPCAP_BIN=/custom/path/to/dumpcap` on Linux staging

### Windows

```powershell
cd apps/cli
.\scripts\package-cli-bundle.ps1
```

Optional inputs:

- `-WiresharkRuntimeRoot C:\path\to\Wireshark`
- `-WiresharkApp C:\path\to\Wireshark.app`
- `-Profile debug`
- `-DistRoot C:\path\to\dist`

## Release Automation

GitHub release assets are built and uploaded by `.github/workflows/cli-release.yml`.

Triggers:

- push a `cli-v*` tag to publish assets to that release
- run the workflow manually with a `tag` input to create or update a release

The one-line CLI installers resolve the newest published `cli-v*` release, so normal app releases can stay focused on the macOS desktop package. Those CLI releases should stay marked as prereleases.

The workflow currently produces:

- macOS Apple Silicon bundle from the repo-bundled `Wireshark.app`
- Linux x86_64 and ARM64 bundles from staged `tshark` / `dumpcap` plus shared libraries on GitHub-hosted Ubuntu runners
- Windows x86_64 and ARM64 bundles from a silent Wireshark install on GitHub-hosted Windows runners

## Notes

- The current installer flow is designed for release assets hosted on GitHub Releases.
- Product builds should bundle `dumpcap` and `tshark` through the packaged runtime rather than rely on system `tcpdump`.
- Platform capture permissions may still be required even when the runtime itself is bundled.
