# The one authoritative check of MCQ Advanced. Run it at the end of every change; its verdict is final.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/check.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
$tools = $PSScriptRoot
$steps = @(
    'guard-content.test.ps1',
    'guard-content.ps1',
    'check-dashes.test.ps1',
    'check-dashes.ps1'
)
$failed = @()
foreach ($step in $steps) {
    Write-Output "== $step"
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $tools $step)
    if ($LASTEXITCODE -ne 0) { $failed += $step }
}
if ($failed.Count -gt 0) {
    Write-Output ("CHECK: FAIL ({0})" -f ($failed -join ', '))
    exit 1
}
Write-Output 'CHECK: OK'
