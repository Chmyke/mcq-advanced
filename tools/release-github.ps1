# Publishes the version the website serves as a release of the GitHub repository: the same
# installer, Linux archive and SHA256.txt, byte for byte. The live SHA256.txt is the reference;
# GitHub's own digest of every uploaded file must match it, or the step fails.
# Runs after main is pushed and the site deployed. The release tags the commit that set the
# version in Cargo.toml, the source of the files built for it, even when main has moved on.
# The notes are the section of the version in CHANGELOG.md: without one, no release.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/release-github.ps1 [-DryRun] [-Replace]
# -SelfTest (run by check.ps1, no network) proves the digest comparison refuses a mismatch, that
# the current version has notes and that a version without a section gets none.
# Keep this file pure ASCII.
param(
    [string]$Repo = 'Chmyke/mcq-advanced',
    [string]$Site = 'https://mickaelmancini.com/mcq-advanced/downloads',
    [switch]$Replace,
    [switch]$DryRun,
    [switch]$SelfTest
)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$root = Split-Path -Parent $PSScriptRoot
$utf8 = New-Object System.Text.UTF8Encoding $false

function Read-Manifest([string]$text) {
    $manifest = [ordered]@{}
    foreach ($line in $text -split "`r?`n") {
        if ($line -match '^([0-9a-fA-F]{64})\s+\*?(\S+)\s*$') { $manifest[$Matches[2]] = $Matches[1].ToLowerInvariant() }
    }
    $manifest
}
# Names of the manifest that the release lacks or serves with another digest.
function Get-Mismatches($manifest, [hashtable]$digests) {
    foreach ($name in $manifest.Keys) {
        if ($digests[$name] -ne "sha256:$($manifest[$name])") { $name }
    }
}
# The bullet list under "## <version>" in CHANGELOG.md, up to the next section.
function Get-Notes([string]$changelog, [string]$version) {
    $match = [regex]::Match($changelog, '(?ms)^## ' + [regex]::Escape($version) + '[ \t]*\r?$(.*?)(?=^## |\z)')
    $notes = $match.Groups[1].Value.Trim()
    if (-not $match.Success -or -not $notes) { throw "CHANGELOG.md has no section for $version." }
    $notes
}

. (Join-Path $PSScriptRoot 'version.ps1')
$version = Get-WorkspaceVersion
$changelog = [IO.File]::ReadAllText((Join-Path $root 'CHANGELOG.md'), $utf8)
$changes = Get-Notes $changelog $version

$planted = Read-Manifest ("$('a' * 64)  same.exe`r`n$('b' * 64)  changed.tar.gz`n$('c' * 64)  missing.tar.gz`n")
$seen = @(Get-Mismatches $planted @{ 'same.exe' = "sha256:$('a' * 64)"; 'changed.tar.gz' = "sha256:$('d' * 64)" })
if (($seen -join ',') -ne 'changed.tar.gz,missing.tar.gz' -or @(Get-Mismatches $planted @{
            'same.exe' = "sha256:$('a' * 64)"; 'changed.tar.gz' = "sha256:$('b' * 64)"; 'missing.tar.gz' = "sha256:$('c' * 64)" }).Count) {
    throw 'Release digest self-test failed: a planted mismatch was not reported, or a match was.'
}
$refused = $false
try { Get-Notes $changelog '0.0.0' | Out-Null } catch { $refused = $true }
# The commit lookup is tested on the committed version: while tools/release.ps1 checks a new
# version, Cargo.toml holds it before any commit does.
$committed = [regex]::Match(((git -C $root show HEAD:Cargo.toml) -join "`n"), '(?m)^version = "([^"]+)"').Groups[1].Value
$found = Get-VersionCommit $committed
$line = "version = `"$committed`""
if (-not ((git -C $root show "${found}:Cargo.toml") -contains $line) -or ((git -C $root show "${found}~1:Cargo.toml") -contains $line)) {
    throw "Release self-test failed: $found is not the commit that set version $committed."
}
if (-not $refused) { throw 'Release self-test failed: a version without a changelog section got notes.' }
if ($SelfTest) { "PASS  release self-test, notes for $version, $committed set at $found"; return }
$release = Get-VersionCommit $version

function Invoke-Gh {
    $out = & gh @args
    if ($LASTEXITCODE -ne 0) { throw "gh $($args -join ' ') failed." }
    $out -join "`n"
}
function Get-ReleaseDigests([string]$tag) {
    $releases = Invoke-Gh api "repos/$Repo/releases?per_page=100" | ConvertFrom-Json
    $release = @($releases | Where-Object { $_.tag_name -eq $tag })[0]
    if (-not $release) { return $null }
    $digests = @{}
    foreach ($asset in $release.assets) { $digests[$asset.name] = "$($asset.digest)" }
    $digests
}

# The version's commit must already be on GitHub's main.
$remote = ((git ls-remote "https://github.com/$Repo.git" refs/heads/main) -split '\s+')[0]
git -C $root merge-base --is-ancestor $release $remote 2>$null
if ($LASTEXITCODE -ne 0) { throw "$release (version $version) is not on GitHub's main ($remote): push it first." }
$head = $release
$tag = "v$version"

# The files as the site serves them. The User-Agent keeps these requests out of the site's counter.
$dir = Join-Path $root "target/github-release/$version"
if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
New-Item -ItemType Directory -Force $dir | Out-Null
function Save-FromSite([string]$name) {
    $path = Join-Path $dir $name
    Invoke-WebRequest -UseBasicParsing -UserAgent 'check-live' -Uri "$Site/$name" -OutFile $path
    $path
}
$sums = Save-FromSite 'SHA256.txt'
$manifest = Read-Manifest ([IO.File]::ReadAllText($sums, $utf8))
$expected = @("MCQ-Advanced-Setup-$version.exe", "mcq-advanced-$version-linux-x86_64.tar.gz")
if ((@($manifest.Keys | Sort-Object) -join ',') -ne (@($expected | Sort-Object) -join ',')) {
    throw "The site serves $(@($manifest.Keys) -join ', '), not version $version."
}
$files = @($sums)
foreach ($name in $expected) {
    $path = Save-FromSite $name
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest[$name]) {
        throw "$name served by the site does not match its SHA256.txt."
    }
    $files += $path
}
# SHA256.txt is published too: its own digest joins the comparison.
$manifest['SHA256.txt'] = (Get-FileHash -LiteralPath $sums -Algorithm SHA256).Hash.ToLowerInvariant()

$existing = Get-ReleaseDigests $tag
if ($DryRun) {
    $state = if ($existing) { 'exists' } else { 'absent' }
    "DRY RUN  $Repo $tag at $head ($state)"
    $files | ForEach-Object { "  $(Split-Path -Leaf $_)" }
    return
}
if (-not $existing) {
    $notes = Join-Path $dir 'notes.md'
    $fence = '```'
    $text = "$changes`n`nThe same files as https://mickaelmancini.com/mcq-advanced/, byte for byte. " +
        "Not code-signed yet: Windows SmartScreen may ask for confirmation.`n`n${fence}text`n" +
        [IO.File]::ReadAllText($sums, $utf8).Trim() + "`n$fence`n"
    [IO.File]::WriteAllText($notes, $text, $utf8)
    Invoke-Gh release create $tag @files --repo $Repo --target $head --title "MCQ Advanced $version" --notes-file $notes --latest | Out-Null
} elseif (@(Get-Mismatches $manifest $existing).Count) {
    if (-not $Replace) { throw "Release $tag exists with other files; -Replace uploads the site's over them." }
    Invoke-Gh release upload $tag @files --repo $Repo --clobber | Out-Null
}

# What GitHub now serves, by its own digests, and the commit the tag names.
$served = Get-ReleaseDigests $tag
$bad = @(Get-Mismatches $manifest $served)
if (-not $served -or $bad.Count) { throw "GitHub release $tag does not serve the site's files: $($bad -join ', ')" }
$tagged = (Invoke-Gh api "repos/$Repo/commits/$tag" --jq .sha).Trim()
if ($tagged -ne $head) { throw "Tag $tag names $tagged, not the commit of version ${version}: $head." }
foreach ($name in $manifest.Keys) { "PASS  $Repo $tag  $name  $($manifest[$name])" }
