> Historische Designreferenz für DBWarden. Für die aktuelle Note-Warden-Oberfläche gelten [ui-guidelines.md](ui-guidelines.md) und der [Audit 1.1.0](ui-audit-1.1.0.md).

# DBWarden – Design-Spezifikation für Rust und GPUI

DBWarden ist ein nativer Desktop-Client für SQL-Arbeit. Der Standardbildschirm folgt der Referenz `preview.png` bei 1280 × 720. Die dunkle, kompakte Arbeitsfläche, die durchgehende Seitenleiste und das vertikal geteilte Editor-Ergebnis-Layout bestimmen das Erscheinungsbild. Das bisherige Design liefert die Grundlage für Farben, Typografie und Komponenten. Bei Abweichungen bestimmt der Screenshot die Geometrie des SQL-Arbeitsbereichs.

## 1. Technische Grundlage

- Rust und [GPUI](https://gpui.rs/) rendern das Fenster und alle UI-Elemente. Layout, Farben und Abstände sind typisierte Rust-Werte. WebView, DOM, CSS und Browser-Assets zur Laufzeit entfallen.
- Eine kompatible GPUI-Version oder Git-Revision festlegen und `Cargo.lock` für die Desktop-App einchecken. Beispiele und optionale Komponenten gegen genau diese Version prüfen. Fenstersteuerung und Eingabe auf jeder unterstützten Zielplattform prüfen.
- Die Shell und zustandsbehaftete Ansichten implementieren `Render`. Editordokumente, Tabs und Ergebniszustände leben in `Entity<T>` oder in entsprechend stabilen Rust-Modellen. GPUI-Elemente erhalten stabile IDs. Einfaches Layout entsteht mit `div()` und den GPUI-Layoutprimitiven.
- Aktionen wie `RunQuery`, `CancelQuery`, `NewQueryTab` und `CloseQueryTab` über GPUI-Actions und fokusspezifische Keybindings binden. Buttons rufen dieselben Aktionen auf. Fokus über `FocusHandle` führen.
- GPUI Kit ist eine optionale native Komponentenbibliothek. Für Textfelder, SQL-Editor und Tabelle zuerst Eingabeverhalten, Gestaltungsspielraum und GPUI-Kompatibilität prüfen. Für große Ergebnismengen kommt das virtualisierte DataTable oder ein eigener GPUI-Viewport mit virtualisierten Zeilen infrage.
- SVG-Icons und Schriftdateien lokal mitliefern und über die GPUI-Assets laden. Eigene DBWarden-Schildmarke, einheitliche Outline-Icons für Bedienaktionen und keine Abhängigkeit von einem Icon-CDN.

## 2. Fenster und Layout

Alle Maße sind logische GPUI-Pixel. 1280 × 720 bezeichnet die Referenzfläche der App. Falls das Betriebssystem einen separaten Fensterrahmen rendert, kommt dieser außen hinzu. Die Maße sind Startwerte für die visuelle Abnahme.

| Bereich | Maß bei 1280 × 720 | Aufbau |
| --- | --- | --- |
| Titelzeile | 40 px hoch | Schild, **DBWarden** und kleines `PREVIEW`-Badge links. Verbindungstitel optisch mittig. Hilfe rechts. Native Fensterknöpfe und Drag-Flächen plattformgerecht integrieren. |
| Seitenleiste | 240 px breit | 12 px Außenabstand für Navigation. Explorer nimmt die flexible Höhe ein. Verbindungsaktion, Einstellungen und lokaler Arbeitsbereich bilden den unteren Block. |
| Kontextzeile | 48 px hoch | Datenbank-Icon, **MariaDB · Demo**, Trenner, **SQL-Arbeitsbereich** und `DEMO`. Rechts Schloss und **Lesemodus**. |
| Tab-Leiste | 40 px hoch | **Abfrage 1**, Schließen-Icon und Plus. 2 px Sky-Unterstreichung unter dem aktiven Tab. Bei vielen Tabs horizontal scrollen. |
| Editor-Werkzeugleiste | 48 px hoch | 16 px Seitenabstand. **Ausführen** als 32 px hoher Primärbutton, Shortcut im Button, **bookshop** als Outline-Auswahl daneben. Rechts Ausführungshinweis, sofern Platz vorhanden. |
| Editor | ca. 200 px hoch | Zeilennummern, SQL-Eingabe und eigene Statuszeile für Dialekt, Kodierung und Ausführungsmodus. |
| Vertikaler Splitter | 4 px sichtbar | Trennt Editor und Ergebnis. Vergrößerte Trefferzone, Größenänderung per Maus und Tastatur. |
| Ergebnis-Werkzeugleiste | 44 px hoch | **Ergebnis**, Zeilen-Badge, Filterfeld und Kopieraktion. |
| Ergebnisraster | füllt den Rest | Kopf und Zeilen jeweils ca. 36 px. Separater Scrollbereich mit fixiertem Kopf und gemeinsamem horizontalem Offset für Kopf und Zellen. |
| Ergebnisstatus | 32 px hoch | Trefferzahl und Ausführungsdauer links, **Beispieldaten** rechts. |
| Globale Statusleiste | 28 px hoch | Verbindungsstatus links, App-Version mittig, Treiber und Datenquelle rechts. |

### Verhalten bei kleinerem Fenster

Die Shell bleibt vertikal in Titelzeile, Arbeitsfläche und Statusleiste gegliedert. Sidebar und Hauptbereich teilen die Arbeitsfläche horizontal. Der Explorer, der Editor und das Ergebnis scrollen unabhängig. Die Sidebar startet bei 240 px. Unter 960 px Fensterbreite kann sie eingeklappt und per beschrifteter Umschaltaktion wieder geöffnet werden. Im Hauptbereich zuerst den Toolbar-Hinweis und sekundäre Statusdetails ausblenden. Tabs und Ergebnisspalten bleiben horizontal erreichbar. Editor und Ergebnis behalten am Splitter sinnvolle Mindesthöhen. Bei 720 px Referenzhöhe liegt der Splitter bei etwa y = 378 px. Danach folgt er dem gespeicherten Verhältnis innerhalb der verfügbaren Höhe.

## 3. Farben, Typografie und Form

| Token | Standard | Verwendung |
| --- | --- | --- |
| `canvas` / `surface` | `#171717` / `#191919` | App-Hintergrund, Seitenleiste und angehobene Arbeitsflächen |
| `hover` / `active_row` | `#232323` / `#1D1D1D` | Hoverflächen und aktive Editorzeile |
| `separator` / `control_border` | `#272727` / `#3A3A3A` | Raster, Trennlinien und Feldkonturen |
| `text` / `text_secondary` / `text_muted` | `#F0F0F0` / `#B4B4B4` / `#929292` | Primärtext, Nebeninformationen und Metatext |
| `accent` / `accent_text` | `#0EA5E9` / `#38BDF8` | Primärfläche, Fokus, aktiver Tab, Navigation und SQL-Keywords |
| `accent_subtle` / `on_accent` | `#142832` / `#0A0A0A` | Aktive Navigation und lesbare Button-Beschriftung |
| `success` / `warning` / `danger` | `#34D399` / `#FBBF24` / `#F87171` | Statusfarben mit Icon oder Textlabel |

Die Palette orientiert sich am Screenshot. Status- und Metatexte sind für Lesbarkeit etwas heller gesetzt. Der Primärbutton nutzt dunklen Vordergrund. Pro Theme Text und relevante Kontrollzustände auf tatsächlichem Hintergrund prüfen. Zielwerte sind 4,5:1 für normalen Text und 3:1 für wesentliche Bedienelementgrenzen oder Fokusindikatoren. Deko-Trennlinien dürfen subtiler ausfallen.

- **UI-Schrift** Inter, 14 px Standard, 13 px für dichte Bedienelemente, 11–12 px für Status, Zähler und Labels. Explorer-Überschrift in Versalien mit leicht erhöhtem Zeichenabstand. Plattformschrift als Fallback.
- **Mono** JetBrains Mono, 13 px für SQL und Datenwerte, 12 px für Tabellennamen und technische Metadaten. Plattform-Monospace als Fallback. Editor-Zeilenhöhe ca. 24 px.
- **Konturen** 1 px. Radien 6–8 px für Felder und Buttons, 10–12 px für Karten und Menüs, 16 px für Dialoge. Ergebnisraster und Shell haben eckige, durchgehende Kanten.
- **Icons** 16–18 px für Navigation und Aktionen, 12–14 px für Inline-Metadaten. Icon-only-Buttons erhalten Tooltips und verständliche Bezeichnungen, wo das Plattform-Backend sie ausgeben kann.
- **Bewegung** 120–180 ms für Hover/Fokus und ca. 160–200 ms für Overlays. Arbeitsflächen und Tabellen bleiben bei Datenänderungen stabil. Bei reduzierter Bewegung Übergänge unmittelbar ausführen.

### Theme-System

`ui/theme.rs` definiert `ThemePreset` für gespeicherte Eingaben und `ThemeTokens` für aufgelöste semantische Farben. Der App-Kontext hält das aktive Theme als GPUI-`Global`. Ein Theme-Wechsel aktualisiert diesen Zustand und invalidiert die betroffenen Ansichten. Aktive Tabs, Fokus, Scrollposition und Editorinhalt bleiben erhalten.

Der Referenzzustand ist **DBWarden Dunkel**. Weitere Presets aus der Vorlage können als **Mitternacht**, **Wald**, **Sonnenuntergang**, **Rosé** und **Hell** übernommen werden. Dunkle und helle Presets lösen ihre Oberflächen und Textfarben getrennt auf. Bei einem benutzerdefinierten Akzent die Vordergrundfarbe aus dem Kontrast gegen die tatsächliche Akzentfläche ableiten. Eigene Themes und Presets werden über stabile IDs in den lokalen UI-Einstellungen gespeichert.

## 4. Arbeitsflächen und Komponenten

### Sidebar und Explorer

- Navigation in dieser Reihenfolge **Verbindungen**, **SQL-Arbeitsbereich**, **Verlauf**. Einträge 40 px hoch, mit Icon links und optionalem Zähler rechts. Aktive Navigation hat eine getönte Fläche und Sky-Text. Hover und Tastaturfokus sind getrennte Zustände.
- Explorer enthält Überschrift und Aktualisieren, Verbindungszeile mit Statuspunkt, Suche und einen einrückbaren Schema-/Tabellenbaum. Demo zeigt **MariaDB · Demo** und das geöffnete Schema **bookshop**. Suche filtert Tabellennamen und lässt passende Elternknoten sichtbar.
- Pfeiltasten bewegen den Fokus im Baum. Rechts öffnet einen Knoten, links schließt ihn oder wechselt zum Elternknoten. Enter aktiviert die fokussierte Zeile. Ladezustand, Verbindungsfehler und leere Trefferliste erhalten eigene Beschriftungen.
- Unten stehen **Verbindung trennen**, **Einstellungen** und **Lokaler Arbeitsbereich** mit der Unterzeile **Auf deinem Gerät**. Im Demo-Modus beendet die Verbindungsaktion die lokale Demo-Session.

### SQL-Editor und Tabs

- Tabs behalten Text, Auswahl, Cursor, Scrollposition, Schema und letztes Ergebnis pro Dokument. Neue Tabs erhalten fortlaufende Namen. Ein dirty markierter Tab mit ungesichertem Inhalt braucht eine Schließen-Bestätigung. Nach dem Schließen wandert der Fokus zum benachbarten Tab oder zum leeren Arbeitsbereich.
- Der native mehrzeilige Editor unterstützt Texteingabe inklusive IME, Auswahl, Kopieren/Einfügen, Undo/Redo, vertikalen und horizontalen Scroll sowie sichtbare Zeilennummern. Als Minimal-Highlighting gelten SQL-Keywords, Kommentare, Strings und Zahlen. Die Highlighting-Schicht verändert den Dokumenttext und seine Auswahlpositionen nicht.
- **Ausführen** verwendet bei aktiver Textauswahl diese Auswahl, andernfalls den gesamten Inhalt des aktiven Tabs. Standardshortcut **Strg+Enter** auf Windows/Linux und **⌘+Enter** auf macOS, im Button passend angezeigt. Leeres SQL deaktiviert die Aktion. Während einer laufenden Abfrage zeigt die Oberfläche den Laufzustand und bietet **Abbrechen** an, sofern der Datenpfad Abbruch unterstützt.
- Ergebnisse gehören zur auslösenden Tab- und Request-ID. Eine verspätete Antwort aktualisiert nur die zugehörige, noch aktuelle Anfrage. Ein Tabwechsel während der Abfrage blockiert die Oberfläche nicht.

### Ergebnisraster

- Kopfzeile mit schmaler Zeilennummernspalte `#`, danach Spaltenname plus leiser SQL-Typ. Demo-Reihenfolge `id INT`, `title VARCHAR`, `author_id INT`, `price DECIMAL`, `stock INT`. Der Titel erhält den größten Platz. Werte bleiben einzeilig, lange Zellwerte sind vollständig einsehbar und kopierbar.
- **Ergebnis filtern …** filtert lokal die bereits geladenen Zeilen. Bei aktivem Filter zeigt der Footer Treffer und geladene Gesamtzahl. Die Kopieraktion übernimmt die Zellenauswahl, andernfalls die sichtbare gefilterte Ergebnismenge als TSV mit Spaltenköpfen. SQL-`NULL` ist sichtbar von leeren Strings unterscheidbar.
- Leeres Resultset, kein Filtertreffer, laufende Abfrage, Abbruch und SQL-Fehler haben unterscheidbare Zustände. Ein Ergebnislimit oder eine Teilmenge wird im Status als solche ausgewiesen. Bei großen Ergebnissen nur sichtbare Zeilen und einen kleinen Puffer rendern. Datenmenge und Filterarbeit begrenzen oder aus dem UI-Thread auslagern.

### Verbindungen, Verlauf und Einstellungen

**Verbindungen** und **Verlauf** verwenden dieselbe Shell. Verbindungsprofile erscheinen als kompakte Karten mit Name, Treiber, Adresse in Monospace, beschriftetem Verbindungsstatus und Aktionen. Der Verlauf nutzt eine dichte Liste mit SQL-Vorschau, Zeit und Status. Ein Eintrag kann in einem neuen Abfrage-Tab geöffnet werden. Einstellungen nutzen beschriftete Felder, Hilfetext, Toggle-Zeilen und eine Theme-Auswahl. Passwortfelder sind standardmäßig verdeckt und haben eine beschriftete Sichtbarkeitsaktion.

### Overlays und Bedienelemente

- Varianten für Buttons sind `Primary`, `Outline`, `Ghost` und `Danger`. Felder und Buttons sind im Arbeitsbereich 32 px, in Formularen 36 px hoch. Hover, gedrückter Zustand, deaktivierter Zustand und sichtbarer 2-px-Tastaturfokus sind separat gestaltet.
- Dialoge liegen zentriert auf einem gedimmten Overlay. Standardbreite bis 512 px, Kopf und Footer bleiben sichtbar, Body scrollt bei Bedarf. Rechte Detail-Drawer sind bis 560 px breit. Modale Overlays binden den Fokus, Escape schließt die oberste Ebene und der Fokus kehrt zum Auslöser zurück. Klick außerhalb schließt nur, wenn dadurch keine Eingabe verloren geht.
- Kontextmenüs öffnen am Auslöser oder Zeiger, bleiben mit 8 px Rand im Fenster und unterstützen Pfeiltasten, Enter und Escape. Toasts liegen unten rechts über der Statusleiste. Fehler, die eine Handlung verlangen, erscheinen zusätzlich am betroffenen Bereich.
- Deutsche UI-Texte, SQL-Begriffe und Treibernamen im Original. Status über Text und/oder Symbol vermitteln, Farbe dient als zusätzliche Hilfe.

## 5. Reproduzierbarer Demo-Start

Die App startet für die Designabnahme direkt im **SQL-Arbeitsbereich** mit **MariaDB · Demo**, aktivem **Lesemodus**, Schema **bookshop** und Tab **Abfrage 1**. Explorer und Ergebnis werden aus lokalen Fixtures aufgebaut. Die Statusleiste meldet **Demo · kein Server verbunden**. Die Referenzwerte **6 Zeilen** und **3 ms** sind Demo-Metadaten. Live-Zeilenanzahl und Laufzeit kommen später vom tatsächlichen Abfrageergebnis.

```sql
-- Beispieldaten · kein Server verbunden

SELECT * FROM books
LIMIT 100;
```

Die Tabelle zeigt die sechs Buchzeilen der Demo. Sichtbare Titel aus der Referenz sind **The Design of Everyday Things**, **A Philosophy of Software Design**, **Designing Data-Intensive Applications**, **Refactoring UI**, **The Pragmatic Programmer** und **Clean Architecture**. Der Fokus liegt beim Start auf einem stabilen Referenzzustand mit bereits sichtbarem Ergebnis. Ausführen, Suche, Filter, Kopieren, Tabs und Splitter lassen sich lokal bedienen.

## 6. Vorgeschlagene Code-Struktur

```text
src/
  main.rs                  Fensterstart, Assets, Keybindings
  ui/
    theme.rs               Presets, Tokens und aktives Theme
    shell.rs               Titel, Sidebar, Hauptbereich und Status
    components/            Buttons, Felder, Badges und Overlays
    views/                 Explorer, Editor, Ergebnisse und weitere Ansichten
  workspace.rs             Tabs, Sessions und Abfragezustand
  demo.rs                  Lokale Demo-Daten und Demo-Ausführung
assets/
  icons/
  fonts/
```

Zuerst Shell und Theme aufbauen, dann SQL-Ansicht mit Demo-Fixtures, danach Eingabe, Aktionen und wiederverwendbare Overlays. Views rendern aus dem Workspace-Zustand. Abfrageausführung läuft außerhalb des Renderpfads und liefert ein typisiertes Ergebnis zurück. Eine echte MariaDB-Anbindung ist ein eigener Umsetzungsschritt. Für eine spätere echte Verbindung muss der Lesemodus über den Ausführungspfad und Datenbankberechtigungen durchgesetzt werden.

## 7. Abnahme

- `cargo fmt --check`, `cargo check --locked` und `cargo build --locked` laufen durch. Die native App startet auf der festgelegten Referenzplattform.
- Bei 1280 × 720 stimmen visuelle Hierarchie, Sidebar-Breite, Zeilenhöhen, Splitterlage und Ergebnisraster mit der Referenz überein. Betriebssystem-Fensterdekorationen und Font-Rendering dürfen plattformbedingt abweichen.
- Bei kleinerem Fenster und erhöhter Skalierung bleiben Navigation, Ausführen, Editor und Ergebnis erreichbar. Scrollbereiche und Splitter konkurrieren nicht um den Fokus oder dieselbe Scrollposition.
- Tastaturprüfung umfasst Navigation, Eingabefokus, Textauswahl, Ausführen, Tabwechsel, Ergebniskopie und Escape in Dialogen. Theme-Wechsel bewahrt den Arbeitszustand.
- Demo-Ergebnis mit sechs Zeilen, Nulltreffer, Fehler- und Ladezustand sowie ein größeres Resultset zur Kontrolle des Scrollverhaltens manuell prüfen.

## Technische Referenzen

- [GPUI – Einstieg und Beispiele](https://gpui.rs/)
- [GPUI – API-Referenz](https://docs.rs/gpui/latest/gpui/)
- [Zed – natives GPUI-Textinput-Beispiel](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs)
- [GPUI Kit – Table und DataTable im Vergleich](https://gpui-kit.com/component/table/)
