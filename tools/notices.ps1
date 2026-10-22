# Builds THIRD-PARTY-NOTICES.txt from what the two web builds (player and Studio) embed: every
# crate linked into either program for wasm32-unknown-unknown with the `web` feature, the
# workspace's own crates aside. With -Check, writes nothing and fails when the committed file is
# out of date.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/notices.ps1 [-Check]
# Adapted from the same script in Asciitect. Keep this file pure ASCII.
param([switch]$Check)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root 'THIRD-PARTY-NOTICES.txt'
$utf8 = New-Object Text.UTF8Encoding($false)
$platform = 'wasm32-unknown-unknown'
$apps = @('mcq-player', 'mcq-studio')
# Words the text of each license always contains. A license missing here is never relied on:
# someone has to read it before it ships.
$markers = [ordered]@{
    'MIT'          = 'Permission is hereby granted'
    'Apache-2.0'   = 'Apache License'
    'BSD-2-Clause' = 'Redistributions of source code'
    'BSD-3-Clause' = 'Redistributions of source code'
    'ISC'          = 'Permission to use, copy, modify'
    'Zlib'         = 'This software is provided ''as-is'''
    'BSL-1.0'      = 'Boost Software License'
    'Unicode-3.0'  = 'UNICODE LICENSE V3'
    'Unlicense'    = 'This is free and unencumbered software'
    'CC0-1.0'      = 'CC0 1.0 Universal'
    'MIT-0'        = 'MIT No Attribution'
    '0BSD'         = 'Permission to use, copy, modify'
}
# Crates that ship no license text of their own (their repository holds it at the workspace
# root), with the copyright notice taken from their Cargo.toml authors: their published sources
# carry no other copyright line. MIT and Apache-2.0 both ask for that notice to travel with the
# work, so it is printed in the crate's entry. They rely on the standard texts present elsewhere
# in the notices. A text-less crate missing here stops the script until someone has read its
# license and found its holders; an entry that is no longer needed stops it too, so the table
# cannot rot.
$copyright = [ordered]@{
    'dioxus'                      = @('Copyright (c) Jonathan Kelley, Dioxus Labs, ealmloff')
    'dioxus-cli-config'           = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-config-macros'        = @('Copyright (c) Dioxus Labs')
    'dioxus-core'                 = @('Copyright (c) Jonathan Kelley')
    'dioxus-core-types'           = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-devtools'             = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-devtools-types'       = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-document'             = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-history'              = @('Copyright (c) Jonathan Kelley')
    'dioxus-hooks'                = @('Copyright (c) Jonathan Kelley')
    'dioxus-html'                 = @('Copyright (c) Jonathan Kelley')
    'dioxus-interpreter-js'       = @('Copyright (c) Jonathan Kelley')
    'dioxus-signals'              = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-stores'               = @('Copyright (c) Jonathan Kelley, Evan Almloff')
    'dioxus-web'                  = @('Copyright (c) Jonathan Kelley')
    'generational-box'            = @('Copyright (c) Evan Almloff')
    'math-core'                   = @('Copyright (c) Hiromu Sugiura, Thomas MK')
    'math-core-renderer-internal' = @('Copyright (c) Hiromu Sugiura, Thomas MK')
    'sledgehammer_bindgen'        = @('Copyright (c) Evan Almloff')
    'sledgehammer_utils'          = @('Copyright (c) Evan Almloff')
    'subsecond'                   = @('Copyright (c) Jonathan Kelley')
    'subsecond-types'             = @('Copyright (c) Jonathan Kelley')
    'warnings'                    = @('Copyright (c) Evan Almloff')
}
$textless = @($copyright.Keys)
# Whether $script:haystack holds the texts a license expression asks for: every term of an AND,
# one term of an OR. AND binds tighter than OR, "X WITH exception" counts as X, and a
# reciprocal or unknown license is never the one relied on. The whole expression must be
# consumed and its parentheses must balance.
function Test-Term {
    $token = $script:tokens[$script:at]
    $script:at++
    if ($null -eq $token) {
        $script:malformed = $true
        return $false
    }
    if ($token -eq '(') {
        $ok = Test-Or
        if ($script:at -lt $script:tokens.Count -and $script:tokens[$script:at] -eq ')') {
            $script:at++
        } else {
            $script:malformed = $true
        }
    } else {
        $ok = $token -notmatch 'GPL' -and $markers.Contains($token) -and
            $script:haystack.IndexOf($markers[$token], [StringComparison]::OrdinalIgnoreCase) -ge 0
    }
    if ($script:at -lt $script:tokens.Count -and $script:tokens[$script:at] -eq 'WITH') {
        # The exception name must exist and must not be an operator or a parenthesis.
        if ($script:at + 1 -lt $script:tokens.Count -and $script:tokens[$script:at + 1] -notin @('(', ')', 'AND', 'OR', 'WITH')) {
            $script:at += 2
        } else {
            $script:malformed = $true
        }
    }
    $ok
}
function Test-And {
    $ok = Test-Term
    while ($script:at -lt $script:tokens.Count -and $script:tokens[$script:at] -eq 'AND') {
        $script:at++
        $next = Test-Term
        $ok = $ok -and $next
    }
    $ok
}
function Test-Or {
    $ok = Test-And
    while ($script:at -lt $script:tokens.Count -and $script:tokens[$script:at] -eq 'OR') {
        $script:at++
        $next = Test-And
        $ok = $ok -or $next
    }
    $ok
}
# Whether $text holds what a whole license expression asks for. Legacy manifests write
# "MIT/Apache-2.0" for a choice.
function Test-License([string]$expression, [string]$text) {
    $script:tokens = @([regex]::Matches($expression.Replace('/', ' OR '), '\(|\)|[^\s()]+') | ForEach-Object { $_.Value })
    $script:at = 0
    $script:haystack = $text
    $script:malformed = $false
    if ($script:tokens.Count -eq 0) { return $false }
    $ok = Test-Or
    return [bool]($ok -and -not $script:malformed -and $script:at -eq $script:tokens.Count)
}
# The evaluator proves itself on every run: each failure path below is exercised here.
$mit = $markers['MIT']
$apache = $markers['Apache-2.0']
$unicode = $markers['Unicode-3.0']
$cases = @(
    @('MIT', $mit, $true),
    @('MIT', '', $false),
    @('MIT AND GPL-3.0-only', $mit, $false),
    @('GPL-3.0-or-later', $mit, $false),
    @('LGPL-2.1', $mit, $false),
    @('MPL-2.0', "$mit $apache", $false),
    @('Apache-2.0 OR GPL-2.0-only', $apache, $true),
    @('(MIT OR Apache-2.0) AND Unicode-3.0', $mit, $false),
    @('(MIT OR Apache-2.0) AND Unicode-3.0', "$mit $unicode", $true),
    @('MIT OR Apache-2.0 AND Unicode-3.0', $mit, $true),
    @('MIT AND Apache-2.0', $mit, $false),
    @('Apache-2.0 WITH LLVM-exception', $apache, $true),
    @('MIT WITH (', $mit, $false),
    @('Apache-2.0 WITH )', $apache, $false),
    @('(Apache-2.0 WITH) )', $apache, $false),
    @('MIT WITH', $mit, $false),
    @('MIT WITH AND Apache-2.0', "$mit $apache", $false),
    @('MIT/Apache-2.0', $mit, $true),
    @('MIT GPL-3.0-only', $mit, $false),
    @('MIT FOO', $mit, $false),
    @('(MIT', $mit, $false),
    @('MIT)', $mit, $false),
    @('MIT AND', $mit, $false)
)
foreach ($case in $cases) {
    if ((Test-License $case[0] $case[1]) -ne $case[2]) {
        throw "Self-test failed: '$($case[0])' should give $($case[2])"
    }
}
Push-Location $root
try {
    # The graph with every feature on, so that the `web` features of both apps are in it; what is
    # actually built is narrowed below.
    $metadata = cargo metadata --locked --format-version 1 --all-features --filter-platform $platform | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect dependency licenses' }
    $packages = @{}
    $nodes = @{}
    foreach ($package in $metadata.packages) { $packages[$package.id] = $package }
    foreach ($node in $metadata.resolve.nodes) { $nodes[$node.id] = $node }
    # One list for the two programs that are published: a crate built into either is credited.
    $visited = New-Object 'System.Collections.Generic.HashSet[string]'
    foreach ($appName in $apps) {
        # What cargo links into this program: its own web build, normal dependencies only. Build
        # scripts and proc macros run on the build machine and are not shipped; the tests are not
        # either. cargo metadata alone is not enough: it counts every feature and edge kind.
        $tree = cargo tree --locked -p $appName --features web -e normal,no-proc-macro --target $platform --prefix none --format '{p}'
        if ($LASTEXITCODE -ne 0) { throw "Cannot list the crates built into $appName" }
        $built = New-Object 'System.Collections.Generic.HashSet[string]'
        foreach ($line in $tree) {
            if ($line -match '^(\S+) v(\S+)') { [void]$built.Add($Matches[1] + ' ' + $Matches[2]) }
        }
        $app = $metadata.packages | Where-Object { $_.name -eq $appName }
        $queue = New-Object 'System.Collections.Generic.Queue[string]'
        $queue.Enqueue($app.id)
        $reached = New-Object 'System.Collections.Generic.HashSet[string]'
        while ($queue.Count -gt 0) {
            $id = $queue.Dequeue()
            if (-not $reached.Add($id)) { continue }
            [void]$visited.Add($id)
            foreach ($dep in $nodes[$id].deps) {
                $crate = $packages[$dep.pkg]
                if (@($dep.dep_kinds | Where-Object { $null -eq $_.kind -or $_.kind -eq 'normal' }).Count -gt 0 -and
                    $built.Contains($crate.name + ' ' + $crate.version)) {
                    $queue.Enqueue($dep.pkg)
                }
            }
        }
    }
    $text = New-Object Text.StringBuilder
    [void]$text.Append("MCQ Advanced - third-party notices`n`n")
    [void]$text.Append("MCQ Advanced is free software under the GNU General Public License, version 3 (see LICENSE).`n")
    [void]$text.Append("Its web builds (the player and the Studio) include the Rust crates below, each under its`n")
    [void]$text.Append("own license, reproduced here with its copyright notices.`n`n")
    [void]$text.Append("Rust crates built into the player or the Studio for the browser`n")
    # Ordinal order, so the file does not depend on the culture of the machine. Workspace crates
    # have no registry source and are not third-party.
    $byKey = New-Object 'System.Collections.Generic.Dictionary[string,object]' ([StringComparer]::Ordinal)
    foreach ($p in @($visited | ForEach-Object { $packages[$_] } | Where-Object { $_.source })) {
        $byKey.Add($p.name + ' ' + $p.version, $p)
    }
    $sortKeys = [string[]]@($byKey.Keys)
    [Array]::Sort($sortKeys, [StringComparer]::Ordinal)
    $ordered = @($sortKeys | ForEach-Object { $byKey[$_] })
    $crateTexts = @{}
    foreach ($p in $ordered) {
        if (-not $p.license) { throw "No license declared: $($p.name) $($p.version)" }
        [void]$text.Append("`n=== $($p.name) $($p.version) - $($p.license) ===`n")
        if ($p.repository) { [void]$text.Append("$($p.repository)`n") }
        $directory = Split-Path -Parent $p.manifest_path
        $files = Get-ChildItem -LiteralPath $directory -File -Recurse
        # Source files are never license texts, whatever their name starts with.
        $licenses = $files | Where-Object {
            $_.Extension -notin @('.rs', '.c', '.h', '.py', '.js', '.toml', '.json') -and
            $_.Name -match '^((UN)?LICEN[CS]E|COPYING|NOTICE)'
        }
        $relative = @($licenses | ForEach-Object { $_.FullName.Substring($directory.Length + 1).Replace('\', '/') })
        [Array]::Sort($relative, [StringComparer]::Ordinal)
        if ($relative.Count -eq 0 -and $copyright.Contains($p.name)) {
            foreach ($line in $copyright[$p.name]) { [void]$text.Append("$line`n") }
        }
        $own = New-Object Text.StringBuilder
        foreach ($name in $relative) {
            $body = [IO.File]::ReadAllText((Join-Path $directory $name)).Replace("`r`n", "`n").TrimEnd()
            [void]$text.Append("`n--- $name ---`n$body`n")
            [void]$own.Append("$body`n")
        }
        $crateTexts[$p.id] = $own.ToString()
    }
    $script:notices = $text.ToString()
    # Each crate answers for itself, except the pinned text-less ones.
    $textlessFound = @($ordered | Where-Object { $crateTexts[$_.id].Length -eq 0 } | ForEach-Object { $_.name } | Sort-Object -Unique)
    foreach ($p in $ordered) {
        if ($crateTexts[$p.id].Length -eq 0 -and $textless -cnotcontains $p.name) {
            throw ('Crate ships no license text: ' + $p.name + ' ' + $p.version + ' - ' + $p.license +
                '. Read its license, find its copyright holders, then add ''' + $p.name + ''' to $copyright in tools/notices.ps1.')
        }
    }
    foreach ($entry in $textless) {
        if ($textlessFound -cnotcontains $entry) {
            throw ('Stale entry in $copyright: ''' + $entry + ''' is not in the dependency tree or now ships a license text. Remove it from tools/notices.ps1.')
        }
    }
    foreach ($p in $ordered) {
        $searched = if ($crateTexts[$p.id].Length -eq 0) { $script:notices } else { $crateTexts[$p.id] }
        if (-not (Test-License $p.license $searched)) {
            throw "License text missing or license to review: $($p.name) $($p.version) - $($p.license)"
        }
    }
    if ($Check) {
        if (-not (Test-Path -LiteralPath $target) -or [IO.File]::ReadAllText($target) -cne $script:notices) {
            throw 'THIRD-PARTY-NOTICES.txt is out of date: run tools/notices.ps1 and commit the result.'
        }
        Write-Output "Third-party notices are current: $(@($ordered).Count) crates"
    } else {
        [IO.File]::WriteAllText($target, $script:notices, $utf8)
        Write-Output "Third-party notices written: $(@($ordered).Count) crates"
    }
} finally { Pop-Location }
