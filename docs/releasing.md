# Releases and auto-update

The binding procedure is in [windows-release.md](windows-release.md): collect changes under `Unreleased`, prepare a version with `python scripts/release.py prepare x.y.z`, review and merge the pull request, then start the release workflow on `main` with that version.

The workflow builds and verifies the installer and creates a signed **draft release**. Only publish it on GitHub after checking the files and notes. The local build and upload remain possible through `scripts/build-installer.ps1` and `scripts/publish-release.ps1`; the parameters and the signing key management are described in the release guide.

The updater is only active if the endpoint and the public key were set at build time. The private key stays outside the source code. Published versions are never overwritten.
