$ErrorActionPreference = "Stop"

$scriptUrl = "https://raw.githubusercontent.com/MasterYoav/IceSniff/main/apps/cli/install/install.ps1"
$script = Invoke-RestMethod -Uri $scriptUrl

Invoke-Expression $script
