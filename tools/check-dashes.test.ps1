# Self-test of check-dashes.ps1: builds a throwaway git repo and checks what is flagged.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File check-dashes.test.ps1
$ErrorActionPreference = 'Stop'
$em = [string][char]0x2014
$en = [string][char]0x2013
$utf8 = New-Object System.Text.UTF8Encoding($false)
$repo = Join-Path ([IO.Path]::GetTempPath()) ('dashes-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $repo | Out-Null
try {
    git -C $repo init -q
    function Put($path, $text) {
        $full = Join-Path $repo $path
        New-Item -ItemType Directory -Force (Split-Path -Parent $full) | Out-Null
        [IO.File]::WriteAllText($full, $text, $utf8)
    }
    Put 'src/bad.rs' "let a = `"Saved $em done`";`n"
    Put 'src/ok.rs' ("// Comment $em fine`nlet b = `"x`"; // tail $em fine`nlet c = `"$em`";`n" +
        "/* block $em`n still comment $em */`nlet d = `"https://x.y $en 1${en}5`";`nlet e = `"1${en}5`";`n" +
        "let f = '$em'; // dash-ok is not needed here`nlet g = `"input $em data`"; // dash-ok`n")
    Put 'src/after.js' "// see src/*.js`nconst t = 'Saved $em done';`n"
    Put 'src/heading.py' "a = f`"# Mail $em 2`"  # count $em fine`nb = 1  # note $em fine`n"
    Put 'src/url.rs' "let u = `"see https://x $em y`";`n"
    Put 'src/escape.js' "const a = 'Untitled \u2014 App';`nconst b = '<b>x &mdash; y</b>';`nconst c = '\u2014'; // dash-ok`n"
    Put 'src/bad_en.py' "x = f`"a $en b`"`n"
    Put 'web/page.html' "<!-- note $em fine -->`n<td>$em</td>`n<title>Shop $em Home</title>`n"
    Put 'tests/t.rs' "assert!(`"a $em b`");`n"
    Put 'web/stamp.test.mjs' "assert.ok(`"a $em b`");`n"
    Put 'README.md' "Text $em fine`n"
    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'check-dashes.ps1') -Root $repo
    $code = $LASTEXITCODE
    $flagged = @($out | Where-Object { $_ -match '^\s+\S+:\d+:' } | ForEach-Object { ($_.Trim() -split ': ')[0] })
    $expected = @('src/after.js:2', 'src/escape.js:1', 'src/escape.js:2', 'src/heading.py:1', 'src/url.rs:1', 'src/bad.rs:1', 'src/bad_en.py:1', 'src/ok.rs:6', 'web/page.html:3')
    $diff = Compare-Object ($flagged | Sort-Object) ($expected | Sort-Object)
    if ($code -ne 1 -or $diff) {
        Write-Output "FAIL (exit $code). Flagged:"; $flagged | ForEach-Object { Write-Output "  $_" }
        exit 1
    }
    Write-Output 'check-dashes.test: PASS'
} finally { Remove-Item -Recurse -Force $repo -ErrorAction SilentlyContinue }
