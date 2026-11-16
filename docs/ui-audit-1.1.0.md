# UI-Audit 1.1.0 – Stand 16. November 2026

**Aktualisierung:** Die Desktop-Verbindung ist wieder verfügbar. Live-Prüfungen mit synthetischen Daten (140 Updates, lange Projektnamen, Aufgaben und Releases) wurden bei 1280×800 in Dunkel sowie 960×620 in Hell mit großer Schrift durchgeführt. Dabei wurden überbreite Board-Spalten, abgeschnittene Kalenderaktionen, eine überlaufende Tagesliste und überlappende Titelzeilen erkannt und korrigiert. Die früheren Angaben „Live-Abnahme offen“ in der folgenden Erstprüfung sind historisch. Eine vollständige Kombination aller Themes, Dichten und Schriften sowie Screenreader/Windows-Kontrastdesign bleibt weiterhin ungetestet.

## Ergebnis und Grenze der Prüfung

Die vorherige Sitzung endete am Nutzungslimit während des Audits. Ihre Änderungen waren teilweise gespeichert, aber nicht abschließend geprüft. Der aktuelle Arbeitsbereich wurde als maßgeblich verwendet. Im Projektordner und seinen Eltern liegt kein Git-Repository; `git status` ist deshalb nicht verfügbar.

Die unten genannten Quelltextkorrekturen sind implementiert und automatisiert geprüft. **Die visuelle Gesamtabnahme ist offen.** Beide verfügbaren Wege zur App-Steuerung meldeten eine fehlende native Verbindung (`Computer Use native pipe is unavailable`, Windows-Fehler 2); auch der Browser-Host ist nicht verfügbar. Es wurden vorhandene Screenshots der vorherigen Sitzung betrachtet. Diese zeigen einen älteren Build und sind ausdrücklich keine Nachher-Belege für 1.1.0. Die laufende vorherige Testinstanz wurde nicht umgestellt.

Maßstab sind die gemeinsamen Tokens und Primitives in `palette.rs` und `ui/kit.rs`. Apples [Layout-Leitlinien](https://developer.apple.com/design/human-interface-guidelines/layout) dienen als Referenz für anpassbare Fenster, Textgrößen, Abstände und Hierarchie. Die Windows-App erhält dadurch keine Apple-HIG- oder Accessibility-Zertifizierung.

## Geprüfte Bereiche und Korrekturen

| Bereich | Befund und Umsetzung | Beleg / Grenze |
| --- | --- | --- |
| Gemeinsame Zeilenaktionen | Der frühere Verlauf und anschließend deckende Hover-Block konnten Titel und Datum überdecken. Aktionen nutzen jetzt normalen Flex-Platz, auch wenn sie ohne Hover unsichtbar sind. | Quelltextprüfung für Updates, Heute und Aufgaben; Live-Hover und Tastaturfokus offen. |
| Updates | Kompakte Meta-Spalte plus Aktionen ließ kaum Titelbreite. Kompakt zeigt Titel und Priorität; weitere Metadaten liegen im vollständigen Tooltip. Zeilenhöhe folgt der Schriftgröße. | Compiler und Regelprüfung; lange Titel und große Schrift live offen. |
| Große Listen | `uniform_list` mischte 28-px-Gruppenköpfe und 36-/56-px-Einträge. Virtualisierte Gruppen verwenden nun dieselbe skalierte Höhe wie die Einträge. | Quelltextprüfung; Scrollen mit mindestens 100 Einträgen live offen. |
| Neu-Split-Button | Eigene Rundungen an den äußeren Ecken, gemeinsame Fläche, innere Trennlinie. Gruppen-Clipping entfernt, damit Fokus nicht abgeschnitten wird. | Kompiliert; Hover, Fokus und Dropdown live offen. |
| Mehrfachauswahl | Starre Toolbar-Höhe konnte Aktionen in schmalen Listen verdrängen. Mindesthöhe und Umbruch statt fester Höhe. | Kompiliert; schmale Liste live offen. |
| Archiv und Papierkorb | Eigene Leerzustände; leere Bereiche und ausgefilterte Inhalte werden unterschieden. Suche leeren und sekundäre Filter zurücksetzen wechseln nicht aus Archiv oder Papierkorb heraus. | Quelltextprüfung; Kombinationen aus Suche und Filtern live offen. |
| Heute | Öffnen-Aktion reserviert Platz, Datum schrumpft nicht unter seine Beschriftung. Vollständige Titel als Tooltip, leere Update-Titel erhalten einen Platzhalter. | Vorhandener Vorher-Screenshot und Quelltext; Nachher-Prüfung offen. |
| Inbox | Statt Textbutton und numerischer Priorität: gemeinsame Aufgaben-Checkbox und Prioritätssymbol; Datum und Erinnerung sichtbar. Umbruchfähige Zeilen, feste Eingabe über begrenzter Scrollfläche. | Kompiliert; Eingabe, Umwandlung und längere Aufgaben live offen. |
| Board | Gleiche Flex-Basis, Mindestbreite 220 px, Resthöhe unter festem Titel und Scrollbar-Abstand. Lange Titel haben Tooltip, Karten bleiben innerhalb ihrer Spalte. | Vorher-Screenshot zeigt angeschnittene vierte Spalte; Nachher-Prüfung offen. |
| Releases | Inhaltsbreite nutzt den verfügbaren Platz bis zum Leselimit, Aktionen umbrechbar. Download-Asset eingebettet. | Kompiliert; gefüllte Release-Karten live offen. |
| Kalender | Kopf kann umbrechen; Aufgaben-Controls sind größer und beschriftet. Inbox-Einträge öffnen die Inbox statt Update-ID 0. | Quelltextprüfung; Monat, Woche, Drag and Drop und Popover live offen. |
| Einstellungen | Kategorien scrollen unabhängig von Zurück und Suche; skalierte Mindesthöhe verhindert Stauchen. Auswahl reserviert immer denselben Rand. Passende Icons für Startseite, Projekte, Vorlagen und System. | Vorher-Screenshot zeigt fehlendes System-Icon; Nachher-Prüfung offen. |
| Editor, Verlauf und Overlays | Scrollbegrenzungen, deckende Flächen, Größenlimits und vorhandene Fokus-Fallen im Quelltext geprüft. Aufgabenüberschriften linksbündig; Checkbox-Haken zentral ausgerichtet. | Kein Ersatz für Live-Prüfung von Fokus-Rückgabe, Esc und Fenstergrenzen. |
| Assets | Download, Monitor und Flag explizit eingebettet. Neuer Test lädt alle 53 direkt in den UI-Quellen referenzierten Icon-Varianten über die reale `AppAssets`-Quelle und prüft SVG-Inhalt. | Regressionstest bestanden. |
| Dokumentation und Version | Cargo, Lockdateien, Branding-Paket und Installer-Vorgabe auf 1.1.0; README und UI-Richtlinien aktualisiert. Rust-Quellen formatiert und vorhandene Clippy-Hinweise bereinigt. | Alle unten genannten Prüfungen bestanden. |

## Automatisierte Prüfung

| Prüfung | Ergebnis |
| --- | --- |
| `cargo fmt --all --check` | bestanden |
| `npm run check:ui` | bestanden |
| `cargo clippy --locked --all-targets -- -D warnings` | bestanden, keine Warnungen |
| `cargo test --locked` | 34 bestanden, 0 fehlgeschlagen |
| `cargo build --locked` | bestanden, Debug-Build 1.1.0 unter `target/debug/note-warden.exe` |

Clippy und Tests liefen mit dem bereits vorhandenen Build-Cache unter `%LOCALAPPDATA%/Temp/nw-target`; zusätzlich wurde der normale Projekt-Build unter `target/debug` erstellt. Kein Installer und kein signiertes Release wurden veröffentlicht.

## Verbleibende Live-Abnahme

Sobald die native App-Verbindung wieder verfügbar ist, müssen die neuen Änderungen bei 960×620, 1280×800 und breitem Fenster in Hell/Dunkel, komfortabler/kompakter Dichte sowie kleiner/normaler/großer Schrift betrachtet werden. Inhaltsszenarien: leer, lange Titel/Projektnamen/Pfade, viele Aufgaben, mindestens 100 gruppierte Updates, gefüllte Releases und Versionsstände. Interaktionen: Hover, Tab-Fokus, Menüs, Suche zurücksetzen, Datum, Fenstergröße, Scrollbars und Drag and Drop. Kontrasttests decken die vorhandenen Farbtokens ab; Screenreader, Windows-Kontrastdesign und jede konkrete Farbkombination in gerenderten Zuständen sind damit nicht abgenommen.

Diese Prüflücke verhindert die Aussage, dass sämtliche visuellen Fehler der gesamten App beseitigt oder alle modernen Guidelines vollständig erfüllt seien.

## Ergänzung: Installer und GitHub-Updates

In der anschließenden Distributionsarbeit wurden ein optimierter Windows-Build, Setup-/Release-Skripte und zusätzliche Updater-Prüfungen ergänzt. Es bestehen jetzt 36 Tests. Setup-Compiler, öffentliches Download-Repository und Release-Schlüssel sind noch nicht eingerichtet; es wurde kein Setup oder Online-Update veröffentlicht. Aktueller Ablauf und Grenzen: [Windows-Releases](windows-release.md). Die oben genannte visuelle Abnahme bleibt offen.
