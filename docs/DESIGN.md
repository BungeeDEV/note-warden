> Historical design reference for DBWarden. For the current Note Warden interface, [ui-guidelines.md](ui-guidelines.md) and the [audit 1.1.0](ui-audit-1.1.0.md) apply.

# DBWarden – design specification for Rust and GPUI

DBWarden is a native desktop client for SQL work. The default screen follows the reference `preview.png` at 1280 × 720. The dark, compact workspace, the full-height sidebar and the vertically split editor/result layout determine the appearance. The existing design provides the basis for colors, typography and components. Where they differ, the screenshot determines the geometry of the SQL workspace.

## 1. Technical basis

- Rust and [GPUI](https://gpui.rs/) render the window and all UI elements. Layout, colors and spacing are typed Rust values. A WebView, DOM, CSS and browser assets at runtime are omitted.
- Pin a compatible GPUI version or Git revision and check in `Cargo.lock` for the desktop app. Check examples and optional components against exactly this version. Verify window control and input on every supported target platform.
- The shell and stateful views implement `Render`. Editor documents, tabs and result state live in `Entity<T>` or in similarly stable Rust models. GPUI elements get stable IDs. Simple layout is built with `div()` and the GPUI layout primitives.
- Bind actions such as `RunQuery`, `CancelQuery`, `NewQueryTab` and `CloseQueryTab` through GPUI actions and focus-specific key bindings. Buttons call the same actions. Manage focus through `FocusHandle`.
- GPUI Kit is an optional native component library. For text fields, the SQL editor and the table, first check input behavior, design flexibility and GPUI compatibility. For large result sets, the virtualized DataTable or a custom GPUI viewport with virtualized rows is an option.
- Ship SVG icons and font files locally and load them through the GPUI assets. A custom DBWarden shield mark, consistent outline icons for actions and no dependency on an icon CDN.

## 2. Window and layout

All dimensions are logical GPUI pixels. 1280 × 720 denotes the app's reference area. If the operating system renders a separate window frame, it is added on the outside. The dimensions are starting values for the visual acceptance.

| Area | Size at 1280 × 720 | Structure |
| --- | --- | --- |
| Title bar | 40 px high | Shield, **DBWarden** and a small `PREVIEW` badge on the left. Connection title visually centered. Help on the right. Integrate native window buttons and drag areas in a platform-appropriate way. |
| Sidebar | 240 px wide | 12 px outer spacing for navigation. The explorer takes the flexible height. Connection action, settings and local workspace form the bottom block. |
| Context row | 48 px high | Database icon, **MariaDB · Demo**, divider, **SQL workspace** and `DEMO`. On the right a lock and **Read mode**. |
| Tab bar | 40 px high | **Query 1**, close icon and plus. 2 px sky underline under the active tab. Scroll horizontally with many tabs. |
| Editor toolbar | 48 px high | 16 px side spacing. **Run** as a 32 px high primary button, shortcut inside the button, **bookshop** as an outline selector next to it. On the right an execution hint, if there is room. |
| Editor | approx. 200 px high | Line numbers, SQL input and a separate status line for dialect, encoding and execution mode. |
| Vertical splitter | 4 px visible | Separates editor and result. Enlarged hit zone, resizing by mouse and keyboard. |
| Result toolbar | 44 px high | **Result**, row badge, filter field and copy action. |
| Result grid | fills the rest | Header and rows approx. 36 px each. A separate scroll area with a fixed header and a shared horizontal offset for header and cells. |
| Result status | 32 px high | Hit count and execution time on the left, **Sample data** on the right. |
| Global status bar | 28 px high | Connection status on the left, app version in the center, driver and data source on the right. |

### Behavior in a smaller window

The shell stays vertically divided into title bar, workspace and status bar. The sidebar and the main area share the workspace horizontally. The explorer, the editor and the result scroll independently. The sidebar starts at 240 px. Below 960 px window width it can be collapsed and reopened with a labeled toggle action. In the main area, first hide the toolbar hint and secondary status details. Tabs and result columns stay reachable horizontally. The editor and the result keep sensible minimum heights at the splitter. At the 720 px reference height, the splitter is at about y = 378 px. After that it follows the saved ratio within the available height.

## 3. Colors, typography and shape

| Token | Default | Use |
| --- | --- | --- |
| `canvas` / `surface` | `#171717` / `#191919` | App background, sidebar and raised work surfaces |
| `hover` / `active_row` | `#232323` / `#1D1D1D` | Hover surfaces and the active editor row |
| `separator` / `control_border` | `#272727` / `#3A3A3A` | Grid, dividers and field outlines |
| `text` / `text_secondary` / `text_muted` | `#F0F0F0` / `#B4B4B4` / `#929292` | Primary text, secondary information and meta text |
| `accent` / `accent_text` | `#0EA5E9` / `#38BDF8` | Primary surface, focus, active tab, navigation and SQL keywords |
| `accent_subtle` / `on_accent` | `#142832` / `#0A0A0A` | Active navigation and readable button labels |
| `success` / `warning` / `danger` | `#34D399` / `#FBBF24` / `#F87171` | Status colors with an icon or a text label |

The palette is based on the screenshot. Status and meta texts are set somewhat lighter for readability. The primary button uses a dark foreground. Per theme, check text and relevant control states on the actual background. Target values are 4.5:1 for normal text and 3:1 for essential control boundaries or focus indicators. Decorative dividers may be more subtle.

- **UI font** Inter, 14 px by default, 13 px for dense controls, 11–12 px for status, counters and labels. The explorer heading in capitals with slightly increased letter spacing. Platform font as a fallback.
- **Mono** JetBrains Mono, 13 px for SQL and data values, 12 px for table names and technical metadata. Platform monospace as a fallback. Editor line height approx. 24 px.
- **Outlines** 1 px. Radii 6–8 px for fields and buttons, 10–12 px for cards and menus, 16 px for dialogs. The result grid and the shell have square, continuous edges.
- **Icons** 16–18 px for navigation and actions, 12–14 px for inline metadata. Icon-only buttons get tooltips and understandable labels where the platform backend can output them.
- **Motion** 120–180 ms for hover/focus and approx. 160–200 ms for overlays. Work surfaces and tables stay stable on data changes. With reduced motion, perform transitions immediately.

### Theme system

`ui/theme.rs` defines `ThemePreset` for saved inputs and `ThemeTokens` for resolved semantic colors. The app context holds the active theme as a GPUI `Global`. A theme change updates this state and invalidates the affected views. Active tabs, focus, scroll position and editor content are preserved.

The reference state is **DBWarden Dark**. Further presets from the template can be adopted as **Midnight**, **Forest**, **Sunset**, **Rosé** and **Light**. Dark and light presets resolve their surfaces and text colors separately. With a custom accent, derive the foreground color from the contrast against the actual accent surface. Custom themes and presets are stored through stable IDs in the local UI settings.

## 4. Work surfaces and components

### Sidebar and explorer

- Navigation in this order: **Connections**, **SQL workspace**, **History**. Entries 40 px high, with an icon on the left and an optional counter on the right. Active navigation has a tinted surface and sky text. Hover and keyboard focus are separate states.
- The explorer contains a heading and refresh, a connection row with a status dot, search and an indentable schema/table tree. The demo shows **MariaDB · Demo** and the open schema **bookshop**. Search filters table names and keeps matching parent nodes visible.
- Arrow keys move the focus in the tree. Right opens a node, left closes it or moves to the parent node. Enter activates the focused row. The loading state, connection errors and an empty hit list get their own labels.
- At the bottom are **Disconnect**, **Settings** and **Local workspace** with the subline **On your device**. In demo mode, the connection action ends the local demo session.

### SQL editor and tabs

- Tabs keep text, selection, cursor, scroll position, schema and the last result per document. New tabs get consecutive names. A tab marked dirty with unsaved content needs a confirmation on closing. After closing, the focus moves to the neighboring tab or to the empty workspace.
- The native multi-line editor supports text input including IME, selection, copy/paste, undo/redo, vertical and horizontal scrolling and visible line numbers. Minimal highlighting covers SQL keywords, comments, strings and numbers. The highlighting layer does not change the document text or its selection positions.
- **Run** uses the selection when text is selected, and otherwise the entire content of the active tab. The default shortcut is **Ctrl+Enter** on Windows/Linux and **⌘+Enter** on macOS, displayed appropriately in the button. Empty SQL disables the action. While a query is running, the interface shows the running state and offers **Cancel**, provided the data path supports cancellation.
- Results belong to the triggering tab and request ID. A late response only updates the associated, still current request. Switching tabs during the query does not block the interface.

### Result grid

- A header row with a narrow row-number column `#`, followed by the column name plus a quiet SQL type. Demo order `id INT`, `title VARCHAR`, `author_id INT`, `price DECIMAL`, `stock INT`. The title gets the most space. Values stay single-line, and long cell values can be viewed and copied in full.
- **Filter result …** filters the already loaded rows locally. With an active filter, the footer shows hits and the total number loaded. The copy action takes the cell selection, and otherwise the visible filtered result set as TSV with column headers. SQL `NULL` is visibly distinguishable from empty strings.
- An empty result set, no filter hit, a running query, cancellation and an SQL error have distinguishable states. A result limit or a subset is labeled as such in the status. For large results, render only the visible rows and a small buffer. Limit the amount of data and the filter work, or move them off the UI thread.

### Connections, history and settings

**Connections** and **History** use the same shell. Connection profiles appear as compact cards with a name, driver, address in monospace, a labeled connection status and actions. The history uses a dense list with an SQL preview, time and status. An entry can be opened in a new query tab. Settings use labeled fields, help text, toggle rows and a theme selection. Password fields are hidden by default and have a labeled visibility action.

### Overlays and controls

- Button variants are `Primary`, `Outline`, `Ghost` and `Danger`. Fields and buttons are 32 px high in the workspace and 36 px in forms. Hover, pressed state, disabled state and a visible 2 px keyboard focus are designed separately.
- Dialogs sit centered on a dimmed overlay. Default width up to 512 px, header and footer stay visible, and the body scrolls when needed. Right detail drawers are up to 560 px wide. Modal overlays trap the focus, Escape closes the topmost layer and the focus returns to the trigger. A click outside only closes if no input is lost by doing so.
- Context menus open at the trigger or pointer, stay inside the window with an 8 px margin and support arrow keys, Enter and Escape. Toasts sit at the bottom right above the status bar. Errors that require an action additionally appear in the affected area.
- Interface text is in German, SQL terms and driver names in the original. Convey status through text and/or a symbol; color serves as additional help.

## 5. Reproducible demo start

For the design acceptance, the app starts directly in the **SQL workspace** with **MariaDB · Demo**, active **Read mode**, the schema **bookshop** and the tab **Query 1**. The explorer and the result are built from local fixtures. The status bar reports **Demo · no server connected**. The reference values **6 rows** and **3 ms** are demo metadata. The live row count and runtime come later from the actual query result.

```sql
-- Sample data · no server connected

SELECT * FROM books
LIMIT 100;
```

The table shows the six book rows of the demo. Visible titles from the reference are **The Design of Everyday Things**, **A Philosophy of Software Design**, **Designing Data-Intensive Applications**, **Refactoring UI**, **The Pragmatic Programmer** and **Clean Architecture**. On start, the focus is on a stable reference state with an already visible result. Run, search, filter, copy, tabs and the splitter can be operated locally.

## 6. Proposed code structure

```text
src/
  main.rs                  window start, assets, key bindings
  ui/
    theme.rs               presets, tokens and the active theme
    shell.rs               title, sidebar, main area and status
    components/            buttons, fields, badges and overlays
    views/                 explorer, editor, results and further views
  workspace.rs             tabs, sessions and query state
  demo.rs                  local demo data and demo execution
assets/
  icons/
  fonts/
```

First build the shell and the theme, then the SQL view with demo fixtures, then input, actions and reusable overlays. Views render from the workspace state. Query execution runs outside the render path and returns a typed result. A real MariaDB integration is a separate implementation step. For a later real connection, read mode must be enforced through the execution path and database permissions.

## 7. Acceptance

- `cargo fmt --check`, `cargo check --locked` and `cargo build --locked` run through. The native app starts on the defined reference platform.
- At 1280 × 720, the visual hierarchy, sidebar width, row heights, splitter position and result grid match the reference. Operating system window decorations and font rendering may differ depending on the platform.
- In a smaller window and with increased scaling, navigation, run, editor and result stay reachable. Scroll areas and the splitter do not compete for the focus or the same scroll position.
- The keyboard check covers navigation, input focus, text selection, run, tab switching, result copy and Escape in dialogs. A theme change preserves the working state.
- Manually check the demo result with six rows, zero hits, the error and loading states and a larger result set to verify the scrolling behavior.

## Technical references

- [GPUI – introduction and examples](https://gpui.rs/)
- [GPUI – API reference](https://docs.rs/gpui/latest/gpui/)
- [Zed – native GPUI text input example](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs)
- [GPUI Kit – Table vs. DataTable comparison](https://gpui-kit.com/component/table/)
