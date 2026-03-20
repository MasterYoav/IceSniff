$ErrorActionPreference = "Stop"

$repo = if ($env:ICESNIFF_INSTALL_REPO) { $env:ICESNIFF_INSTALL_REPO } else { "MasterYoav/IceSniff" }
$version = if ($env:ICESNIFF_INSTALL_VERSION) { $env:ICESNIFF_INSTALL_VERSION } else { "latest" }
$installRoot = if ($env:ICESNIFF_INSTALL_ROOT) { $env:ICESNIFF_INSTALL_ROOT } else { Join-Path $env:LOCALAPPDATA "IceSniff\cli" }
$binRoot = if ($env:ICESNIFF_INSTALL_BIN) { $env:ICESNIFF_INSTALL_BIN } else { Join-Path $env:LOCALAPPDATA "IceSniff\bin" }

function Get-Arch {
    switch ($env:PROCESSOR_ARCHITECTURE.ToLowerInvariant()) {
        "amd64" { return "x86_64" }
        "arm64" { return "aarch64" }
        default { throw "Unsupported architecture: $($env:PROCESSOR_ARCHITECTURE)" }
    }
}

function Get-AssetCandidates([string]$Arch) {
    if ($Arch -eq "aarch64") {
        return @(
            "icesniff-cli-windows-aarch64.zip",
            "icesniff-cli-windows-x86_64.zip"
        )
    }

    return @("icesniff-cli-windows-$Arch.zip")
}

function Resolve-Tag {
    if ($version -ne "latest") {
        return $version
    }

    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
    if (-not $release.tag_name) {
        throw "Failed to resolve latest release tag for $repo"
    }
    return $release.tag_name
}

function Ensure-UserPathContains([string]$PathEntry) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not $userPath) {
        [Environment]::SetEnvironmentVariable("Path", $PathEntry, "User")
        return
    }

    $parts = $userPath.Split(';') | Where-Object { $_ }
    if ($parts -contains $PathEntry) {
        return
    }

    [Environment]::SetEnvironmentVariable("Path", "$userPath;$PathEntry", "User")
}

$arch = Get-Arch
$tag = Resolve-Tag
$assetCandidates = Get-AssetCandidates $arch

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("icesniff-install-" + [guid]::NewGuid().ToString("N"))
$targetDir = Join-Path $installRoot $tag

New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
New-Item -ItemType Directory -Force -Path $binRoot | Out-Null

try {
    $archive = $null
    foreach ($asset in $assetCandidates) {
        $url = "https://github.com/$repo/releases/download/$tag/$asset"
        $candidateArchive = Join-Path $tempDir $asset
        try {
            Invoke-WebRequest -Uri $url -OutFile $candidateArchive
            $archive = $candidateArchive
            break
        } catch {
            continue
        }
    }

    if (-not $archive) {
        throw "Failed to download a Windows CLI bundle for release $tag. Checked: $($assetCandidates -join ', ')."
    }
    if (Test-Path $targetDir) {
        Remove-Item -Recurse -Force $targetDir
    }
    New-Item -ItemType Directory -Force -Path $targetDir | Out-Null
    Expand-Archive -Path $archive -DestinationPath $targetDir -Force

    $expandedRoot = Get-ChildItem -Path $targetDir | Select-Object -First 1
    if ($expandedRoot -and $expandedRoot.PSIsContainer) {
        Get-ChildItem -Path $expandedRoot.FullName -Force | ForEach-Object {
            Move-Item $_.FullName $targetDir -Force
        }
        Remove-Item -Recurse -Force $expandedRoot.FullName
    }

    Copy-Item (Join-Path $targetDir "bin\icesniff-cli.cmd") (Join-Path $binRoot "icesniff-cli.cmd") -Force
    Copy-Item (Join-Path $targetDir "bin\icesniff.cmd") (Join-Path $binRoot "icesniff.cmd") -Force
    Ensure-UserPathContains $binRoot
} finally {
    if (Test-Path $tempDir) {
        Remove-Item -Recurse -Force $tempDir
    }
}

Write-Host ""
Write-Host "Installed IceSniff CLI $tag to $targetDir"
Write-Host "Launcher: $binRoot\icesniff-cli.cmd"
Write-Host "Open a new PowerShell window if the command is not available immediately."
