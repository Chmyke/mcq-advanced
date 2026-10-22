# Self-test of guard-content.ps1: builds a throwaway git repo with planted leaks and checks what is flagged.
# Run: powershell -NoProfile -ExecutionPolicy Bypass -File guard-content.test.ps1
# Keep this file pure ASCII.
$ErrorActionPreference = 'Stop'
$utf8 = New-Object System.Text.UTF8Encoding($false)
$work = Join-Path ([IO.Path]::GetTempPath()) ('guard-' + [Guid]::NewGuid().ToString('N'))
$repo = Join-Path $work 'repo'
New-Item -ItemType Directory $repo | Out-Null
try {
    git -C $repo init -q
    function Put($path, $text) {
        $full = Join-Path $repo $path
        New-Item -ItemType Directory -Force (Split-Path -Parent $full) | Out-Null
        [IO.File]::WriteAllText($full, $text, $utf8)
    }
    $prints = Join-Path $work 'prints.txt'
    [IO.File]::WriteAllText($prints, "zz-secret-question`nA Real Exam Title`n", $utf8)

    Put 'build/demo.qcmpack' 'x'
    Put 'class/grades.xlsx' 'x'
    Put 'Sujets/exam.md' 'x'
    Put 'docs/resultats/a.txt' 'x'
    Put '_prive/notes.md' 'x'
    Put 'CLAUDE.md' 'x'
    Put 'src/leak.rs' "// id: zz-secret-question`n"
    Put 'docs/title.md' "# a real exam title`n"
    Put 'src/near.rs' "let a = `"zz-secret-question-2`"; let b = `"xzz-secret-question`";`n"
    Put 'src/clean.rs' "fn main() {}`n"
    Put 'examples/demo.qcm.md' "## Q demo-dict-cout | 2 | 30s`n"

    $out = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'guard-content.ps1') -Root $repo -Fingerprints $prints
    $code = $LASTEXITCODE
    $flagged = @($out | Where-Object { $_ -match '^\s+\S.* : ' } | ForEach-Object { ($_.Trim() -split ' : ')[0] })
    $expected = @('build/demo.qcmpack', 'class/grades.xlsx', 'Sujets/exam.md', 'docs/resultats/a.txt',
        '_prive/notes.md', 'CLAUDE.md', 'src/leak.rs', 'docs/title.md')
    $diff = Compare-Object ($flagged | Sort-Object) ($expected | Sort-Object)
    if ($code -ne 1 -or $diff) {
        Write-Output "FAIL (exit $code). Flagged:"; $flagged | ForEach-Object { Write-Output "  $_" }
        exit 1
    }
    Write-Output 'guard-content.test: PASS'
} finally { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
