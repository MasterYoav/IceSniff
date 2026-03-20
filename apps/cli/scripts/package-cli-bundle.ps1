param(
    [string]$DistRoot = "",
    [string]$WiresharkRuntimeRoot = "",
    [string]$WiresharkApp = "",
    [ValidateSet("debug", "release")]
    [string]$Profile = "release"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$cliRoot = Split-Path -Parent $scriptDir
$repoRoot = Split-Path -Parent (Split-Path -Parent $cliRoot)
if (-not $DistRoot) {
    $DistRoot = Join-Path $cliRoot "dist"
}

function Get-HostArch {
    switch ($env:PROCESSOR_ARCHITECTURE.ToLowerInvariant()) {
        "amd64" { return "x86_64" }
        "arm64" { return "aarch64" }
        default { throw "Unsupported host architecture: $($env:PROCESSOR_ARCHITECTURE)" }
    }
}

function Resolve-RuntimeSource {
    if ($WiresharkRuntimeRoot -and (Test-Path $WiresharkRuntimeRoot)) {
        return (Resolve-Path $WiresharkRuntimeRoot).Path
    }

    if ($WiresharkApp -and (Test-Path $WiresharkApp)) {
        return (Resolve-Path $WiresharkApp).Path
    }

    $defaultWindowsRuntime = "C:\Program Files\Wireshark"
    if (Test-Path $defaultWindowsRuntime) {
        return $defaultWindowsRuntime
    }

    throw "No Wireshark runtime source found. Pass -WiresharkRuntimeRoot or install Wireshark."
}

$arch = Get-HostArch
$bundleName = "icesniff-cli-windows-$arch"
$bundleRoot = Join-Path $DistRoot $bundleName
$archivePath = Join-Path $DistRoot "$bundleName.zip"
$runtimeSource = Resolve-RuntimeSource

New-Item -ItemType Directory -Force -Path $DistRoot | Out-Null
if (Test-Path $bundleRoot) { Remove-Item -Recurse -Force $bundleRoot }
if (Test-Path $archivePath) { Remove-Item -Force $archivePath }
New-Item -ItemType Directory -Force -Path (Join-Path $bundleRoot "bin") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $bundleRoot "libexec") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $bundleRoot "runtime") | Out-Null

Push-Location $cliRoot
try {
    if ($Profile -eq "release") {
        cargo build --locked --release
        $cliBinary = Join-Path $cliRoot "target\release\icesniff-cli.exe"
    } else {
        cargo build --locked
        $cliBinary = Join-Path $cliRoot "target\debug\icesniff-cli.exe"
    }
} finally {
    Pop-Location
}

Copy-Item $cliBinary (Join-Path $bundleRoot "libexec\icesniff-cli.exe")
Copy-Item $runtimeSource (Join-Path $bundleRoot "runtime\wireshark") -Recurse

$launcher = @"
@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "BUNDLE_ROOT=%SCRIPT_DIR%.."
set "ICESNIFF_RUNTIME_ROOT=%BUNDLE_ROOT%\runtime"
if exist "%ICESNIFF_RUNTIME_ROOT%\wireshark\bin" set "PATH=%ICESNIFF_RUNTIME_ROOT%\wireshark\bin;%PATH%"
if exist "%ICESNIFF_RUNTIME_ROOT%\wireshark" set "PATH=%ICESNIFF_RUNTIME_ROOT%\wireshark;%PATH%"
"%BUNDLE_ROOT%\libexec\icesniff-cli.exe" %*
"@

$launcherPath = Join-Path $bundleRoot "bin\icesniff-cli.cmd"
$aliasPath = Join-Path $bundleRoot "bin\icesniff.cmd"
Set-Content -Path $launcherPath -Value $launcher -NoNewline
Set-Content -Path $aliasPath -Value $launcher -NoNewline

@"
IceSniff CLI bundle

This bundle contains:
- bin\icesniff-cli.cmd launcher
- libexec\icesniff-cli.exe
- a bundled Wireshark runtime for dumpcap/tshark-backed packet operations

Install with:
  powershell -ExecutionPolicy Bypass -c "iwr https://raw.githubusercontent.com/MasterYoav/IceSniff/main/apps/cli/install/install.ps1 -UseBasicParsing | iex"
"@ | Set-Content -Path (Join-Path $bundleRoot "README.txt")

Compress-Archive -Path $bundleRoot -DestinationPath $archivePath
Write-Host "Created $archivePath"
