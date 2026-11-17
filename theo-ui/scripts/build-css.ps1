# Builds assets/theo-ui.css from css/input.css with the Tailwind CSS standalone CLI.
# No Node.js or npm (REQUIREMENTS.md 3.2, 8.3). The CLI version and checksum are pinned.
# Run it after you change components or css/input.css:  ./scripts/build-css.ps1  [-Watch]
param([switch]$Watch)
$ErrorActionPreference = 'Stop'

$Version = 'v4.3.5'
$Sha256  = '2803583268c53f4713e89943e211081fddb186c0a4a7d07b5c342c6ae853ec95'
$Root    = Split-Path -Parent $PSScriptRoot
$Tools   = Join-Path $Root 'tools'
$Exe     = Join-Path $Tools "tailwindcss-$Version.exe"

if (-not (Test-Path $Exe)) {
    New-Item -ItemType Directory -Force $Tools | Out-Null
    $Url = "https://github.com/tailwindlabs/tailwindcss/releases/download/$Version/tailwindcss-windows-x64.exe"
    Write-Host "Downloading Tailwind CSS $Version ..."
    Invoke-WebRequest -Uri $Url -OutFile $Exe -UseBasicParsing
}
$Actual = (Get-FileHash $Exe -Algorithm SHA256).Hash.ToLower()
if ($Sha256 -and $Actual -ne $Sha256) { Remove-Item $Exe; throw "Checksum mismatch for $Exe" }
if (-not $Sha256) { Write-Host "Tailwind CLI SHA-256: $Actual (pin it in this script)" }

$TwArgs = @('-i', (Join-Path $Root 'css/input.css'), '-o', (Join-Path $Root 'assets/theo-ui.css'), '--minify')
if ($Watch) { $TwArgs += '--watch' }
# Tailwind prints its banner on stderr; judge success by the exit code only.
$ErrorActionPreference = 'Continue'
& $Exe @TwArgs 2>&1 | ForEach-Object { Write-Host $_ }
if ($LASTEXITCODE -ne 0) { throw "Tailwind CLI failed with exit code $LASTEXITCODE" }
