# UI-Richtlinien – Note Warden 1.1

Verbindlicher Maßstab für Änderungen, keine Bescheinigung vollständiger WCAG-Konformität.
Die native GPUI-Oberfläche verwendet die Tokens aus `src/palette.rs` und Primitives aus `src/ui/kit.rs`.

## Checkliste

- [ ] Häufige Aktionen direkt zeigen, erweiterte Optionen schrittweise offenlegen. Leere Ansichten erklären die nächste Aktion.
- [ ] Wiederherstellbare Änderungen mit Undo; konkrete Bestätigung nur für endgültiges Löschen.
- [ ] Windows-Tasten und sichtbaren Fokus konsistent verwenden; Reihenfolge und Fokus-Rückgabe prüfen.
- [ ] Segoe UI Variable, Cascadia Mono für Code; skalierende Schrift und flexible Texthöhen. Große Titel eng, kleine Texte mit positivem Tracking.
- [ ] 4er-Raster, Abstände 8/12/16/24/32/48, Controls 32/40, Trefferflächen mindestens 32. Gemeinsame Oberkanten.
- [ ] Semantische Theme-Farben; Idee violett, Geplant amber, In Arbeit blau, Erledigt grün. Rot für Gefahr. Status immer mit Text.
- [ ] Textkontrast mindestens 4,5:1, relevante Icons, Ränder und Fokus mindestens 3:1. Alle Akzente, beide Themes und Windows-Kontrastdesign prüfen.
- [ ] Popover und Dialoge deckend mit Rand und Schatten. Nur modale Dialoge mit Scrim; keine gestapelten transparenten Flächen.
- [ ] Sofortige Druckreaktion. Animationen unterbrechbar, reduzierte Bewegung beachten. Für Drag kritische Federn und Nicht-Drag-Alternative.
- [ ] Felder beschriften, Fehler inline verknüpfen; Statusmeldungen zugänglich ansagen. Native Accessibility-Semantik prüfen, APG als Interaktionsreferenz nutzen.
- [ ] Deutsche, konkrete Verben und korrekte Plurale; Begriffe Update, Aufgabe, Projekt, Inbox, Board konsistent halten.

## Referenzen

- [Apple HIG – Motion](https://developer.apple.com/design/human-interface-guidelines/motion)
- [Windows – Keyboard accessibility](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/keyboard-accessibility)
- [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- [WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/)

GPUI ist kein DOM: ARIA-Attribute und CSS-Medienabfragen sind nicht unmittelbar übertragbar. Die entsprechenden nativen Accessibility- und Systemeinstellungsfunktionen müssen separat verifiziert werden.

## Regeln ab 0.6.0

- Akzent-Disziplin (60/30/10): neutrale Flächen dominieren; Akzent nur für Primäraktion, Fokus, heute, aktive Switches, Fortschritt, Links und schmale Selektionsbalken.
- Aktive Navigation und Karten: neutrale `bg-active`-Fläche, primärer Text und 2-px-Akzentbalken; kein zusätzlicher Selektionsrahmen. Segmente verwenden neutralen Text.
- Statusfarben bleiben fest und tragen immer eine Textbeschriftung; ähnliche Akzente dürfen keine Bedeutung allein vermitteln. Akzent-Kontrast wird berechnet; ähnliche Farbtöne erhalten einen Hinweis in den Einstellungen. Helle Akzente verwenden dunklen Kontrasttext, wenn dieser den besseren Kontrast liefert.
- Bedienelemente haben sichtbare Affordanz durch Hintergrund, Rand oder Chevron sowie Hover und Fokus. Zusammengehörige Controls bilden eine Gruppe.
- Verfügbare Inhaltsbreite steuert das Layout. GPUI verwendet native Layoutbedingungen statt CSS-Container-Queries. Leere Ansichten bieten eine konkrete Aktion.
- Pro Ansichtsbereich ein Scroll-Container; Flex-Eltern mit `min-height: 0`, erkennbare Scrollbars, Kopf außerhalb der Scrollfläche. Popups auf oberster Ebene und innerhalb der Fenstergrenzen.
- Hick/Fitts: wenige klare Menüoptionen, große Trefferflächen, häufige Aktionen in Reichweite und eine eindeutige Primäraktion.

## Icons und Buttons

Native GPUI-Primitives werden in `src/ui/kit.rs` erzeugt. Feature-Dateien verwenden
`button`, `icon`, `input`, `link_button`, `stat_chip`, `icon_btn` und `keycap`;
keine direkten Bibliotheks-Konstruktoren für Buttons oder Icons.

| Größe | Control | Icon | Text | Radius |
|---|---:|---:|---:|---:|
| sm | 28 | 14 | 12,5 / 500 | 6 |
| md | 32 | 16 | 13 / 500 | 8 |
| lg | 40 | 18 | 14 / 500 | 8 |

Weitere Icon-Tokens: 12 (Chips), 20 (Header), 48 (Leerzustand). Outline-Assets
werden zentral auf Strichstärke 1,75 normalisiert. Keine Textpfeile als Aktionsicons.
Inputs: 32 als Standard, 40 für Quick-Add; Radius 8. Chips sind pillenförmig.

| Variante | Fläche / Text | Verwendung |
|---|---|---|
| Primary | Akzent / Kontrasttext | eine Hauptaktion |
| Secondary | erhöhte Fläche / primärer Text | normale Aktionen |
| Ghost | transparent / sekundärer Text | Toolbar, Abbrechen, Zurück |
| Tonal Success | Grün 14 % / Grün | Abschluss |
| Tonal Warning | Amber 14 % / Amber | Handlungsbedarf |
| Tonal Danger | Rot 14 % / Rot | reversible Löschaktion |
| Solid Danger | Rot / Weiß | endgültiges Löschen bestätigen |
| Link | transparent / sekundärer Text | Kartenlink, Pfeil rechts |

Semantik: Search = Suche, Settings = Einstellungen, ArrowLeft = Zurück,
ArrowRight = Weiter/Alle ansehen, Plus = Hinzufügen, Close = Schließen,
Check = Bestätigen, Calendar = Datum, Folder = Projekt, Sun = Heute,
Inbox = Inbox, FileText = Updates, Archive = Archiv, Trash = Löschen,
Copy = Kopieren, Pencil = Bearbeiten, GripVertical = Verschieben.
Die vorhandene Lucide-Bibliothek bleibt die einzige Icon-Quelle.

Icon-Buttons tragen Tooltip und natives Accessibility-Label. Tooltip-Verzögerung:
400 ms. Fortschrittsringe ohne Aufgaben bleiben unsichtbar. Native GPUI-Klickflächen
benötigen zusätzlich Tastaturaktivierung; der Quelltext-Check ersetzt dieses Audit nicht.
Dev-Styleguide: Strg+Umschalt+D (nur Debug-Build). `npm run check:ui` prüft die
zentralen Konstruktoren, Icon-Token-Nutzung und textuelle Aktionspfeile.


## Listen und Detail

- Layout Liste | Detail: Liste 300–520 px (Breite gemerkt), Detail als eine Textspalte (max. 720 px) ohne Inspector; schmal wechselt die Ansicht zwischen Liste und Detail, Esc geht zurück. Kopfzone und Detail-Kopfleiste stehen fest, je Bereich scrollt genau ein Container.
- Dichte: Komfortabel 56 px, Kompakt 36 px (`ROW_*`-Tokens); Priorität am Titel; weitere Metadaten in der Meta-Zeile bzw. im Tooltip.
- Selektion: neutrale `bg-active`-Fläche plus 2-px-Akzentbalken links, kein Rahmen; Tastaturfokus als innerer Ring.
- Status-Icons unterscheiden sich durch Form und Farbe (Idee gestrichelt, Geplant leer, In Arbeit mit Fortschritt, Erledigt mit Haken). Checkboxen gibt es nur bei Aufgaben, nie bei Updates.
- Undo: Update-Änderungen laufen über einen Verlauf mit höchstens 50 Schritten (Strg+Z, Strg+Y); Texteingabe im Editor hat ihr eigenes Undo. Löschen und Archivieren zeigen zusätzlich einen Toast mit „Rückgängig“.
- Entwürfe mit leerem Titel und ohne Eingabe werden beim Verlassen verworfen.

## Toolbars, Filter-Popover, Chip-Zeilen, Tabs vs. Segment

- Listenkopf: zwei Zeilen (Titel mit Zähler und Split-Button „Neu“; Suche mit einem IconButton „Filter & Ansicht“). Aktive Filter erscheinen als dritte, einzeilige Chip-Reihe, sonst gibt es sie nicht. Pro Bereich höchstens ein gefülltes Akzent-Element.
- Filter-Popover (320 px): Sortieren, Gruppieren, Anzeige, Filter, jeweils sofort wirksam; Seltenes lebt hier, nicht als Pillen in der Liste. Eine Aktion hat einen Ort.
- Eigenschaften als Chip-Zeile (28 px, Pille) unter dem Titel: Status ist Pill-Dropdown und als Einziges getönt, leere Chips sind transparent mit Rand und `text-tertiary`. Platzhalter in Listenzeilen nur bei Hover oder Auswahl.
- Tabs (Seitenabschnitte) sind Underline-Tabs (`kit::tabs`, 36 px, Pfeiltasten nach APG). Segment (Modus, z. B. Schreiben | Vorschau | Geteilt) ist der kompakte Container (`kit::segment`, 28 px). Zwei gleich aussehende Elemente mit verschiedenen Aufgaben gibt es nie.
- Schriftgewichte nur 400, 500 und 600; Textfarben: `text-primary` für Gelesenes, `text-secondary` für Labels und Werte, `text-tertiary` für Platzhalter und Zeiten.

## Typografie, Elevation, Motion, Undo (ab 0.9.0)

- Typografie: Stufen `FS_SM` 12, `FS_BASE` 13, `FS_MD` 14, `FS_LG` 16, `FS_XL` 20, `FS_TITLE` 24, `DETAIL_TITLE` 28; Gewichte nur 400/500/600, Zahlen und Zeiten ruhig und gleich breit gesetzt. Abgeschnittener Text trägt einen Tooltip mit dem vollen Wert.
- Elevation: Panel = 1-px-Rand subtil; Karte = Rand strong; Popover, Menü, Toast = `shadow_lg` auf deckender `bg_elevated`-Fläche; Dialog = zusätzlich Scrim. Keine transluzenten Flächen übereinander.
- Motion: `MOTION_FAST_MS` 120, `MOTION_NORMAL_MS` 200, `MOTION_SLOW_MS` 300 über `palette::motion`, Kurve `palette::ease_out` (kritisch gedämpft). Nur Transform und Deckkraft animieren; bei reduzierter Bewegung entfallen Bewegungen.
- Undo: Löschen, Archivieren und Statuswechsel zeigen einen Toast mit "Rückgängig" (6 s, pausiert bei Hover) plus Strg+Z; Bestätigungsdialoge nur für Unumkehrbares, mit benannter Aktion ("Update löschen").

## Seitenpanel (ab 1.0.0)

Seitenpanel (z. B. Versionsverlauf): rechts, 360 px, deckende `bg_elevated`-Fläche mit Rand links und `shadow_lg`, Fokus-Falle, Esc schließt zuerst das Panel. Liste oben, Vorschau darunter, genau eine Primäraktion am Fuß.

## Abstände und Scrollbars

- Abstände (`p/px/py/pl/pr/pt/pb/gap`) nur über `palette::SP_*` (4/8/12/16/24/32); keine Literale wie 6, 10, 14 oder 20.
- Scrollbars liegen als Overlay über dem Inhalt. Scrollende Bereiche setzen rechts `palette::SCROLL_GUTTER` (12 px), damit Zeilen, Aktionen und Kürzel nicht verdeckt werden. Track unsichtbar, Daumen `border_popover`, Hover `text_tertiary`.
- Kopf, Suchzeile und Liste teilen dieselben Seitenkanten (12 px); Titel und Zeileninhalt stehen auf einer Linie (+8 px).
- Leading-Slots (Sidebar-Icons, Status-Punkte, Zeilen-Status) nutzen einen festen Slot, damit Mittellinien übereinstimmen.


## Layout-Korrekturen ab 1.1.0

- Zeilenaktionen stehen im normalen Flex-Layout und reservieren ihren Platz auch ohne Hover. Titel und Datum dürfen niemals unter Aktionen liegen. Kein Verlauf und keine separate Aktionsfläche über dem Zeileninhalt.
- Kompakte Update-Zeilen priorisieren Titel und Priorität; Projekt, Fortschritt und Datum stehen vollständig im Tooltip. Komfortable Zeilen zeigen die Meta-Zeile.
- Zeilenhöhen skalieren mit der Schriftgröße. In `uniform_list` besitzen Gruppenköpfe und Einträge dieselbe Höhe.
- Split-Buttons runden ihre äußeren Ecken selbst. Der Gruppencontainer darf Fokusumrandungen nicht abschneiden.
- Board-Spalten teilen den verfügbaren Platz mit gleicher Flex-Basis und mindestens 220 px. Unterhalb dieser Gesamtbreite bleibt horizontales Scrollen möglich. Die Scrollfläche nutzt die Resthöhe unter dem Titel.
- Inbox-Eingabe und Board-Titel bleiben außerhalb der Scrollfläche. Einstellungen halten Zurück und Suche fest; die Kategorienliste scrollt separat.
- Jede direkt referenzierte `IconName`-Variante muss über `AppAssets` offline ladbar sein. Der Asset-Test prüft die tatsächlich geladenen SVG-Daten.
- Prüfstatus und verbleibende Live-Abnahme: [Audit 1.1.0](ui-audit-1.1.0.md).
