use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Weekday};
use serde::{Deserialize, Serialize};

/// Current schema version of the workspace file. Files without a version
/// (0.1 / 0.2) are migrated on load; a backup is created beforehand.
pub const SCHEMA_VERSION: u32 = 8;
pub const TRASH_DAYS: i64 = 30;

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Idee,
    Geplant,
    InArbeit,
    Erledigt,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idee => "Idee",
            Self::Geplant => "Geplant",
            Self::InArbeit => "In Arbeit",
            Self::Erledigt => "Erledigt",
        }
    }

    pub const ALL: [Self; 4] = [Self::Idee, Self::Geplant, Self::InArbeit, Self::Erledigt];

    pub fn order(self) -> u8 {
        match self {
            Self::InArbeit => 0,
            Self::Geplant => 1,
            Self::Idee => 2,
            Self::Erledigt => 3,
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    #[default]
    Added,
    Changed,
    Removed,
}

impl ChangeKind {
    pub const ALL: [Self; 3] = [Self::Added, Self::Changed, Self::Removed];

    pub fn label(self) -> &'static str {
        match self {
            Self::Added => "Added",
            Self::Changed => "Changed",
            Self::Removed => "Removed",
        }
    }
}

/// Reminder (local time). Everything is optional; due dates alone do not remind.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Remind {
    #[serde(default, rename = "remind_at", skip_serializing_if = "Option::is_none")]
    pub at: Option<NaiveDateTime>,
    #[serde(
        default,
        rename = "reminded_at",
        skip_serializing_if = "Option::is_none"
    )]
    pub reminded: Option<NaiveDateTime>,
    #[serde(
        default,
        rename = "snoozed_until",
        skip_serializing_if = "Option::is_none"
    )]
    pub snoozed: Option<NaiveDateTime>,
}

impl Remind {
    /// Set a new reminder; an earlier notification and snooze no longer apply.
    pub fn set(&mut self, at: Option<NaiveDateTime>) {
        *self = Self {
            at,
            reminded: None,
            snoozed: None,
        };
    }

    /// Next notification time, if not yet notified.
    pub fn due_at(&self) -> Option<NaiveDateTime> {
        if self.reminded.is_some() {
            return None;
        }
        self.snoozed.or(self.at)
    }
}

/// Published state of an update. The changelog text is stored without a version header.
#[derive(Clone, Deserialize, Serialize)]
pub struct Release {
    pub version: String,
    pub changelog: String,
    /// Unix milliseconds of the publication.
    pub at: i64,
    /// Status before publication (for "Veröffentlichung zurücknehmen", i.e. withdrawing the publication).
    pub previous_status: Status,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Task {
    #[serde(default)]
    pub priority: u8,
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub planned_for: Option<NaiveDate>,
    /// Subtask of the preceding main task (one level deep).
    #[serde(default)]
    pub sub: bool,
    #[serde(default)]
    pub kind: ChangeKind,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, flatten)]
    pub remind: Remind,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Update {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub activity: Vec<Activity>,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub project_id: Option<u64>,
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub planned_for: Option<NaiveDate>,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub archived: bool,
    /// Set = in the trash (removed permanently after `TRASH_DAYS` days).
    #[serde(default)]
    pub deleted_on: Option<NaiveDate>,
    /// Unix milliseconds of the last edit (0 = unknown).
    #[serde(default)]
    pub edited_at: i64,
    #[serde(default)]
    pub release: Option<Release>,
    #[serde(default, flatten)]
    pub remind: Remind,
}

/// Tag text as stored: no leading `#`, inner whitespace turned into `-`, at most 24 characters.
pub fn clean_tag(raw: &str) -> Option<String> {
    let tag: String = raw
        .trim()
        .trim_start_matches('#')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(24)
        .collect();
    (!tag.is_empty()).then_some(tag)
}

/// Adds `raw` unless the same tag (ignoring case) is already there; true if it was added.
pub fn push_tag(tags: &mut Vec<String>, raw: &str) -> bool {
    let Some(tag) = clean_tag(raw) else {
        return false;
    };
    if tags
        .iter()
        .any(|t| t.eq_ignore_ascii_case(&tag) || t.to_lowercase() == tag.to_lowercase())
    {
        return false;
    }
    tags.push(tag);
    true
}

impl Update {
    /// Tagged directly or through one of its tasks (ignoring case).
    pub fn has_tag(&self, tag: &str) -> bool {
        let tag = tag.to_lowercase();
        self.tags
            .iter()
            .chain(self.tasks.iter().flat_map(|t| t.tags.iter()))
            .any(|t| t.to_lowercase() == tag)
    }

    /// The title or, if empty, the first text line of the note (without a Markdown prefix).
    pub fn display_title(&self) -> Option<String> {
        let title = self.title.trim();
        if !title.is_empty() {
            return Some(title.to_owned());
        }
        self.notes
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| {
                line.trim_start_matches(['#', '-', '*', ' '])
                    .chars()
                    .take(80)
                    .collect::<String>()
            })
            .filter(|title| !title.is_empty())
    }

    /// Progress across all tasks including subtasks.
    pub fn task_progress(&self) -> (usize, usize) {
        (
            self.tasks.iter().filter(|task| task.done).count(),
            self.tasks.len(),
        )
    }

    pub fn is_live(&self) -> bool {
        !self.archived && self.deleted_on.is_none()
    }

    pub fn touch(&mut self) {
        self.edited_at = now_millis();
    }

    /// Is the update's due date overdue?
    pub fn overdue(&self, today: NaiveDate) -> bool {
        self.status != Status::Erledigt && self.planned_for.is_some_and(|d| d < today)
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeChoice {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorMode {
    #[default]
    Write,
    Preview,
    Split,
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DateFormat {
    /// "10. November 2026"
    #[default]
    German,
    Iso,
    /// "10. Nov. 2026"
    Medium,
    /// "10.11.2026"
    Numeric,
}

impl DateFormat {
    pub const ALL: [Self; 4] = [Self::Medium, Self::Numeric, Self::Iso, Self::German];
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
}

impl WeekStart {
    pub fn weekday(self) -> Weekday {
        match self {
            Self::Monday => Weekday::Mon,
            Self::Sunday => Weekday::Sun,
        }
    }

    pub fn offset(self, day: Weekday) -> i64 {
        match self {
            Self::Monday => day.num_days_from_monday() as i64,
            Self::Sunday => day.num_days_from_sunday() as i64,
        }
    }

    pub fn names(self) -> [&'static str; 7] {
        match self {
            Self::Monday => ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
            Self::Sunday => ["So", "Mo", "Di", "Mi", "Do", "Fr", "Sa"],
        }
    }

    /// Spaltenindex (0..7) eines Datums in einer Kalenderzeile.
    pub fn column(self, date: NaiveDate) -> usize {
        self.offset(date.weekday()) as usize
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Template {
    pub name: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tasks: Vec<String>,
}

impl Template {
    pub fn release() -> Self {
        Self {
            name: "Release-Update".into(),
            notes: "## Ziel\n\n## Änderungen\n\n## Risiken\n\n## Release-Checkliste\n".into(),
            tasks: vec![
                "Features abschließen".into(),
                "Changelog schreiben".into(),
                "Testen".into(),
                "Version erhöhen".into(),
                "Release veröffentlichen".into(),
            ],
        }
    }
}

macro_rules! choice_enum {
    ($(#[$m:meta])* $name:ident { $($var:ident),+ $(,)? } default $def:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($var),+ }
        impl Default for $name {
            fn default() -> Self { Self::$def }
        }
    };
}

choice_enum!(StartView { Today, Updates, Calendar, Last } default Today);
choice_enum!(Density { Comfortable, Compact } default Comfortable);
choice_enum!(FontScale { Small, Standard, Large } default Standard);
choice_enum!(Motion { System, Reduced, Full } default System);
choice_enum!(CalView { Month, Week } default Month);
choice_enum!(EditorFont { Proportional, Mono } default Proportional);
choice_enum!(
    Accent { Sky, Violet, Pink, Red, Orange, Yellow, Green, Teal } default Sky
);

impl FontScale {
    pub fn factor(self) -> f32 {
        match self {
            Self::Small => 12. / 13.,
            Self::Standard => 1.,
            Self::Large => 15. / 13.,
        }
    }
}

impl Accent {
    pub const ALL: [Self; 8] = [
        Self::Sky,
        Self::Violet,
        Self::Pink,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Teal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Sky => "Himmelblau",
            Self::Violet => "Violett",
            Self::Pink => "Pink",
            Self::Red => "Magenta",
            Self::Orange => "Orange",
            Self::Yellow => "Gelb",
            Self::Green => "Grün",
            Self::Teal => "Türkis",
        }
    }

    pub fn base(self) -> u32 {
        match self {
            Self::Sky => 0x38bdf8,
            Self::Violet => 0xa78bfa,
            Self::Pink => 0xf472b6,
            Self::Red => 0xd946ef,
            Self::Orange => 0xfb923c,
            Self::Yellow => 0xfacc15,
            Self::Green => 0x4ade80,
            Self::Teal => 0x2dd4bf,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    pub compact: crate::compact::CompactSettings,
    #[serde(default)]
    pub updates_ui: UpdatesUi,
    // Allgemein
    #[serde(default)]
    pub start_view: StartView,
    #[serde(default = "yes")]
    pub undo_instead_of_dialog: bool,
    /// Days in the trash; 0 = never empty automatically.
    #[serde(default = "default_trash_days")]
    pub trash_days: u32,
    #[serde(default)]
    pub sidebar_collapsed: bool,
    #[serde(default = "yes")]
    pub remember_window: bool,
    #[serde(default)]
    pub last_update: Option<u64>,
    #[serde(default)]
    pub window_size: Option<(f32, f32)>,

    #[serde(default = "default_home_cards")]
    pub home_cards: [bool; 7],
    #[serde(default)]
    pub home_completions: Vec<(u64, Option<u64>, NaiveDate)>,
    #[serde(default)]
    pub last_settings_category: usize,

    // Darstellung
    #[serde(default)]
    pub theme: ThemeChoice,
    #[serde(default)]
    pub accent: Accent,
    #[serde(default)]
    pub density: Density,
    #[serde(default)]
    pub font_scale: FontScale,
    #[serde(default)]
    pub motion: Motion,

    // Editor
    #[serde(default)]
    pub editor_mode: EditorMode,
    #[serde(default)]
    pub editor_font: EditorFont,
    #[serde(default = "yes")]
    pub word_wrap: bool,
    #[serde(default = "yes")]
    pub word_count: bool,
    #[serde(default = "yes")]
    pub toolbar: bool,
    #[serde(default = "yes")]
    pub code_highlight: bool,
    #[serde(default = "default_autosave")]
    pub autosave_ms: u32,
    #[serde(default = "yes")]
    pub live_preview: bool,

    // Kalender & Datum
    #[serde(default)]
    pub date_format: DateFormat,
    #[serde(default)]
    pub week_start: WeekStart,
    #[serde(default = "yes")]
    pub relative_dates: bool,
    #[serde(default)]
    pub cal_view: CalView,
    #[serde(default = "yes")]
    pub highlight_weekends: bool,
    #[serde(default)]
    pub week_numbers: bool,
    #[serde(default = "yes")]
    pub show_done_in_calendar: bool,
    #[serde(default = "default_pills")]
    pub max_pills: u32,

    // Benachrichtigungen
    #[serde(default = "yes")]
    pub reminders_on: bool,
    #[serde(default = "yes")]
    pub daily_on: bool,
    /// Minuten seit Mitternacht.
    #[serde(default = "default_daily_time")]
    pub daily_time: u32,
    #[serde(default)]
    pub quiet_on: bool,
    #[serde(default = "default_quiet_from")]
    pub quiet_from: u32,
    #[serde(default = "default_quiet_to")]
    pub quiet_to: u32,
    #[serde(default)]
    pub last_daily: Option<NaiveDate>,

    // System
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "yes")]
    pub tray_on_close: bool,
    #[serde(default)]
    pub tray_hint_shown: bool,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub last_update_check: Option<i64>,
    #[serde(default)]
    pub seen_version: Option<String>,

    // Layout memory
    #[serde(default = "default_list_width")]
    pub list_width: f32,
    #[serde(default = "yes")]
    pub status_group_open: bool,
    #[serde(default = "yes")]
    pub tags_group_open: bool,
    /// Sidebar entries the user turned off (the views stay reachable from elsewhere).
    #[serde(default)]
    pub hide_inbox: bool,
    #[serde(default)]
    pub hide_board: bool,
    #[serde(default)]
    pub hide_stickies: bool,
    #[serde(default)]
    pub hide_releases: bool,
    #[serde(default)]
    pub hide_calendar: bool,
    #[serde(default = "yes")]
    pub filter_group_open: bool,
    #[serde(default = "yes")]
    pub projects_group_open: bool,
    #[serde(default = "yes")]
    pub views_group_open: bool,
    #[serde(default = "yes")]
    pub cal_panel_open: bool,
    #[serde(default = "default_panel_width")]
    pub cal_panel_width: f32,
    #[serde(default)]
    pub cal_hint_dismissed: bool,

    #[serde(default)]
    pub templates: Vec<Template>,
    #[serde(default)]
    pub default_template: Option<usize>,
}

fn default_home_cards() -> [bool; 7] {
    [true; 7]
}

fn yes() -> bool {
    true
}

fn default_list_width() -> f32 {
    340.
}

fn default_panel_width() -> f32 {
    280.
}

fn default_trash_days() -> u32 {
    TRASH_DAYS as u32
}

fn default_autosave() -> u32 {
    400
}

fn default_pills() -> u32 {
    3
}

fn default_daily_time() -> u32 {
    9 * 60
}

fn default_quiet_from() -> u32 {
    22 * 60
}

fn default_quiet_to() -> u32 {
    7 * 60
}

fn default_hotkey() -> String {
    "Strg+Alt+N".into()
}

impl Default for Settings {
    fn default() -> Self {
        // The only source of default values: serde defaults.
        let mut settings: Self =
            serde_json::from_str("{}").expect("leere Einstellungen sind gültig");
        settings.templates = vec![Template::release()];
        settings
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub color: u32,
    pub archived: bool,
    pub created_at: i64,
}

/// What deleting a project detached, so that the deletion can be undone.
#[derive(Clone)]
pub struct RemovedProject {
    pub project: Project,
    pub index: usize,
    pub updates: Vec<u64>,
    pub tasks: Vec<u64>,
}

/// Project name as stored: whitespace collapsed, at most 40 characters; `None` if empty.
pub fn clean_project_name(raw: &str) -> Option<String> {
    let name: String = raw
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(40)
        .collect();
    let name = name.trim().to_owned();
    (!name.is_empty()).then_some(name)
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StandaloneTask {
    pub id: u64,
    pub title: String,
    pub done: bool,
    pub date: Option<NaiveDate>,
    pub project_id: Option<u64>,
    pub priority: u8,
    pub created_at: i64,
    #[serde(default, flatten)]
    pub remind: Remind,
}

/// Small note without a title and status: order = position in `Workspace::stickies`.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Sticky {
    pub id: u64,
    #[serde(default)]
    pub text: String,
    /// Index in `palette::STICKY_COLORS`.
    #[serde(default)]
    pub color: u8,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Workspace {
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub standalone_tasks: Vec<StandaloneTask>,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub updates: Vec<Update>,
    #[serde(default)]
    pub stickies: Vec<Sticky>,
    #[serde(default = "first_id")]
    next_id: u64,
    #[serde(default)]
    pub settings: Settings,
}

fn first_id() -> u64 {
    1
}

impl Workspace {
    /// Is `name` (ignoring case) already used by another project than `except`?
    pub fn project_name_taken(&self, name: &str, except: Option<u64>) -> bool {
        let name = name.to_lowercase();
        self.projects
            .iter()
            .any(|p| Some(p.id) != except && p.name.to_lowercase() == name)
    }

    /// Live updates and open inbox tasks that belong to the project.
    pub fn project_usage(&self, id: u64) -> (usize, usize) {
        let updates = self
            .updates
            .iter()
            .filter(|u| u.is_live() && u.project_id == Some(id))
            .count();
        let tasks = self
            .standalone_tasks
            .iter()
            .filter(|t| !t.done && t.project_id == Some(id))
            .count();
        (updates, tasks)
    }

    /// Deletes the project; its updates and tasks stay and merely lose the assignment.
    pub fn remove_project(&mut self, id: u64) -> Option<RemovedProject> {
        let index = self.projects.iter().position(|p| p.id == id)?;
        let project = self.projects.remove(index);
        let mut updates = Vec::new();
        for update in self.updates.iter_mut().filter(|u| u.project_id == Some(id)) {
            update.project_id = None;
            updates.push(update.id);
        }
        let mut tasks = Vec::new();
        for task in self
            .standalone_tasks
            .iter_mut()
            .filter(|t| t.project_id == Some(id))
        {
            task.project_id = None;
            tasks.push(task.id);
        }
        Some(RemovedProject {
            project,
            index,
            updates,
            tasks,
        })
    }

    /// Undoes `remove_project`: the project returns to its place and takes its items back.
    pub fn restore_project(&mut self, removed: RemovedProject) {
        let id = removed.project.id;
        let at = removed.index.min(self.projects.len());
        self.projects.insert(at, removed.project);
        for update in self
            .updates
            .iter_mut()
            .filter(|u| removed.updates.contains(&u.id))
        {
            update.project_id = Some(id);
        }
        for task in self
            .standalone_tasks
            .iter_mut()
            .filter(|t| removed.tasks.contains(&t.id))
        {
            task.project_id = Some(id);
        }
    }

    /// Every tag in use by a live update or its tasks with the number of updates using it,
    /// most used first. The spelling of the first occurrence wins.
    pub fn all_tags(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for update in self.updates.iter().filter(|u| u.is_live()) {
            let mut seen: Vec<String> = Vec::new();
            for tag in update
                .tags
                .iter()
                .chain(update.tasks.iter().flat_map(|t| t.tags.iter()))
            {
                let key = tag.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key.clone());
                match counts.iter_mut().find(|(t, _)| t.to_lowercase() == key) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((tag.clone(), 1)),
                }
            }
        }
        counts.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        counts
    }

    /// Tags used by post-its with the number of post-its, most used first.
    pub fn sticky_tags(&self) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = Vec::new();
        for tag in self.stickies.iter().flat_map(|s| s.tags.iter()) {
            match counts
                .iter_mut()
                .find(|(t, _)| t.to_lowercase() == tag.to_lowercase())
            {
                Some((_, n)) => *n += 1,
                None => counts.push((tag.clone(), 1)),
            }
        }
        counts.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        counts
    }

    /// New post-it at the very front; the color rotates.
    pub fn new_sticky(&mut self) -> u64 {
        let id = self.next_id();
        let color = (self.stickies.len() % crate::palette::STICKY_COLORS.len()) as u8;
        self.stickies.insert(
            0,
            Sticky {
                id,
                text: String::new(),
                color,
                created_at: now_millis(),
                tags: Vec::new(),
            },
        );
        id
    }

    /// Moves a post-it before `before` (to the end if `None` or unknown).
    pub fn move_sticky(&mut self, id: u64, before: Option<u64>) {
        let Some(from) = self.stickies.iter().position(|s| s.id == id) else {
            return;
        };
        let sticky = self.stickies.remove(from);
        let to = before
            .and_then(|b| self.stickies.iter().position(|s| s.id == b))
            .unwrap_or(self.stickies.len());
        self.stickies.insert(to, sticky);
    }

    pub fn new_update(&mut self, date: Option<NaiveDate>) -> u64 {
        let id = self.next_id();
        self.updates.push(Update {
            tags: Vec::new(),
            version: String::new(),
            created_at: now_millis(),
            activity: Vec::new(),
            priority: 0,
            project_id: None,
            id,
            title: String::new(),
            notes: String::new(),
            status: if date.is_some() {
                Status::Geplant
            } else {
                Status::Idee
            },
            planned_for: date,
            tasks: Vec::new(),
            archived: false,
            deleted_on: None,
            edited_at: now_millis(),
            release: None,
            remind: Remind::default(),
        });
        id
    }

    pub fn new_update_from(&mut self, date: Option<NaiveDate>, template: Option<&Template>) -> u64 {
        let id = self.new_update(date);
        if let Some(template) = template {
            let titles = template.tasks.clone();
            if let Some(update) = self.update_mut(id) {
                update.notes = template.notes.clone();
            }
            for title in titles {
                self.new_task(id, title, None);
            }
        }
        id
    }

    pub fn new_task(
        &mut self,
        update_id: u64,
        title: String,
        date: Option<NaiveDate>,
    ) -> Option<u64> {
        let title = title.trim();
        if title.is_empty() {
            return None;
        }
        let id = self.next_id();
        let update = self
            .updates
            .iter_mut()
            .find(|update| update.id == update_id)?;
        update.tasks.push(Task {
            tags: Vec::new(),
            priority: 0,
            id,
            title: title.to_owned(),
            done: false,
            planned_for: date,
            sub: false,
            kind: ChangeKind::Added,
            description: String::new(),
            remind: Remind::default(),
        });
        update.touch();
        Some(id)
    }

    /// Copy of an update with its tasks, without a date.
    pub fn duplicate(&mut self, id: u64) -> Option<u64> {
        let mut copy = self.update(id)?.clone();
        copy.id = self.next_id();
        copy.title = format!("{} (Kopie)", copy.title);
        copy.planned_for = None;
        copy.release = None;
        copy.remind = Remind::default();
        copy.archived = false;
        copy.deleted_on = None;
        copy.status = if copy.status == Status::Erledigt {
            Status::Idee
        } else {
            copy.status
        };
        for task in &mut copy.tasks {
            task.id = self.next_id();
            task.planned_for = None;
            task.done = false;
            task.remind = Remind::default();
        }
        copy.touch();
        let id = copy.id;
        self.updates.push(copy);
        Some(id)
    }

    pub fn update(&self, id: u64) -> Option<&Update> {
        self.updates.iter().find(|update| update.id == id)
    }

    pub fn update_mut(&mut self, id: u64) -> Option<&mut Update> {
        self.updates.iter_mut().find(|update| update.id == id)
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        id
    }

    pub fn alloc_id(&mut self) -> u64 {
        self.next_id()
    }

    pub fn repair_ids(&mut self) {
        let max_id = self
            .updates
            .iter()
            .flat_map(|update| {
                std::iter::once(update.id).chain(update.tasks.iter().map(|task| task.id))
            })
            .max()
            .unwrap_or(0);
        let max_id = self
            .projects
            .iter()
            .map(|p| p.id)
            .chain(self.standalone_tasks.iter().map(|t| t.id))
            .chain(self.stickies.iter().map(|s| s.id))
            .fold(max_id, u64::max);
        self.next_id = self.next_id.max(max_id.saturating_add(1));
    }

    /// Raises older schemas to the current version. Returns `true`
    /// if something changed (the caller then backs up the file first).
    pub fn migrate(&mut self) -> bool {
        if self.schema_version >= SCHEMA_VERSION {
            return false;
        }
        // serde fills new settings fields with defaults; only check value ranges here.
        self.settings.max_pills = self.settings.max_pills.clamp(2, 4);
        self.settings.list_width = self.settings.list_width.clamp(300., 520.);
        for u in &mut self.updates {
            if u.activity.len() > 200 {
                u.activity.drain(..u.activity.len() - 200);
            }
            if u.created_at <= 0 {
                u.created_at = u.edited_at.max(0);
            }
        }
        self.schema_version = SCHEMA_VERSION;
        if self.settings.templates.is_empty() {
            self.settings.templates.push(Template::release());
        }
        true
    }

    /// Sets missing creation times to the modification time of the data file.
    pub fn fill_created_at(&mut self, fallback: i64) {
        for u in &mut self.updates {
            if u.created_at <= 0 {
                u.created_at = fallback;
            }
        }
    }

    /// Removes trash entries older than `TRASH_DAYS`.
    pub fn purge_trash(&mut self, today: NaiveDate) -> usize {
        let before = self.updates.len();
        let days = self.settings.trash_days;
        if days == 0 {
            return 0;
        }
        self.updates.retain(|update| {
            update
                .deleted_on
                .is_none_or(|day| today - day <= Duration::days(days as i64))
        });
        before - self.updates.len()
    }

    /// Adds updates from a backup (new IDs). Identical entries
    /// (title, date, notes) are skipped. Returns (new, skipped).
    pub fn merge(&mut self, other: Workspace) -> (usize, usize) {
        let (mut added, mut skipped) = (0, 0);
        for mut update in other.updates {
            let exists = self.updates.iter().any(|own| {
                own.title == update.title
                    && own.planned_for == update.planned_for
                    && own.notes == update.notes
            });
            if exists {
                skipped += 1;
                continue;
            }
            update.id = self.next_id();
            for task in &mut update.tasks {
                task.id = self.next_id();
            }
            self.updates.push(update);
            added += 1;
        }
        (added, skipped)
    }
}

pub fn now_millis() -> i64 {
    Local::now().timestamp_millis()
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn month_start(date: NaiveDate) -> NaiveDate {
    date.with_day(1).expect("gültiger Monat")
}

pub fn shift_month(date: NaiveDate, offset: i32) -> NaiveDate {
    let month = date.year() * 12 + date.month0() as i32 + offset;
    NaiveDate::from_ymd_opt(month.div_euclid(12), month.rem_euclid(12) as u32 + 1, 1)
        .expect("Monat im unterstützten Datumsbereich")
}

/// Plural: `plural(1, "Eintrag", "Einträge")` -> "1 Eintrag".
pub fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deleting_a_project_keeps_its_items_and_can_be_undone() {
        assert_eq!(
            clean_project_name("  Mein   Projekt "),
            Some("Mein Projekt".into())
        );
        assert_eq!(clean_project_name("   "), None);
        assert_eq!(
            clean_project_name(&"x".repeat(60)).map(|n| n.len()),
            Some(40)
        );
        let mut ws = Workspace::default();
        let id = ws.alloc_id();
        let other = ws.alloc_id();
        for (pid, name) in [(id, "Alpha"), (other, "Beta")] {
            ws.projects.push(Project {
                id: pid,
                name: name.into(),
                color: 0,
                archived: false,
                created_at: 0,
            });
        }
        let update = ws.new_update(None);
        ws.update_mut(update).unwrap().project_id = Some(id);
        assert!(ws.project_name_taken("alpha", None));
        assert!(!ws.project_name_taken("alpha", Some(id)));
        assert_eq!(ws.project_usage(id), (1, 0));
        let removed = ws.remove_project(id).unwrap();
        assert_eq!(ws.projects.len(), 1);
        assert_eq!(ws.update(update).unwrap().project_id, None);
        ws.restore_project(removed);
        assert_eq!(ws.projects[0].name, "Alpha");
        assert_eq!(ws.update(update).unwrap().project_id, Some(id));
        assert!(ws.remove_project(999).is_none());
    }

    #[test]
    fn tags_are_cleaned_deduplicated_and_counted_across_updates_and_tasks() {
        assert_eq!(clean_tag("  #Pre  Release "), Some("Pre-Release".into()));
        assert_eq!(clean_tag("#"), None);
        assert_eq!(clean_tag(&"x".repeat(40)).map(|t| t.len()), Some(24));
        let mut tags = Vec::new();
        assert!(push_tag(&mut tags, "#Bug"));
        assert!(!push_tag(&mut tags, "bug"));
        let mut ws = Workspace::default();
        let (a, b) = (ws.new_update(None), ws.new_update(None));
        ws.update_mut(a).unwrap().tags = vec!["Bug".into(), "api".into()];
        let task = ws.new_task(b, "Fix".into(), None).unwrap();
        ws.update_mut(b)
            .unwrap()
            .tasks
            .iter_mut()
            .find(|t| t.id == task)
            .unwrap()
            .tags = vec!["bug".into()];
        assert!(ws.update(b).unwrap().has_tag("BUG"));
        assert!(!ws.update(b).unwrap().has_tag("api"));
        assert_eq!(ws.all_tags(), vec![("Bug".into(), 2), ("api".into(), 1)]);
    }

    #[test]
    fn stickies_keep_order_ids_and_survive_old_files() {
        let mut ws: Workspace = serde_json::from_str(r#"{"next_id":4}"#).unwrap();
        assert!(ws.stickies.is_empty());
        let (a, b, c) = (ws.new_sticky(), ws.new_sticky(), ws.new_sticky());
        let ids = |ws: &Workspace| ws.stickies.iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(ids(&ws), vec![c, b, a]);
        ws.move_sticky(a, Some(c));
        assert_eq!(ids(&ws), vec![a, c, b]);
        ws.move_sticky(a, None);
        assert_eq!(ids(&ws), vec![c, b, a]);
        ws.move_sticky(99, Some(a));
        assert_eq!(ids(&ws), vec![c, b, a]);
        let mut restored: Workspace =
            serde_json::from_str(&serde_json::to_string(&ws).unwrap()).unwrap();
        restored.next_id = 1;
        restored.repair_ids();
        assert!(restored.new_sticky() > a.max(b).max(c));
    }

    #[test]
    fn display_title_prefers_title_then_first_note_line() {
        let mut ws = Workspace::default();
        let id = ws.new_update(None);
        let update = ws.update_mut(id).unwrap();
        assert_eq!(update.display_title(), None);
        update.notes = "
## Einkauf planen
- Milch"
            .into();
        assert_eq!(update.display_title().as_deref(), Some("Einkauf planen"));
        update.notes = "- 
"
        .into();
        assert_eq!(update.display_title(), None);
        update.title = "  Release  ".into();
        assert_eq!(update.display_title().as_deref(), Some("Release"));
    }

    #[test]
    fn month_navigation_handles_year_boundary() {
        let december = NaiveDate::from_ymd_opt(2026, 12, 18).unwrap();
        assert_eq!(
            shift_month(december, 1),
            NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()
        );
        assert_eq!(
            shift_month(december, -12),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap()
        );
    }

    #[test]
    fn task_and_update_ids_remain_distinct_after_loading() {
        let mut workspace = Workspace::default();
        let update_id = workspace.new_update(None);
        let task_id = workspace
            .new_task(update_id, "Feature planen".into(), None)
            .unwrap();
        let mut loaded: Workspace =
            serde_json::from_slice(&serde_json::to_vec(&workspace).unwrap()).unwrap();
        loaded.repair_ids();
        let next_id = loaded.new_update(None);
        assert_ne!(update_id, task_id);
        assert!(next_id > task_id);
    }

    #[test]
    fn plural_forms() {
        assert_eq!(plural(1, "Eintrag", "Einträge"), "1 Eintrag");
        assert_eq!(plural(0, "Eintrag", "Einträge"), "0 Einträge");
        assert_eq!(plural(2, "Eintrag", "Einträge"), "2 Einträge");
    }

    #[test]
    fn duplicate_drops_dates_and_progress() {
        let mut ws = Workspace::default();
        let id = ws.new_update(NaiveDate::from_ymd_opt(2026, 11, 20));
        let task = ws.new_task(id, "A".into(), NaiveDate::from_ymd_opt(2026, 11, 19));
        ws.update_mut(id).unwrap().tasks[0].done = task.is_some();
        let copy = ws.duplicate(id).unwrap();
        let copy = ws.update(copy).unwrap();
        assert!(copy.planned_for.is_none());
        assert!(copy.tasks[0].planned_for.is_none() && !copy.tasks[0].done);
    }

    #[test]
    fn trash_purge_respects_retention() {
        let mut ws = Workspace::default();
        let today = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let old = ws.new_update(None);
        let recent = ws.new_update(None);
        ws.update_mut(old).unwrap().deleted_on = Some(today - Duration::days(31));
        ws.update_mut(recent).unwrap().deleted_on = Some(today - Duration::days(5));
        assert_eq!(ws.purge_trash(today), 1);
        assert!(ws.update(recent).is_some());
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: String,
    pub at: i64,
    pub data: String,
    #[serde(default)]
    pub count: usize,
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct UpdatesUi {
    pub sort: crate::SortKey,
    pub group: crate::GroupBy,
    pub filter: crate::Filter,
    pub project: Option<u64>,
    pub compact: bool,
    pub reverse: bool,
    pub statuses: Vec<Status>,
    pub priorities: Vec<u8>,
    pub date_filter: usize,
    #[serde(default)]
    pub tags: Vec<String>,
    pub range_start: Option<NaiveDate>,
    pub range_end: Option<NaiveDate>,
    pub collapsed: Vec<String>,
    pub presets: Vec<ViewPreset>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct ViewPreset {
    pub name: String,
    pub state: Box<UpdatesUi>,
}
