# Note Warden

[![CI](https://github.com/BungeeDEV/note-warden/actions/workflows/ci.yml/badge.svg)](https://github.com/BungeeDEV/note-warden/actions/workflows/ci.yml)
[![CodeQL](https://github.com/BungeeDEV/note-warden/actions/workflows/codeql.yml/badge.svg)](https://github.com/BungeeDEV/note-warden/actions/workflows/codeql.yml)
[![Release](https://img.shields.io/github/v/release/BungeeDEV/note-warden)](https://github.com/BungeeDEV/note-warden/releases/latest)

Note Warden ist eine lokale Windows-Desktop-App für Updatepläne. Ein Update hat Status, Markdown-Notizen, Aufgaben (mit Unteraufgaben) und Termine; Updates und Aufgaben lassen sich einzeln terminieren. Version `1.1.0`, Änderungen siehe [CHANGELOG.md](CHANGELOG.md).

## Installation

Windows-Installer: [GitHub Releases](https://github.com/BungeeDEV/note-warden/releases). Das Setup installiert Note Warden pro Benutzer und legt einen Startmenü-Eintrag an. Eine Desktop-Verknüpfung ist optional. Daten und Einstellungen bleiben bei Updates und Deinstallation erhalten.

Die signierte Release-Fassung prüft automatisch auf Updates. Eine manuelle Prüfung gibt es unter **Einstellungen → Über & Updates**. Installation eines Updates erst nach Signaturprüfung und Datenbackup. Release-Builds und Installer werden über GitHub Actions erzeugt; der Wartungsablauf steht in [docs/windows-release.md](docs/windows-release.md).

## Aus dem Quellcode starten

Voraussetzung sind eine Rust-Toolchain für Windows und die MSVC-Buildtools.

```powershell
cargo run --locked
```

Ansichten: **Heute**, **Inbox**, **Updates** (Suche, Sortierung, Gruppierung, Filter, Archiv und Papierkorb), **Board**, **Releases** und **Kalender** (Monat/Woche, Drag and Drop, Panel „Ohne Termin“).

Tastaturkürzel (`?` zeigt die vollständige Übersicht): `Strg+N` neues Update, `Strg+K` Befehlspalette, `Strg+,` Einstellungen, `N` neuer Eintrag in Heute, `Strg+B` Seitenleiste (im Editor: Fett), `Strg+F` Suche, `Strg+1/2/3` Heute/Updates/Board, `Entf` Update löschen (mit Rückgängig), `Esc` schließt Dialoge. Im Editor: `Strg+I` kursiv, `Strg+Umschalt+K` Link, `Tab`/`Umschalt+Tab` Listen ein-/ausrücken.

## Lokale Daten und Sicherung

Der Arbeitsbereich liegt als JSON-Datei unter Windows normalerweise in `%LOCALAPPDATA%\Note Warden\Note Warden\data\workspace.json` (der genaue Pfad wird von `directories` bestimmt). Vor dem Ersetzen der Hauptdatei wird deren bisheriger Stand als `workspace.json.bak` gesichert. Note Warden speichert Änderungen in Reihenfolge und zeigt den Speicherzustand in der Oberfläche an. Bei einem Schreibfehler bleiben die Änderungen im geöffneten Arbeitsbereich; **Erneut speichern** versucht es wieder.

Wenn die Hauptdatei fehlt, kann die Sicherung automatisch wiederhergestellt werden. Ist die Hauptdatei unlesbar, sperrt die App Änderungen und zeigt einen Fehler. **Sicherung wiederherstellen** nutzt die Sicherung ausdrücklich; die beschädigte Datei bleibt mit dem Zusatz `.corrupt-…` erhalten. Ältere Arbeitsbereiche werden beim ersten Start auf Schema 8 migriert; vorher entsteht einmalig `workspace.json.bak-0.2.0` (0.1/0.2) bzw. `workspace.json.bak-0.3.0`. Gelöschte Updates liegen standardmäßig 30 Tage im Papierkorb (einstellbar). Theme, Akzent und Fenstergröße stehen zusätzlich in `prefs.json` neben der Datendatei. Für Entwicklungstests kann `NOTE_WARDEN_DATA_DIR` auf ein anderes Datenverzeichnis und `NOTE_WARDEN_WINDOW_SIZE` etwa auf `800x600` gesetzt werden.

Portabler Modus: Eine leere Datei `.portable` neben der `Note Warden.exe` lässt Note Warden alle Daten (Arbeitsbereich, Sicherungen, Protokoll) in einem Ordner `Data/` daneben ablegen, etwa für einen USB-Stick. Der automatische Updater ist in diesem Modus aus; neue Versionen werden von Hand ersetzt. `NOTE_WARDEN_DATA_DIR` hat Vorrang.

Die Arbeitsdaten bleiben lokal. Erinnerungen verwenden Windows-Benachrichtigungen; Accounts und Synchronisierung sind nicht vorgesehen. Export (Markdown, Changelog, JSON-Backup) und Import liegen im Update-Menü bzw. in den Einstellungen.

## Technische Basis

Rust 2024, [GPUI](https://gpui.rs/) über GPUI Kit 0.9.1, `chrono` und JSON-Speicherung. Die Oberfläche verwendet semantische Farben für helle und dunkle Themes sowie gemeinsame Komponenten nach [docs/ui-guidelines.md](docs/ui-guidelines.md). Der aktuelle Audit- und Prüfstand steht in [docs/ui-audit-1.1.0.md](docs/ui-audit-1.1.0.md). `Cargo.lock` fixiert die Abhängigkeiten.

Branding (Logo, Icons): `npm install` und `npm run brand` erzeugen `branding/logo.svg`, `logo-small.svg`, `logo-1024.png` und `app.ico` (nur Entwicklungswerkzeug, `@resvg/resvg-js`). Das Icon wird per `build.rs` in die Exe eingebettet.

Entwicklung, Pull Requests und CI: [CONTRIBUTING.md](CONTRIBUTING.md). Alle lokalen Prüfungen zusammen:

```powershell
./scripts/check.ps1
```
