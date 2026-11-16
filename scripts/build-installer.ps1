[CmdletBinding()]
param(
    [ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')]
    [string]$GitHubRepository,
    [string]$PublicKeyFile,
    [string]$SigningKeyFile,
    [string]$InnoCompiler,
    [string]$VCRuntimeDirectory
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}

# Tools are installed explicitly by the owner; this script never downloads or installs them.
if (-not $InnoCompiler) {
    $command = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($command) { $InnoCompiler = $command.Source }
    else {
        $candidates = @(
            (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
            (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
            (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
            (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
            (Join-Path $env:LOCALAPPDATA 'NoteWardenBuildTools\InnoSetup\ISCC.exe')
        )
        $InnoCompiler = $candidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    }
}
if (-not $InnoCompiler -or -not (Test-Path -LiteralPath $InnoCompiler -PathType Leaf)) {
    throw 'Inno Setup compiler missing. Install Inno Setup 6/7 from jrsoftware.org, or supply -InnoCompiler.'
}
if (-not $VCRuntimeDirectory) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $visualStudio = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($LASTEXITCODE -eq 0 -and $visualStudio) {
            $redistRoot = Join-Path $visualStudio 'VC\Redist\MSVC'
            $VCRuntimeDirectory = Get-ChildItem -LiteralPath $redistRoot -Directory -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending | ForEach-Object {
                    Get-ChildItem -Path (Join-Path $_.FullName 'x64\Microsoft.VC*.CRT') -Directory -ErrorAction SilentlyContinue
                } | Select-Object -First 1 -ExpandProperty FullName
        }
    }
}
$runtime = if ($VCRuntimeDirectory) { Join-Path $VCRuntimeDirectory 'vcruntime140.dll' } else { $null }
if (-not $runtime -or -not (Test-Path -LiteralPath $runtime -PathType Leaf)) {
    throw 'MSVC x64 redistributable vcruntime140.dll missing. Supply -VCRuntimeDirectory from the build toolchain Redist directory.'
}
$minisign = $null
$publicKey = $null
if ($GitHubRepository) {
    if (-not $PublicKeyFile -or -not $SigningKeyFile) { throw 'GitHub updates require -PublicKeyFile and -SigningKeyFile.' }
    $PublicKeyFile = (Resolve-Path -LiteralPath $PublicKeyFile).Path
    $SigningKeyFile = (Resolve-Path -LiteralPath $SigningKeyFile).Path
    $rootPrefix = [IO.Path]::GetFullPath($repoRoot).TrimEnd('\') + '\'
    if ($SigningKeyFile.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Store the private signing key outside this source folder.'
    }
    $minisign = (Get-Command minisign.exe -ErrorAction Stop).Source
    $keyLines = @(Get-Content -LiteralPath $PublicKeyFile | Where-Object { $_ -and -not $_.StartsWith('untrusted comment:') })
    if ($keyLines.Count -ne 1) { throw 'Invalid minisign public key file.' }
    $publicKey = $keyLines[0].Trim()
    try { $keyBytes = [Convert]::FromBase64String($publicKey) } catch { throw 'Invalid public key encoding.' }
    if ($keyBytes.Length -ne 42) { throw 'Invalid minisign public key length.' }
}
$oldEndpoint = [Environment]::GetEnvironmentVariable('NOTE_WARDEN_UPDATE_URL', 'Process')
$oldPublicKey = [Environment]::GetEnvironmentVariable('NOTE_WARDEN_UPDATE_PUBKEY', 'Process')
Push-Location $repoRoot
try {
    $metadataText = & cargo metadata --locked --format-version 1 --no-deps
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
    $metadata = $metadataText | ConvertFrom-Json
    $package = $metadata.packages | Where-Object { $_.name -eq 'note-warden' } | Select-Object -First 1
    $version = $package.version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Only stable x.y.z versions can be released.' }
    $endpoint = if ($GitHubRepository) { "https://github.com/$GitHubRepository/releases/latest/download/latest.json" } else { $null }
    [Environment]::SetEnvironmentVariable('NOTE_WARDEN_UPDATE_URL', $endpoint, 'Process')
    [Environment]::SetEnvironmentVariable('NOTE_WARDEN_UPDATE_PUBKEY', $publicKey, 'Process')
    Invoke-Checked 'cargo' @('build', '--release', '--locked')
    $binary = Join-Path $metadata.target_directory 'release\note-warden.exe'
    $runtimeCopy = Join-Path $metadata.target_directory 'release\vcruntime140.dll'
    Copy-Item -LiteralPath $runtime -Destination $runtimeCopy -Force
    $channel = if ($GitHubRepository) { 'github' } else { 'local' }
    $outputDir = Join-Path (Join-Path $metadata.target_directory 'installer') $channel
    New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
    Invoke-Checked $InnoCompiler @("/DAppVersion=$version", "/DAppBinary=$binary", "/DAppRuntime=$runtimeCopy", "/O$outputDir", 'installer\note-warden.iss')
    $installerName = "NoteWarden-$version-setup.exe"
    $installer = Join-Path $outputDir $installerName
    if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'Compiler did not produce the expected installer.' }
    $resourceVersion = (Get-Item -LiteralPath $installer).VersionInfo
    $expectedVersion = [Version]$version
    Write-Host ("Installer resource version: " + $resourceVersion.ProductVersion)
    if ($resourceVersion.ProductMajorPart -ne $expectedVersion.Major -or
        $resourceVersion.ProductMinorPart -ne $expectedVersion.Minor -or
        $resourceVersion.ProductBuildPart -ne $expectedVersion.Build -or
        $resourceVersion.ProductPrivatePart -ne 0) {
        throw ('Installer version mismatch: ' + $resourceVersion.ProductVersion)
    }
    $utf8 = [Text.UTF8Encoding]::new($false)
    $changelog = [IO.File]::ReadAllText((Join-Path $repoRoot 'CHANGELOG.md'))
    $section = [regex]::Match($changelog, '(?ms)^## ' + [regex]::Escape($version) + '\r?\n.*?(?=^## |\z)')
    if (-not $section.Success) { throw "CHANGELOG.md has no section for $version." }
    $notes = $section.Value.Trim()
    $notesPath = Join-Path $outputDir 'release-notes.md'
    [IO.File]::WriteAllText($notesPath, $notes + "`n", $utf8)
    $artifacts = @($installer)
    if ($GitHubRepository) {
        $signatureFile = "$installer.minisig"
        # Encrypted keys prompt locally for their password; no password is embedded in commands.
        Invoke-Checked $minisign @('-Sm', $installer, '-s', $SigningKeyFile, '-x', $signatureFile)
        Invoke-Checked $minisign @('-Vm', $installer, '-p', $PublicKeyFile, '-x', $signatureFile)
        $manifest = [ordered]@{
            version = $version
            notes = $notes
            url = "https://github.com/$GitHubRepository/releases/download/v$version/$installerName"
            signature = [IO.File]::ReadAllText($signatureFile)
        }
        $manifestPath = Join-Path $outputDir 'latest.json'
        [IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 4) + "`n", $utf8)
        $artifacts += @($signatureFile, $manifestPath)
    }
    $hashLines = @($artifacts | ForEach-Object { (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + [IO.Path]::GetFileName($_) })
    [IO.File]::WriteAllText((Join-Path $outputDir 'SHA256SUMS.txt'), ($hashLines -join "`n") + "`n", $utf8)
    Write-Host "Installer ready: $installer"
    if ($GitHubRepository) { Write-Host "Signed update files ready for $GitHubRepository. Nothing was uploaded." }
    else { Write-Host 'Local installer ready. In-app updates are disabled until a repository and signing key are configured.' }
}
finally {
    [Environment]::SetEnvironmentVariable('NOTE_WARDEN_UPDATE_URL', $oldEndpoint, 'Process')
    [Environment]::SetEnvironmentVariable('NOTE_WARDEN_UPDATE_PUBKEY', $oldPublicKey, 'Process')
    Pop-Location
}
