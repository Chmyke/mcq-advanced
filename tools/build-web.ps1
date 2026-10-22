# Builds a web app (the player or the Studio) into target/web/<app>: a static folder for any web
# server or for GitHub Pages. Needs wasm-bindgen at the version of Cargo.lock.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-web.ps1 [-App player|studio] [-Debug]
# Keep this file pure ASCII.
param(
    [ValidateSet('player', 'studio')][string]$App = 'player',
    [switch]$Debug
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'toolchain.ps1')
# The tool that cuts the program for the browser must be the version of the library inside it.
$lock = [IO.File]::ReadAllText((Join-Path $root 'Cargo.lock'))
$entry = [regex]::Match($lock, '(?m)^name = "wasm-bindgen"\r?\nversion = "([^"]+)"')
if (-not $entry.Success) { throw 'wasm-bindgen is not in Cargo.lock.' }
$wanted = $entry.Groups[1].Value
$install = "cargo install wasm-bindgen-cli --version $wanted --locked --force"
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) { throw "wasm-bindgen $wanted is required: run $install" }
$have = [regex]::Match((wasm-bindgen --version | Out-String), '\d+\.\d+\.\d+').Value
if ($have -ne $wanted) { throw "wasm-bindgen $have is installed, Cargo.lock needs $wanted : run $install" }

$package = "mcq-$App"
$module = "mcq_$App"
$profile = if ($Debug) { 'debug' } else { 'release' }
$out = Join-Path $root "target/web/$App"
# A failed build leaves no folder behind that could be published by mistake.
if (Test-Path -LiteralPath $out) { Remove-Item -LiteralPath $out -Recurse -Force }
try {
    $ErrorActionPreference = 'Continue'
    $cargoArgs = @('build', '--locked', '--target', 'wasm32-unknown-unknown', '-p', $package, '--features', 'web')
    if (-not $Debug) { $cargoArgs += '--release' }
    & cargo @cargoArgs 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw 'Web build failed' }
    $wasm = Join-Path $root "target/wasm32-unknown-unknown/$profile/$package.wasm"
    & wasm-bindgen --target web --no-typescript --out-dir $out --out-name $module $wasm
    if ($LASTEXITCODE -ne 0) { throw 'wasm-bindgen failed' }
    $ErrorActionPreference = 'Stop'
    Copy-Item -LiteralPath (Join-Path $root "apps/$package/web/index.html") $out
    Copy-Item -LiteralPath (Join-Path $root 'LICENSE') $out
} catch {
    if (Test-Path -LiteralPath $out) { Remove-Item -LiteralPath $out -Recurse -Force }
    throw
}
$size = (Get-Item -LiteralPath (Join-Path $out "$($module)_bg.wasm")).Length
Write-Output ("Web {0}: target/web/{0} ({1:N2} MiB program)" -f $App, ($size / 1MB))
