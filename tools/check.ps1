# The one authoritative check of MCQ Advanced. Run it at the end of every change; its verdict is final.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
$tools = $PSScriptRoot
$root = Split-Path -Parent $tools
$failed = @()

$scripts = @(
    'guard-content.test.ps1',
    'guard-content.ps1',
    'check-dashes.test.ps1',
    'check-dashes.ps1'
)
foreach ($step in $scripts) {
    Write-Output "== $step"
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $tools $step)
    if ($LASTEXITCODE -ne 0) { $failed += $step }
}

. (Join-Path $tools 'toolchain.ps1')
# Native tools write progress to stderr: keep it from turning into terminating errors.
$ErrorActionPreference = 'Continue'
# The pure core must build for the browser: no file, clock or window inside it.
$core = @('-p', 'mcq-model', '-p', 'mcq-format', '-p', 'mcq-grade', '-p', 'mcq-lint', '-p', 'mcq-crypto', '-p', 'mcq-engine', '-p', 'mcq-edit', '-p', 'mcq-publish')
$cargo = @(
    @{ Name = 'rustfmt'; Args = @('fmt', '--all', '--check') },
    @{ Name = 'clippy'; Args = @('clippy', '--workspace', '--all-targets', '--quiet', '--', '-D', 'warnings') },
    @{ Name = 'tests'; Args = @('test', '--workspace', '--quiet') },
    @{ Name = 'wasm core'; Args = @('build', '--quiet', '--target', 'wasm32-unknown-unknown') + $core },
    # Both apps in both shells: the browser build and the desktop window.
    @{ Name = 'player web'; Args = @('build', '--quiet', '--target', 'wasm32-unknown-unknown', '-p', 'mcq-player', '--features', 'web') },
    @{ Name = 'player desktop'; Args = @('check', '--quiet', '-p', 'mcq-player', '--features', 'desktop') },
    @{ Name = 'studio web'; Args = @('build', '--quiet', '--target', 'wasm32-unknown-unknown', '-p', 'mcq-studio', '--features', 'web') },
    @{ Name = 'studio desktop'; Args = @('check', '--quiet', '-p', 'mcq-studio', '--features', 'desktop') }
)
Push-Location $root
try {
    foreach ($step in $cargo) {
        Write-Output "== cargo $($step.Name)"
        & cargo @($step.Args) 2>&1 | ForEach-Object { "$_" }
        if ($LASTEXITCODE -ne 0) { $failed += "cargo $($step.Name)" }
    }

    # Real papers never enter the repository. When a private list of their folders exists, every
    # answer key in them must pass the CLI without error; on CI the list is absent and this is skipped.
    $corpus = Join-Path $root '_prive/corpus-local.txt'
    if (Test-Path -LiteralPath $corpus) {
        Write-Output '== local corpus'
        $utf8 = New-Object System.Text.UTF8Encoding($false)
        $folders = @([IO.File]::ReadAllLines($corpus, $utf8) | ForEach-Object { $_.Trim() } | Where-Object { $_ -and -not $_.StartsWith('#') })
        $papers = @($folders | ForEach-Object { Get-ChildItem -LiteralPath $_ -Filter '*.md' -File } |
            Where-Object { $_.Name -notmatch ' - Sujet\.md$' } | ForEach-Object { $_.FullName })
        if ($papers.Count -eq 0) {
            Write-Output 'local corpus: no paper found'
            $failed += 'local corpus'
        } else {
            & cargo run --quiet -p mcq-cli -- check --quiet @papers 2>&1 | Where-Object { "$_" -match 'error\[|FAIL|OK:' } | ForEach-Object { "$_" }
            if ($LASTEXITCODE -ne 0) { $failed += 'local corpus' }
        }
    }
} finally { Pop-Location }

if ($failed.Count -gt 0) {
    Write-Output ("CHECK: FAIL ({0})" -f ($failed -join ', '))
    exit 1
}
Write-Output 'CHECK: OK'
