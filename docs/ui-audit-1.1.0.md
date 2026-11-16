# UI audit 1.1.0 – as of 16 November 2026

**Update:** The desktop connection is available again. Live checks with synthetic data (140 updates, long project names, tasks and releases) were carried out at 1280×800 in dark and at 960×620 in light with a large font. They revealed and fixed over-wide board columns, clipped calendar actions, an overflowing day list and overlapping title rows. The earlier statements "live acceptance pending" in the initial review below are historical. A complete combination of all themes, densities and fonts, as well as screen readers and the Windows contrast themes, remains untested.

## Result and limits of the review

The previous session ended at the usage limit during the audit. Its changes were partly saved but not finally verified. The current workspace was used as authoritative. There is no Git repository in the project folder or its parents; `git status` is therefore not available.

The source code corrections listed below are implemented and verified automatically. **The overall visual acceptance is pending.** Both available ways of controlling the app reported a missing native connection (`Computer Use native pipe is unavailable`, Windows error 2); the browser host is not available either. Existing screenshots from the previous session were looked at. They show an older build and are explicitly not after-fix evidence for 1.1.0. The running previous test instance was not changed.

The yardstick is the shared tokens and primitives in `palette.rs` and `ui/kit.rs`. Apple's [layout guidelines](https://developer.apple.com/design/human-interface-guidelines/layout) serve as a reference for resizable windows, text sizes, spacing and hierarchy. This gives the Windows app no Apple HIG or accessibility certification.

## Areas reviewed and corrections

| Area | Finding and implementation | Evidence / limit |
| --- | --- | --- |
| Shared row actions | The former gradient and the opaque hover block that followed it could cover titles and dates. Actions now use normal flex space, even when they are invisible without hover. | Source review for Updates, Today and tasks; live hover and keyboard focus pending. |
| Updates | The compact meta column plus actions left hardly any title width. Compact shows the title and priority; further metadata is in the full tooltip. Row height follows the font size. | Compiler and rule check; long titles and a large font pending live. |
| Large lists | `uniform_list` mixed 28 px group headers with 36/56 px entries. Virtualized groups now use the same scaled height as the entries. | Source review; scrolling with at least 100 entries pending live. |
| New split button | Own rounding on the outer corners, a shared surface, an inner divider. Group clipping removed so the focus is not cut off. | Compiles; hover, focus and dropdown pending live. |
| Multi-selection | A rigid toolbar height could push actions out in narrow lists. Minimum height and wrapping instead of a fixed height. | Compiles; narrow list pending live. |
| Archive and trash | Own empty states; empty areas and filtered-out content are distinguished. Clearing the search and resetting secondary filters do not leave the archive or trash. | Source review; combinations of search and filters pending live. |
| Today | The open action reserves space, and the date does not shrink below its label. Full titles as a tooltip, empty update titles get a placeholder. | Existing before-screenshot and source; after-check pending. |
| Inbox | Instead of a text button and a numeric priority: the shared task checkbox and priority symbol; date and reminder visible. Wrappable rows, a fixed input above a bounded scroll area. | Compiles; input, conversion and longer tasks pending live. |
| Board | Equal flex basis, minimum width 220 px, remaining height below a fixed title, and scrollbar spacing. Long titles have a tooltip, and cards stay within their column. | The before-screenshot shows a clipped fourth column; after-check pending. |
| Releases | The content width uses the available space up to the reading limit, and actions can wrap. Download asset embedded. | Compiles; filled release cards pending live. |
| Calendar | The header can wrap; task controls are larger and labeled. Inbox entries open the inbox instead of update ID 0. | Source review; month, week, drag and drop and popovers pending live. |
| Settings | Categories scroll independently of Back and search; a scaled minimum height prevents squashing. The selection always reserves the same edge. Matching icons for Start page, Projects, Templates and System. | The before-screenshot shows a missing System icon; after-check pending. |
| Editor, history and overlays | Scroll limits, opaque surfaces, size limits and existing focus traps checked in the source. Task headings left-aligned; checkbox check marks centered. | No replacement for a live check of focus return, Esc and window bounds. |
| Assets | Download, Monitor and Flag embedded explicitly. A new test loads all 53 icon variants referenced directly in the UI sources through the real `AppAssets` source and checks the SVG content. | Regression test passed. |
| Documentation and version | Cargo, lock files, branding package and installer defaults set to 1.1.0; README and UI guidelines updated. Rust sources formatted and existing Clippy hints cleaned up. | All checks listed below passed. |

## Automated checks

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | passed |
| `npm run check:ui` | passed |
| `cargo clippy --locked --all-targets -- -D warnings` | passed, no warnings |
| `cargo test --locked` | 34 passed, 0 failed |
| `cargo build --locked` | passed, debug build 1.1.0 at `target/debug/note-warden.exe` |

Clippy and the tests ran with the already existing build cache under `%LOCALAPPDATA%/Temp/nw-target`; in addition, the normal project build was created under `target/debug`. No installer and no signed release were published.

## Remaining live acceptance

As soon as the native app connection is available again, the new changes have to be viewed at 960×620, 1280×800 and a wide window in light/dark, comfortable/compact density and small/normal/large font. Content scenarios: empty, long titles/project names/paths, many tasks, at least 100 grouped updates, filled releases and version states. Interactions: hover, Tab focus, menus, resetting the search, date, window size, scrollbars and drag and drop. Contrast tests cover the existing color tokens; screen readers, the Windows contrast themes and every concrete color combination in rendered states are therefore not accepted.

This gap in the review prevents the claim that all visual errors of the entire app have been removed or that all modern guidelines are fully met.

## Addendum: installer and GitHub updates

Version 1.1.0 is published in the public repository. The installer was built and signed on GitHub; first install, reinstall and an uninstall that preserves data passed the CI test. The 37 regular tests pass. After publication, the additional online test downloaded the real manifest and setup and verified their signature. Desktop live tests and the errors fixed during them are in the update at the top. Current procedure and limits: [Windows releases](windows-release.md).
