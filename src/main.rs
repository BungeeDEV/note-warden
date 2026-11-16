#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod compact;
mod dates;
mod log;
mod logo;
mod md;
mod model;
mod palette;
mod quick;
mod reminders;
mod storage;
mod system;
mod ui;
mod updater;

use chrono::{Datelike, Duration, NaiveDate};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::*;
use model::{ChangeKind, EditorMode, Status, Task, ThemeChoice, Workspace};
use reminders::ToastAction;
use std::path::PathBuf;
use std::time::{Duration as StdDuration, Instant};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

actions!(
    note_warden,
    [
        HistoryUndo,
        HistoryRedo,
        PreviousUpdate,
        NextUpdate,
        ToggleFocusMode,
        ToggleCompactMode,
        ToggleStyleguide,
        NewUpdate,
        FocusSearch,
        CloseOverlay,
        ToggleSidebar,
        OpenPalette,
        ShowToday,
        ShowUpdates,
        ShowCalendar,
        ShowBoard,
        ShowInbox,
        ShowShortcuts,
        DeleteSelected,
        CalPrev,
        CalNext,
        CalToday,
        CalMonth,
        CalWeek,
        CalFocusPrevDay,
        CalFocusNextDay,
        CalFocusPrevWeek,
        CalFocusNextWeek,
        CalOpenDay,
        OpenSettings,
        FocusQuickAdd,
        EditorBold,
        EditorItalic,
        EditorLink,
        TaskIndent,
        TaskOutdent,
        TaskMoveUp,
        TaskMoveDown,
        TaskToggleKey,
        TaskEditKey,
        TaskDeleteKey,
    ]
);

/// Wird aus der Markdown-Vorschau gesendet (Checkbox angeklickt).
#[derive(Clone, PartialEq, Action)]
#[action(namespace = note_warden, no_json)]
pub struct NoteTaskToggle {
    pub start: usize,
    pub end: usize,
    pub checked: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Today,
    Updates,
    Calendar,
    Board,
    Inbox,
    Releases,
    Stickies,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Filter {
    #[default]
    All,
    Status(Status),
    Overdue,
    NoDate,
    ThisWeek,
    Archive,
    Trash,
}

impl Filter {
    pub fn title(self) -> &'static str {
        match self {
            Self::All => "Updates",
            Self::Status(s) => s.label(),
            Self::Overdue => "Überfällig",
            Self::NoDate => "Ohne Termin",
            Self::ThisWeek => "Diese Woche",
            Self::Archive => "Archiv",
            Self::Trash => "Papierkorb",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortKey {
    #[default]
    Edited,
    Date,
    Status,
    Title,
    Priority,
}

impl SortKey {
    pub const ALL: [Self; 5] = [
        Self::Edited,
        Self::Date,
        Self::Status,
        Self::Title,
        Self::Priority,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Edited => "Zuletzt bearbeitet",
            Self::Date => "Termin",
            Self::Status => "Status",
            Self::Title => "Titel A–Z",
            Self::Priority => "Priorität",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GroupBy {
    #[default]
    None,
    Status,
    Period,
    Project,
    Priority,
}

impl GroupBy {
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Status,
        Self::Period,
        Self::Project,
        Self::Priority,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Keine Gruppierung",
            Self::Status => "Nach Status",
            Self::Period => "Nach Zeitraum",
            Self::Project => "Nach Projekt",
            Self::Priority => "Nach Priorität",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DateTarget {
    Update(u64),
    Task(u64, u64),
    /// Datum für das Quick-Add-Feld der Heute-Ansicht.
    Quick,
    Inbox(u64),
}

pub struct DatePop {
    pub target: DateTarget,
    pub anchor: Point<Pixels>,
    pub month: NaiveDate,
    pub cursor: NaiveDate,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Dialog {
    Shortcuts,
    /// Release-Dialog: veröffentlichen oder (bei vorhandenem Release) bearbeiten.
    Publish(u64),
    ConfirmPurge(u64),
    ConfirmTrash(u64),
    Import,
    /// Änderungsprotokoll der App (CHANGELOG.md, ohne Netzwerk).
    AppChangelog,
    Licenses,
    ConfirmEmptyTrash,
    ConfirmWipe,
    Template,
    /// Daten wurden automatisch aus einer Sicherung (Unix-ms) wiederhergestellt.
    Recovered(i64),
    ConfirmRestore(i64),
    /// Release-Notes der verfügbaren App-Version.
    UpdateNotes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsCat {
    General,
    Appearance,
    Home,
    Editor,
    Calendar,
    Shortcuts,
    Data,
    Notifications,
    System,
    Templates,
    Projects,
    About,
}

impl SettingsCat {
    pub const ALL: [Self; 12] = [
        Self::General,
        Self::Appearance,
        Self::Home,
        Self::Editor,
        Self::Calendar,
        Self::Shortcuts,
        Self::Data,
        Self::Notifications,
        Self::System,
        Self::Projects,
        Self::Templates,
        Self::About,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "Allgemein",
            Self::Appearance => "Darstellung",
            Self::Home => "Startseite",
            Self::Editor => "Editor",
            Self::Calendar => "Kalender & Datum",
            Self::Shortcuts => "Tastenkürzel",
            Self::Data => "Daten & Backups",
            Self::Notifications => "Benachrichtigungen",
            Self::System => "System",
            Self::Templates => "Vorlagen",
            Self::Projects => "Projekte",
            Self::About => "Über & Updates",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Self::General => "Start, Papierkorb und Fensterverhalten.",
            Self::Appearance => "Theme, Akzentfarbe, Größe und Bewegung.",
            Self::Home => "Karten und Kennzahlen der Heute-Ansicht.",
            Self::Editor => "Schreiben und Speichern von Notizen.",
            Self::Calendar => "Wochenstart, Datumsformat und Kalenderansicht.",
            Self::Shortcuts => "Alle Tastenkürzel auf einen Blick.",
            Self::Data => "Speicherort, Backups, Export und Aufräumen.",
            Self::Notifications => "Erinnerungen, Tagesübersicht und ruhige Zeiten.",
            Self::System => "Autostart, Tray und globaler Hotkey.",
            Self::Templates => "Vorlagen für neue Updates.",
            Self::Projects => "Projekte anlegen und archivieren.",
            Self::About => "Version, Updates, Änderungsprotokoll und Lizenzen.",
        }
    }
}

pub enum Undo {
    Status(u64, Status),
    Completed(u64, Option<u64>, Option<Status>),
    Tasks {
        update_id: u64,
        index: usize,
        tasks: Vec<Task>,
    },
    Trashed(u64),
    Archived(u64),
    /// Vollständiger Stand eines Updates vor Veröffentlichen/Zurücknehmen.
    Update(Box<model::Update>),
    /// Notiztext vor einer Wiederherstellung aus dem Versionsverlauf.
    Notes(u64, String),
    /// Gelöschtes Post-it samt Position.
    Sticky(usize, model::Sticky),
}

pub struct Toast {
    pub id: u64,
    pub message: String,
    pub undo: Option<Undo>,
    pub created: Instant,
    /// Bereits sichtbare Zeit in ms; pausiert bei Hover/Fokus.
    pub elapsed_ms: u64,
    pub paused: bool,
    /// Zusätzliche Schaltflächen (Erinnerungen); solche Toasts bleiben bis zum Schließen stehen.
    pub actions: Vec<(String, ToastAction)>,
}

/// Sichtbarkeit des Splash-Screens beim Start.
#[derive(Clone, Copy, PartialEq)]
pub enum Splash {
    Showing,
    /// Fade-out läuft seit dem Zeitpunkt.
    Fading(Instant),
    Gone,
}

pub const SPLASH_MIN_MS: u64 = 500;
pub const SPLASH_FADE_MS: u64 = 200;

/// Seitenpanel "Versionsverlauf" eines Updates; `snapshots` älteste zuerst, `selected` indiziert dort.
pub struct HistoryPanel {
    pub update_id: u64,
    pub snapshots: Vec<storage::Snapshot>,
    pub selected: usize,
}

pub struct ImportPreview {
    pub workspace: Workspace,
    pub new_updates: usize,
    pub duplicates: usize,
    pub tasks: usize,
}

/// Kontextmenü eines Kalendereintrags (Rechtsklick).
#[derive(Clone, Copy)]
pub struct PillMenu {
    pub update_id: u64,
    pub task_id: Option<u64>,
    pub anchor: Point<Pixels>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CalMode {
    Month,
    Week,
}

/// Everything the popup instance takes over from the main window.
pub struct PopupInit {
    pub workspace: Workspace,
    pub writable: bool,
    pub main: WeakEntity<NoteWarden>,
    pub main_window: AnyWindowHandle,
}

pub struct NoteWarden {
    pub sticky_input: Entity<TextareaState>,
    pub tag_input: Entity<InputState>,
    pub editing_sticky: Option<u64>,
    /// Only post-its with this tag are shown (not saved).
    pub sticky_tag: Option<String>,
    pub compact_list: bool,
    pub compact_preview: bool,
    /// This instance renders the "Notizzettel" popup window instead of the main window.
    pub popup: bool,
    /// The other window's instance; both stay in sync through `adopt_workspace`.
    pub peer: Option<(WeakEntity<NoteWarden>, AnyWindowHandle)>,
    /// Main window only: the open popup.
    pub popup_win: Option<(AnyWindowHandle, Entity<NoteWarden>)>,
    /// Unsynced local changes that still have to reach the other window.
    pub peer_dirty: bool,
    pub keyboard_menu: Option<(u64, usize)>,
    pub keyboard_index: usize,
    pub search_gen: u64,
    pub notes_focused: bool,
    pub slash_index: usize,
    pub history: ui::workbench::History,
    pub drafts: std::collections::BTreeMap<u64, String>,
    pub detail_offsets: std::collections::BTreeMap<u64, Point<Pixels>>,
    pub list_scroll: ScrollHandle,
    pub virtual_scroll: UniformListScrollHandle,
    pub detail_tab: usize,
    pub view_groups_open: [bool; 4],
    pub changelog_done_only: bool,
    pub focus_mode: bool,
    pub content_width: f32,
    pub task_grouped: bool,
    pub completion_dismissed: Option<u64>,
    pub version_input: Entity<InputState>,
    #[cfg(debug_assertions)]
    pub styleguide_open: bool,
    pub project_filter: Option<u64>,
    pub workspace: Workspace,
    pub path: PathBuf,
    pub writable: bool,
    pub error: Option<String>,
    pub recovered: bool,
    pub events: Option<futures::channel::mpsc::UnboundedReceiver<system::Event>>,
    pub remind_custom: bool,
    pub update_state: updater::Shared,
    pub update_dismissed: bool,
    pub window_active: bool,
    pub remind_gen: u64,
    pub quitting: bool,
    pub remind_time_input: Entity<InputState>,
    pub notice_failed: bool,
    pub hotkey_capture: bool,
    pub hotkey_focus: FocusHandle,
    pub hotkey_error: Option<&'static str>,
    pub release_version_input: Entity<InputState>,
    pub release_text_input: Entity<TextareaState>,
    pub release_close_open: bool,
    pub release_generated: String,
    pub releases_project: Option<u64>,
    pub releases_expanded: std::collections::BTreeSet<u64>,
    pub history_panel: Option<HistoryPanel>,
    pub history_pending: std::collections::BTreeSet<u64>,
    pub history_gen: std::collections::BTreeMap<u64, u64>,
    pub crashed: bool,
    pub restore_notice: Option<i64>,
    pub changed_since_backup: bool,
    pub backups: Vec<(storage::BackupInfo, usize)>,
    pub dirty: bool,
    pub saved_at: Option<Instant>,
    pub save_gen: u64,
    pub last_edit: Option<Instant>,
    pub applied_look: Option<(bool, model::Accent, model::Density, model::FontScale)>,

    pub view: View,
    pub filter: Filter,
    pub sort: SortKey,
    pub group: GroupBy,
    pub selected_update: Option<u64>,
    pub detail_open: bool,
    pub search_query: String,
    pub editing_task: Option<u64>,
    pub expanded_task: Option<u64>,
    pub hide_done: bool,
    /// Update, bei dem der Nutzer trotz Status "Erledigt" bewusst "Schreiben" gewählt hat.
    pub edit_forced: Option<u64>,
    pub resizing_list: bool,
    pub views_open: bool,

    pub date_pop: Option<DatePop>,
    pub dialog: Option<Dialog>,
    pub import_preview: Option<ImportPreview>,
    pub palette_open: bool,
    pub palette_index: usize,
    pub toasts: Vec<Toast>,
    pub next_toast: u64,
    pub drag_over_task: Option<u64>,

    pub cal_mode: CalMode,
    pub cal_anchor: NaiveDate,
    pub cal_inline: Option<NaiveDate>,
    pub cal_more: Option<NaiveDate>,
    pub cal_unscheduled: bool,

    pub settings_open: bool,
    pub settings_cat: SettingsCat,
    pub settings_saved_at: Option<Instant>,
    pub tpl_edit: Option<Option<usize>>,
    pub splash: Splash,
    pub splash_start: Instant,
    pub today_collapsed: [bool; 5],
    pub completing: Vec<(u64, Option<u64>)>,
    pub done_today: Vec<(u64, Option<u64>)>,
    pub cal_focus: Option<NaiveDate>,
    pub pill_menu: Option<PillMenu>,
    pub resizing_panel: bool,
    pub cal_hint_show: bool,
    pub multi: std::collections::BTreeSet<u64>,
    pub quick_date: Option<NaiveDate>,
    pub home_day: NaiveDate,
    pub quick_bounds: Option<Bounds<Pixels>>,
    pub quick_suggestion: usize,
    pub quick_suggestions_open: bool,
    pub settings_jump: Option<String>,
    pub settings_jump_pending: bool,
    pub settings_bounds: Option<Bounds<Pixels>>,
    pub settings_target_focus: FocusHandle,
    pub settings_scroll: ScrollHandle,
    pub detail_scroll: ScrollHandle,

    pub settings_search: Entity<InputState>,
    pub shortcut_search: Entity<InputState>,
    pub wipe_input: Entity<InputState>,
    pub quick_input: Entity<InputState>,
    pub inbox_quick_input: Entity<InputState>,
    pub panel_search: Entity<InputState>,
    pub tpl_notes_input: Entity<TextareaState>,
    pub tpl_tasks_input: Entity<TextareaState>,
    pub title_input: Entity<TextareaState>,
    pub notes_input: Entity<TextareaState>,
    pub search_input: Entity<InputState>,
    pub task_input: Entity<InputState>,
    pub task_edit_input: Entity<InputState>,
    pub desc_input: Entity<InputState>,
    pub date_input: Entity<InputState>,
    pub palette_input: Entity<InputState>,
    pub cal_input: Entity<InputState>,
    pub template_name_input: Entity<InputState>,
    pub focus_handle: FocusHandle,
    pub overlay_focus: FocusHandle,
    pub _subscriptions: Vec<Subscription>,
}

impl NoteWarden {
    fn new(
        window: &mut Window,
        events: Option<futures::channel::mpsc::UnboundedReceiver<system::Event>>,
        popup_of: Option<PopupInit>,
        cx: &mut Context<Self>,
    ) -> Self {
        let path = storage::data_path();
        let (mut workspace, writable, error, recovered, migrated, restore_notice) = if let Some(
            init,
        ) = &popup_of
        {
            (
                init.workspace.clone(),
                init.writable,
                None,
                false,
                false,
                None,
            )
        } else {
            match storage::load(&path) {
                Ok(loaded) => (
                    loaded.workspace,
                    true,
                    None,
                    loaded.recovered,
                    loaded.migrated,
                    loaded.restored_from,
                ),
                Err(error) => (
                    Workspace::default(),
                    false,
                    Some(format!(
                        "Arbeitsbereich konnte nicht geladen werden: {error}. Änderungen sind gesperrt."
                    )),
                    false,
                    false,
                    None,
                ),
            }
        };
        if popup_of.is_some() {
        } else if !writable {
            workspace.migrate();
        } else {
            let ids = workspace.updates.iter().map(|u| u.id).collect();
            storage::prune_history(&path, &ids);
        }
        if !workspace.settings.remember_window {
            workspace.settings.compact.enabled = false;
        }
        if std::env::args().any(|arg| arg == "--compact") {
            workspace.settings.compact.enabled = true;
        }
        let selected_update = workspace
            .updates
            .iter()
            .rev()
            .find(|u| u.is_live())
            .map(|u| u.id);
        let today = model::today();
        let title_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 100)
                .placeholder("Titel")
        });
        let notes_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(7, 10000)
                .placeholder("Notizen in Markdown …")
        });
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Suchen …"));
        let task_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Aufgabe hinzufügen …"));
        let task_edit_input = cx.new(|cx| InputState::new(window, cx));
        let desc_input = cx.new(|cx| InputState::new(window, cx).placeholder("Beschreibung …"));
        let date_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("z. B. morgen, fr, 14.11., in 2 Wochen")
        });
        let palette_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Suchen oder Befehl eingeben …"));
        let cal_input = cx.new(|cx| InputState::new(window, cx).placeholder("Titel …"));
        let template_name_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Name der Vorlage"));
        let settings_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Einstellungen durchsuchen"));
        let shortcut_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Kürzel suchen …"));
        let wipe_input = cx.new(|cx| InputState::new(window, cx).placeholder("LÖSCHEN"));
        let quick_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Aufgabe hinzufügen …"));
        let inbox_quick_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Schnell erfassen …"));
        let inbox_quick_sub = cx.subscribe_in(
            &inbox_quick_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_capture(this.inbox_quick_input.clone(), window, cx);
                }
            },
        );
        let panel_search = cx.new(|cx| InputState::new(window, cx).placeholder("Suchen …"));
        let tpl_notes_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(6, 14)
                .placeholder("Notiz-Text in Markdown …")
        });
        let tpl_tasks_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(4, 10)
                .placeholder("Eine Aufgabe pro Zeile …")
        });
        let settings_sub = cx.subscribe(&settings_search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let shortcut_sub = cx.subscribe(&shortcut_search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let panel_sub = cx.subscribe(&panel_search, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        });
        let quick_sub = cx.subscribe_in(
            &quick_input,
            window,
            |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } => this.commit_quick_add(window, cx),
                InputEvent::Change => {
                    this.quick_suggestion = 0;
                    this.quick_suggestions_open = true;
                    cx.defer_in(window, |this, window, cx| this.decorate_quick(window, cx));
                    cx.notify();
                }
                InputEvent::Focus | InputEvent::Blur => cx.notify(),
            },
        );

        let title_sub = cx.subscribe(&title_input, |this, input, event: &InputEvent, cx| {
            if this.writable && matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                if let Some(update) = this
                    .selected_update
                    .and_then(|id| this.workspace.update_mut(id))
                    && update.title != value
                {
                    update.title = value;
                    update.touch();
                    this.persist(cx);
                    cx.notify();
                }
            }
        });
        let notes_sub = cx.subscribe(&notes_input, |this, input, event: &InputEvent, cx| {
            match event {
                InputEvent::Focus => {
                    this.notes_focused = true;
                    cx.notify();
                }
                InputEvent::Blur => {
                    this.notes_focused = false;
                    cx.notify();
                }
                _ => {}
            }
            if this.writable && matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                this.set_notes(value, cx);
            }
        });
        let search_sub = cx.subscribe(&search_input, |this, input, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                this.search_gen += 1;
                let generation = this.search_gen;
                cx.spawn(async move |entity, cx| {
                    cx.background_executor()
                        .timer(StdDuration::from_millis(120))
                        .await;
                    entity
                        .update(cx, |a, cx| {
                            if a.search_gen == generation {
                                a.search_query = value;
                                cx.notify();
                            }
                        })
                        .ok();
                })
                .detach();
            }
        });
        let task_sub = cx.subscribe_in(
            &task_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.add_task_from_input(window, cx);
                }
            },
        );
        let task_edit_sub = cx.subscribe_in(
            &task_edit_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.save_task_edit(window, cx);
                }
            },
        );
        let desc_sub = cx.subscribe(&desc_input, |this, input, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change)
                && let (Some(task_id), Some(update_id)) = (this.expanded_task, this.selected_update)
            {
                let value = input.read(cx).value().to_string();
                if let Some(task) = this
                    .workspace
                    .update_mut(update_id)
                    .and_then(|u| u.tasks.iter_mut().find(|t| t.id == task_id))
                    && task.description != value
                {
                    task.description = value;
                    this.persist(cx);
                }
            }
        });
        let date_sub = cx.subscribe_in(
            &date_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.confirm_date_pop(window, cx);
                }
            },
        );
        let palette_sub = cx.subscribe(&palette_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.palette_index = 0;
                cx.notify();
            }
        });
        let cal_sub = cx.subscribe_in(
            &cal_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if let InputEvent::PressEnter { secondary, .. } = event {
                    this.commit_calendar_inline(*secondary, window, cx);
                }
            },
        );
        let quit_sub = cx.on_app_quit(|this, _| {
            let discard: Vec<_> = this
                .drafts
                .iter()
                .filter_map(|(id, sig)| {
                    this.workspace
                        .update(*id)
                        .filter(|u| u.title.trim().is_empty() && Self::draft_signature(u) == *sig)
                        .map(|_| *id)
                })
                .collect();
            if !discard.is_empty() {
                this.workspace.updates.retain(|u| !discard.contains(&u.id));
                this.dirty = true;
            }
            this.snapshot_pending();
            this.flush();
            async {}
        });

        let settings_cat = SettingsCat::ALL[workspace
            .settings
            .last_settings_category
            .min(SettingsCat::ALL.len() - 1)];
        let history = ui::workbench::History::new(&workspace.updates);
        let version_input = cx.new(|cx| InputState::new(window, cx).placeholder("Version"));
        let version_sub = cx.subscribe_in(&version_input, window, |this, input, event, _, cx| {
            if matches!(event, InputEvent::Change) && this.writable {
                let value = input.read(cx).value().to_string();
                if let Some(u) = this
                    .selected_update
                    .and_then(|id| this.workspace.update_mut(id))
                    && u.version != value
                {
                    u.version = value;
                    u.touch();
                    this.persist(cx);
                    cx.notify();
                }
            }
        });
        let remind_time_input = cx.new(|cx| InputState::new(window, cx).placeholder("hh:mm"));
        let remind_sub = cx.subscribe_in(
            &remind_time_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.apply_custom_remind(window, cx);
                }
            },
        );
        let release_version_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("z. B. 1.2.0"));
        let release_text_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(8, 16)
                .placeholder("Changelog")
        });
        let sticky_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(2, 8)
                .placeholder("Notiz …")
        });
        let sticky_sub =
            cx.subscribe(
                &sticky_input,
                |this, input, event: &InputEvent, cx| match event {
                    InputEvent::Change => {
                        let value = input.read(cx).value().to_string();
                        this.set_sticky_text(&value, cx);
                    }
                    InputEvent::Blur => this.finish_sticky_edit(cx),
                    _ => {}
                },
            );
        let tag_input = cx.new(|cx| InputState::new(window, cx).placeholder("+ Tag"));
        let tag_sub = cx.subscribe_in(
            &tag_input,
            window,
            |this, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.commit_tag_input(window, cx);
                }
            },
        );
        let mut app = Self {
            tag_input,
            sticky_input,
            editing_sticky: None,
            sticky_tag: None,
            compact_list: true,
            compact_preview: false,
            popup: popup_of.is_some(),
            peer: popup_of.as_ref().map(|i| (i.main.clone(), i.main_window)),
            popup_win: None,
            peer_dirty: false,
            keyboard_menu: None,
            keyboard_index: 0,
            search_gen: 0,
            notes_focused: false,
            slash_index: 0,
            history,
            drafts: Default::default(),
            detail_offsets: Default::default(),
            list_scroll: ScrollHandle::new(),
            virtual_scroll: UniformListScrollHandle::new(),
            detail_tab: 0,
            view_groups_open: [false; 4],
            changelog_done_only: true,
            focus_mode: false,
            content_width: 0.,
            task_grouped: false,
            completion_dismissed: None,
            version_input,
            workspace,
            path,
            writable,
            error,
            recovered,
            events,
            remind_custom: false,
            update_state: Default::default(),
            update_dismissed: false,
            window_active: true,
            remind_gen: 0,
            quitting: false,
            remind_time_input,
            notice_failed: false,
            hotkey_capture: false,
            hotkey_focus: cx.focus_handle(),
            hotkey_error: None,
            release_version_input,
            release_text_input,
            release_close_open: false,
            release_generated: String::new(),
            releases_project: None,
            releases_expanded: Default::default(),
            history_panel: None,
            history_pending: Default::default(),
            history_gen: Default::default(),
            crashed: false,
            restore_notice,
            changed_since_backup: false,
            backups: Vec::new(),
            dirty: false,
            saved_at: None,
            save_gen: 0,
            last_edit: None,
            applied_look: None,
            settings_open: false,
            settings_cat,
            settings_saved_at: None,
            tpl_edit: None,
            splash: if popup_of.is_some() {
                Splash::Gone
            } else {
                Splash::Showing
            },
            splash_start: Instant::now(),
            #[cfg(debug_assertions)]
            styleguide_open: false,
            today_collapsed: [false, false, false, false, true],
            completing: Vec::new(),
            done_today: Vec::new(),
            cal_focus: None,
            pill_menu: None,
            resizing_panel: false,
            cal_hint_show: false,
            multi: Default::default(),
            project_filter: None,
            quick_date: None,
            home_day: today,
            quick_bounds: None,
            quick_suggestion: 0,
            quick_suggestions_open: true,
            settings_jump: None,
            settings_jump_pending: false,
            settings_bounds: None,
            settings_target_focus: cx.focus_handle(),
            settings_scroll: ScrollHandle::new(),
            detail_scroll: ScrollHandle::new(),
            settings_search,
            shortcut_search,
            wipe_input,
            quick_input,
            inbox_quick_input,
            panel_search,
            tpl_notes_input,
            tpl_tasks_input,
            view: View::Today,
            filter: Filter::All,
            sort: SortKey::Edited,
            group: GroupBy::None,
            selected_update,
            detail_open: false,
            search_query: String::new(),
            editing_task: None,
            expanded_task: None,
            hide_done: false,
            edit_forced: None,
            resizing_list: false,
            views_open: true,
            date_pop: None,
            dialog: None,
            import_preview: None,
            palette_open: false,
            palette_index: 0,
            toasts: Vec::new(),
            next_toast: 1,
            drag_over_task: None,
            cal_mode: CalMode::Month,
            cal_anchor: today,
            cal_inline: None,
            cal_more: None,
            cal_unscheduled: true,
            title_input,
            notes_input,
            search_input,
            task_input,
            task_edit_input,
            desc_input,
            date_input,
            palette_input,
            cal_input,
            template_name_input,
            focus_handle: cx.focus_handle(),
            overlay_focus: cx.focus_handle(),
            _subscriptions: vec![
                title_sub,
                notes_sub,
                search_sub,
                task_sub,
                task_edit_sub,
                desc_sub,
                date_sub,
                palette_sub,
                cal_sub,
                quit_sub,
                settings_sub,
                shortcut_sub,
                panel_sub,
                quick_sub,
                inbox_quick_sub,
            ],
        };
        if migrated {
            app.dirty = true;
            app.flush();
        }
        app.apply_startup_settings();
        if app.popup {
            app.start_popup(window, cx);
        } else {
            app.start_backups(cx);
            app.start_system(window, cx);
            app.start_splash(cx);
            let reopen = app.workspace.settings.compact.enabled
                || std::env::args().any(|arg| arg == "--compact");
            if reopen {
                cx.defer_in(window, |this, window, cx| this.open_popup(window, cx));
            }
        }
        app._subscriptions.push(sticky_sub);
        app._subscriptions.push(tag_sub);
        app._subscriptions.push(version_sub);
        app._subscriptions.push(remind_sub);
        app.sort = app.workspace.settings.updates_ui.sort;
        app.group = app.workspace.settings.updates_ui.group;
        app.filter = app.workspace.settings.updates_ui.filter;
        app.project_filter = app.workspace.settings.updates_ui.project;
        app.sync_fields(window, cx);
        app.focus_handle.focus(window, cx);
        app
    }

    /// Leaves a view whose sidebar entry was just turned off.
    fn ensure_view_visible(&mut self) {
        let st = &self.workspace.settings;
        let hidden = match self.view {
            View::Inbox => st.hide_inbox,
            View::Board => st.hide_board,
            View::Stickies => st.hide_stickies,
            View::Releases => st.hide_releases,
            View::Calendar => st.hide_calendar,
            View::Today | View::Updates => false,
        };
        if hidden {
            self.view = View::Today;
            self.detail_open = false;
        }
    }

    /// Startansicht, Kalender-Standard und gemerkte Layout-Werte aus den Einstellungen.
    fn apply_startup_settings(&mut self) {
        let st = &self.workspace.settings;
        self.cal_mode = match st.cal_view {
            model::CalView::Month => CalMode::Month,
            model::CalView::Week => CalMode::Week,
        };
        self.cal_unscheduled = st.cal_panel_open;
        self.views_open = st.status_group_open;
        match st.start_view {
            model::StartView::Today => {}
            model::StartView::Updates => self.view = View::Updates,
            model::StartView::Calendar => self.view = View::Calendar,
            model::StartView::Last => {
                if let Some(id) = st
                    .last_update
                    .filter(|id| self.workspace.update(*id).is_some_and(|u| u.is_live()))
                {
                    self.selected_update = Some(id);
                    self.view = View::Updates;
                    self.detail_open = true;
                }
            }
        }
        self.ensure_view_visible();
    }

    /// Tray-Verhalten, Hotkey, Ereigniskanal und Erinnerungen.
    fn start_system(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let st = &self.workspace.settings;
        let tray_mode = st.tray_on_close && st.reminders_on;
        window.on_window_should_close(cx, {
            let entity = cx.entity();
            move |_, cx| {
                let hide = entity.update(cx, |app, _| {
                    let st = &app.workspace.settings;
                    let hide = st.tray_on_close && st.reminders_on && !app.quitting;
                    if hide && !st.tray_hint_shown {
                        app.workspace.settings.tray_hint_shown = true;
                        app.dirty = true;
                        app.flush();
                        system::notify(
                            "Note Warden",
                            "Damit Erinnerungen ankommen, läuft Note Warden im Tray weiter.",
                            None,
                        );
                    }
                    hide
                });
                if hide {
                    system::hide_main_window();
                } else if entity.read(cx).popup_win.is_some() {
                    // The popup alone must not keep the app alive without its main window.
                    cx.quit();
                }
                !hide
            }
        });
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            this.window_active = window.is_window_active();
            if this.window_active {
                this.schedule_reminders(cx);
            }
        });
        self._subscriptions.push(activation);
        if let Some(mut events) = self.events.take() {
            cx.spawn_in(window, async move |this, cx| {
                use futures::StreamExt as _;
                while let Some(event) = events.next().await {
                    if this
                        .update_in(cx, |this, window, cx| {
                            this.handle_system_event(event, window, cx)
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        self.window_active =
            window.is_window_active() && !std::env::args().any(|arg| arg == "--minimized");
        let hotkey = self.workspace.settings.hotkey.clone();
        match system::set_hotkey(&hotkey) {
            Ok(()) => {}
            Err(system::HotkeyError::Taken) => {
                self.hotkey_error = Some("Dieses Kürzel ist bereits belegt.")
            }
            Err(system::HotkeyError::Invalid) => {
                self.hotkey_error = Some("Dieses Kürzel ist ungültig.")
            }
        }
        if self.workspace.settings.autostart != system::autostart_enabled() {
            let _ = system::set_autostart(self.workspace.settings.autostart);
        }
        if tray_mode && std::env::args().any(|arg| arg == "--minimized") {
            cx.spawn(async move |_, cx| {
                cx.background_executor()
                    .timer(StdDuration::from_millis(400))
                    .await;
                system::hide_main_window();
            })
            .detach();
        }
        self.schedule_reminders(cx);
        let last = self.workspace.settings.last_update_check.unwrap_or(0);
        if updater::configured() && model::now_millis() - last > 24 * 3_600_000 {
            self.check_updates(false, cx);
        }
        let seen = self.workspace.settings.seen_version.clone();
        if seen.as_deref() != Some(VERSION) {
            let upgraded = seen.is_some() || !self.workspace.updates.is_empty();
            self.workspace.settings.seen_version = Some(VERSION.to_owned());
            self.dirty = true;
            if upgraded {
                self.dialog = Some(Dialog::AppChangelog);
            }
        }
    }

    /// Update-Prüfung im Hintergrund; `manual` zeigt auch "aktuell" und Fehler als Hinweis.
    pub fn check_updates(&mut self, manual: bool, cx: &mut Context<Self>) {
        if !updater::configured() {
            if manual {
                self.toast(
                    if storage::is_portable() {
                        "Portabler Modus: neue Version bitte manuell herunterladen"
                    } else {
                        "Automatische Updates sind nicht eingerichtet"
                    },
                    None,
                    cx,
                );
            }
            return;
        }
        self.update_dismissed = false;
        self.workspace.settings.last_update_check = Some(model::now_millis());
        self.dirty = true;
        updater::check(&self.update_state);
        cx.notify();
    }

    /// Sichert, startet den geprüften Installer und beendet die App.
    pub fn install_update(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.snapshot_pending();
        if !self.backup_now(false) {
            self.toast("Update abgebrochen: Backup nicht möglich", None, cx);
            return;
        }
        if updater::launch_installer(&path) {
            log::info("Installer gestartet");
            self.quitting = true;
            cx.quit();
        } else {
            self.toast("Installer konnte nicht gestartet werden", None, cx);
        }
    }

    /// Backups: beim Start (wenn das letzte älter als 1 Stunde ist), danach alle 30 Minuten bei Änderungen.
    fn start_backups(&mut self, cx: &mut Context<Self>) {
        let cutoff = self.now_today() - Duration::days(6);
        self.workspace
            .settings
            .home_completions
            .retain(|(_, _, date)| *date >= cutoff);
        if !self.writable {
            return;
        }
        let latest = storage::list_backups(&self.path).first().map(|b| b.at);
        if latest.is_none_or(|at| model::now_millis() - at > 3_600_000) {
            self.backup_now(false);
        } else {
            self.refresh_backups();
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(StdDuration::from_secs(30 * 60))
                    .await;
                if this
                    .update(cx, |this, _| {
                        if this.changed_since_backup {
                            this.backup_now(false);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Sichert den aktuellen Stand; `pinned` = manuell, wird nie automatisch gelöscht.
    pub fn backup_now(&mut self, pinned: bool) -> bool {
        if !self.writable {
            return false;
        }
        self.flush();
        match storage::create_backup(&self.path, &self.workspace, pinned) {
            Ok(_) => {
                self.changed_since_backup = false;
                self.refresh_backups();
                true
            }
            Err(error) => {
                log::error(&format!("Backup fehlgeschlagen: {error}"));
                self.error = Some(format!("Backup fehlgeschlagen: {error}"));
                false
            }
        }
    }

    /// Liste der Backups samt Anzahl Updates neu einlesen (nur für die Einstellungen).
    pub fn refresh_backups(&mut self) {
        self.backups = storage::list_backups(&self.path)
            .into_iter()
            .map(|info| {
                let count = storage::read_backup_file(&info.path)
                    .map(|w| w.updates.iter().filter(|u| u.deleted_on.is_none()).count())
                    .unwrap_or(0);
                (info, count)
            })
            .collect();
    }

    /// Splash: mindestens `SPLASH_MIN_MS` sichtbar, danach Fade-out. Bei Ladefehler bleibt er stehen.
    fn start_splash(&mut self, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(StdDuration::from_millis(SPLASH_MIN_MS))
                .await;
            this.update(cx, |this, cx| this.dismiss_splash(cx)).ok();
        })
        .detach();
    }

    pub fn dismiss_splash(&mut self, cx: &mut Context<Self>) {
        if self.splash != Splash::Showing {
            return;
        }
        if self.workspace.settings.motion == model::Motion::Reduced {
            self.splash = Splash::Gone;
            cx.notify();
            return;
        }
        self.splash = Splash::Fading(Instant::now());
        cx.spawn(async move |this, cx| {
            for _ in 0..16 {
                cx.background_executor()
                    .timer(StdDuration::from_millis(SPLASH_FADE_MS / 12))
                    .await;
                let done = this
                    .update(cx, |this, cx| {
                        let over = matches!(this.splash, Splash::Fading(t)
                            if t.elapsed() >= StdDuration::from_millis(SPLASH_FADE_MS));
                        if over {
                            this.splash = Splash::Gone;
                        }
                        cx.notify();
                        over
                    })
                    .unwrap_or(true);
                if done {
                    break;
                }
            }
            this.update(cx, |this, cx| {
                this.splash = Splash::Gone;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // ---------- Speichern ----------

    /// Vormerken; geschrieben wird nach kurzer Ruhe (Debounce ~400 ms).
    pub fn persist(&mut self, cx: &mut Context<Self>) {
        self.peer_dirty = true;
        self.persist_local(cx);
    }

    /// Schedules the autosave without telling the other window (used when adopting its data).
    fn persist_local(&mut self, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        self.record_history();
        self.dirty = true;
        self.changed_since_backup = true;
        self.last_edit = Some(Instant::now());
        self.save_gen += 1;
        let generation = self.save_gen;
        let delay = self.workspace.settings.autosave_ms.clamp(100, 5000) as u64;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(StdDuration::from_millis(delay))
                .await;
            this.update(cx, |this, cx| {
                if this.save_gen == generation {
                    this.flush();
                    this.sync_peer(cx);
                    cx.notify();
                }
            })
            .ok();
            // Nach 2 Sekunden schrumpft "Gespeichert" auf das dezente Check-Icon.
            cx.background_executor()
                .timer(StdDuration::from_millis(2100))
                .await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
    }

    pub fn flush(&mut self) {
        if self.popup {
            // Only the main window owns the file; the popup hands its data over instead.
            self.dirty = false;
            self.saved_at = Some(Instant::now());
            return;
        }
        if self.writable && self.dirty {
            match storage::save(&self.path, &self.workspace) {
                Ok(()) => {
                    storage::write_prefs(
                        &self.path,
                        &storage::Prefs::from(&self.workspace.settings),
                    );
                    self.error = None;
                    self.recovered = false;
                    self.dirty = false;
                    self.saved_at = Some(Instant::now());
                }
                Err(error) => {
                    self.error = Some(format!("Speichern fehlgeschlagen: {error}"));
                }
            }
        }
    }

    pub fn retry_save(&mut self, cx: &mut Context<Self>) {
        self.dirty = true;
        self.flush();
        cx.notify();
    }

    // ---------- Felder ----------

    pub fn sync_fields(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let update = self
            .selected_update
            .and_then(|id| self.workspace.update(id));
        let title = update.map(|u| u.title.clone()).unwrap_or_default();
        let notes = update.map(|u| u.notes.clone()).unwrap_or_default();
        let version = update.map(|u| u.version.clone()).unwrap_or_default();
        self.version_input
            .update(cx, |i, cx| i.set_value(version, window, cx));
        self.title_input
            .update(cx, |input, cx| input.set_value(title, window, cx));
        self.notes_input
            .update(cx, |input, cx| input.set_value(notes, window, cx));
    }

    pub fn set_notes(&mut self, value: String, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self
            .selected_update
            .and_then(|id| self.workspace.update_mut(id))
            && update.notes != value
        {
            let id = update.id;
            let previous = std::mem::replace(&mut update.notes, value);
            update.touch();
            self.note_edited(id, &previous, cx);
            self.persist(cx);
            cx.notify();
        }
    }

    pub fn select_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if id == 0 {
            self.set_view(View::Inbox, cx);
            return;
        }
        self.snapshot_pending();
        self.discard_untouched_drafts(Some(id), cx);
        if let Some(previous) = self.selected_update {
            self.detail_offsets
                .insert(previous, self.detail_scroll.offset());
        }
        self.detail_scroll.set_offset(
            self.detail_offsets
                .get(&id)
                .copied()
                .unwrap_or(point(px(0.), px(0.))),
        );
        self.detail_tab = 0;
        if self.workspace.update(id).is_some() {
            if self.popup {
                self.compact_list = false;
            }
            self.selected_update = Some(id);
            if self.workspace.settings.last_update != Some(id) {
                self.workspace.settings.last_update = Some(id);
                self.persist(cx);
            }
            self.detail_open = true;
            self.editing_task = None;
            self.expanded_task = None;
            self.date_pop = None;
            self.dialog = None;
            if self.view != View::Updates {
                self.view = View::Updates;
            }
            self.sync_fields(window, cx);
            cx.notify();
        }
    }

    pub fn selected(&self) -> Option<&model::Update> {
        self.selected_update
            .and_then(|id| self.workspace.update(id))
    }

    pub fn set_view(&mut self, view: View, cx: &mut Context<Self>) {
        self.snapshot_pending();
        self.discard_untouched_drafts(None, cx);
        self.view = view;
        self.detail_open = false;
        self.date_pop = None;
        if view == View::Updates && matches!(self.filter, Filter::Archive | Filter::Trash) {
            self.filter = Filter::All;
        }
        cx.notify();
    }

    pub fn set_filter(&mut self, filter: Filter, cx: &mut Context<Self>) {
        self.view = View::Updates;
        self.filter = filter;
        self.save_updates_ui(cx);
        self.detail_open = false;
        cx.notify();
    }

    // ---------- Updates ----------

    pub fn create_update(
        &mut self,
        date: Option<NaiveDate>,
        template: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.writable {
            return;
        }
        if self.popup {
            self.create_compact_note(window, cx);
            return;
        }
        let template = template
            .or(self.workspace.settings.default_template)
            .and_then(|i| self.workspace.settings.templates.get(i).cloned());
        self.discard_untouched_drafts(None, cx);
        let id = self.workspace.new_update_from(date, template.as_ref());
        if template.is_none() {
            self.drafts.insert(
                id,
                Self::draft_signature(self.workspace.update(id).unwrap()),
            );
        }
        self.detail_tab = 0;
        self.selected_update = Some(id);
        self.detail_open = true;
        self.view = View::Updates;
        if matches!(self.filter, Filter::Archive | Filter::Trash) {
            self.filter = Filter::All;
        }
        self.search_query.clear();
        self.search_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.sync_fields(window, cx);
        self.persist(cx);
        self.title_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub fn set_status(&mut self, id: u64, status: Status, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self.workspace.update_mut(id) {
            if update.status == status {
                return;
            }
            let previous = update.status;
            update.status = status;
            update.touch();
            self.persist(cx);
            self.toast("Status geändert", Some(Undo::Status(id, previous)), cx);
            cx.notify();
        }
    }

    pub fn duplicate_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(copy) = self.workspace.duplicate(id) {
            self.persist(cx);
            self.select_update(copy, window, cx);
            self.toast("Update dupliziert", None, cx);
        }
    }

    pub fn trash_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self.workspace.update_mut(id) {
            update.deleted_on = Some(model::today());
            self.after_removed(id, window, cx);
            self.toast(
                "Update in den Papierkorb verschoben",
                Some(Undo::Trashed(id)),
                cx,
            );
        }
    }

    pub fn archive_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self.workspace.update_mut(id) {
            update.archived = true;
            self.after_removed(id, window, cx);
            self.toast("Update archiviert", Some(Undo::Archived(id)), cx);
        }
    }

    pub fn restore_update(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(update) = self.workspace.update_mut(id) {
            update.archived = false;
            update.deleted_on = None;
            self.persist(cx);
            self.toast("Update wiederhergestellt", None, cx);
        }
    }

    pub fn purge_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace.updates.retain(|u| u.id != id);
        self.dialog = None;
        self.after_removed(id, window, cx);
    }

    fn after_removed(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_update == Some(id) {
            self.selected_update = self
                .workspace
                .updates
                .iter()
                .rev()
                .find(|u| u.is_live())
                .map(|u| u.id);
            self.detail_open = false;
            self.sync_fields(window, cx);
        }
        self.persist(cx);
        cx.notify();
    }

    // ---------- Aufgaben ----------

    pub fn add_task_from_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let raw = self.task_input.read(cx).value().to_string();
        self.add_tasks(&raw, cx);
        self.task_input
            .update(cx, |i, cx| i.set_value("", window, cx));
    }

    pub fn add_tasks(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(id) = self.selected_update {
            for line in text.lines() {
                let line = line
                    .trim()
                    .trim_start_matches(['-', '*', '•'])
                    .trim_start_matches("[ ]")
                    .trim();
                let (kind, clean) = quick::task_kind(line);
                let (clean, tags) = quick::split_tags(&clean);
                let parsed =
                    quick::parse_quick_input(&clean, model::today(), &self.workspace.projects);
                if let Some(tid) = self.workspace.new_task(id, parsed.title, parsed.date)
                    && let Some(u) = self.workspace.update_mut(id)
                    && let Some(t) = u.tasks.iter_mut().find(|t| t.id == tid)
                {
                    t.kind = kind;
                    t.priority = parsed.priority;
                    t.tags = tags;
                }
            }
            self.persist(cx);
            cx.notify();
        }
    }

    pub fn task_mut(&mut self, uid: u64, tid: u64) -> Option<&mut Task> {
        self.workspace
            .update_mut(uid)?
            .tasks
            .iter_mut()
            .find(|t| t.id == tid)
    }

    pub fn toggle_task(&mut self, update_id: u64, task_id: u64, cx: &mut Context<Self>) {
        if update_id == 0 {
            self.toggle_inbox(task_id, cx);
            return;
        }
        if !self.writable {
            return;
        }
        if let Some(task) = self.task_mut(update_id, task_id) {
            task.done = !task.done;
            let done = task.done;
            self.record_completion(update_id, Some(task_id), done);
            if let Some(update) = self.workspace.update_mut(update_id) {
                update.touch();
            }
            self.persist(cx);
            cx.notify();
        }
    }

    pub fn start_task_edit(&mut self, task_id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(title) = self
            .selected()
            .and_then(|u| u.tasks.iter().find(|t| t.id == task_id))
            .map(|t| t.title.clone())
        else {
            return;
        };
        self.editing_task = Some(task_id);
        self.task_edit_input.update(cx, |input, cx| {
            input.set_value(title, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub fn save_task_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task_id) = self.editing_task else {
            return;
        };
        let title = self.task_edit_input.read(cx).value().trim().to_owned();
        if title.is_empty() || !self.writable {
            return;
        }
        if let Some(update_id) = self.selected_update
            && let Some(task) = self.task_mut(update_id, task_id)
        {
            task.title = title;
            self.persist(cx);
        }
        self.editing_task = None;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn toggle_description(
        &mut self,
        task_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.expanded_task == Some(task_id) {
            self.expanded_task = None;
        } else {
            let text = self
                .selected()
                .and_then(|u| u.tasks.iter().find(|t| t.id == task_id))
                .map(|t| t.description.clone())
                .unwrap_or_default();
            self.expanded_task = Some(task_id);
            self.desc_input.update(cx, |input, cx| {
                input.set_value(text, window, cx);
                input.focus(window, cx);
            });
        }
        cx.notify();
    }

    pub fn set_task_kind(
        &mut self,
        update_id: u64,
        task_id: u64,
        kind: ChangeKind,
        cx: &mut Context<Self>,
    ) {
        if let Some(task) = self.task_mut(update_id, task_id) {
            task.kind = kind;
            self.persist(cx);
            cx.notify();
        }
    }

    /// Entfernt eine Aufgabe samt Unteraufgaben (mit Undo-Toast).
    pub fn delete_task(&mut self, update_id: u64, task_id: u64, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let Some(update) = self.workspace.update_mut(update_id) else {
            return;
        };
        let Some(index) = update.tasks.iter().position(|t| t.id == task_id) else {
            return;
        };
        let mut end = index + 1;
        if !update.tasks[index].sub {
            while end < update.tasks.len() && update.tasks[end].sub {
                end += 1;
            }
        }
        let tasks: Vec<Task> = update.tasks.drain(index..end).collect();
        update.touch();
        self.persist(cx);
        self.toast(
            "Aufgabe gelöscht",
            Some(Undo::Tasks {
                update_id,
                index,
                tasks,
            }),
            cx,
        );
    }

    pub fn indent_task(&mut self, update_id: u64, task_id: u64, sub: bool, cx: &mut Context<Self>) {
        if let Some(update) = self.workspace.update_mut(update_id)
            && let Some(index) = update.tasks.iter().position(|t| t.id == task_id)
            && (index > 0 || !sub)
        {
            // Hauptaufgaben mit eigenen Unteraufgaben werden nicht eingerückt (nur eine Ebene).
            let has_children = update.tasks.get(index + 1).is_some_and(|t| t.sub);
            if sub && has_children && !update.tasks[index].sub {
                return;
            }
            update.tasks[index].sub = sub;
            self.persist(cx);
            cx.notify();
        }
    }

    /// Verschiebt eine Aufgabe (bei Hauptaufgaben samt Unteraufgaben) vor `before`
    /// bzw. ans Ende.
    pub fn move_task(
        &mut self,
        update_id: u64,
        task_id: u64,
        before: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        let Some(update) = self.workspace.update_mut(update_id) else {
            return;
        };
        if Some(task_id) == before {
            return;
        }
        let Some(from) = update.tasks.iter().position(|t| t.id == task_id) else {
            return;
        };
        let mut end = from + 1;
        if !update.tasks[from].sub {
            while end < update.tasks.len() && update.tasks[end].sub {
                end += 1;
            }
        }
        let block: Vec<Task> = update.tasks.drain(from..end).collect();
        let mut at = match before {
            Some(b) => match update.tasks.iter().position(|t| t.id == b) {
                Some(i) => i,
                None => {
                    update.tasks.splice(from..from, block);
                    return;
                }
            },
            None => update.tasks.len(),
        };
        if !block[0].sub {
            while at < update.tasks.len() && update.tasks[at].sub {
                at += 1;
            }
        }
        // Ein Block, der vor eine Unteraufgabe fällt, rutscht hinter deren Gruppe (oben).
        let first_becomes_main = at == 0;
        update.tasks.splice(at..at, block);
        if first_becomes_main {
            update.tasks[0].sub = false;
        }
        update.touch();
        self.persist(cx);
        cx.notify();
    }

    /// Alt+↑/↓: Aufgabe auf ihrer Ebene um eine Position verschieben.
    pub fn nudge_task(&mut self, update_id: u64, task_id: u64, up: bool, cx: &mut Context<Self>) {
        let Some(update) = self.workspace.update(update_id) else {
            return;
        };
        let tasks = &update.tasks;
        let Some(i) = tasks.iter().position(|t| t.id == task_id) else {
            return;
        };
        let level = tasks[i].sub;
        let same: Vec<usize> = (0..tasks.len())
            .filter(|&k| tasks[k].sub == level)
            .collect();
        let Some(pos) = same.iter().position(|&k| k == i) else {
            return;
        };
        if up {
            if pos == 0 {
                return;
            }
            let target = tasks[same[pos - 1]].id;
            self.move_task(update_id, task_id, Some(target), cx);
        } else {
            if pos + 1 >= same.len() {
                return;
            }
            // vor das übernächste Element gleicher Ebene bzw. ans Ende
            let before = same.get(pos + 2).map(|&k| tasks[k].id);
            self.move_task(update_id, task_id, before, cx);
        }
    }

    // ---------- Datum ----------

    pub fn open_date_pop(
        &mut self,
        target: DateTarget,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let base = self.target_date(target).unwrap_or_else(model::today);
        self.date_pop = Some(DatePop {
            target,
            anchor,
            month: model::month_start(base),
            cursor: base,
        });
        self.remind_custom = false;
        self.date_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.date_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub fn target_date(&self, target: DateTarget) -> Option<NaiveDate> {
        match target {
            DateTarget::Quick => self.quick_date,
            DateTarget::Inbox(id) => self
                .workspace
                .standalone_tasks
                .iter()
                .find(|t| t.id == id)
                .and_then(|t| t.date),
            DateTarget::Update(id) => self.workspace.update(id).and_then(|u| u.planned_for),
            DateTarget::Task(0, tid) => self
                .workspace
                .standalone_tasks
                .iter()
                .find(|t| t.id == tid)
                .and_then(|t| t.date),
            DateTarget::Task(uid, tid) => self
                .workspace
                .update(uid)
                .and_then(|u| u.tasks.iter().find(|t| t.id == tid))
                .and_then(|t| t.planned_for),
        }
    }

    pub fn set_date(
        &mut self,
        target: DateTarget,
        date: Option<NaiveDate>,
        cx: &mut Context<Self>,
    ) {
        if !self.writable {
            return;
        }
        match target {
            DateTarget::Inbox(id) => {
                if let Some(t) = self
                    .workspace
                    .standalone_tasks
                    .iter_mut()
                    .find(|t| t.id == id)
                {
                    t.date = date;
                }
            }
            DateTarget::Quick => {
                self.quick_date = date;
                cx.notify();
                return;
            }
            DateTarget::Update(id) => {
                if let Some(update) = self.workspace.update_mut(id) {
                    update.planned_for = date;
                    if date.is_some() && update.status == Status::Idee {
                        update.status = Status::Geplant;
                    }
                    update.touch();
                }
            }
            DateTarget::Task(0, tid) => {
                if let Some(t) = self
                    .workspace
                    .standalone_tasks
                    .iter_mut()
                    .find(|t| t.id == tid)
                {
                    t.date = date;
                }
            }
            DateTarget::Task(uid, tid) => {
                if let Some(task) = self.task_mut(uid, tid) {
                    task.planned_for = date;
                }
                if let Some(update) = self.workspace.update_mut(uid) {
                    update.touch();
                }
            }
        }
        self.persist(cx);
        cx.notify();
    }

    pub fn choose_date(
        &mut self,
        date: Option<NaiveDate>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(pop) = self.date_pop.take() {
            self.set_date(pop.target, date, cx);
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn confirm_date_pop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.date_input.read(cx).value().to_string();
        let Some(pop) = &self.date_pop else { return };
        if text.trim().is_empty() {
            let cursor = pop.cursor;
            self.choose_date(Some(cursor), window, cx);
        } else if let Some(date) = dates::parse(&text, model::today()) {
            self.choose_date(Some(date), window, cx);
        } else {
            self.toast("Datum nicht erkannt", None, cx);
        }
    }

    pub fn move_date_cursor(&mut self, days: i64, cx: &mut Context<Self>) {
        if let Some(pop) = &mut self.date_pop {
            pop.cursor += Duration::days(days);
            pop.month = model::month_start(pop.cursor);
            cx.notify();
        }
    }

    // ---------- Toasts ----------

    pub fn toast(
        &mut self,
        message: impl Into<String>,
        undo: Option<Undo>,
        cx: &mut Context<Self>,
    ) {
        let id = self.next_toast;
        self.next_toast += 1;
        if self.toasts.len() >= 3 {
            self.toasts.remove(0);
        }
        self.toasts.push(Toast {
            id,
            message: message.into(),
            undo,
            created: Instant::now(),
            elapsed_ms: 0,
            paused: false,
            actions: Vec::new(),
        });
        cx.spawn(async move |this, cx| {
            const TICK_MS: u64 = 100;
            loop {
                cx.background_executor()
                    .timer(StdDuration::from_millis(TICK_MS))
                    .await;
                let alive = this
                    .update(cx, |this, cx| {
                        let Some(toast) = this.toasts.iter_mut().find(|t| t.id == id) else {
                            return false;
                        };
                        if !toast.paused {
                            toast.elapsed_ms += TICK_MS;
                        }
                        if toast.elapsed_ms >= palette::TOAST_MS && toast.actions.is_empty() {
                            this.toasts.retain(|t| t.id != id);
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !alive {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub fn toast_with_actions(
        &mut self,
        message: String,
        actions: Vec<(String, ToastAction)>,
        cx: &mut Context<Self>,
    ) {
        self.toast(message, None, cx);
        if let Some(toast) = self.toasts.last_mut() {
            toast.actions = actions;
        }
    }

    pub fn pause_toast(&mut self, id: u64, paused: bool, cx: &mut Context<Self>) {
        if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
            toast.paused = paused;
            cx.notify();
        }
    }

    pub fn dismiss_toast(&mut self, id: u64, cx: &mut Context<Self>) {
        self.toasts.retain(|t| t.id != id);
        cx.notify();
    }

    pub fn run_undo(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pos) = self.toasts.iter().position(|t| t.id == id) else {
            return;
        };
        let toast = self.toasts.remove(pos);
        match toast.undo {
            Some(Undo::Completed(uid, tid, previous)) => {
                self.reopen_item(uid, tid, cx);
                if let Some(status) = previous
                    && let Some(u) = self.workspace.update_mut(uid)
                {
                    u.status = status;
                    u.touch();
                }
            }
            Some(Undo::Status(id, status)) => {
                if let Some(u) = self.workspace.update_mut(id) {
                    u.status = status;
                    u.touch();
                }
            }
            Some(Undo::Tasks {
                update_id,
                index,
                tasks,
            }) => {
                if let Some(update) = self.workspace.update_mut(update_id) {
                    let at = index.min(update.tasks.len());
                    update.tasks.splice(at..at, tasks);
                }
            }
            Some(Undo::Trashed(uid)) | Some(Undo::Archived(uid)) => {
                if let Some(update) = self.workspace.update_mut(uid) {
                    update.deleted_on = None;
                    update.archived = false;
                }
                if self.selected_update.is_none() {
                    self.selected_update = Some(uid);
                    self.sync_fields(window, cx);
                }
            }
            Some(Undo::Sticky(index, sticky)) => {
                let at = index.min(self.workspace.stickies.len());
                self.workspace.stickies.insert(at, sticky);
            }
            Some(Undo::Update(previous)) => {
                if let Some(update) = self
                    .workspace
                    .updates
                    .iter_mut()
                    .find(|u| u.id == previous.id)
                {
                    *update = *previous;
                }
                self.sync_fields(window, cx);
            }
            Some(Undo::Notes(uid, text)) => {
                self.snapshot_now(uid, true);
                if let Some(update) = self.workspace.update_mut(uid) {
                    update.notes = text;
                    update.touch();
                }
                self.sync_fields(window, cx);
            }
            None => {}
        }
        self.persist(cx);
        cx.notify();
    }

    // ---------- Versionsverlauf ----------

    fn snapshot_now(&mut self, id: u64, force: bool) -> storage::Snapshotted {
        let Some(text) = self.workspace.update(id).map(|u| u.notes.clone()) else {
            return storage::Snapshotted::Unchanged;
        };
        let result = storage::add_snapshot(&self.path, id, &text, model::now_millis(), force);
        if result != storage::Snapshotted::TooSoon {
            self.history_pending.remove(&id);
        }
        result
    }

    /// Alle Updates mit ungesicherten Textänderungen sofort festhalten (Wechsel, Beenden).
    pub fn snapshot_pending(&mut self) {
        if !self.writable {
            return;
        }
        for id in std::mem::take(&mut self.history_pending) {
            self.snapshot_now(id, true);
        }
    }

    /// Nach einer Textänderung: Ausgangstext sichern (erste Änderung) und nach 60 s Ruhe einen Snapshot anlegen.
    fn note_edited(&mut self, id: u64, previous: &str, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if self.history_pending.insert(id) {
            storage::add_snapshot(&self.path, id, previous, model::now_millis(), true);
        }
        let generation = self.history_gen.entry(id).or_insert(0);
        *generation += 1;
        let generation = *generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(StdDuration::from_secs(60))
                    .await;
                let again = this
                    .update(cx, |this, _| {
                        if this.history_gen.get(&id) != Some(&generation)
                            || !this.history_pending.contains(&id)
                        {
                            return false;
                        }
                        this.snapshot_now(id, false) == storage::Snapshotted::TooSoon
                    })
                    .unwrap_or(false);
                if !again {
                    break;
                }
            }
        })
        .detach();
    }

    pub fn open_history(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.snapshot_pending();
        let snapshots = storage::read_history(&self.path, id);
        let selected = snapshots.len().saturating_sub(1);
        self.history_panel = Some(HistoryPanel {
            update_id: id,
            snapshots,
            selected,
        });
        self.date_pop = None;
        self.overlay_focus.focus(window, cx);
        cx.notify();
    }

    pub fn restore_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = &self.history_panel else {
            return;
        };
        let Some(snapshot) = panel.snapshots.get(panel.selected).cloned() else {
            return;
        };
        let id = panel.update_id;
        let Some(previous) = self.workspace.update(id).map(|u| u.notes.clone()) else {
            return;
        };
        if previous == snapshot.text {
            return;
        }
        self.snapshot_now(id, true);
        if let Some(update) = self.workspace.update_mut(id) {
            update.notes = snapshot.text;
            update.touch();
        }
        self.snapshot_now(id, true);
        self.sync_fields(window, cx);
        self.persist(cx);
        if let Some(panel) = &mut self.history_panel {
            panel.snapshots = storage::read_history(&self.path, id);
            panel.selected = panel.snapshots.len().saturating_sub(1);
        }
        self.toast(
            "Version wiederhergestellt",
            Some(Undo::Notes(id, previous)),
            cx,
        );
    }

    // ---------- Overlays ----------

    pub fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_sticky.is_some() {
            self.finish_sticky_edit(cx);
            self.focus_handle.focus(window, cx);
            return;
        }
        if self.popup
            && !self.compact_list
            && self.dialog.is_none()
            && self.date_pop.is_none()
            && !self.palette_open
            && self.history_panel.is_none()
            && self.editing_task.is_none()
            && self.keyboard_menu.is_none()
        {
            self.snapshot_pending();
            self.discard_untouched_drafts(None, cx);
            self.compact_list = true;
            self.focus_handle.focus(window, cx);
            cx.notify();
            return;
        }
        if self.keyboard_menu.take().is_some() {
            self.focus_handle.focus(window, cx);
            cx.notify();
            return;
        }
        if self.dialog.is_none() && self.history_panel.take().is_some() {
            self.focus_handle.focus(window, cx);
            cx.notify();
            return;
        }
        if self.focus_mode {
            self.focus_mode = false;
            cx.notify();
            return;
        }
        if !self.search_query.is_empty()
            && self
                .search_input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        {
            self.search_query.clear();
            self.search_input
                .update(cx, |i, cx| i.set_value("", window, cx));
            cx.notify();
            return;
        }
        if self.view == View::Updates
            && self.detail_open
            && self.dialog.is_none()
            && self.date_pop.is_none()
            && !self.palette_open
        {
            self.discard_untouched_drafts(None, cx);
            self.detail_open = false;
            self.focus_handle.focus(window, cx);
            cx.notify();
            return;
        }
        // Esc schließt zuerst einen Dialog über den Einstellungen, dann die Einstellungen.
        if self.dialog.is_none() && self.tpl_edit.is_none() && self.settings_open {
            self.settings_open = false;
        }
        self.tpl_edit = None;
        self.dialog = None;
        self.date_pop = None;
        self.palette_open = false;
        self.editing_task = None;
        self.cal_inline = None;
        self.cal_more = None;
        self.pill_menu = None;
        self.import_preview = None;
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    pub fn open_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.palette_open = true;
        self.palette_index = 0;
        self.palette_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.palette_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub fn open_settings(
        &mut self,
        cat: Option<SettingsCat>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.popup {
            self.forward_to_main(cx, move |main, window, cx| {
                main.open_settings(cat, window, cx)
            });
            return;
        }
        if let Some(cat) = cat {
            self.settings_cat = cat;
        }
        self.settings_jump = None;
        self.settings_scroll.set_offset(point(px(0.), px(0.)));
        self.settings_open = true;
        self.dialog = None;
        self.date_pop = None;
        self.palette_open = false;
        self.settings_search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.overlay_focus.focus(window, cx);
        cx.notify();
    }

    /// Ändert Einstellungen: persistiert, wendet Look sofort an und zeigt „Gespeichert“.
    pub fn update_settings(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut model::Settings),
    ) {
        f(&mut self.workspace.settings);
        self.ensure_view_visible();
        self.schedule_reminders(cx);
        self.applied_look = None;
        self.settings_saved_at = Some(Instant::now());
        self.persist(cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(StdDuration::from_millis(1800))
                .await;
            this.update(cx, |_, cx| cx.notify()).ok();
        })
        .detach();
        cx.notify();
    }

    pub fn open_dialog(&mut self, dialog: Dialog, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup {
            self.forward_to_main(cx, move |main, window, cx| {
                main.open_dialog(dialog, window, cx)
            });
            return;
        }
        self.dialog = Some(dialog);
        self.date_pop = None;
        self.palette_open = false;
        self.overlay_focus.focus(window, cx);
        cx.notify();
    }

    pub fn focus_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup {
            self.compact_list = true;
            self.search_input
                .update(cx, |input, cx| input.focus(window, cx));
            cx.notify();
            return;
        }
        self.set_view(View::Updates, cx);
        self.search_input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.workspace.settings.sidebar_collapsed = !self.workspace.settings.sidebar_collapsed;
        self.persist(cx);
        cx.notify();
    }

    pub fn set_theme(&mut self, theme: ThemeChoice, cx: &mut Context<Self>) {
        self.update_settings(cx, |s| s.theme = theme);
    }

    pub fn cycle_theme(&mut self, cx: &mut Context<Self>) {
        let next = match self.workspace.settings.theme {
            ThemeChoice::System => ThemeChoice::Dark,
            ThemeChoice::Dark => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::System,
        };
        self.set_theme(next, cx);
    }

    pub fn set_editor_mode(&mut self, mode: EditorMode, cx: &mut Context<Self>) {
        self.update_settings(cx, |s| s.editor_mode = mode);
    }

    // ---------- Kalender ----------

    /// Enter legt ein Update an, Strg+Enter eine Aufgabe im ausgewählten Update.
    pub fn commit_calendar_inline(
        &mut self,
        as_task: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(day) = self.cal_inline else { return };
        let title = self.cal_input.read(cx).value().trim().to_owned();
        if title.is_empty() || !self.writable {
            return;
        }
        match self.selected_update.filter(|_| as_task) {
            Some(update_id) => {
                self.workspace.new_task(update_id, title, Some(day));
            }
            None => {
                let id = self.workspace.new_update(Some(day));
                if let Some(update) = self.workspace.update_mut(id) {
                    update.title = title;
                }
            }
        }
        self.cal_inline = None;
        self.cal_input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.persist(cx);
        self.toast(
            if as_task {
                "Aufgabe angelegt"
            } else {
                "Update angelegt"
            },
            None,
            cx,
        );
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    /// Fokus (Pfeiltasten) durch die Tage bewegen; springt bei Bedarf in den Nachbarmonat.
    pub fn move_cal_focus(&mut self, days: i64, cx: &mut Context<Self>) {
        let from = self.cal_focus.unwrap_or(self.cal_anchor);
        let to = from + Duration::days(days);
        self.cal_focus = Some(to);
        if self.cal_mode == CalMode::Month {
            if model::month_start(to) != model::month_start(self.cal_anchor) {
                self.cal_anchor = to;
            }
        } else {
            let start = |d: NaiveDate| {
                d - Duration::days(self.workspace.settings.week_start.offset(d.weekday()))
            };
            if start(to) != start(self.cal_anchor) {
                self.cal_anchor = to;
            }
        }
        cx.notify();
    }

    pub fn shift_calendar(&mut self, direction: i32, cx: &mut Context<Self>) {
        self.cal_anchor = match self.cal_mode {
            CalMode::Month => model::shift_month(self.cal_anchor, direction),
            CalMode::Week => self.cal_anchor + Duration::days(7 * direction as i64),
        };
        cx.notify();
    }

    pub fn drop_on_day(
        &mut self,
        item: &ui::CalItem,
        day: Option<NaiveDate>,
        cx: &mut Context<Self>,
    ) {
        // Erste erfolgreiche Zuordnung aus „Ohne Termin“ blendet den Hinweis aus.
        let had_no_date = match item {
            ui::CalItem::Update(id, _) => self
                .workspace
                .update(*id)
                .is_some_and(|u| u.planned_for.is_none()),
            ui::CalItem::Task(uid, tid, _) => {
                self.target_date(DateTarget::Task(*uid, *tid)).is_none()
            }
        };
        if had_no_date && day.is_some() && !self.workspace.settings.cal_hint_dismissed {
            self.workspace.settings.cal_hint_dismissed = true;
            self.persist(cx);
        }
        match item {
            ui::CalItem::Update(id, _) => self.set_date(DateTarget::Update(*id), day, cx),
            ui::CalItem::Task(uid, tid, _) => self.set_date(DateTarget::Task(*uid, *tid), day, cx),
        }
    }

    // ---------- Export / Import ----------

    pub fn copy_text(&mut self, text: String, message: &str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.toast(message.to_owned(), None, cx);
    }

    fn documents_dir() -> PathBuf {
        directories::UserDirs::new()
            .and_then(|d| d.document_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn save_with_dialog(
        &mut self,
        name: String,
        bytes: Vec<u8>,
        what: &'static str,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_new_path(&Self::documents_dir(), Some(&name));
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                let result = storage::write_atomic(&path, &bytes);
                this.update(cx, |this, cx| match result {
                    Ok(()) => this.toast(format!("{what}: {}", path.display()), None, cx),
                    Err(error) => this.toast(format!("{what} fehlgeschlagen: {error}"), None, cx),
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn export_update_md(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(update) = self.workspace.update(id) else {
            return;
        };
        let content = md::export_update(update).into_bytes();
        let name = format!("{}.md", md::file_stem(&update.title));
        self.save_with_dialog(name, content, "Exportiert", cx);
    }

    pub fn export_backup(&mut self, cx: &mut Context<Self>) {
        self.flush();
        let name = format!(
            "note-warden-backup-{}.json",
            model::today().format("%Y-%m-%d")
        );
        let bytes = serde_json::to_vec_pretty(&self.workspace).unwrap_or_default();
        self.save_with_dialog(name, bytes, "Backup gespeichert", cx);
    }

    pub fn pick_import(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Backup auswählen".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                let loaded = storage::read_backup_file(&path);
                this.update(cx, |this, cx| {
                    match loaded {
                        Ok(workspace) => {
                            let duplicates = workspace
                                .updates
                                .iter()
                                .filter(|n| {
                                    this.workspace.updates.iter().any(|o| {
                                        o.title == n.title
                                            && o.planned_for == n.planned_for
                                            && o.notes == n.notes
                                    })
                                })
                                .count();
                            let tasks = workspace.updates.iter().map(|u| u.tasks.len()).sum();
                            this.import_preview = Some(ImportPreview {
                                new_updates: workspace.updates.len() - duplicates,
                                duplicates,
                                tasks,
                                workspace,
                            });
                            this.dialog = Some(Dialog::Import);
                        }
                        Err(error) => this.toast(format!("Backup nicht lesbar: {error}"), None, cx),
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn apply_import(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(preview) = self.import_preview.take() else {
            return;
        };
        if !self.writable {
            return;
        }
        // Vorher sichern, damit auch "Ersetzen" nie stillschweigend Daten vernichtet.
        if !self.backup_now(false) {
            self.toast("Import abgebrochen: Backup nicht möglich", None, cx);
            return;
        }
        let message = if replace {
            let settings = self.workspace.settings.clone();
            self.workspace = preview.workspace;
            self.workspace.settings = settings;
            self.workspace.repair_ids();
            "Backup ersetzt den Arbeitsbereich".to_owned()
        } else {
            let (added, skipped) = self.workspace.merge(preview.workspace);
            format!(
                "{} importiert, {skipped} übersprungen",
                model::plural(added, "Update", "Updates")
            )
        };
        self.dialog = None;
        self.selected_update = self
            .workspace
            .updates
            .iter()
            .rev()
            .find(|u| u.is_live())
            .map(|u| u.id);
        self.sync_fields(window, cx);
        self.persist(cx);
        self.toast(message, None, cx);
        cx.notify();
    }

    // ---------- Helfer ----------

    pub fn week_end(&self, today: NaiveDate) -> NaiveDate {
        let start = self.workspace.settings.week_start;
        today + Duration::days(6 - start.offset(today.weekday()))
    }
}

gpui_kit::assets::icon_assets!(
    ExtraIcons,
    [
        Archive,
        Bold,
        Code,
        CornerDownRight,
        GripVertical,
        Heading2,
        Italic,
        Link,
        List,
        ListChecks,
        Pencil,
        TextQuote,
        Trash,
        Keyboard,
        Database,
        Layers,
        SlidersHorizontal,
        Grip,
        ListFilter,
        CalendarPlus,
        Columns2,
        Tag,
        CalendarDays,
        Download,
        Monitor,
        Flag,
        Pin,
        Minimize2,
        Maximize2,
        StickyNote,
    ]
);

/// Standard-Icons plus die wenigen zusätzlichen Lucide-Icons der Editor-Leiste.
struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        let bytes = match ExtraIcons.load(path)? {
            Some(bytes) => Some(bytes),
            None => gpui_kit::assets::Assets.load(path)?,
        };
        Ok(bytes.map(|bytes| {
            if path.ends_with(".svg") {
                std::borrow::Cow::Owned(
                    String::from_utf8_lossy(&bytes)
                        .replace("stroke-width=\"2\"", "stroke-width=\"1.75\"")
                        .into_bytes(),
                )
            } else {
                bytes
            }
        }))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(ExtraIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

fn main() {
    if !system::claim_single_instance() {
        return;
    }
    log::init(&storage::data_dir(&storage::data_path()));
    system::register_app_id(&storage::data_dir(&storage::data_path()));
    log::install_panic_hook();
    log::info(&format!("Start {VERSION}"));
    gpui_kit::application().with_assets(AppAssets).run(|cx| {
        gpui_kit::init(cx);
        system::create_tray();
        let events = system::init_events();
        cx.bind_keys([
            KeyBinding::new(
                "ctrl-z",
                HistoryUndo,
                Some("NoteWarden && !Input && !Editor"),
            ),
            KeyBinding::new(
                "ctrl-y",
                HistoryRedo,
                Some("NoteWarden && !Input && !Editor"),
            ),
            KeyBinding::new(
                "ctrl-shift-z",
                HistoryRedo,
                Some("NoteWarden && !Input && !Editor"),
            ),
            KeyBinding::new("alt-up", PreviousUpdate, Some("NoteWarden && !TaskRow")),
            KeyBinding::new("alt-down", NextUpdate, Some("NoteWarden && !TaskRow")),
            KeyBinding::new("ctrl-shift-f", ToggleFocusMode, Some("NoteWarden")),
            KeyBinding::new("ctrl-n", NewUpdate, None),
            KeyBinding::new("ctrl-f", FocusSearch, None),
            KeyBinding::new(
                "escape",
                CloseOverlay,
                Some("NoteWarden && !Popover && !PopoverContent && !PopupMenu && !AppMenuBar"),
            ),
            KeyBinding::new("ctrl-k", OpenPalette, None),
            KeyBinding::new("ctrl-,", OpenSettings, None),
            KeyBinding::new("n", FocusQuickAdd, Some("Today && !Input")),
            KeyBinding::new("m", CalMonth, Some("Calendar && !Input")),
            KeyBinding::new("w", CalWeek, Some("Calendar && !Input")),
            KeyBinding::new("ctrl-1", ShowToday, None),
            KeyBinding::new("ctrl-2", ShowUpdates, None),
            KeyBinding::new("ctrl-3", ShowBoard, None),
            KeyBinding::new("ctrl-4", ShowCalendar, None),
            KeyBinding::new("ctrl-b", ToggleSidebar, Some("NoteWarden && !Editor")),
            KeyBinding::new("shift-/", ShowShortcuts, Some("NoteWarden && !Input")),
            KeyBinding::new(
                "delete",
                DeleteSelected,
                Some("NoteWarden && !Input && !TaskRow"),
            ),
            KeyBinding::new("left", CalPrev, Some("Calendar && !Input")),
            KeyBinding::new("right", CalNext, Some("Calendar && !Input")),
            KeyBinding::new("t", CalToday, Some("Calendar && !Input")),
            KeyBinding::new("shift-left", CalFocusPrevDay, Some("Calendar && !Input")),
            KeyBinding::new("shift-right", CalFocusNextDay, Some("Calendar && !Input")),
            KeyBinding::new("up", CalFocusPrevWeek, Some("Calendar && !Input")),
            KeyBinding::new("down", CalFocusNextWeek, Some("Calendar && !Input")),
            KeyBinding::new("enter", CalOpenDay, Some("Calendar && !Input")),
            KeyBinding::new("ctrl-b", EditorBold, Some("Editor")),
            KeyBinding::new("ctrl-i", EditorItalic, Some("Editor")),
            KeyBinding::new("ctrl-shift-d", ToggleStyleguide, None),
            KeyBinding::new("ctrl-shift-m", ToggleCompactMode, None),
            KeyBinding::new("ctrl-shift-k", EditorLink, Some("Editor")),
            KeyBinding::new("tab", TaskIndent, Some("TaskRow")),
            KeyBinding::new("shift-tab", TaskOutdent, Some("TaskRow")),
            KeyBinding::new("alt-up", TaskMoveUp, Some("TaskRow")),
            KeyBinding::new("alt-down", TaskMoveDown, Some("TaskRow")),
            KeyBinding::new("space", TaskToggleKey, Some("TaskRow")),
            KeyBinding::new("enter", TaskEditKey, Some("TaskRow")),
            KeyBinding::new("delete", TaskDeleteKey, Some("TaskRow")),
        ]);
        // Theme, Akzent und Fenstergröße aus der kleinen Prefs-Datei, noch vor dem Laden
        // des Arbeitsbereichs: kein Aufblitzen im falschen Theme.
        let prefs = storage::read_prefs(&storage::data_path());
        let initial = model::Settings {
            theme: prefs.theme,
            accent: prefs.accent,
            ..Default::default()
        };
        let dark = match prefs.theme {
            ThemeChoice::Dark => true,
            ThemeChoice::Light => false,
            ThemeChoice::System => !matches!(cx.window_appearance(), WindowAppearance::Light),
        };
        palette::apply(dark, &initial, cx);
        let saved_size = prefs.window_size;
        let (width, height) = std::env::var("NOTE_WARDEN_WINDOW_SIZE")
            .ok()
            .and_then(|value| {
                let (width, height) = value.split_once('x')?;
                Some((width.parse::<f32>().ok()?, height.parse::<f32>().ok()?))
            })
            .filter(|(width, height)| {
                (960. ..=2560.).contains(width) && (620. ..=1600.).contains(height)
            })
            .or(saved_size
                .filter(|(w, h)| (960. ..=2560.).contains(w) && (620. ..=1600.).contains(h)))
            .unwrap_or((1280., 800.));
        let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Note Warden".into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(compact::MIN_SIZE.0), px(compact::MIN_SIZE.1))),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| NoteWarden::new(window, Some(events), None, cx)),
        )
        .expect("Fenster konnte nicht geöffnet werden");
    });
}

#[cfg(test)]
mod asset_tests {
    use super::*;
    use gpui_kit::assets::IconName;

    /// Compiling an IconName does not ensure its SVG is shipped. Exercise the actual
    /// application asset source for every icon referenced by the UI, including aliases.
    #[::core::prelude::v1::test]
    fn ui_icons_are_available_offline() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec![root.join("src/ui.rs")];
        files.extend(
            std::fs::read_dir(root.join("src/ui"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "rs")),
        );
        let icons: std::collections::HashMap<_, _> = IconName::ALL
            .iter()
            .map(|icon| (format!("{icon:?}"), *icon))
            .collect();
        let mut checked = std::collections::HashSet::new();
        for file in files {
            let source = std::fs::read_to_string(&file).unwrap();
            for part in source.split("IconName::").skip(1) {
                let name: String = part
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                if name.is_empty() || name == "ALL" || !checked.insert(name.clone()) {
                    continue;
                }
                let icon = icons
                    .get(&name)
                    .unwrap_or_else(|| panic!("Unknown UI icon {name}"));
                let path = icon.path();
                let bytes = AppAssets
                    .load(&path)
                    .unwrap_or_else(|e| panic!("{name}: {e}"))
                    .unwrap_or_else(|| panic!("Missing SVG for {name}"));
                assert!(
                    String::from_utf8_lossy(&bytes).contains("<svg"),
                    "Invalid SVG for {name}"
                );
            }
        }
        assert!(
            checked.len() >= 50,
            "UI icon audit did not cover the UI sources"
        );
    }
}
