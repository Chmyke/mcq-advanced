# Downloads the Linux archive built by the "Linux desktop" workflow into target/dist, after
# checking that the run built the commit checked out here and that the archive matches its hash.
# Start the workflow first (Actions tab, or: gh workflow run linux-desktop.yml), wait for it.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/fetch-linux.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$repo = 'Chmyke/mcq-advanced'
$head = (git -C $root rev-parse HEAD).Trim()
$runs = gh run list -R $repo --workflow linux-desktop.yml --status success --limit 20 --json databaseId,headSha | ConvertFrom-Json
$run = @($runs | Where-Object { $_.headSha -eq $head })[0]
if (-not $run) { throw "No successful Linux build of $head: push it, run the workflow, then try again." }
$dist = Join-Path $root 'target/dist'
New-Item -ItemType Directory -Force $dist | Out-Null
Get-ChildItem -LiteralPath $dist -Filter '*-linux-*.tar.gz' | Remove-Item
gh run download $run.databaseId -R $repo -n linux-x86_64 -D $dist
if ($LASTEXITCODE -ne 0) { throw 'Download failed' }
foreach ($line in [IO.File]::ReadAllLines((Join-Path $dist 'SHA256-linux.txt'))) {
    $hash, $name = $line -split '  ', 2
    if ((Get-FileHash -LiteralPath (Join-Path $dist $name) -Algorithm SHA256).Hash -ne $hash) { throw "Hash mismatch: $name" }
    Write-Output "Linux archive: $name ($hash)"
}
