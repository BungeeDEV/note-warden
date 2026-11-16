[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')]
    [string]$GitHubRepository,
    [string]$ArtifactDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'target\installer\github')
)
$ErrorActionPreference = 'Stop'
$manifestPath = Join-Path $ArtifactDirectory 'latest.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$version = $manifest.version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid update version.' }
$repoRoot = Split-Path -Parent $PSScriptRoot
$metadataText = & cargo metadata --manifest-path (Join-Path $repoRoot 'Cargo.toml') --locked --format-version 1 --no-deps
if ($LASTEXITCODE) { throw 'Cannot read source version.' }
$sourceVersion = (($metadataText | ConvertFrom-Json).packages | Where-Object { $_.name -eq 'note-warden' }).version
if ($sourceVersion -cne $version) { throw 'Artifact version differs from the source version. Rebuild before publishing.' }
$sourceChanges = & git -C $repoRoot status --porcelain
if ($LASTEXITCODE -or $sourceChanges) { throw 'Commit all source changes before building and publishing a release.' }
$installerName = "NoteWarden-$version-setup.exe"
$expectedUrl = "https://github.com/$GitHubRepository/releases/download/v$version/$installerName"
if ($manifest.url -cne $expectedUrl) { throw 'The artifacts were built for a different repository. Rebuild with the intended repository.' }
if ([string]::IsNullOrWhiteSpace($manifest.signature)) { throw 'Update signature missing.' }
$names = @($installerName, "$installerName.minisig", 'latest.json', 'SHA256SUMS.txt')
$files = @($names | ForEach-Object { (Resolve-Path -LiteralPath (Join-Path $ArtifactDirectory $_)).Path })
$hashes = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $ArtifactDirectory 'SHA256SUMS.txt')) {
    if ($line -notmatch '^([a-fA-F0-9]{64})  ([^/\\]+)$') { throw 'Invalid checksum file.' }
    $hashes[$Matches[2]] = $Matches[1]
}
foreach ($name in @($installerName, "$installerName.minisig", 'latest.json')) {
    $actualHash = (Get-FileHash -LiteralPath (Join-Path $ArtifactDirectory $name) -Algorithm SHA256).Hash
    if (-not $hashes.ContainsKey($name) -or $actualHash -ine $hashes[$name]) {
        throw "Artifact changed or incomplete: $name. Rebuild before publishing."
    }
}
if ((Get-Content -LiteralPath (Join-Path $ArtifactDirectory "$installerName.minisig") -Raw) -cne $manifest.signature) {
    throw 'Manifest and signature file do not match.'
}
$notesPath = (Resolve-Path -LiteralPath (Join-Path $ArtifactDirectory 'release-notes.md')).Path
# Anonymous update clients can only download release assets from a public repository.
$repoJson = & gh repo view $GitHubRepository --json isPrivate
if ($LASTEXITCODE -ne 0) { throw 'GitHub repository cannot be read.' }
if (($repoJson | ConvertFrom-Json).isPrivate) { throw 'Anonymous in-app updates require a public release repository.' }
$commit = & git -C (Split-Path -Parent $PSScriptRoot) rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify the source commit.' }
$releaseList = & gh release list --repo $GitHubRepository --limit 100 --json tagName,isDraft
if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect existing releases.' }
$existing = $releaseList | ConvertFrom-Json | Where-Object { $_.tagName -eq "v$version" } | Select-Object -First 1
if ($existing) {
    if (-not $existing.isDraft) { throw 'This version is already published. Bump the version; published assets are never replaced.' }
    $draftJson = & gh release view "v$version" --repo $GitHubRepository --json targetCommitish
    if ($LASTEXITCODE) { throw 'Cannot identify the existing draft source commit.' }
    if (($draftJson | ConvertFrom-Json).targetCommitish -cne $commit) { throw 'This draft belongs to a different source commit. Inspect and remove the obsolete draft before rebuilding it.' }
    & gh release upload "v$version" @files --repo $GitHubRepository --clobber
    if ($LASTEXITCODE -ne 0) { throw 'Updating draft artifacts failed.' }
    & gh release edit "v$version" --repo $GitHubRepository --target $commit --title "Note Warden $version" --notes-file $notesPath
    if ($LASTEXITCODE -ne 0) { throw 'Updating draft metadata failed.' }
} else {
    & gh release create "v$version" @files --repo $GitHubRepository --draft --target $commit --title "Note Warden $version" --notes-file $notesPath
    if ($LASTEXITCODE -ne 0) { throw 'Creating the draft release failed.' }
}
Write-Host 'Draft release ready. Review it on GitHub, then publish it to make the update available.'
