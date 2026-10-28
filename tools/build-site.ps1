# Builds the online version the website serves into one folder: player/ and studio/ (the web
# builds, side by side so the player's "open in the editor" finds the Studio), demo/ (the
# practice packages and the .qcmz sources, the same bytes at every build), shots/ (the
# tutorial's screenshots, taken from this very Studio by tools/tutorial-shots.mjs; needs Node
# and Microsoft Edge) and ai-prompt/ (the AI prompt in each language, which the website's
# prompt pages show word for word).
# The online version is always a released one, the same as the downloads: the build refuses a
# checkout that is not exactly the commit that set its version, and writes that version into
# player/version.txt and studio/version.txt, which the website's check compares with its
# downloads. -Unreleased builds any checkout for a local look and stamps it "unreleased", which
# the website refuses.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-site.ps1 -Out <folder> [-Unreleased]
# The five subfolders of <folder> are replaced; nothing else in it is touched.
# Keep this file pure ASCII.
param([Parameter(Mandatory = $true)][string]$Out, [switch]$Unreleased)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'version.ps1')
$stamp = 'unreleased'
if (-not $Unreleased) {
    if (-not (Test-ReleasedCheckout)) {
        throw "This checkout is not the commit that set version $(Get-WorkspaceVersion), or has local changes: the website only serves released versions (-Unreleased for a local look)."
    }
    $stamp = Get-WorkspaceVersion
}
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
foreach ($app in 'player', 'studio') {
    [IO.File]::WriteAllText((Join-Path $Out "$app/version.txt"), "$stamp`n", (New-Object System.Text.UTF8Encoding $false))
}
$prompt = Join-Path $Out 'ai-prompt'
if (Test-Path -LiteralPath $prompt) { Remove-Item -LiteralPath $prompt -Recurse -Force }
Copy-Item -LiteralPath (Join-Path $root 'examples/ai-prompt') $prompt -Recurse
$shots = Join-Path $Out 'shots'
if (Test-Path -LiteralPath $shots) { Remove-Item -LiteralPath $shots -Recurse -Force }
& node (Join-Path $PSScriptRoot 'tutorial-shots.mjs') $Out $shots
if ($LASTEXITCODE -ne 0) { throw 'Tutorial screenshots failed' }
Write-Output "Online version ${stamp}: $Out (player, studio, demo, shots, ai-prompt)"
