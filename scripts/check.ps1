[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    foreach ($arguments in @(
        @('fmt', '--all', '--check'),
        @('clippy', '--locked', '--all-targets', '--', '-D', 'warnings'),
        @('test', '--locked', '--all-targets')
    )) {
        & cargo @arguments
        if ($LASTEXITCODE) { throw "cargo $arguments failed: $LASTEXITCODE" }
    }
    node scripts/check-ui.mjs
    if ($LASTEXITCODE) { throw 'UI check failed.' }
    python scripts/release.py check
    if ($LASTEXITCODE) { throw 'Release metadata check failed.' }
    python -m unittest discover -s scripts -p 'test_*.py'
    if ($LASTEXITCODE) { throw 'Release tooling tests failed.' }
} finally {
    Pop-Location
}
