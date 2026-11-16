# Windows-Installer und GitHub-Updates

## Aktueller Stand

Das öffentliche Projekt- und Release-Repository ist **[BungeeDEV/note-warden](https://github.com/BungeeDEV/note-warden)**. Der Nutzer hat die Veröffentlichung des App-Quellcodes freigegeben. Persönliche Entwicklungsdaten, Arbeitsdaten und private Schlüssel sind ausgeschlossen; `.claude` verweist auf persönliche Entwicklungsdaten und wird nicht hochgeladen.

Der lokale Setup-Compiler konnte aufgrund einer automatischen Freigabeablehnung nicht eingerichtet werden. Der reproduzierbare Installer-Build läuft deshalb auf GitHubs `windows-2025`-Runner mit vorhandenem Inno Setup. Der Workflow erstellt zunächst einen Entwurf, der vor der Veröffentlichung geprüft wird. Der Status steht unter [Actions](https://github.com/BungeeDEV/note-warden/actions).

## Normaler Release-Ablauf auf GitHub

1. Version und Changelog im Quellcode aktualisieren und die Änderungen pushen.
2. Den Workflow **Windows installer release** unter Actions manuell starten, alternativ `gh workflow run release.yml --repo BungeeDEV/note-warden`.
3. Der Workflow prüft Formatierung, Clippy, Tests und UI-Regeln. Anschließend erstellt er Setup und signierte Update-Dateien.
4. Setup, `.minisig`, `latest.json` und Prüfsummen erscheinen in einem Draft Release. Nach Review den Entwurf veröffentlichen.

Der Workflow verwendet `installer/update-signing.pub` und das Actions-Secret `NOTE_WARDEN_MINISIGN_KEY`. Der öffentliche Schlüssel ist fest im jeweiligen Release-Build eingebettet. Der private Schlüssel wird nur auf dem kurzlebigen Runner in eine temporäre Datei geschrieben und danach entfernt.

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

Erstinstallation und ein echter Wechsel von einer älteren Installation müssen mit Testdaten geprüft werden: eigenes Installationsziel, laufende App/Tray, Abbruch, fehlgeschlagenes Backup, Downloadfehler, ungültige Signatur, Neustart und Datenerhalt. Das Repository, der GitHub-Endpunkt und das Release-Schlüsselpaar sind eingerichtet. Setup-Kompilation und Artefaktprüfung werden durch den Release-Workflow ausgeführt.

Die Updater-Tests decken gültige Signaturen, manipulierte Downloads, ungültige Schlüssel/Signaturen, unsichere Versionsnamen und HTTP-Adressen ab. Netzwerkzugriffe sind auf HTTPS begrenzt; Manifest und Installer besitzen Größenlimits. Zusätzlich wird das tatsächliche Release-Schlüsselpaar anhand einer veröffentlichten Testsignatur geprüft. Die PowerShell-Skripte sind syntaktisch geprüft; ihre Ausführung erfolgt im Release-Workflow.

Die explizite Neustart-Aktion des Setups verwendet den dokumentierten [Inno-Update-Modus](https://jrsoftware.org/inno-setup/documentation/topic_technotes.htm); sie setzt keine Windows-Restart-Manager-Registrierung voraus, die [RestartApplications](https://jrsoftware.org/ishelp/topic_setup_restartapplications.htm) sonst benötigt.
