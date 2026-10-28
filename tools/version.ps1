# The workspace version and the commit that set it, shared by the delivery scripts.
# Dot-source it: . tools/version.ps1 ; it needs $root, the repository folder.
# A version's files (installer, Linux archive, online version) are built from the commit that set
# `version = "<version>"` in Cargo.toml, and its GitHub release tags that same commit.
# Keep this file pure ASCII.

function Get-WorkspaceVersion {
    $cargo = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'), (New-Object System.Text.UTF8Encoding $false))
    $version = [regex]::Match($cargo, '(?m)^version = "([^"]+)"').Groups[1].Value
    if (-not $version) { throw 'No workspace version in Cargo.toml' }
    $version
}

# The commit that introduced `version = "<version>"` in Cargo.toml: the oldest that touched it.
function Get-VersionCommit([string]$version) {
    # A regex with dots for the quotes: Windows PowerShell mangles quotes passed to git.
    $pattern = 'version = .' + [regex]::Escape($version) + '.$'
    $commits = @(git -C $root log --format=%H --pickaxe-regex "-S$pattern" -- Cargo.toml)
    if (-not $commits) { throw "No commit sets version $version in Cargo.toml." }
    $commits[-1]
}

# True when the checkout is exactly the commit that set its version, with no local change:
# what a published build must come from.
function Test-ReleasedCheckout {
    if (git -C $root status --porcelain --untracked-files=no) { return $false }
    (git -C $root rev-parse HEAD).Trim() -eq (Get-VersionCommit (Get-WorkspaceVersion))
}
