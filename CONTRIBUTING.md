# Entwicklung und Pull Requests

Note Warden ist eine Windows-App. Rust und MSVC-Buildtools, Node.js 24+ und Python 3.11+ sind für die lokalen Prüfungen erforderlich. Die Rust-Version einschließlich Clippy und rustfmt steht in `rust-toolchain.toml`.

## Eine Änderung einbauen

```powershell
git switch main
git pull --ff-only
git switch -c feat/kurze-beschreibung
# Änderung und passende Tests umsetzen
./scripts/check.ps1
git add <geänderte-dateien>
git commit -m "feat: kurze Beschreibung"
git push -u origin HEAD
gh pr create
```

Für Fehlerkorrekturen `fix/...`, für Wartung `chore/...` verwenden. Kleine, abgeschlossene Pull Requests halten Review und Rücknahme übersichtlich. Nutzerrelevante Änderungen unter `## Unreleased` in `CHANGELOG.md` sammeln. Bei UI-Änderungen die [UI-Regeln](docs/ui-guidelines.md) und eine manuelle Prüfung der betroffenen Ansichten und Tastaturbedienung einbeziehen.

Die CI startet auf jedem Pull Request und bei jedem Push auf `main`, ohne Pfadfilter. Sie prüft:

- Formatierung, Clippy mit `-D warnings` und sämtliche Rust-Test-Targets.
- Einen optimierten Windows-Build und einen echten Installer-Durchlauf mit Erstinstallation, erneuter Installation, Dateiprüfung und datenerhaltender Deinstallation.
- UI-Regeln, Versionsdateien, Changelog, Release-Werkzeug und PowerShell-Syntax.
- Reproduzierbare npm-Installation, bekannte npm-Sicherheitslücken ab Schweregrad „high“ und die Syntax der Actions-Workflows.
- CodeQL für Rust, GitHub Actions, JavaScript und Python; zusätzlich wöchentlich auf dem Hauptzweig.

`CI passed` wird nur grün, wenn alle zugehörigen Jobs erfolgreich waren. Zusammen mit den vier CodeQL-Jobs ist dieser Status für `main` erforderlich. Neue Commits im PR starten die Prüfungen erneut; überholte Läufe werden abgebrochen. Vor dem Merge muss der Branch aktuell sein. Zusammenführen erfolgt per Squash; erledigte Branches werden automatisch gelöscht.

Für das Einzelentwickler-Projekt ist keine fremde Freigabe vorgeschrieben, weil der Autor den eigenen PR nicht freigeben kann. Der PR und die bestandenen Prüfungen sind trotzdem Pflicht. Mit weiteren Maintainers kann später eine verpflichtende Freigabe ergänzt werden.

Dependabot öffnet wöchentlich getrennte Updates für Rust, Branding-Werkzeuge und Actions. Kompatible Updates werden gruppiert; ein automatischer Merge von Abhängigkeiten ist nicht eingerichtet. Änderungen erst nach grüner CI und Review übernehmen. Dependabot-Meldungen ergänzen die Tests; nicht jede transitive Rust-Schwachstelle wird von Clippy oder CodeQL erkannt.

## Releases

Die Versionsnummer wird erst zur Release-Vorbereitung erhöht. [Release-Anleitung](docs/windows-release.md). Der Installer ist nur für stabile Versionen `x.y.z` vorgesehen; Vorabversionen benötigen einen getrennten Update-Kanal.

## Repository-Einstellungen verwalten

`./scripts/configure-github.ps1` zeigt die geplanten GitHub-Einstellungen an. Mit `-Apply` werden sie als Repository-Admin angewendet: Merge-Verfahren, Schutzregeln, Release-Umgebung, unveränderliche Releases, Secret-Scanning und Sicherheitsupdates. Das Skript aktualisiert nur das eigene benannte Ruleset und erhält andere Regeln. Vor Änderungen speichert es die bisherigen Einstellungen ohne Secret-Werte in `.git/github-settings-before-*.json`. Die Release-Umgebung erlaubt ausschließlich den Branch `main`; GitHub-Actions-Tokens haben standardmäßig nur Leserechte. Externe Actions sind an vollständige Commit-SHAs gebunden und werden von Dependabot gepflegt, entsprechend [GitHubs Sicherheitsempfehlung](https://docs.github.com/en/actions/reference/security/secure-use).

## Git-Einstellungen

Die projektlokale Identität verwendet `BungeeDEV` und die bestehende GitHub-Noreply-Adresse. `pull.ff=only` verhindert unbeabsichtigte Merge-Commits beim Aktualisieren, `fetch.prune=true` räumt entfernte Branch-Verweise auf und `push.default=simple` begrenzt den normalen Push auf den aktuellen Branch. `push.autoSetupRemote=true` vereinfacht den ersten Push. `rerere.enabled=true` merkt sich Konfliktlösungen. `.gitattributes` bestimmt die Zeilenenden; lokal ist `core.autocrlf=false` gesetzt.

Diese Werte gelten für dieses Repository. Die globale Git-Identität und andere Projekte bleiben unberührt. GitHub-Biografie und Profil-README sind unabhängig davon.
