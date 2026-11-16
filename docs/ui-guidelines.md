# UI guidelines – Note Warden 1.1

The binding yardstick for changes; not a certification of full WCAG conformance.
The native GPUI interface uses the tokens from `src/palette.rs` and the primitives from `src/ui/kit.rs`.

## Checklist

- [ ] Show frequent actions directly and reveal advanced options step by step. Empty views explain the next action.
- [ ] Recoverable changes come with undo; ask for explicit confirmation only for permanent deletion.
- [ ] Use Windows keys and a visible focus consistently; check the order and the focus return.
- [ ] Segoe UI Variable, Cascadia Mono for code; scaling type and flexible text heights. Large titles tight, small text with positive tracking.
- [ ] 4 px grid, spacing 8/12/16/24/32/48, controls 32/40, hit areas at least 32. Shared top edges.
- [ ] Semantic theme colors; Idea violet, Planned amber, In progress blue, Done green. Red for danger. Status always comes with text.
- [ ] Text contrast at least 4.5:1, relevant icons, borders and focus at least 3:1. Check all accents, both themes and the Windows contrast themes.
- [ ] Popovers and dialogs are opaque with a border and shadow. Only modal dialogs get a scrim; no stacked transparent surfaces.
- [ ] Immediate press feedback. Animations are interruptible and respect reduced motion. Springs for drag are critical, and there is a non-drag alternative.
- [ ] Label fields, link errors inline, and announce status messages accessibly. Check the native accessibility semantics and use the APG as the interaction reference.
- [ ] Interface text is German for now: concrete verbs and correct plurals; keep the terms Update, Aufgabe (task), Projekt, Inbox and Board consistent.

## References

- [Apple HIG – Motion](https://developer.apple.com/design/human-interface-guidelines/motion)
- [Windows – Keyboard accessibility](https://learn.microsoft.com/en-us/windows/apps/design/accessibility/keyboard-accessibility)
- [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- [WAI-ARIA APG](https://www.w3.org/WAI/ARIA/apg/)

GPUI is not a DOM: ARIA attributes and CSS media queries do not transfer directly. The corresponding native accessibility and system-settings features have to be verified separately.

## Rules since 0.6.0

- Accent discipline (60/30/10): neutral surfaces dominate; use the accent only for the primary action, focus, today, active switches, progress, links and narrow selection bars.
- Active navigation and cards: a neutral `bg-active` surface, primary text and a 2 px accent bar; no additional selection border. Segments use neutral text.
- Status colors stay fixed and always carry a text label; similar accents must not convey meaning on their own. Accent contrast is computed; similar hues get a hint in the settings. Light accents use dark contrast text when that gives the better contrast.
- Controls have a visible affordance through a background, a border or a chevron, plus hover and focus. Related controls form a group.
- The available content width drives the layout. GPUI uses native layout constraints instead of CSS container queries. Empty views offer a concrete action.
- One scroll container per view area; flex parents with `min-height: 0`, recognizable scrollbars, header outside the scroll area. Popups at the top level and inside the window bounds.
- Hick/Fitts: few clear menu options, large hit areas, frequent actions within reach and one unambiguous primary action.

## Icons and buttons

Native GPUI primitives are created in `src/ui/kit.rs`. Feature files use
`button`, `icon`, `input`, `link_button`, `stat_chip`, `icon_btn` and `keycap`;
no direct library constructors for buttons or icons.

| Size | Control | Icon | Text | Radius |
|---|---:|---:|---:|---:|
| sm | 28 | 14 | 12.5 / 500 | 6 |
| md | 32 | 16 | 13 / 500 | 8 |
| lg | 40 | 18 | 14 / 500 | 8 |

Further icon tokens: 12 (chips), 20 (header), 48 (empty state). Outline assets
are normalized centrally to a stroke width of 1.75. No text arrows as action icons.
Inputs: 32 as the default, 40 for quick add; radius 8. Chips are pill-shaped.

| Variant | Surface / text | Use |
|---|---|---|
| Primary | accent / contrast text | one main action |
| Secondary | elevated surface / primary text | normal actions |
| Ghost | transparent / secondary text | toolbar, cancel, back |
| Tonal Success | green 14 % / green | completion |
| Tonal Warning | amber 14 % / amber | action needed |
| Tonal Danger | red 14 % / red | reversible delete action |
| Solid Danger | red / white | confirm permanent deletion |
| Link | transparent / secondary text | card link, right arrow |

Semantics: Search = search, Settings = settings, ArrowLeft = back,
ArrowRight = next / show all, Plus = add, Close = close,
Check = confirm, Calendar = date, Folder = project, Sun = today,
Inbox = inbox, FileText = updates, Archive = archive, Trash = delete,
Copy = copy, Pencil = edit, GripVertical = move.
The existing Lucide library remains the only icon source.

Icon buttons carry a tooltip and a native accessibility label. Tooltip delay:
400 ms. Progress rings without tasks stay invisible. Native GPUI click areas
additionally need keyboard activation; the source check does not replace this audit.
Dev styleguide: Ctrl+Shift+D (debug build only). `npm run check:ui` checks the
central constructors, icon token usage and textual action arrows.


## Lists and detail

- List | detail layout: list 300–520 px (width remembered), detail as one text column (max. 720 px) without an inspector; when narrow, the view switches between list and detail, and Esc goes back. The header area and the detail header bar are fixed, and exactly one container scrolls per area.
- Density: Comfortable 56 px, Compact 36 px (`ROW_*` tokens); priority next to the title; further metadata in the meta line or the tooltip.
- Selection: a neutral `bg-active` surface plus a 2 px accent bar on the left, no border; keyboard focus as an inner ring.
- Status icons differ in shape and color (Idea dashed, Planned empty, In progress with progress, Done with a check mark). Checkboxes exist only for tasks, never for updates.
- Undo: update changes go through a history of at most 50 steps (Ctrl+Z, Ctrl+Y); text input in the editor has its own undo. Deleting and archiving additionally show a toast with "Rückgängig" (Undo).
- Drafts with an empty title and no input are discarded when leaving them.

## Toolbars, filter popover, chip rows, tabs vs. segment

- List header: two rows (title with a counter and the "Neu" (New) split button; search with one icon button "Filter & Ansicht" (Filter & View)). Active filters appear as a third, single-line chip row and do not exist otherwise. At most one filled accent element per area.
- Filter popover (320 px): sort, group, display, filter, each taking effect immediately; rarely used things live here, not as pills in the list. An action has one place.
- Properties as a chip row (28 px, pill) below the title: status is a pill dropdown and the only tinted chip, empty chips are transparent with a border and `text-tertiary`. Placeholders in list rows only on hover or selection.
- Tabs (page sections) are underline tabs (`kit::tabs`, 36 px, arrow keys per APG). A segment (mode, for example Schreiben | Vorschau | Geteilt, i.e. Write | Preview | Split) is the compact container (`kit::segment`, 28 px). There are never two similar-looking elements with different jobs.
- Font weights only 400, 500 and 600; text colors: `text-primary` for content that is read, `text-secondary` for labels and values, `text-tertiary` for placeholders and times.

## Typography, elevation, motion, undo (since 0.9.0)

- Typography: steps `FS_SM` 12, `FS_BASE` 13, `FS_MD` 14, `FS_LG` 16, `FS_XL` 20, `FS_TITLE` 24, `DETAIL_TITLE` 28; weights only 400/500/600, numbers and times set calmly and with equal widths. Truncated text carries a tooltip with the full value.
- Elevation: panel = subtle 1 px border; card = strong border; popover, menu and toast = `shadow_lg` on an opaque `bg_elevated` surface; dialog = additionally a scrim. No translucent surfaces on top of each other.
- Motion: `MOTION_FAST_MS` 120, `MOTION_NORMAL_MS` 200, `MOTION_SLOW_MS` 300 via `palette::motion`, curve `palette::ease_out` (critically damped). Animate only transform and opacity; with reduced motion, movement is omitted.
- Undo: deleting, archiving and status changes show a toast with "Rückgängig" (6 s, paused on hover) plus Ctrl+Z; confirmation dialogs only for irreversible things, with a named action ("Update löschen").

## Side panel (since 1.0.0)

Side panel (for example version history): on the right, 360 px, an opaque `bg_elevated` surface with a border on the left and `shadow_lg`, a focus trap, and Esc closes the panel first. List on top, preview below, exactly one primary action at the bottom.

## Spacing and scrollbars

- Spacing (`p/px/py/pl/pr/pt/pb/gap`) only through `palette::SP_*` (4/8/12/16/24/32); no literals such as 6, 10, 14 or 20.
- Scrollbars sit as an overlay above the content. Scrolling areas set `palette::SCROLL_GUTTER` (12 px) on the right so rows, actions and shortcuts are not covered. Track invisible, thumb `border_popover`, hover `text_tertiary`.
- Header, search row and list share the same side edges (12 px); titles and row content are on one line (+8 px).
- Leading slots (sidebar icons, status dots, row status) use a fixed slot so center lines match.


## Layout corrections since 1.1.0

- Row actions sit in the normal flex layout and reserve their space even without hover. Titles and dates must never lie under actions. No gradient and no separate action surface above the row content.
- Compact update rows prioritize the title and priority; project, progress and date are fully in the tooltip. Comfortable rows show the meta line.
- Row heights scale with the font size. In a `uniform_list`, group headers and entries have the same height.
- Split buttons round their outer corners themselves. The group container must not clip focus outlines.
- Board columns share the available space with an equal flex basis and at least 220 px. Below that total width, horizontal scrolling remains possible. The scroll area uses the remaining height below the title.
- The inbox input and the board title stay outside the scroll area. The settings keep Back and search fixed; the category list scrolls separately.
- Every directly referenced `IconName` variant must be loadable offline through `AppAssets`. The asset test checks the SVG data that is actually loaded.
- Test status and remaining live acceptance: [Audit 1.1.0](ui-audit-1.1.0.md).
