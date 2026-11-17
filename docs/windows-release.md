# Windows installer and GitHub updates

## Current state

The current published version and Windows setup are available under [GitHub Releases](https://github.com/BungeeDEV/note-warden/releases/latest). Source versions are prepared separately from publication: merging a version change does not yet make an update available. The desktop acceptance record for the 1.2.2 Sticky Mode improvements is in [ui-audit-1.2.2.md](ui-audit-1.2.2.md).

The release workflow builds and signs the Windows installer, tests first install/reinstall/uninstall with data retention on a fresh runner, then creates a draft. Before publishing, review the workflow result, release notes, artifact checksums and signature. After publication, verify that the public latest manifest and setup are downloadable and that the Rust updater accepts their signature.

The public project and release repository is **[BungeeDEV/note-warden](https://github.com/BungeeDEV/note-warden)**. The user approved publishing the app's source code. Personal development data, working data and private keys are excluded; `.claude` points to personal development data and is not uploaded.

Local installer builds require Inno Setup. The reproducible GitHub installer build runs on a `windows-2025` runner, which has Inno Setup. Its draft is reviewed before publishing. The status is shown under [Actions](https://github.com/BungeeDEV/note-warden/actions).

## Normal release flow on GitHub

1. Collect changes under `## Unreleased` in `CHANGELOG.md` during development. For a release, create a branch and run, for example, `python scripts/release.py prepare 1.2.0`. The tool raises the Cargo, npm and README versions together, updates both lockfiles without any dependency updates and moves the collected notes into the new version section. `1.2.0` is an example; choose the next suitable version.
2. Run `./scripts/check.ps1`, review the diff and open a pull request. After successful CI and review, merge it into `main` by squash.
3. Start the **Windows installer release** workflow on the current **main** and enter the prepared version, or alternatively run `gh workflow run release.yml --ref main --repo BungeeDEV/note-warden -f version=1.2.0`.
4. Before signing, the workflow rejects mismatching version input, other branches, a main branch that has changed in the meantime, and versions that are already published. It runs the complete CI again, including the installer test, and then creates the signed setup in the `release` environment, which is restricted to `main`. Pull request checks do not need a signing key. CI and release caches are separate.
5. The setup, `.minisig`, `latest.json` and checksums appear in a **draft release**. Check the files and release notes in the Actions artifact and in the draft. Only then publish the draft on GitHub. Immutable releases then protect the assets and the tag; corrections require a new version. The updater does not see drafts.

The version number follows SemVer: patch for compatible bug fixes, minor for compatible new features and major for incompatible changes. The automation does not publish releases on a normal push or merge. The configuration for automatically generated GitHub notes is in `.github/release.yml`; the app release notes shipped with a release come from the verified version section in `CHANGELOG.md`.

The workflow uses `installer/update-signing.pub` and the environment secret `NOTE_WARDEN_MINISIGN_KEY` in the GitHub environment `release`. That environment only allows the `main` branch. The public key is embedded into each release build. The private key is only written to a temporary file on the short-lived runner and removed afterwards. `python scripts/manage-release-key.py --repository BungeeDEV/note-warden --upload` stores the existing local key copy in this environment by default; for a fresh setup, run `scripts/configure-github.ps1 -Apply` first.

An additional local key copy is stored outside the project, encrypted with Windows DPAPI, at `%LOCALAPPDATA%\NoteWardenSigning\note-warden.key.dpapi`. It is bound to the current Windows user profile. Before changing machines, create a separate, protected key backup. `scripts/manage-release-key.py` can store the existing copy as a GitHub secret again without generating a new key. An explicitly requested `--export` writes an unencrypted Minisign key exclusively outside the project; such an export file must then be protected separately.

The signed public test vector at `installer/release-key-check.txt` is verified by both the Rust verifier and the official Minisign on the runner.

## Flow for users

Run the setup once, then start Note Warden from the Start menu. A desktop shortcut is optional. By default the installation is located at `%LOCALAPPDATA%\Programs\Note Warden`, without administrator rights.

The app checks for updates at most once a day on start. A manual check is available under **Einstellungen → Über & Updates** (Settings → About & Updates). After the user agrees, it downloads the update, verifies the Minisign signature, creates a data backup and starts the setup. The app exits; the setup updates the existing installation and starts Note Warden again. User data is stored separately and is kept across update and uninstall.

## Build requirements

- Rust and the MSVC build tools.
- [Inno Setup 6 or 7](https://jrsoftware.org/inno-setup/downloads), with `ISCC.exe` on the PATH or given via `-InnoCompiler`.
- [Minisign](https://jedisct1.github.io/minisign/) for signed releases, with `minisign.exe` on the PATH.
- GitHub CLI with access to the public download repository.

The script does not install or download any tools. The required `vcruntime140.dll` is bundled from the official redist directory of the existing Visual Studio toolchain. Other paths can be given with `-VCRuntimeDirectory`. See [Microsoft on the app-local VC++ runtime](https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files).

## Signing key

Generate a Minisign key pair outside the project. Example paths:

- public: `%USERPROFILE%\NoteWardenSigning\note-warden.pub`
- private: `%USERPROFILE%\NoteWardenSigning\note-warden.key`

Generate them with `minisign -G -p <public-path> -s <private-path>` and set a password locally. Back up the key and the password separately. The private key belongs neither in the source code nor in release files. Existing installations trust the embedded public key; the private key has to be kept for future updates.

Minisign protects the app updates. Windows Authenticode publisher signing is a separate step and is not set up at the moment.

## Building the installer

Local only, without the online updater:

```powershell
./scripts/build-installer.ps1
```

For the signed GitHub update channel:

```powershell
./scripts/build-installer.ps1 `
  -GitHubRepository BungeeDEV/note-warden `
  -PublicKeyFile "$env:USERPROFILE\NoteWardenSigning\note-warden.pub" `
  -SigningKeyFile "$env:USERPROFILE\NoteWardenSigning\note-warden.key"
```

Use the repository name that was actually approved. The script reads the version from the Cargo metadata, embeds the endpoint and the public key, builds the optimized Windows app without a console and creates the setup. It signs and verifies the setup and creates `latest.json`, `.minisig`, `release-notes.md` and `SHA256SUMS.txt`. GitHub files go to `target/installer/github` by default and local installers separately to `target/installer/local`; Cargo target directories are respected. It uploads nothing.

Before a new release, raise the version in the Cargo and package files and add a matching section to `CHANGELOG.md`. The installer takes the Cargo version automatically.

## Preparing a GitHub release

After the public download repository has been created with confirmation and the setup has been checked locally:

```powershell
./scripts/publish-release.ps1 -GitHubRepository BungeeDEV/note-warden
```

The script checks the repository and the manifest and creates a **draft release** with the setup, the Minisign file, the update file and checksums. The source code is not uploaded. The draft does not deliver an update to installed apps yet. Only publish it on GitHub after review.

The embedded endpoint is `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`. The manifest points to the versioned setup download of the same release.

## Acceptance still pending

First install, reinstall at the same custom path, payload checksums, runtime files and an uninstall that preserves data were successfully checked on a fresh CI machine. The public download and the signature verification were then checked with the actual updater code. A complete switch between two different published app versions, including a running app/tray and an automatic restart, is still pending. The backup and signature failure paths are additionally covered by the existing tests.

The updater tests cover valid signatures, tampered downloads, invalid keys/signatures, unsafe version names and HTTP addresses. Network access is limited to HTTPS; the manifest and the installer have size limits. In addition, the actual release key pair is verified against a published test signature. The PowerShell scripts are syntax-checked; they are executed in the release workflow.

The setup's explicit restart action uses the documented [Inno update mode](https://jrsoftware.org/inno-setup/documentation/topic_technotes.htm); it does not require the Windows Restart Manager registration that [RestartApplications](https://jrsoftware.org/ishelp/topic_setup_restartapplications.htm) would otherwise need.

If changes to `.github/workflows/` are pushed to the main branch while a release build is running, GitHub may refuse to create a tag on the older commit with the restricted CI token. In that case, start the workflow again on the current main branch. The script never replaces published release files; the version has to be raised before another release.
