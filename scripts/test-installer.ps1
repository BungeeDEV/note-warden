[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'This installation smoke test runs only on an ephemeral GitHub Actions runner.'
}
$repoRoot = Split-Path -Parent $PSScriptRoot
$metadata = (& cargo metadata --locked --format-version 1 --no-deps) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'Cannot read package metadata.' }
$version = ($metadata.packages | Where-Object { $_.name -eq 'note-warden' } | Select-Object -First 1).version
$setup = Join-Path $metadata.target_directory "installer\github\NoteWarden-$version-setup.exe"
$installDir = Join-Path $env:RUNNER_TEMP 'Note Warden Install Test'
$dataDir = Join-Path $env:RUNNER_TEMP 'note-warden-smoke-data'
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
$marker = Join-Path $dataDir 'preserve-data.txt'
[IO.File]::WriteAllText($marker, 'Preserve this test data during install and uninstall.')
$env:NOTE_WARDEN_DATA_DIR = $dataDir
$arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', '/NOICONS', ('/DIR="' + $installDir + '"'))
for ($pass = 1; $pass -le 2; $pass++) {
    $log = Join-Path $env:RUNNER_TEMP "note-warden-install-$pass.log"
    $process = Start-Process -FilePath $setup -ArgumentList ($arguments + ('/LOG="' + $log + '"')) -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0) { throw "Installation pass $pass failed: $($process.ExitCode)" }
    $binary = Join-Path $installDir 'note-warden.exe'
    $expectedHash = (Get-FileHash -LiteralPath (Join-Path $metadata.target_directory 'release\note-warden.exe') -Algorithm SHA256).Hash
    if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash -ne $expectedHash) {
        throw 'Installed binary differs from the signed release payload.'
    }
    foreach ($name in @('vcruntime140.dll', 'LICENSE-LUCIDE.txt', 'unins000.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $installDir $name))) { throw "Installed file missing: $name" }
    }
    if (-not (Test-Path -LiteralPath $marker)) { throw 'User data was removed during installation.' }
    if (Get-Process note-warden -ErrorAction SilentlyContinue) { throw 'An ordinary silent install unexpectedly started the app.' }
}
$uninstaller = Join-Path $installDir 'unins000.exe'
$process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -Wait -PassThru -WindowStyle Hidden
if ($process.ExitCode -ne 0) { throw "Uninstall failed: $($process.ExitCode)" }
if (Test-Path -LiteralPath (Join-Path $installDir 'note-warden.exe')) { throw 'App binary remains after uninstall.' }
if (-not (Test-Path -LiteralPath $marker)) { throw 'User data was removed during uninstall.' }
Write-Host 'Installer smoke passed: first install, reinstall to the same path, payload hashes, runtime, uninstall, and data retention.'
