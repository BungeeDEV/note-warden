# Release und Auto-Update

Der verbindliche Ablauf steht in [windows-release.md](windows-release.md): Änderungen unter `Unreleased` sammeln, mit `python scripts/release.py prepare x.y.z` eine Version vorbereiten, den PR prüfen und zusammenführen, dann den Release-Workflow auf `main` mit dieser Version starten.

Der Workflow baut und prüft den Installer und erzeugt einen signierten **Draft Release**. Erst nach Prüfung der Dateien und Notizen auf GitHub veröffentlichen. Der lokale Build und Upload bleiben über `scripts/build-installer.ps1` und `scripts/publish-release.ps1` möglich; Parameter und Signierschlüssel-Verwaltung sind in der Release-Anleitung beschrieben.

Der Updater ist nur aktiv, wenn beim Build Endpunkt und öffentlicher Schlüssel gesetzt sind. Der private Schlüssel bleibt außerhalb des Quellcodes. Veröffentlichte Versionen werden nicht überschrieben.
