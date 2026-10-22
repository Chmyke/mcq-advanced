# Builds what the website serves for the demo into one folder: player/ and studio/ (the web
# builds, side by side so the player's "open in the editor" finds the Studio) and demo/ (the
# practice packages and the .qcmz sources, the same bytes at every build).
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-site.ps1 -Out <folder>
# The three subfolders of <folder> are replaced; nothing else in it is touched.
# Keep this file pure ASCII.
param([Parameter(Mandatory = $true)][string]$Out)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
foreach ($app in 'player', 'studio') {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build-web.ps1') -App $app
    if ($LASTEXITCODE -ne 0) { throw "Web build of the $app failed" }
}
. (Join-Path $PSScriptRoot 'toolchain.ps1')
$demo = Join-Path $root 'target/web/demo'
if (Test-Path -LiteralPath $demo) { Remove-Item -LiteralPath $demo -Recurse -Force }
Push-Location $root
try {
    $ErrorActionPreference = 'Continue'
    & cargo run --quiet --locked -p mcq-demo -- $demo 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw 'Demo build failed' }
    $ErrorActionPreference = 'Stop'
} finally { Pop-Location }

New-Item -ItemType Directory -Force $Out | Out-Null
foreach ($part in 'player', 'studio', 'demo') {
    $target = Join-Path $Out $part
    if (Test-Path -LiteralPath $target) { Remove-Item -LiteralPath $target -Recurse -Force }
    Copy-Item -LiteralPath (Join-Path $root "target/web/$part") $target -Recurse
}
Write-Output "Site demo: $Out (player, studio, demo)"
