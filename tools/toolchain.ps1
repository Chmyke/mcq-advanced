# Loads the MSVC linker into the current shell and pins the Rust flags every build shares.
# Dot-source it before any cargo command: . tools/toolchain.ps1
# PITFALL: never run cargo from Git Bash on Windows; its link.exe shadows MSVC.
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $locator = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $locator)) { throw 'Visual Studio C++ build tools are required.' }
    $studio = & $locator -latest -products '*' -property installationPath
    if (-not $studio) { throw 'Visual Studio C++ build tools are required.' }
    # Launch-VsDevShell calls vswhere by name: make it findable.
    $env:PATH = (Split-Path -Parent $locator) + ';' + $env:PATH
    & (Join-Path $studio 'Common7/Tools/Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null
    if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
        $installed = Get-ChildItem (Join-Path $studio 'VC/Tools/MSVC') -Directory |
            Where-Object { Test-Path (Join-Path $_.FullName 'bin/HostX64/x64/link.exe') } |
            Sort-Object Name -Descending | Select-Object -First 1
        if (-not $installed) { throw 'No x64 MSVC linker found.' }
        $env:PATH = (Join-Path $installed.FullName 'bin/HostX64/x64') + ';' + $env:PATH
        $env:LIB = (Join-Path $installed.FullName 'lib/x64') + ';' + $env:LIB
    }
}
# Binaries are public: map the profile folder to "~" so no build reveals the author's paths.
# cargo keys its cache on this flag, so every build must get the same one.
$remap = "--remap-path-prefix=$env:USERPROFILE=~"
$flags = @($env:CARGO_ENCODED_RUSTFLAGS -split [char]0x1f | Where-Object { $_ })
if ($flags -notcontains $remap) { $env:CARGO_ENCODED_RUSTFLAGS = ($flags + $remap) -join [char]0x1f }
