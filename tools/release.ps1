# Delivers a version in one command: everything a release publishes comes from the one commit
# that sets the version, so the downloads, the online version and the GitHub release always match.
#   1. Cargo.toml takes the version; tools/check.ps1 must pass; the commit "Version <x.y.z>"
#      (with its CHANGELOG.md section, written beforehand) goes to GitHub's main.
#   2. The Windows installer is built here; the Linux archive comes from the Build workflow's run
#      of that commit (tools/fetch-linux.ps1), once it has passed.
#   3. The website checkout gets the online version (tools/build-site.ps1) and the downloads
#      (its tools/import-mcq-advanced.py), passes its MCQ check, is committed, pushed, deployed
#      and checked live.
#   4. tools/release-github.ps1 publishes the site's files as the GitHub release.
# Pushing and deploying publish: run it only with the maintainer's go-ahead.
# Run again after a failure: steps already done are recognised and skipped.
# Run from a checkout at origin/main (a fresh worktree keeps other work out of it):
# powershell -NoProfile -ExecutionPolicy Bypass -File tools/release.ps1 -Version <x.y.z> -Site <website checkout>
# The website checkout must be clean and contain its origin/main, e.g. a fresh worktree of it.
# Keep this file pure ASCII.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Site
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$repo = 'Chmyke/mcq-advanced'
. (Join-Path $PSScriptRoot 'version.ps1')

function Step([string]$name) { Write-Output "== release: $name" }
function Invoke-Checked([string]$what, [scriptblock]$block) {
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed." }
}
function Invoke-Script([string]$name, [string[]]$arguments) {
    Invoke-Checked $name { & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot $name) @arguments }
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version $Version is not x.y.z." }
$Site = (Resolve-Path -LiteralPath $Site).Path
if (-not (Test-Path -LiteralPath (Join-Path $Site 'tools/import-mcq-advanced.py'))) { throw "$Site is not the website checkout." }
$changelog = [IO.File]::ReadAllText((Join-Path $root 'CHANGELOG.md'))
if ($changelog -notmatch "(?m)^## $([regex]::Escape($Version))\s*$") { throw "CHANGELOG.md has no section for ${Version}: write it first." }

# 1. The version commit, on GitHub's main.
if ((Get-WorkspaceVersion) -ne $Version) {
    Step "version $Version"
    $dirty = @(git -C $root status --porcelain --untracked-files=no | Where-Object { $_ -notmatch '^.. CHANGELOG\.md$' })
    if ($dirty) { throw "Local changes besides CHANGELOG.md:`n$($dirty -join "`n")" }
    Invoke-Checked 'git fetch' { git -C $root fetch -q origin }
    if ((git -C $root rev-parse HEAD) -ne (git -C $root rev-parse origin/main)) { throw 'This checkout is not at origin/main: release from a fresh checkout of it.' }
    $toml = Join-Path $root 'Cargo.toml'
    $text = [IO.File]::ReadAllText($toml)
    [IO.File]::WriteAllText($toml, ([regex]'(?m)^version = "[^"]+"').Replace($text, "version = `"$Version`"", 1), (New-Object System.Text.UTF8Encoding $false))
    . (Join-Path $PSScriptRoot 'toolchain.ps1')
    # The workspace crates take the version in Cargo.lock before any --locked command.
    Invoke-Checked 'cargo metadata' { cargo metadata --format-version 1 --offline --manifest-path $toml | Out-Null }
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check.ps1')
    if ($LASTEXITCODE -ne 0) {
        git -C $root checkout -- Cargo.toml Cargo.lock
        throw 'check.ps1 failed: the version was not set.'
    }
    Invoke-Checked 'git commit' { git -C $root commit -q -m "Version $Version" -- Cargo.toml Cargo.lock CHANGELOG.md }
}
if (-not (Test-ReleasedCheckout)) { throw "The checkout is not the commit that set version $Version, or has local changes." }
$commit = (git -C $root rev-parse HEAD).Trim()
$remote = ((git ls-remote "https://github.com/$repo.git" refs/heads/main) -split '\s+')[0]
if ($remote -ne $commit) {
    Step 'push'
    Invoke-Checked 'git push' { git -C $root push -q origin HEAD:main }
}

# 2. Both desktop builds of that commit.
$dist = Join-Path $root 'target/dist'
if (-not (Test-Path -LiteralPath (Join-Path $dist "MCQ-Advanced-Setup-$Version.exe"))) {
    Step 'Windows installer'
    Invoke-Script 'package-windows.ps1' @()
}
if (-not (Test-Path -LiteralPath (Join-Path $dist "mcq-advanced-$Version-linux-x86_64.tar.gz"))) {
    Step 'Linux archive (waiting for the Build workflow)'
    $run = $null
    foreach ($try in 1..30) {
        $run = @(gh run list -R $repo --workflow build.yml --commit $commit --limit 1 --json databaseId | ConvertFrom-Json)[0]
        if ($run) { break }
        Start-Sleep -Seconds 10
    }
    if (-not $run) { throw "No Build run of $commit on GitHub." }
    gh run watch $run.databaseId -R $repo --exit-status | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "The Build run of $commit failed: https://github.com/$repo/actions/runs/$($run.databaseId)" }
    Invoke-Script 'fetch-linux.ps1' @()
}

# 3. The website: online version, downloads, check, commit, push, deploy, live check.
$stamp = Join-Path $Site 'mcq-advanced/studio/version.txt'
$live = $null
try { $live = (Invoke-WebRequest -UseBasicParsing -UserAgent 'check-live' -Uri 'https://mickaelmancini.com/mcq-advanced/studio/version.txt').Content.Trim() } catch { }
if ($live -ne $Version) {
    if ((Test-Path -LiteralPath $stamp) -and ([IO.File]::ReadAllText($stamp).Trim() -eq $Version) -and
        ((git -C $Site log -1 --format=%s) -eq "mcq-advanced: version $Version")) {
        Step 'website already committed, deploying'
    } else {
        Step 'website'
        if (git -C $Site status --porcelain) { throw "The website checkout has local changes: $Site" }
        Invoke-Checked 'git fetch (website)' { git -C $Site fetch -q origin }
        # Commits of its own ahead of origin/main ship with the version.
        git -C $Site merge-base --is-ancestor origin/main HEAD
        if ($LASTEXITCODE -ne 0) { throw "The website checkout does not contain its origin/main: $Site" }
        Invoke-Script 'build-site.ps1' @('-Out', (Join-Path $Site 'mcq-advanced'))
        Invoke-Checked 'import-mcq-advanced.py' { python (Join-Path $Site 'tools/import-mcq-advanced.py') $dist }
        Invoke-Checked 'check-mcq-advanced.py' { python (Join-Path $Site 'tools/check-mcq-advanced.py') }
        Invoke-Checked 'git add (website)' { git -C $Site add -A mcq-advanced }
        Invoke-Checked 'git commit (website)' { git -C $Site commit -q -m "mcq-advanced: version $Version" }
        Invoke-Checked 'git push (website)' { git -C $Site push -q origin HEAD:main }
    }
    Push-Location $Site
    try {
        Invoke-Checked 'npm run deploy' { & npm.cmd run deploy }
        Invoke-Checked 'npm run check:live' { & npm.cmd run check:live }
    } finally { Pop-Location }
}

# 4. The GitHub release, the site's files byte for byte.
Step 'GitHub release'
Invoke-Script 'release-github.ps1' @()
Write-Output "RELEASED  MCQ Advanced $Version at $commit"
