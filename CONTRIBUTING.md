# Development and pull requests

Note Warden is a Windows app. Rust and the MSVC build tools, Node.js 24+ and Python 3.11+ are required for the local checks. The Rust version, including Clippy and rustfmt, is pinned in `rust-toolchain.toml`.

Repository documentation, code comments, commit messages and pull request descriptions are written in English. The app's interface is German for now, so user-facing strings in `src/` stay German, and so does `CHANGELOG.md`: the app shows it in its changelog dialog and the release notes are built from it.

## Making a change

```powershell
git switch main
git pull --ff-only
git switch -c feat/short-description
# implement the change and matching tests
./scripts/check.ps1
git add <changed-files>
git commit -m "feat: short description"
git push -u origin HEAD
gh pr create
```

Use `fix/...` for bug fixes and `chore/...` for maintenance. Small, self-contained pull requests keep review and rollback manageable. Collect user-relevant changes under `## Unreleased` in `CHANGELOG.md`. For UI changes, follow the [UI guidelines](docs/ui-guidelines.md) and check the affected views and keyboard operation manually.

CI runs on every pull request and every push to `main`, without path filters. It checks:

- Formatting, Clippy with `-D warnings` and all Rust test targets.
- An optimized Windows build and a real installer run covering first install, reinstall, file checks and an uninstall that preserves data.
- UI rules, version files, changelog, the release tooling and PowerShell syntax.
- A reproducible npm install, known npm vulnerabilities of severity "high" or above, and the syntax of the Actions workflows.
- Dependency review blocks newly introduced dependencies with known vulnerabilities of severity "high" or above, including development dependencies.
- CodeQL for Rust, GitHub Actions, JavaScript and Python, and additionally weekly on the main branch.

`CI passed` only turns green if all related jobs succeeded. Together with the four CodeQL jobs, this status is required for `main`. New commits in a pull request restart the checks; superseded runs are cancelled. The branch must be up to date before merging. Merging is done by squash; finished branches are deleted automatically.

The code scanning rule additionally blocks new CodeQL security findings of severity "high" or above as well as error findings. When setting up from scratch, run CodeQL on the main branch first so that GitHub has the necessary baseline, and then apply the configuration script.

This single-developer project does not require anyone else's approval, because an author cannot approve their own pull request. The pull request and the passing checks are still mandatory. A mandatory approval can be added later once there are more maintainers.

Dependabot opens separate weekly updates for Rust, the branding tools and Actions. Compatible updates are grouped; automatic merging of dependencies is not set up. Adopt changes only after green CI and review. Dependabot alerts complement the tests; not every transitive Rust vulnerability is detected by Clippy or CodeQL.

## Releases

The version number is only raised while preparing a release. See the [release guide](docs/windows-release.md). The installer is intended for stable versions `x.y.z` only; pre-releases need a separate update channel.

## Managing repository settings

`./scripts/configure-github.ps1` shows the planned GitHub settings. With `-Apply` they are applied as a repository admin: merge methods, protection rules, the release environment, immutable releases, secret scanning and security updates. The script only updates its own named ruleset and keeps other rules. Before making changes it saves the previous settings, without secret values, to `.git/github-settings-before-*.json`. The release environment only allows the `main` branch; GitHub Actions tokens have read-only permissions by default. External actions are pinned to full commit SHAs and maintained by Dependabot, following [GitHub's security guidance](https://docs.github.com/en/actions/reference/security/secure-use).

## Git settings

The project-local identity uses `BungeeDEV` and the existing GitHub noreply address; commits carry no co-author trailers. `pull.ff=only` prevents unintended merge commits when updating, `fetch.prune=true` cleans up removed branch references and `push.default=simple` limits a normal push to the current branch. `push.autoSetupRemote=true` simplifies the first push. `rerere.enabled=true` remembers conflict resolutions. `.gitattributes` determines line endings; `core.autocrlf=false` is set locally.

These values apply to this repository. The global Git identity and other projects are unaffected. The GitHub biography and profile README are independent of this.
