# Release und Auto-Update

Der aktuelle, skriptgestützte Ablauf und der noch offene Einrichtungsstand stehen in [windows-release.md](windows-release.md). Die folgenden Abschnitte beschreiben die zugrunde liegenden manuellen Schritte.

Der Updater ist nur aktiv, wenn beim Bauen Endpunkt und öffentlicher Schlüssel gesetzt sind. Sonst zeigt "Über & Updates" "Automatische Updates sind nicht eingerichtet". Ein privater Schlüssel gehört nie ins Repository.

## Einmalig: Schlüsselpaar erzeugen

1. [minisign](https://jedisct1.github.io/minisign/) installieren.
2. `minisign -G -p note-warden.pub -s note-warden.key` (Passwort setzen).
3. `note-warden.key` bleibt lokal oder wird als CI-Secret abgelegt, nie committen.
4. Die zweite Zeile von `note-warden.pub` ist der öffentliche Schlüssel.

## Pro Release

1. Version in `Cargo.toml`, `package.json` und `package-lock.json` erhöhen, `CHANGELOG.md` ergänzen.
2. Bauen mit Endpunkt und öffentlichem Schlüssel (PowerShell):
   `$env:NOTE_WARDEN_UPDATE_URL="https://github.com/<konto>/<repo>/releases/latest/download/latest.json"`
   `$env:NOTE_WARDEN_UPDATE_PUBKEY="<zweite Zeile aus note-warden.pub>"`
   `cargo build --release`
3. Installer erzeugen (Inno Setup 6): `iscc /DAppVersion=<version> installer\note-warden.iss` ergibt `target\installer\NoteWarden-<version>-setup.exe`.
4. Signieren: `minisign -Sm target\installer\NoteWarden-<version>-setup.exe -s note-warden.key`.
5. `latest.json` schreiben (der Inhalt der `.minisig`-Datei steht komplett, mit `\n`, im Feld `signature`):

```json
{
  "version": "1.0.1",
  "notes": "## 1.0.1\n\n### Added\n* ...",
  "url": "https://github.com/<konto>/<repo>/releases/download/v1.0.1/NoteWarden-1.0.1-setup.exe",
  "signature": "untrusted comment: ...\nRUQ...\ntrusted comment: ...\n..."
}
```

6. Installer und `latest.json` als Dateien eines GitHub-Releases (oder auf einem anderen Host) ablegen. Der Endpunkt aus Schritt 2 muss auf `latest.json` zeigen.

Die App prüft beim Start höchstens einmal in 24 Stunden und manuell. Sie lädt den Installer, prüft die Signatur, sichert die Daten und startet den Installer erst danach. Die Installation läuft pro Benutzer und lässt den Datenordner unberührt.
