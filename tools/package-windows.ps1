# Builds the Windows installer of the two desktop programs (Studio and player) into target/dist:
# MCQ-Advanced-Setup-<version>.exe and its line in target/dist/SHA256-windows.txt.
# Needs Inno Setup 6 (ISCC.exe). The version is the workspace's, from Cargo.toml.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/package-windows.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
# What is published must be what is committed.
if (git -C $root status --porcelain) { throw 'Commit every change before building what is published.' }
$version = [regex]::Match([IO.File]::ReadAllText((Join-Path $root 'Cargo.toml')), '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $version) { throw 'No workspace version in Cargo.toml' }
# The licenses of what the desktop builds link travel with them: the notices must be current.
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'notices.ps1') -Check
if ($LASTEXITCODE -ne 0) { throw 'Third-party notices are out of date' }

. (Join-Path $PSScriptRoot 'toolchain.ps1')
Push-Location $root
try {
    $ErrorActionPreference = 'Continue'
    foreach ($app in 'mcq-player', 'mcq-studio') {
        & cargo build --release --locked -p $app --features desktop 2>&1 | ForEach-Object { "$_" }
        if ($LASTEXITCODE -ne 0) { throw "Desktop build of $app failed" }
    }
    $ErrorActionPreference = 'Stop'
} finally { Pop-Location }

$compiler = Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'
if (-not (Test-Path -LiteralPath $compiler)) { $compiler = (Get-Command ISCC.exe -ErrorAction Stop).Source }
& $compiler /Q "/DAppVersion=$version" (Join-Path $root 'packaging/windows/mcq-advanced.iss')
if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed' }
$name = "MCQ-Advanced-Setup-$version.exe"
$setup = Join-Path $root "target/dist/$name"
$hash = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash
[IO.File]::WriteAllText((Join-Path $root 'target/dist/SHA256-windows.txt'), "$hash  $name`n", (New-Object Text.UTF8Encoding($false)))
Write-Output "Installer: $setup"
Write-Output "SHA256: $hash"
