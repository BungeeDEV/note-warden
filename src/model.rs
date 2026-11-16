use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Weekday};
use serde::{Deserialize, Serialize};

/// Aktuelle Schemaversion der Arbeitsbereich-Datei. Dateien ohne Version
/// (0.1 / 0.2) werden beim Laden migriert, vorher wird ein Backup angelegt.
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

/// Erinnerung (lokale Zeit). Alles optional; Fälligkeitsdaten allein erinnern nicht.
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
    /// Neue Erinnerung setzen; frühere Meldung und Schlummern gelten nicht mehr.
    pub fn set(&mut self, at: Option<NaiveDateTime>) {
        *self = Self {
            at,
            reminded: None,
            snoozed: None,
        };
    }

    /// Nächster Meldezeitpunkt, falls noch nicht gemeldet.
    pub fn due_at(&self) -> Option<NaiveDateTime> {
        if self.reminded.is_some() {
            return None;
        }
        self.snoozed.or(self.at)
    }
}

/// Veröffentlichter Stand eines Updates. Der Changelog-Text ist ohne Versions-Header gespeichert.
#[derive(Clone, Deserialize, Serialize)]
pub struct Release {
    pub version: String,
    pub changelog: String,
    /// Unix-Millisekunden der Veröffentlichung.
    pub at: i64,
    /// Status vor der Veröffentlichung (für "Veröffentlichung zurücknehmen").
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
    /// Unteraufgabe der vorhergehenden Hauptaufgabe (eine Ebene tief).
    #[serde(default)]
    pub sub: bool,
    #[serde(default)]
    pub kind: ChangeKind,
    #[serde(default)]
    pub description: String,
    #[serde(default, flatten)]
    pub remind: Remind,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Update {
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
    /// Gesetzt = im Papierkorb (wird nach `TRASH_DAYS` Tagen endgültig entfernt).
    #[serde(default)]
    pub deleted_on: Option<NaiveDate>,
    /// Unix-Millisekunden der letzten Bearbeitung (0 = unbekannt).
    #[serde(default)]
    pub edited_at: i64,
    #[serde(default)]
    pub release: Option<Release>,
    #[serde(default, flatten)]
    pub remind: Remind,
}

impl Update {
    /// Fortschritt über alle Aufgaben inklusive Unteraufgaben.
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

    /// Ist der Termin des Updates überfällig?
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
    pub updates_ui: UpdatesUi,
    // Allgemein
    #[serde(default)]
    pub start_view: StartView,
    #[serde(default = "yes")]
    pub undo_instead_of_dialog: bool,
    /// Tage im Papierkorb; 0 = nie automatisch leeren.
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

    // Layout-Gedächtnis
    #[serde(default = "default_list_width")]
    pub list_width: f32,
    #[serde(default = "yes")]
    pub status_group_open: bool,
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
        // Einzige Quelle der Standardwerte: serde-Defaults.
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

#[derive(Default, Deserialize, Serialize)]
pub struct Workspace {
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub standalone_tasks: Vec<StandaloneTask>,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub updates: Vec<Update>,
    #[serde(default = "first_id")]
    next_id: u64,
    #[serde(default)]
    pub settings: Settings,
}

fn first_id() -> u64 {
    1
}

impl Workspace {
    pub fn new_update(&mut self, date: Option<NaiveDate>) -> u64 {
        let id = self.next_id();
        self.updates.push(Update {
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

    /// Kopie eines Updates mit Aufgaben, ohne Datum.
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
            .fold(max_id, u64::max);
        self.next_id = self.next_id.max(max_id.saturating_add(1));
    }

    /// Hebt ältere Schemas auf die aktuelle Version. Gibt `true` zurück,
    /// wenn etwas geändert wurde (Aufrufer sichert dann vorher die Datei).
    pub fn migrate(&mut self) -> bool {
        if self.schema_version >= SCHEMA_VERSION {
            return false;
        }
        // Neue Einstellungsfelder füllt serde mit Defaults; hier nur Wertebereiche prüfen.
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

    /// Setzt fehlende Erstellzeiten auf den Änderungszeitpunkt der Datendatei.
    pub fn fill_created_at(&mut self, fallback: i64) {
        for u in &mut self.updates {
            if u.created_at <= 0 {
                u.created_at = fallback;
            }
        }
    }

    /// Entfernt Papierkorb-Einträge, die älter als `TRASH_DAYS` sind.
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

    /// Fügt Updates aus einem Backup hinzu (neue IDs). Identische Einträge
    /// (Titel, Termin, Notizen) werden übersprungen. Gibt (neu, übersprungen) zurück.
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
