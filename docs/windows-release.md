# Windows-Installer und GitHub-Updates

## Aktueller Stand

**Version 1.1.0 ist veröffentlicht:** [Release](https://github.com/BungeeDEV/note-warden/releases/tag/v1.1.0) · [Windows-Setup](https://github.com/BungeeDEV/note-warden/releases/download/v1.1.0/NoteWarden-1.1.0-setup.exe). Der [endgültige CI-Lauf](https://github.com/BungeeDEV/note-warden/actions/runs/43859880740) hat Formatierung, Clippy, 37 Tests, offizielle Minisign-Prüfung, Setup-Build, Erstinstallation, erneute Installation und datenerhaltende Deinstallation bestanden. Nach Veröffentlichung hat der tatsächliche Rust-Updater das öffentliche Manifest und den Installer erfolgreich geladen und dessen Signatur geprüft.

Das öffentliche Projekt- und Release-Repository ist **[BungeeDEV/note-warden](https://github.com/BungeeDEV/note-warden)**. Der Nutzer hat die Veröffentlichung des App-Quellcodes freigegeben. Persönliche Entwicklungsdaten, Arbeitsdaten und private Schlüssel sind ausgeschlossen; `.claude` verweist auf persönliche Entwicklungsdaten und wird nicht hochgeladen.

Der lokale Setup-Compiler konnte aufgrund einer automatischen Freigabeablehnung nicht eingerichtet werden. Der reproduzierbare Installer-Build läuft deshalb auf GitHubs `windows-2025`-Runner mit vorhandenem Inno Setup. Der Workflow erstellt zunächst einen Entwurf, der vor der Veröffentlichung geprüft wird. Der Status steht unter [Actions](https://github.com/BungeeDEV/note-warden/actions).

## Normaler Release-Ablauf auf GitHub

1. Änderungen während der Entwicklung unter `## Unreleased` in `CHANGELOG.md` sammeln. Für ein Release einen Branch anlegen und beispielsweise `python scripts/release.py prepare 1.2.0` ausführen. Das Werkzeug erhöht Cargo-, npm- und README-Versionen gemeinsam, aktualisiert beide Lockfiles ohne Abhängigkeitsupdates und verschiebt die gesammelten Notizen in den neuen Versionsabschnitt. `1.2.0` ist ein Beispiel; die nächste passende Version wählen.
2. `./scripts/check.ps1` ausführen, den Diff prüfen und einen Pull Request öffnen. Nach erfolgreicher CI und Review per Squash in `main` übernehmen.
3. Den Workflow **Windows installer release** auf dem aktuellen **main** starten und die vorbereitete Version eingeben, alternativ `gh workflow run release.yml --ref main --repo BungeeDEV/note-warden -f version=1.2.0`.
4. Der Workflow lehnt abweichende Versionsangaben, andere Branches, einen inzwischen veränderten Hauptzweig und bereits veröffentlichte Versionen vor dem Signieren ab. Er führt die vollständige CI einschließlich Installer-Test erneut aus und erstellt danach in der auf `main` beschränkten Umgebung `release` das signierte Setup. PR-Prüfungen benötigen keinen Signierschlüssel. CI- und Release-Caches sind getrennt.
5. Setup, `.minisig`, `latest.json` und Prüfsummen erscheinen in einem **Draft Release**. Im Actions-Artefakt und im Entwurf Dateien und Release-Notizen prüfen. Erst dann den Entwurf auf GitHub veröffentlichen. Unveränderliche Releases schützen danach Assets und Tag; Korrekturen benötigen eine neue Version. Der Updater sieht Entwürfe nicht.

Die Versionsnummer folgt SemVer: Patch für kompatible Fehlerkorrekturen, Minor für kompatible neue Funktionen und Major für inkompatible Änderungen. Die Automatisierung veröffentlicht keine Releases beim normalen Push oder Merge. Die Konfiguration für automatisch generierte GitHub-Notizen liegt in `.github/release.yml`; die ausgelieferten App-Notizen stammen aus dem geprüften Versionsabschnitt in `CHANGELOG.md`.

Der Workflow verwendet `installer/update-signing.pub` und das Environment-Secret `NOTE_WARDEN_MINISIGN_KEY` in der GitHub-Umgebung `release`. Diese erlaubt nur den Branch `main`. Der öffentliche Schlüssel ist fest im jeweiligen Release-Build eingebettet. Der private Schlüssel wird nur auf dem kurzlebigen Runner in eine temporäre Datei geschrieben und danach entfernt. `python scripts/manage-release-key.py --repository BungeeDEV/note-warden --upload` hinterlegt die bestehende lokale Schlüsselkopie standardmäßig in dieser Umgebung; bei einer Neueinrichtung zuerst `scripts/configure-github.ps1 -Apply` ausführen.

Eine zusätzliche lokale Schlüsselkopie liegt durch Windows DPAPI verschlüsselt außerhalb des Projekts unter `%LOCALAPPDATA%\NoteWardenSigning\note-warden.key.dpapi`. Sie ist an das aktuelle Windows-Benutzerprofil gebunden. Vor einem Rechnerwechsel muss eine separate, geschützte Schlüssel-Sicherung angelegt werden. `scripts/manage-release-key.py` kann die vorhandene Kopie erneut als GitHub-Secret hinterlegen, ohne einen neuen Schlüssel zu erzeugen. Ein bewusst angeforderter `--export` schreibt einen unverschlüsselten Minisign-Schlüssel ausschließlich außerhalb des Projekts; eine solche Exportdatei muss anschließend separat geschützt werden.

Der signierte öffentliche Testvektor unter `installer/release-key-check.txt` wird sowohl vom Rust-Verifier als auch vom offiziellen Minisign auf dem Runner geprüft.

## Ablauf für Nutzer

Einmal das Setup ausführen, danach Note Warden über das Startmenü starten. Eine Desktop-Verknüpfung ist optional. Die Installation liegt standardmäßig unter `%LOCALAPPDATA%\Programs\Note Warden`, ohne Administratorrechte.

Die App prüft beim Start höchstens einmal täglich auf Updates. Unter **Einstellungen → Über & Updates** ist eine manuelle Prüfung möglich. Nach Zustimmung lädt sie das Update, prüft die Minisign-Signatur, erstellt ein Datenbackup und startet das Setup. Die App beendet sich; das Setup aktualisiert die vorhandene Installation und startet Note Warden wieder. Nutzerdaten liegen separat und bleiben bei Update und Deinstallation erhalten.

## Build-Voraussetzungen

- Rust und MSVC-Buildtools.
- [Inno Setup 6 oder 7](https://jrsoftware.org/inno-setup/downloads), `ISCC.exe` im PATH oder über `-InnoCompiler` angegeben.
- [Minisign](https://jedisct1.github.io/minisign/) für signierte Releases, `minisign.exe` im PATH.
- GitHub CLI mit Zugriff auf das öffentliche Download-Repository.

Das Skript installiert oder lädt keine Werkzeuge nach. Die benötigte `vcruntime140.dll` wird aus dem offiziellen Redist-Verzeichnis der vorhandenen Visual-Studio-Toolchain mitgeliefert. Abweichende Pfade sind über `-VCRuntimeDirectory` möglich. Siehe [Microsoft zur app-lokalen VC++-Laufzeit](https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files).

## Signaturschlüssel

Ein Minisign-Schlüsselpaar außerhalb des Projekts erzeugen. Beispielpfade:

- öffentlich: `%USERPROFILE%\NoteWardenSigning\note-warden.pub`
- privat: `%USERPROFILE%\NoteWardenSigning\note-warden.key`

Mit `minisign -G -p <öffentlicher-Pfad> -s <privater-Pfad>` erzeugen und ein Passwort lokal vergeben. Schlüssel und Passwort getrennt sichern. Der private Schlüssel gehört weder in den Quellcode noch in Release-Dateien. Bestehende Installationen vertrauen dem eingebetteten öffentlichen Schlüssel; der private Schlüssel muss für künftige Updates erhalten bleiben.

Minisign schützt die App-Updates. Die Windows-Authenticode-Herausgebersignierung ist ein separater Schritt und derzeit nicht eingerichtet.

## Installer bauen

Nur lokal, ohne Online-Updater:

```powershell
./scripts/build-installer.ps1
```

Für den signierten GitHub-Update-Kanal:

```powershell
./scripts/build-installer.ps1 `
  -GitHubRepository BungeeDEV/note-warden `
  -PublicKeyFile "$env:USERPROFILE\NoteWardenSigning\note-warden.pub" `
  -SigningKeyFile "$env:USERPROFILE\NoteWardenSigning\note-warden.key"
```

Den tatsächlich freigegebenen Repository-Namen verwenden. Das Skript liest die Version aus Cargo-Metadaten, bettet Endpunkt und öffentlichen Schlüssel ein, baut die optimierte Windows-App ohne Konsole und erzeugt das Setup. Es signiert und verifiziert das Setup, erstellt `latest.json`, `.minisig`, `release-notes.md` und `SHA256SUMS.txt`. GitHub-Dateien liegen standardmäßig in `target/installer/github`, lokale Installer separat in `target/installer/local`; Cargo-Zielverzeichnisse werden berücksichtigt. Es lädt nichts hoch.

Version vor einem neuen Release in Cargo- und Paketdateien erhöhen und einen passenden Abschnitt in `CHANGELOG.md` ergänzen. Der Installer übernimmt die Cargo-Version automatisch.

## GitHub-Release vorbereiten

Nach bestätigtem Anlegen des öffentlichen Download-Repositories und lokal geprüftem Setup:

```powershell
./scripts/publish-release.ps1 -GitHubRepository BungeeDEV/note-warden
```

Das Skript prüft Repository und Manifest und erstellt einen **Draft Release** mit Setup, Minisign-Datei, Update-Datei und Prüfsummen. Der Quellcode wird nicht hochgeladen. Der Entwurf liefert noch kein Update an installierte Apps aus. Erst nach Review auf GitHub veröffentlichen.

Der fest eingebettete Endpunkt ist `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`. Das Manifest verweist auf den versionierten Setup-Download desselben Releases.

## Noch ausstehende Abnahme

Erstinstallation, erneute Installation am selben benutzerdefinierten Pfad, Payload-Prüfsummen, Laufzeitdateien und datenerhaltende Deinstallation wurden auf einem frischen CI-Rechner erfolgreich geprüft. Der öffentliche Download und die Signaturprüfung wurden danach mit dem tatsächlichen Updater-Code geprüft. Ein vollständiger Wechsel zwischen zwei unterschiedlichen veröffentlichten App-Versionen einschließlich laufender App/Tray und automatischem Neustart steht noch aus. Die Backup- und Signaturfehlerpfade sind zusätzlich durch die vorhandenen Tests abgedeckt.

Die Updater-Tests decken gültige Signaturen, manipulierte Downloads, ungültige Schlüssel/Signaturen, unsichere Versionsnamen und HTTP-Adressen ab. Netzwerkzugriffe sind auf HTTPS begrenzt; Manifest und Installer besitzen Größenlimits. Zusätzlich wird das tatsächliche Release-Schlüsselpaar anhand einer veröffentlichten Testsignatur geprüft. Die PowerShell-Skripte sind syntaktisch geprüft; ihre Ausführung erfolgt im Release-Workflow.

Die explizite Neustart-Aktion des Setups verwendet den dokumentierten [Inno-Update-Modus](https://jrsoftware.org/inno-setup/documentation/topic_technotes.htm); sie setzt keine Windows-Restart-Manager-Registrierung voraus, die [RestartApplications](https://jrsoftware.org/ishelp/topic_setup_restartapplications.htm) sonst benötigt.

Wenn während eines Release-Builds Änderungen an `.github/workflows/` auf den Hauptzweig gepusht werden, kann GitHub das Erstellen eines Tags auf dem älteren Commit mit dem eingeschränkten CI-Token ablehnen. In diesem Fall den Workflow auf dem aktuellen Hauptzweig erneut starten. Veröffentlichte Release-Dateien werden vom Skript niemals ersetzt; vor einem weiteren Release muss die Version erhöht werden.
