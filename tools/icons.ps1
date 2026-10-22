# Renders the application icon from packaging/icons/mcq-advanced.svg (the website's icon) into
# packaging/icons: mcq-advanced-256.png (desktop windows, Linux menus) and mcq-advanced.ico
# (Windows programs and installer, 16 to 256 pixels). Microsoft Edge draws the SVG once, large;
# System.Drawing scales it down. Run it again after changing the SVG, and commit the result.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File tools/icons.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$icons = Join-Path $root 'packaging/icons'
$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge is required to draw the SVG.' }

$work = Join-Path ([IO.Path]::GetTempPath()) ('mcq-icons-' + [guid]::NewGuid())
New-Item -ItemType Directory $work | Out-Null
try {
    $big = 1024
    $svg = [IO.File]::ReadAllText((Join-Path $icons 'mcq-advanced.svg'))
    $page = "<!doctype html><html><body style=`"margin:0;background:transparent`"><div style=`"width:${big}px;height:${big}px`">$svg</div></body></html>"
    $page = $page.Replace('<svg ', '<svg width="100%" height="100%" ')
    [IO.File]::WriteAllText((Join-Path $work 'icon.html'), $page)
    $shot = Join-Path $work 'icon.png'
    $process = Start-Process -FilePath $edge -Wait -PassThru -WindowStyle Hidden -ArgumentList @(
        '--headless', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=1',
        '--default-background-color=00000000', "--user-data-dir=$work\profile",
        "--window-size=$big,$big", "--screenshot=$shot", ([Uri](Join-Path $work 'icon.html')).AbsoluteUri)
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $shot)) { throw 'Edge could not draw the icon' }
    $source = [Drawing.Image]::FromFile($shot)
    try {
        if ($source.Width -ne $big -or $source.Height -ne $big) { throw "Edge drew $($source.Width)x$($source.Height), not ${big}x$big" }
        # One PNG per size, scaled down in one step from the large drawing.
        $sizes = @(16, 24, 32, 48, 64, 128, 256)
        $pngs = @{}
        foreach ($size in $sizes) {
            $bitmap = New-Object Drawing.Bitmap($size, $size, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
            $graphics = [Drawing.Graphics]::FromImage($bitmap)
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $graphics.CompositingQuality = [Drawing.Drawing2D.CompositingQuality]::HighQuality
            $graphics.DrawImage($source, 0, 0, $size, $size)
            $graphics.Dispose()
            $stream = New-Object IO.MemoryStream
            $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
            $bitmap.Dispose()
            $pngs[$size] = $stream.ToArray()
        }
    } finally { $source.Dispose() }
    [IO.File]::WriteAllBytes((Join-Path $icons 'mcq-advanced-256.png'), $pngs[256])
    # ICO: a 6-byte header, one 16-byte entry per image, then the PNG images themselves.
    $ico = New-Object IO.MemoryStream
    $writer = New-Object IO.BinaryWriter($ico)
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    foreach ($size in $sizes) {
        $edgeByte = if ($size -ge 256) { 0 } else { $size }
        $writer.Write([byte]$edgeByte); $writer.Write([byte]$edgeByte); $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$pngs[$size].Length); $writer.Write([uint32]$offset)
        $offset += $pngs[$size].Length
    }
    foreach ($size in $sizes) { $writer.Write($pngs[$size]) }
    $writer.Flush()
    [IO.File]::WriteAllBytes((Join-Path $icons 'mcq-advanced.ico'), $ico.ToArray())
    Write-Output "Icons written: packaging/icons/mcq-advanced-256.png, packaging/icons/mcq-advanced.ico ($($sizes -join ', ') px)"
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
