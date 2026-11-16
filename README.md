# Note Warden

[![CI](https://github.com/BungeeDEV/note-warden/actions/workflows/ci.yml/badge.svg)](https://github.com/BungeeDEV/note-warden/actions/workflows/ci.yml)
[![CodeQL](https://github.com/BungeeDEV/note-warden/actions/workflows/codeql.yml/badge.svg)](https://github.com/BungeeDEV/note-warden/actions/workflows/codeql.yml)
[![Release](https://img.shields.io/github/v/release/BungeeDEV/note-warden)](https://github.com/BungeeDEV/note-warden/releases/latest)

Note Warden is a local Windows desktop app for planning updates. An update has a status, Markdown notes, tasks (with subtasks) and dates; updates and tasks can be scheduled individually. Version `1.1.0`, see [CHANGELOG.md](CHANGELOG.md) for changes.

The app's interface is currently in German. Interface labels are quoted in German in this document, with the English meaning in parentheses.

## Installation

Windows installer: [GitHub Releases](https://github.com/BungeeDEV/note-warden/releases). The setup installs Note Warden per user and adds a Start menu entry. A desktop shortcut is optional. Data and settings are kept across updates and uninstalls.

The signed release build checks for updates automatically. A manual check is available under **Einstellungen → Über & Updates** (Settings → About & Updates). An update is only installed after its signature has been verified and a data backup has been made. Release builds and installers are produced by GitHub Actions; the maintenance procedure is described in [docs/windows-release.md](docs/windows-release.md).

## Running from source

You need a Rust toolchain for Windows and the MSVC build tools.

```powershell
cargo run --locked
```

Views: **Heute** (Today), **Inbox**, **Updates** (search, sorting, grouping, filters, archive and trash), **Board**, **Post-its**, **Releases** and **Kalender** (Calendar: month/week, drag and drop, an "unscheduled" panel). Sidebar entries can be hidden under Einstellungen → Darstellung → Seitenleiste (Settings → Appearance → Sidebar).

The **Notizzettel** (sticky notes window) is a small frameless window that stays open next to the main window. It shows your updates as cards with a post-it board below, and offers search, the editor and tasks. Open it with `Ctrl+Shift+M`, from the sidebar, the tray menu or the command palette; both windows keep their changes in sync.

Tags can be added to updates, their tasks (`#tag` in the task field) and post-its (`#tag` in the text). They appear as colored chips, form a **Tags** section in the sidebar, work as a filter and are found by the search.

Keyboard shortcuts (`?` shows the complete list): `Ctrl+N` new update, `Ctrl+K` command palette, `Ctrl+,` settings, `N` new entry in Today, `Ctrl+B` sidebar (bold inside the editor), `Ctrl+F` search, `Ctrl+1/2/3` Today/Updates/Board, `Ctrl+Shift+M` sticky notes window, `Space` mark the selected update done, `Del` delete an update (with undo), `Esc` closes dialogs. In the editor: `Ctrl+I` italic, `Ctrl+Shift+K` link, `Tab`/`Shift+Tab` indent/outdent lists.

## Local data and backups

The workspace is a JSON file, normally at `%LOCALAPPDATA%\Note Warden\Note Warden\data\workspace.json` on Windows (the exact path is determined by the `directories` crate). Before the main file is replaced, its previous state is saved as `workspace.json.bak`. Note Warden saves changes in order and shows the save state in the interface. If a write fails, the changes stay in the open workspace; **Erneut speichern** (Save again) retries.

If the main file is missing, the backup can be restored automatically. If the main file is unreadable, the app locks editing and shows an error. **Sicherung wiederherstellen** (Restore backup) uses the backup explicitly; the damaged file is kept with the suffix `.corrupt-…`. Older workspaces are migrated to schema 8 on first start; beforehand a one-time `workspace.json.bak-0.2.0` (0.1/0.2) or `workspace.json.bak-0.3.0` is created. Deleted updates stay in the trash for 30 days by default (configurable). Theme, accent color and window size are also stored in `prefs.json` next to the data file. For development tests, `NOTE_WARDEN_DATA_DIR` can point to a different data directory and `NOTE_WARDEN_WINDOW_SIZE` can be set to, for example, `800x600`.

Portable mode: an empty file named `.portable` next to `Note Warden.exe` makes Note Warden keep all data (workspace, backups, log) in a `Data/` folder beside it, for example on a USB stick. The automatic updater is off in this mode; new versions are replaced by hand. `NOTE_WARDEN_DATA_DIR` takes precedence.

Your data stays local. Reminders use Windows notifications; accounts and synchronization are not planned. Export (Markdown, changelog, JSON backup) and import are available in the update menu and in the settings.

## Technical basis

Rust 2024, [GPUI](https://gpui.rs/) through GPUI Kit 0.9.1, `chrono` and JSON storage. The interface uses semantic colors for light and dark themes and shared components following [docs/ui-guidelines.md](docs/ui-guidelines.md). The current audit and test status is in [docs/ui-audit-1.1.0.md](docs/ui-audit-1.1.0.md). `Cargo.lock` pins the dependencies.

Branding (logo, icons): `npm install` and `npm run brand` generate `branding/logo.svg`, `logo-small.svg`, `logo-1024.png` and `app.ico` (development tool only, `@resvg/resvg-js`). `build.rs` embeds the icon into the executable.

Development, pull requests and CI: [CONTRIBUTING.md](CONTRIBUTING.md). All local checks together:

```powershell
./scripts/check.ps1
```
