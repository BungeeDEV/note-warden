from pathlib import Path

def edit(path,f):
 p=Path(path);p.write_text(f(p.read_text(encoding='utf8')),encoding='utf8')
# Serializable persisted UI preferences.
for name,default in [('SortKey','Edited'),('GroupBy','None'),('Filter','All')]:
 edit('src/main.rs',lambda s,n=name,d=default:s.replace('#[derive(Clone, Copy, PartialEq, Eq)]\npub enum '+n+' {','#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\npub enum '+n+' {').replace('pub enum '+n+' {\n    '+d+',','pub enum '+n+' {\n    #[default]\n    '+d+','))
edit('src/main.rs',lambda s:s.replace('    Period,\n}', '    Period,\n    Project,\n    Priority,\n}').replace('pub const ALL: [Self; 3] = [Self::None, Self::Status, Self::Period];','pub const ALL: [Self; 5] = [Self::None, Self::Status, Self::Period, Self::Project, Self::Priority];').replace('Self::Period => "Nach Zeitraum",','Self::Period => "Nach Zeitraum",\n            Self::Project => "Nach Projekt",\n            Self::Priority => "Nach Priorität",'))
edit('src/model.rs',lambda s:s.replace('SCHEMA_VERSION: u32 = 5','SCHEMA_VERSION: u32 = 6').replace('pub struct Update {','''pub struct Update {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub activity: Vec<Activity>,''').replace('pub struct Settings {','''pub struct Settings {
    #[serde(default)]
    pub updates_ui: UpdatesUi,''').replace('self.updates.push(Update {','self.updates.push(Update {\n            version: String::new(), created_at: now_millis(), activity: Vec::new(),').replace('title: "Neues Update".into(),','title: String::new(),'))
edit('src/model.rs',lambda s:s+'''
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
    pub inspector_hidden: bool,
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
''')
edit('src/storage.rs',lambda s:s.replace('let label = if from >= 4','let label = if from >= 5 { "0.6.0" } else if from >= 4'))
# UI token completion and primitive behavior.
edit('src/palette.rs',lambda s:s+'''
pub const DETAIL_TITLE: f32 = 28.;
pub const INSPECTOR_W: f32 = 288.;
pub const DETAIL_MAX_W: f32 = 760.;
pub const MENU_RADIUS: f32 = 10.;
pub const MIN_HIT: f32 = 32.;
pub const CONTROL_PAD_SM: f32 = 10.;
pub const CONTROL_PAD_MD: f32 = 12.;
pub const CONTROL_PAD_LG: f32 = 16.;
''')
edit('src/ui/kit.rs',lambda s:s.replace('.font_weight(FontWeight::MEDIUM)\n        .tooltip_show_delay', '.font_weight(FontWeight::MEDIUM)\n        .gap(px(palette::ICON_GAP))\n        .px(px(palette::CONTROL_PAD_MD))\n        .tooltip_show_delay'))
# App runtime-only workbench state.
edit('src/main.rs',lambda s:s.replace('pub struct NoteWarden {','''pub struct NoteWarden {
    pub history: ui::workbench::History,
    pub drafts: std::collections::BTreeMap<u64, String>,
    pub detail_offsets: std::collections::BTreeMap<u64, Point<Pixels>>,
    pub list_scroll: ScrollHandle,
    pub detail_tab: usize,
    pub changelog_done_only: bool,
    pub focus_mode: bool,
    pub content_width: f32,
    pub task_grouped: bool,
    pub completion_dismissed: Option<u64>,
    pub version_input: Entity<InputState>,''').replace('        let mut app = Self {','''        let history = ui::workbench::History::new(&workspace.updates);
        let version_input = cx.new(|cx| InputState::new(window, cx).placeholder("Version"));
        let version_sub = cx.subscribe_in(&version_input, window, |this, input, event, _, cx| {
            if matches!(event, InputEvent::Change) && this.writable {
                let value = input.read(cx).value().to_string();
                if let Some(u) = this.selected_update.and_then(|id| this.workspace.update_mut(id)) && u.version != value {
                    u.version = value; u.touch(); this.persist(cx); cx.notify();
                }
            }
        });
        let mut app = Self {
            history, drafts: Default::default(), detail_offsets: Default::default(),
            list_scroll: ScrollHandle::new(), detail_tab: 0, changelog_done_only: true,
            focus_mode: false, content_width: 0., task_grouped: false, completion_dismissed: None, version_input,''').replace('        app.sync_fields(window, cx);','        app._subscriptions.push(version_sub);\n        app.sort = app.workspace.settings.updates_ui.sort;\n        app.group = app.workspace.settings.updates_ui.group;\n        app.filter = app.workspace.settings.updates_ui.filter;\n        app.project_filter = app.workspace.settings.updates_ui.project;\n        app.sync_fields(window, cx);',1))
# Capture mutations once at persistence boundary; notes use their own editor history.
edit('src/main.rs',lambda s:s.replace('        self.dirty = true;\n        self.last_edit', '        self.record_history();\n        self.dirty = true;\n        self.last_edit',1).replace('        let notes = update.map(|u| u.notes.clone()).unwrap_or_default();','        let notes = update.map(|u| u.notes.clone()).unwrap_or_default();\n        let version = update.map(|u| u.version.clone()).unwrap_or_default();\n        self.version_input.update(cx, |i,cx| i.set_value(version, window,cx));').replace('        self.detail_scroll.set_offset(point(px(0.),px(0.)));','''        self.discard_untouched_drafts(Some(id), cx);
        if let Some(previous) = self.selected_update { self.detail_offsets.insert(previous, self.detail_scroll.offset()); }
        self.detail_scroll.set_offset(self.detail_offsets.get(&id).copied().unwrap_or(point(px(0.),px(0.))));
        self.detail_tab = 0;''').replace('    pub fn set_view(&mut self, view: View, cx: &mut Context<Self>) {','    pub fn set_view(&mut self, view: View, cx: &mut Context<Self>) {\n        self.discard_untouched_drafts(None, cx);').replace('        let id = self.workspace.new_update_from(date, template.as_ref());','''        self.discard_untouched_drafts(None, cx);
        let id = self.workspace.new_update_from(date, template.as_ref());
        if template.is_none() { self.drafts.insert(id, Self::draft_signature(self.workspace.update(id).unwrap())); }
        self.detail_tab = 0;''').replace('            let previous = update.status;','            if update.status == status { return; }\n            let previous = update.status;',1))
# Whole-app local elapsed time.
edit('src/dates.rs',lambda s:s+'''
/// Kalenderwochen immer aus einem lokalen Kalendertag berechnen.
pub fn week_start(date: NaiveDate, start: crate::model::WeekStart) -> NaiveDate {
    date - Duration::days(start.offset(date.weekday()))
}
pub fn elapsed(at: i64) -> String { elapsed_at(at, crate::model::now_millis()) }
pub fn elapsed_at(at: i64, now: i64) -> String {
    use chrono::{Local, TimeZone};
    if at <= 0 { return "Zeit unbekannt".into(); }
    let minutes = ((now - at) / 60_000).max(0);
    if minutes == 0 { return "gerade eben".into(); }
    if minutes < 60 { return format!("vor {minutes} Min."); }
    let Some(date) = Local.timestamp_millis_opt(at).single() else { return "Zeit unbekannt".into(); };
    let Some(current) = Local.timestamp_millis_opt(now).single() else { return "Zeit unbekannt".into(); };
    let days = (current.date_naive() - date.date_naive()).num_days();
    if days == 1 { return "gestern".into(); }
    if minutes < 1440 { return format!("vor {} Std.", minutes / 60); }
    if days < 7 { return format!("vor {days} Tagen"); }
    format!("{}. {}", date.day(), month_short(date.date_naive()))
}
''')
edit('src/ui/today.rs',lambda s:s[:s.index('fn elapsed_label(')]+'''fn elapsed_label(edited: i64) -> String { dates::elapsed(edited) }
''')
edit('src/ui/today.rs',lambda s:s.replace('.filter(|u| u.is_live() && u.status != Status::Erledigt && u.planned_for.is_none())','.filter(|u| u.is_live() && !matches!(u.status, Status::Erledigt | Status::InArbeit) && u.planned_for.is_none())').replace('.child(text_secondary(\n                    &p,\n                    format!(','.when(!cards[6], |d| d.child(text_secondary(\n                    &p,\n                    format!(',1).replace('                )),\n        );\n        if cards[6]', '                ))),\n        );\n        if cards[6]',1))
