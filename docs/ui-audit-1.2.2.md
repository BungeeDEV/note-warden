# Local acceptance for 1.2.2

This record covers local acceptance of 1.2.2 before publication. Cargo, npm, both lockfiles, README and the release notes agree on the version. The local checks did not replace the installed app.

## Automated validation

- `./scripts/check.ps1` passed: formatting, Clippy with warnings denied, 54 Rust tests, the UI source rules, release metadata and six release-tooling tests.
- One existing online updater test remains intentionally ignored by the ordinary test run; it requires the published endpoint and release configuration.
- `cargo build --release --locked` passed with the repository's update endpoint and public verification key embedded. The optimized executable is under `target/release/note-warden.exe`.

## Desktop acceptance

The app was exercised using isolated `NOTE_WARDEN_DATA_DIR` fixtures, without replacing the installed app or its workspace.

| Scenario | Observed result |
| --- | --- |
| Sticky toolbar at 320 px width | All window controls and the view-options menu remain reachable. Both submenus fit inside the window. |
| Hide completed updates | Nine updates become seven; the two completed entries remain in the workspace and the main list. A visible action restores them. |
| Card density | Switching from details to title-only reduces row height while retaining priority and status indicators. Full metadata remains available in tooltips. |
| General settings | The new Sticky Mode section reflects the popup's settings. Changing density in General settings is saved and applied to the popup. |
| Restart with different selections | Main selection was update 2 and sticky selection update 1. Restarting the optimized release build with `--compact` restored sticky update 1, with the main window hidden. |
| Persistent view preferences | Completed-update visibility and the card density chosen in settings survived a process restart. |
| Hidden startup target | With all nine updates completed and hidden, the last-update start option falls back to the overview. |
| All-completed empty state | At 320 px width in the light theme, the explanation wraps without clipping and the show-completed action restores all nine cards in their saved order. |

During implementation, the included manual update/task ordering, shared ordering between windows, per-update notes height, long task titles and sticky-only/tray-only launch modes were also checked in the native app. Unit tests cover older preference defaults, search/filter ordering, serialization and invalid startup targets (missing, archived, trashed or hidden completed updates).

## Remaining release gates

At the time of local acceptance, the machine had no Inno Setup compiler, so no new installer was built or installation test run locally. The existing CI/release workflow builds and signs the installer, tests installation/reinstallation/uninstallation on an ephemeral runner and creates a draft release after the changes have passed PR checks and reached `main`. Review those artifacts before publication, following [the release procedure](windows-release.md).
