# Fails when something that must never be public could be committed: real subjects, answer keys,
# results, keys, or private working notes.
# Rules, checked on every file git would commit (tracked + untracked not ignored):
#   1. forbidden extensions (packs, results, keys, office files that hold class data);
#   2. forbidden paths (private notes, subject and result folders, agent instructions);
#   3. private fingerprints: ids and titles of real questions, one per line in
#      _prive/empreintes-contenu.txt. That file is excluded from git, so on CI only rules 1 and 2 run.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File guard-content.ps1 [-Root dir] [-Fingerprints file]
# Keep this file pure ASCII.
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [string]$Fingerprints = ''
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)

$badExt = '(?i)\.(qcmpack|qcmres|qcmsource|key|secret|pem|p12|pfx|xlsx|xls|docx|doc)$'
$badPath = '(?i)((^|/)(_prive|sujets|resultats|\.claude)(/|$)|(^|/)(claude|agents|cadrage)\.md$)'
if (-not $Fingerprints) { $Fingerprints = Join-Path $Root '_prive/empreintes-contenu.txt' }

$utf8 = New-Object System.Text.UTF8Encoding($false)
$pattern = $null
if (Test-Path -LiteralPath $Fingerprints) {
    $tokens = @([IO.File]::ReadAllLines($Fingerprints, $utf8) | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    if ($tokens.Count -gt 0) {
        # A fingerprint matches as a whole token: "q-intro" must not fire inside "q-intro-2".
        $alternation = ($tokens | ForEach-Object { [regex]::Escape($_) }) -join '|'
        $pattern = New-Object System.Text.RegularExpressions.Regex("(?<![\w-])($alternation)(?![\w-])", 'IgnoreCase')
    }
    Write-Output ("guard-content: {0} private fingerprint(s) loaded" -f $tokens.Count)
} else {
    Write-Output 'guard-content: no private fingerprint file, structural rules only'
}

Push-Location $Root
try {
    $files = @(git -c core.quotepath=off ls-files --cached --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) { throw "git ls-files failed in $Root" }
    $hits = New-Object System.Collections.Generic.List[string]
    foreach ($file in $files) {
        if ($file -match $badExt) { $hits.Add("$file : forbidden extension"); continue }
        if ($file -match $badPath) { $hits.Add("$file : forbidden path"); continue }
        if (-not $pattern -or -not (Test-Path -LiteralPath $file)) { continue }
        $bytes = [IO.File]::ReadAllBytes((Join-Path $Root $file))
        # Binary files (a NUL in the first 8 KB) are judged by rules 1 and 2 only.
        if ([Array]::IndexOf($bytes, [byte]0, 0, [Math]::Min($bytes.Length, 8192)) -ge 0) { continue }
        $match = $pattern.Match($utf8.GetString($bytes))
        if ($match.Success) { $hits.Add("$file : private fingerprint '$($match.Value)'") }
    }
    if ($hits.Count -gt 0) {
        Write-Output 'Private content could be committed (move it out, or to _prive/):'
        $hits | ForEach-Object { Write-Output "  $_" }
        exit 1
    }
    Write-Output 'guard-content: OK'
} finally { Pop-Location }
