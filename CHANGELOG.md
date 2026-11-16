## 1.1.0

### Distribution
* Build-Skript für Windows-Installer und signierte GitHub-Update-Dateien sowie separates Skript für einen GitHub-Draft-Release.
* Installation pro Benutzer mit MSVC-Laufzeit, festem Installationsziel bei Updates und explizitem Neustart nach einem In-App-Update.
* Updater: HTTPS, Größenlimits, Validierung der Manifest-Version und Tests für manipulierte Downloads und ungültige Signaturen.
* Optimierter Windows-Build ohne zusätzliches Konsolenfenster.

### Fixed
* Live-Abnahme: Board-Breiten werden aus dem verfügbaren Fenster berechnet, damit lange Projektnamen die Spalten nicht verbreitern.
* Kalender: Mindesthöhe hält Einträge und „weitere“-Aktion sichtbar; die vollständige Tagesliste besitzt eine begrenzte Scrollfläche.
* Detail: Mehrzeilige Titel verwenden eine zur Schriftgröße passende Zeilenhöhe, damit sich große Titel nicht überlagern.
* Zeilen: Aktionen reservieren Platz in Updates, Heute und Aufgaben; keine Verläufe oder deckenden Aktionsblöcke über Text und Datum.
* Updates: Abgerundete äußere Ecken des Neu-Split-Buttons ohne Clipping des Fokus; kompakte Zeilen lassen dem Titel mehr Platz und zeigen Metadaten im Tooltip.
* Listen: Schriftabhängige Zeilenhöhe und einheitliche Höhe für Gruppenköpfe in der virtualisierten Liste; Mehrfachaktionen können umbrechen.
* Archiv und Papierkorb: Eigene Leerzustände; Suche leeren und sekundäre Filter zurücksetzen erhalten den aktuellen Bereich.
* Inbox: Gemeinsame Aufgaben-Checkboxen und Prioritätssymbole, Datumsaktionen, umbruchfähige Zeilen und feste Eingabe oberhalb der Scrollfläche.
* Board: Flexible, gleichmäßige Spalten mit Mindestbreite; begrenzte Scrollfläche, Scrollbar-Abstand und Titel-Tooltips.
* Einstellungen: Scrollbare Kategorien, gleichbleibende Icon-Kante bei Auswahl, passende Kategorien-Icons und Zurück-Button mit Icon.
* Releases: Volle verfügbare Inhaltsbreite bis zum Leselimit und umbruchfähige Kartenaktionen.
* Kalender: Umbruchfähiger Kopf; Inbox-Einträge öffnen die Inbox, Aufgaben-Checkboxen sind beschriftet.
* Icons: Download, Monitor und Flag werden offline eingebettet; Regressionstest lädt sämtliche in der Oberfläche referenzierten Icon-Assets.

### Changed
* Versionsangaben und Startdokumentation aktualisiert; Audit mit ausdrücklich noch offener Live-Abnahme dokumentiert.

## 1.0.0

### Added
* Backups: Automatische Sicherungen im Ordner `backups/` beim Start und alle 30 Minuten bei Änderungen, vor Migration, Import, Wiederherstellung und Update; Aufbewahrung nach Tagen und Wochen, manuelle Backups bleiben erhalten.
* Daten & Backups: Liste der Backups mit Wiederherstellen, Im Ordner zeigen und Löschen sowie "Jetzt sichern".
* Versionsverlauf: Notizstände je Update als Snapshots (60 s Ruhe, höchstens alle 5 Minuten, höchstens 30) mit Seitenpanel, Vorschau und Wiederherstellen samt Rückgängig.
* Releases: Dialog "Veröffentlichen" erzeugt den Changelog im Standardformat, setzt das Update auf Erledigt und speichert den Release; Zurücknehmen mit Rückgängig.
* Releases: Ansicht mit Zeitleiste, Projektfilter, Kopieren und Export aller Versionen als CHANGELOG.md.
* Erinnerungen: Erinnerung an Updates und Aufgaben (Fälligkeit, Heute 18:00, Morgen 9:00, eigene Zeit) mit Windows-Benachrichtigung, Schlummern, Tagesübersicht, ruhigen Zeiten und gesammelter Meldung verpasster Erinnerungen.
* System: Tray-Symbol, Weiterlaufen beim Schließen, Autostart minimiert, globaler Hotkey für Quick-Capture mit Konflikterkennung und nur eine laufende Instanz.
* Updates: Auto-Updater mit Signaturprüfung, aktiv nur bei eingetragenem Endpunkt und Schlüssel; Installer-Skript für Windows und Anleitung `docs/releasing.md`.
* Stabilität: Logdatei mit Rotation, Fehlerbildschirm statt Absturz der Oberfläche, automatische Wiederherstellung aus der neuesten gültigen Sicherung bei defekter Datendatei.

### Changed
* Daten: Schema 8 mit optionalen Feldern für Release und Erinnerung; Migration legt `.bak-0.9.2` und ein Backup an.
* Changelog: Eine zentrale Funktion erzeugt das Standardformat für Tab, Export und Release.
* Einstellungen: Neue Abschnitte Benachrichtigungen und System, "Über" heißt "Über & Updates".

### Removed
* Backups: Die alten Einstellungen für automatische Backups (Intervall, Anzahl, Zielordner) und der Dialog "Changelog erzeugen".

## 0.9.2

### Added
* Board: Karten zeigen Projekt (Farbpunkt und Name), Fortschritt und Datum in der Meta-Zeile; leere Spalten zeigen eine gestrichelte Ablagefläche, Karten lassen sich auf Spalten ziehen.
* Projekte: Zentrale Funktion `projectColor` und feste Palette mit 8 Tönen; neue Projekte erhalten ihre Farbe deterministisch aus der Projekt-ID.

### Changed
* Zeilen: Gemeinsamer Leading-Slot (24 px) für Checkbox und Status-Icon, Titel in Heute, Inbox und Listen beginnen auf derselben Kante.
* Zeilen: Hover-Aktionen (Öffnen, Datum, Mehr) liegen absolut am Zeilenende mit Verlauf und erscheinen auch bei Tastaturfokus, ohne Platz zu reservieren.
* Priorität: Dreibalken-Symbol mit klar abgestuften Farben (Niedrig, Mittel, Hoch), mindestens 3:1 Kontrast und Tooltip; auch in Heute statt der Flagge.
* Menüs und Popover: Hover-Fläche deutlicher, Rand und Trennlinien mit mindestens 3:1 Kontrast.
* Navigation: Projektpunkte in der Sidebar zeigen die Projektfarbe statt des Akzents.

### Removed
* Zeilen: Reservierter Platz für unsichtbare Hover-Buttons und die abweichenden Einrückungswerte.

## 0.9.1

### Changed
* Board: Karten mit linksbündigem Titel, Beschriftung "Verschieben nach" und drei Status-Buttons statt dreier Textzeilen; Updates ohne Titel erscheinen als "Ohne Titel".
* Einstellungen: Kategorien in der Navigation linksbündig mit fluchtenden Icons.
* Heute: Woche und Inbox zeigen keine Projekt-Chips und Fortschrittsbalken mehr, die Titel verdrängten; in der Wochenliste entfällt der redundante Datums-Chip.
* Oberfläche: Zeilenhöhe in Listen- und Navigationszeilen, damit Unterlängen nicht abgeschnitten werden.

## 0.9.0

### Added
* Bewegung: Zentrale Motion-Tokens (120, 200, 300 ms) mit gemeinsamer Auslaufkurve und Hilfsfunktion für Dauern.
* Toasts: Restzeit-Balken pausiert bei Hover, damit "Rückgängig" erreichbar bleibt.
* Oberfläche: Tooltips mit vollem Titel an Update-Zeilen und Projekten in der Sidebar.
* Qualität: `npm run check:ui` prüft zusätzlich Farbliterale, Schriftgewichte, Radius-Literale und Animationsdauern außerhalb der Tokens.

### Changed
* Navigation: Sidebar wechselt bereits ab 1100 px Fensterbreite auf die Icon-Leiste.
* Oberfläche: Radien 4, 6 und 8 px laufen über Tokens statt über Einzelwerte.
* Dokumentation: UI-Richtlinien um Typografie, Elevation, Motion und Undo-Regel ergänzt.

### Removed
* Toasts: Fest laufende Balkenanimation, die bei Hover nicht anhalten konnte.

## 0.8.0

### Added
* Updates: Listenkopf mit zwei Zeilen, Split-Button "Neu" mit Chevron und einem Filter-&-Ansicht-Popover für Sortierung, Gruppierung, Anzeige und Filter mit Zählern.
* Updates: Reihe aktiver Filter-Chips unter der Suche, die nur bei aktiven Filtern erscheint.
* Detail: Eigenschaften als Chip-Zeile mit Status-Dropdown, Datum, Priorität, Projekt und Version, Underline-Tabs und kompaktes Segment für den Editormodus.
* Detail: Editor-Fläche mit Formatierungsleiste bei Fokus, Aufgabenkopf mit Fortschrittsbalken und ruhiger Speicherstatus mit Wiederholen bei Fehlern.
* Navigation: Gespeicherte Ansichten als Sidebar-Gruppe "Ansichten" und Projekte als Gruppe "Projekte".
* Daten: Schema 7 mit Backup als `.bak-0.7.0` vor der Migration; fehlende Erstellzeiten werden aus der Änderungszeit oder der Datendatei ergänzt.

### Changed
* Updates: Listenzeilen mit ruhiger Meta-Zeile aus Projekt, Fortschritt und Datum, Aktionen und Platzhalter nur bei Hover oder Auswahl.
* Navigation: Gruppenköpfe linksbündig mit Chevron links, 12 px und Gewicht 600 in `text-tertiary`.
* Detail: Eine Textspalte mit höchstens 720 px und Zeitangaben als dezente Textzeile, die fehlende Zeiten ausblendet.

### Removed
* Updates: Sortier- und Gruppierungspillen, breiter Filter-Button, Preset- und Filter-Chip-Reihen, Dichte-Chip und "Sortierung umkehren".
* Detail: Inspector samt Kürzel Strg+Umschalt+I und der Anzeige "Zeit unbekannt".

## 0.7.0

### Added
* UI-System: Zentrale Icon-Zuordnung, einheitliche Größen-Tokens, Button-Varianten, StatChip, Chips, Tastenkappen und Eingabehöhen sowie `npm run check:ui` und ein Dev-Styleguide mit Strg+Umschalt+D.
* Updates: Drei-Bereiche-Layout mit Inspector, Status-Icons nach Form und Farbe, segmentierte Statusleiste, Filter-Chips, Sortierung, Gruppierung und Dichte.
* Updates: Ansichten mit eingebauten und gespeicherten Filtern, Mehrfachauswahl, Tastaturbedienung und Entwürfe ohne Titel, die automatisch verschwinden.
* Detail: Tabs für Übersicht, Changelog und Aktivität, Versionsfeld, Slash-Menü und Fokus-Modus mit Strg+Umschalt+F.
* Bedienung: Globales Rückgängig und Wiederholen mit Strg+Z und Strg+Y für Update-Änderungen.
* Daten: Schema 6 mit Version, Aktivität, Aufgabentyp, Beschreibung und gespeicherten Ansichten sowie Backup als `.bak-0.6.0` vor der Migration.

### Changed
* Startseite: Wochenstreifen nutzt einen einzigen Anker und bietet den Rückweg zur aktuellen Woche, Wochentage folgen dem eingestellten Wochenstart.
* Startseite: Einheitliche relative Zeitangaben, Status-Icons statt Checkboxen bei Updates und keine leeren Fortschrittskreise ohne Aufgaben.

### Removed
* Oberfläche: Uneinheitliche Icon-Größen, Textzeichen als Aktionsicons und Klickflächen ohne Button-Semantik.

## 0.6.0

### Added
* Startseite: Konfigurierbare Fokus-, Wochen-, Inbox-, Fortschritts- und Aktivitätskarten mit Kennzahlen und direkten Aktionen.
* Quick-Add: Token-Markierung, Vorschläge für Projekte, Termine und Prioritäten sowie Vorschau bei Fokus.
* Einstellungen: Eigene Ansicht mit zehn Kategorien, Suche mit Sprung zum Treffer und live gespeicherten Startseiten-Optionen.

### Changed
* Updates: Zusammenhängender Neu-Button, Auswahlmenüs, neutrale Selektion, großer Titel und einheitliche Eigenschaften und Editor-Aktionen.
* Bedienung: Sichtbare Scrollbars, flexible Inhalte, Popups auf oberster Ebene und Mindestfenstergröße von 960 × 620.
* Windows-Build: Laufende eigene EXE-Artefakte werden vor dem Linken umbenannt; cargo run --locked kann erneut bauen.

### Removed
* Heute: Mini-Monatskalender, doppelter Leerzustand und dauerhaft sichtbarer veralteter Quick-Add-Hinweis.
* Oberfläche: Getönte aktive Navigationsflächen, zusätzliche Selektionsrahmen und separater Aufgaben-Plusbutton.

## 0.5.0

### Added
* Inbox: Eigenständige Aufgaben mit Abhaken und Konvertierung in Updates
* Board: Statusspalten mit Öffnen und Verschieben von Updates
* Quick-Add: Gemeinsamer Parser für Datum, Projekt und Priorität sowie Aufgabenanlage aus der Befehlspalette
* Daten: Schema 5 mit Projekten, Inbox-Aufgaben und Priorität; unverändertes Backup vor Migration aus 0.4.0

### Changed
* Farben: Statusfarben bleiben unabhängig vom Akzent; Magenta ersetzt roten Akzent
* Navigation: Board mit Strg+3 und Kalender mit Strg+4
* Heute: Quick-Add legt Aufgaben in der Inbox an; Strg+Enter erzeugt ein Update

### Removed
* Heute: Doppelter Leertext der leeren Heute-Gruppe

## 0.4.0

### Added
* Logo und Branding: neues Pixel-Schild als App-Icon (Exe, Taskleiste, Titelleiste), Sidebar-Logo, Splash beim Start, Leerzustände mit Schild-Motiv und Logo-Generator unter `branding/`
* Splash-Screen: Logo, Name und Version mit Fortschrittslinie, mindestens 500 ms sichtbar, sanftes Ausblenden und Fehlerkarte mit „Trotzdem öffnen“ bei Ladefehlern
* Heute-Ansicht: Zusammenfassungszeile, Quick-Add mit `@`-Datum und Taste `N`, Gruppen Überfällig, Heute, Diese Woche, Später und Erledigt heute, Abhaken mit kurzer Verzögerung, Seitenleiste mit Mini-Kalender und zuletzt bearbeiteten Updates ab 1200 px Breite
* Kalender: Rechtsklick-Menü, Hover-Plus und Doppelklick zum Anlegen, Wochenansicht mit Karten, Kalenderwochen, Tastaturfokus mit Umschalt+Pfeilen, Taste `M` und `W`, durchsuchbares und in der Breite anpassbares Panel „Ohne Termin“
* Einstellungen: große Ansicht mit Suche und den Kategorien Allgemein, Darstellung, Editor, Kalender & Datum, Tastenkürzel, Daten & Backup, Vorlagen und Über, sofortiges Speichern mit „Gespeichert“-Hinweis und Zurücksetzen pro Abschnitt
* Darstellung: acht Akzentfarben mit automatisch berechnetem Kontrasttext, Dichte Komfortabel oder Kompakt, drei Schriftgrößen und Option zum Reduzieren von Animationen
* Daten: automatische Backups mit Intervall, Anzahl und Zielordner, Export aller Updates als Markdown-Ordner, Papierkorb-Aufbewahrung wählbar, Gefahrenzone mit Bestätigung durch Eintippen
* Vorlagen: Bearbeiten, Duplizieren, Löschen und Standardvorlage für neue Updates
* Updates: Mehrfachauswahl mit Strg und Umschalt samt Aktionsleiste für Status, Datum, Archivieren und Löschen
* UI-Grundlagen: Spacing-Skala, Control-Höhen, Schalter, Segment-Control, Farbfelder, Karten, Einstellungszeilen und Tastenkappen unter `ui/kit` sowie `docs/ui-guidelines.md`
* Datenmigration: Arbeitsbereiche mit Schema 3 werden auf Schema 4 gehoben, vorher entsteht eine Kopie `workspace.json.bak-0.3.0`

### Changed
* Seitenleiste: Gruppen Status und Filter mit merkbarem Zustand, Zähler 0 ausgeblendet, Badge bei Heute (rot bei Überfälligem), Archiv und Papierkorb unten abgesetzt, Tooltips mit Kürzeln im eingeklappten Zustand
* Kalender: Zeilen füllen die Höhe, Wochenkopf linksbündig, dezentere Gitterlinien, Pills mit Statusbalken und Mini-Checkbox, Drop-Ziel mit Akzentrand
* Toasts: maximal drei gleichzeitig, mit Icon und Fortschrittsbalken bis zum Ausblenden
* Updates: Karten mit Statusbalken, Eigenschaften mit festen Label-Breiten
* Editor: Formatierungsleiste, Wortzähler und Schriftart sind in den Einstellungen wählbar
* Datumsformat: zusätzliche Formate „10. Nov. 2026“ und „10.11.2026“, relative Angaben abschaltbar
* Speichern: Verzögerung des Autosaves ist einstellbar, Theme, Akzent und Fenstergröße werden zusätzlich in `prefs.json` gehalten
* Papierkorb: der Hinweis nennt keine feste Frist mehr, die Aufbewahrungsdauer ist einstellbar

### Removed
* Einstellungsdialog: der schmale Dialog mit Textfeld für Vorlagennamen wich der neuen Einstellungsansicht
* Platzhalter-Logo: die blaue „N“-Kachel der Seitenleiste
* Flache Ansichten-Liste: die neun gleich gewichteten Zeilen der Seitenleiste

## 0.3.0

### Added
* Markdown-Notizen: Notizen werden als Markdown gerendert (Überschriften, Listen, Aufgabenlisten, Tabellen, Zitate, Links, Code) mit den Modi Schreiben, Vorschau und Geteilt, Werkzeugleiste, Listenfortsetzung, Einrücken per Tab und klickbaren Checkboxen in der Vorschau
* Datums-Chips: Termine für Updates und Aufgaben über einen Chip mit relativem Text, Popover mit Schnellauswahl, Monatskalender und Texteingabe wie „morgen“ oder „in 2 wochen“
* Heute-Ansicht: neue Startansicht mit den Gruppen Überfällig, Heute und Diese Woche
* Kalender: Monats- und Wochenansicht, Drag and Drop zwischen Tagen, Panel „Ohne Termin“ und schnelles Anlegen per Klick auf einen Tag
* Aufgaben: Unteraufgaben, Umsortieren per Drag and Drop oder Alt+Pfeil, Typ und Beschreibung pro Aufgabe, mehrzeiliges Einfügen und Rückgängig-Hinweis beim Löschen
* Befehlspalette: Strg+K für Suche und Befehle
* Archiv und Papierkorb: Archivieren, Wiederherstellen und 30 Tage Aufbewahrung gelöschter Updates
* Vorlagen, Duplizieren, Export als Markdown, Changelog-Export, Backup-Export und -Import mit Vorschau und Merge
* Einstellungen: Theme (System, Dunkel, Hell), Wochenstart, Datumsformat und Editor-Standardmodus
* Datenmigration: Arbeitsbereiche aus 0.2.0 werden automatisch auf Schema 3 gehoben, vorher entsteht eine Kopie `workspace.json.bak-0.2.0`

### Changed
* Design: zentrale Design-Tokens, dunkles und helles Theme, höherer Kontrast für Labels, einheitliche Icons
* Layout: einklappbare Seitenleiste mit Ansichten, in der Breite anpassbare Liste, zentrierte Detailspalte mit Eigenschaften-Zeile und Fortschrittsbalken
* Speichern: Änderungen werden nach kurzer Ruhe geschrieben, der Speicherstatus erscheint nur noch in der Detail-Kopfzeile
* Listenkarten: linksbündig mit Status, Termin-Chip und Fortschritt

### Removed
* Statusleiste: die untere Leiste, der doppelte „Neues Update“-Button der Seitenleiste und der Hinweis „Auf diesem Gerät“
* Dauerhafte Aufgaben-Buttons: „Bearbeiten“, „×“ und der einzelne Termin-Button je Aufgabe wichen Hover-Aktionen und Chips
