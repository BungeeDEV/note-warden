//! Shared change history and interaction of the updates workspace.
use super::*;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::popover::{Popover, PopoverState};

#[derive(Default)]
pub struct History {
    baseline: Vec<model::Update>,
    undo: Vec<Vec<model::Update>>,
    redo: Vec<Vec<model::Update>>,
    applying: bool,
    last_title: Option<Instant>,
}
impl History {
    pub fn new(updates: &[model::Update]) -> Self {
        Self {
            baseline: updates.to_vec(),
            ..Default::default()
        }
    }
}
fn comparable(updates: &[model::Update]) -> serde_json::Value {
    let mut value = serde_json::to_value(updates).unwrap_or_default();
    if let Some(items) = value.as_array_mut() {
        for u in items {
            if let Some(o) = u.as_object_mut() {
                for k in ["notes", "edited_at", "activity"] {
                    o.remove(k);
                }
            }
        }
    }
    value
}
impl NoteWarden {
    pub fn record_history(&mut self) {
        let before = &self.history.baseline;
        let changed = comparable(before) != comparable(&self.workspace.updates);
        if changed && !self.history.applying {
            let title_only = before.len() == self.workspace.updates.len()
                && before.iter().zip(&self.workspace.updates).all(|(a, b)| {
                    let mut a = a.clone();
                    a.title = b.title.clone();
                    comparable(&[a]) == comparable(std::slice::from_ref(b))
                });
            if !(title_only
                && self
                    .history
                    .last_title
                    .is_some_and(|t| t.elapsed() < StdDuration::from_millis(800)))
            {
                self.history.undo.push(before.clone());
                if self.history.undo.len() > 50 {
                    self.history.undo.remove(0);
                }
            }
            self.history.last_title = title_only.then(Instant::now);
            self.history.redo.clear();
            let now = model::now_millis();
            for u in &mut self.workspace.updates {
                let old = before.iter().find(|x| x.id == u.id);
                let (kind, text) = match old {
                    None => ("created", "Update erstellt".to_owned()),
                    Some(a) if a.status != u.status => (
                        "status",
                        format!("Status: {} → {}", a.status.label(), u.status.label()),
                    ),
                    Some(a) if a.planned_for != u.planned_for => (
                        "date",
                        format!(
                            "Datum: {}",
                            u.planned_for
                                .map(|d| d.to_string())
                                .unwrap_or("Ohne Termin".into())
                        ),
                    ),
                    Some(a) if a.project_id != u.project_id => {
                        ("project", "Projekt geändert".into())
                    }
                    Some(a) if a.priority != u.priority => {
                        ("priority", "Priorität geändert".into())
                    }
                    Some(a) if a.tasks.len() != u.tasks.len() => {
                        ("tasks", "Aufgaben hinzugefügt oder entfernt".into())
                    }
                    Some(a) if a.tasks.iter().zip(&u.tasks).any(|(a, b)| a.done != b.done) => {
                        ("completed", "Aufgabenstatus geändert".into())
                    }
                    Some(a) if a.title != u.title => ("title", "Titel bearbeitet".into()),
                    Some(a) if a.version != u.version => ("version", "Version geändert".into()),
                    _ => continue,
                };
                if let Some(last) = u.activity.last_mut()
                    && last.kind == kind
                    && now - last.at < 600_000
                {
                    last.at = now;
                    last.count += 1;
                    last.data = text;
                } else {
                    u.activity.push(model::Activity {
                        kind: kind.into(),
                        at: now,
                        data: text,
                        count: 1,
                    });
                }
                if u.activity.len() > 200 {
                    u.activity.drain(..u.activity.len() - 200);
                }
            }
        }
        self.history.baseline = self.workspace.updates.clone();
    }
    pub fn history_step(&mut self, redo: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let target = if redo {
            self.history.redo.pop()
        } else {
            self.history.undo.pop()
        };
        let Some(mut target) = target else {
            return;
        };
        // The note text keeps its own editor history.
        for u in &mut target {
            if let Some(current) = self.workspace.update(u.id) {
                u.notes = current.notes.clone();
            }
        }
        let current = std::mem::replace(&mut self.workspace.updates, target);
        if redo {
            self.history.undo.push(current);
        } else {
            self.history.redo.push(current);
        }
        self.history.applying = true;
        self.persist(cx);
        self.history.applying = false;
        if self
            .selected_update
            .is_some_and(|id| self.workspace.update(id).is_none())
        {
            self.selected_update = self
                .workspace
                .updates
                .iter()
                .find(|u| u.is_live())
                .map(|u| u.id);
        }
        self.sync_fields(window, cx);
        self.toast(
            if redo {
                "Änderung wiederholt"
            } else {
                "Änderung rückgängig gemacht"
            },
            None,
            cx,
        );
        cx.notify();
    }
    pub fn draft_signature(u: &model::Update) -> String {
        let mut u = u.clone();
        u.edited_at = 0;
        u.activity.clear();
        serde_json::to_string(&u).unwrap_or_default()
    }
    pub fn discard_untouched_drafts(&mut self, keep: Option<u64>, cx: &mut Context<Self>) {
        let remove: Vec<_> = self
            .drafts
            .iter()
            .filter_map(|(id, signature)| {
                if Some(*id) == keep {
                    return None;
                }
                self.workspace
                    .update(*id)
                    .filter(|u| u.title.trim().is_empty() && Self::draft_signature(u) == *signature)
                    .map(|_| *id)
            })
            .collect();
        if remove.is_empty() {
            return;
        }
        self.workspace.updates.retain(|u| !remove.contains(&u.id));
        for id in remove {
            self.drafts.remove(&id);
            if self.selected_update == Some(id) {
                self.selected_update = None;
            }
        }
        self.persist(cx);
    }
    pub fn save_updates_ui(&mut self, cx: &mut Context<Self>) {
        let ui = &mut self.workspace.settings.updates_ui;
        ui.sort = self.sort;
        ui.group = self.group;
        ui.filter = self.filter;
        ui.project = self.project_filter;
        self.persist(cx);
        cx.notify();
    }
    pub fn status_control(&self, u: &model::Update, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let id = u.id;
        let e = cx.entity();
        kit::button(("status-control", id))
            .ghost()
            .with_size(gpui_kit::component::Size::Small)
            .size(px(palette::ROW_LEADING_SLOT))
            .p(px(0.))
            .child(kit::status_icon(&p, u.status, u.task_progress()))
            .tooltip(format!("Status ändern: {} · 1–4", u.status.label()))
            .accessibility_label("Status ändern")
            .dropdown_menu(move |menu, _, _| {
                Status::ALL.into_iter().fold(menu, |m, status| {
                    let e = e.clone();
                    m.item(
                        PopupMenuItem::new(status.label()).on_click(move |_, _, cx| {
                            e.update(cx, |a, cx| a.set_status(id, status, cx))
                        }),
                    )
                })
            })
    }
    /// Space in the update list: marks the selected update done, or reopens it.
    pub fn toggle_selected_done(&mut self, cx: &mut Context<Self>) {
        if let Some(update) = self.selected() {
            let (id, next) = if update.status == Status::Erledigt {
                (update.id, Status::Idee)
            } else {
                (update.id, Status::Erledigt)
            };
            self.set_status(id, next, cx);
        }
    }

    pub fn select_relative(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<_> = self.visible_updates().into_iter().map(|u| u.id).collect();
        if ids.is_empty() {
            return;
        }
        let index = ids
            .iter()
            .position(|id| Some(*id) == self.selected_update)
            .map(|i| (i as isize + delta).clamp(0, ids.len() as isize - 1) as usize)
            .unwrap_or(0);
        self.select_update(ids[index], window, cx);
    }
    pub fn update_keys(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = ev.keystroke.key.as_str();
        let mods = &ev.keystroke.modifiers;
        if mods.control && key == "a" {
            self.multi = self.visible_updates().iter().map(|u| u.id).collect();
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if mods.control && key == "d" {
            if let Some(id) = self.selected_update {
                self.duplicate_update(id, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if mods.control || mods.alt || mods.platform {
            return;
        }
        match key {
            "down" | "j" => self.select_relative(1, window, cx),
            "up" | "k" => self.select_relative(-1, window, cx),
            "enter" => {
                self.detail_open = true;
                self.title_input.update(cx, |i, cx| i.focus(window, cx));
            }
            "space" => self.toggle_selected_done(cx),
            "1" | "2" | "3" | "4" => {
                if let Some(id) = self.selected_update {
                    self.set_status(id, Status::ALL[key.parse::<usize>().unwrap() - 1], cx);
                }
            }
            "x" => {
                if let Some(id) = self.selected_update {
                    if !self.multi.remove(&id) {
                        self.multi.insert(id);
                    }
                    cx.notify();
                }
            }
            "s" => {
                if let Some(id) = self.selected_update {
                    self.keyboard_menu = Some((id, 0));
                    self.keyboard_index = 0;
                    self.overlay_focus.focus(window, cx);
                    cx.notify();
                }
            }
            "p" => {
                if let Some(id) = self.selected_update {
                    self.keyboard_menu = Some((id, 1));
                    self.keyboard_index = 0;
                    self.overlay_focus.focus(window, cx);
                    cx.notify();
                }
            }
            "l" => {
                if let Some(id) = self.selected_update {
                    self.keyboard_menu = Some((id, 2));
                    self.keyboard_index = 0;
                    self.overlay_focus.focus(window, cx);
                    cx.notify();
                }
            }
            "/" => self.focus_search(window, cx),
            "d" => {
                if let Some(id) = self.selected_update {
                    self.open_date_pop(DateTarget::Update(id), window.mouse_position(), window, cx);
                }
            }
            _ => return,
        }
        cx.stop_propagation();
    }
}

/// Counters for the filter lists in the popover (only active, non-deleted updates).
struct ViewCounts {
    status: [usize; 4],
    priority: [usize; 4],
    date: [usize; 5],
    projects: Vec<(u64, String, usize)>,
}

const GROUP_ORDER: [(GroupBy, &str); 5] = [
    (GroupBy::None, "Keine"),
    (GroupBy::Status, "Status"),
    (GroupBy::Project, "Projekt"),
    (GroupBy::Period, "Termin"),
    (GroupBy::Priority, "Priorität"),
];
const PRIORITY_LABELS: [&str; 4] = ["Keine", "Niedrig", "Mittel", "Hoch"];
const DATE_LABELS: [&str; 5] = [
    "Überfällig",
    "Heute",
    "Diese Woche",
    "Nächste 7 Tage",
    "Ohne Termin",
];

type StateFn = Box<dyn Fn(&mut NoteWarden, &mut Context<NoteWarden>)>;

/// Click handler in the filter popover: changes the state and re-renders the popover.
fn pop_click(
    app: &Entity<NoteWarden>,
    pop: &Entity<PopoverState>,
    f: StateFn,
) -> impl Fn(&mut Window, &mut App) + 'static {
    let (app, pop) = (app.clone(), pop.clone());
    move |_, cx| {
        app.update(cx, |a, cx| f(a, cx));
        pop.update(cx, |_, cx| cx.notify());
    }
}

/// Row with a radio or checkbox display in the popover.
fn option_row(
    id: SharedString,
    p: &Palette,
    label: impl Into<SharedString>,
    count: Option<usize>,
    selected: bool,
    check: bool,
    on: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui_kit::base::Button {
    let mark = if check {
        div()
            .size(px(palette::ICON_MD))
            .flex_none()
            .rounded(px(palette::R_XS))
            .border_1()
            .flex()
            .items_center()
            .justify_center()
            .border_color(if selected { p.accent } else { p.border_strong })
            .when(selected, |d| {
                d.bg(p.accent).child(
                    kit::icon(icons::CONFIRM, palette::ICON_XS).text_color(p.accent_contrast),
                )
            })
    } else {
        div()
            .size(px(palette::ICON_MD))
            .flex_none()
            .rounded_full()
            .border_1()
            .flex()
            .items_center()
            .justify_center()
            .border_color(if selected { p.accent } else { p.border_strong })
            .when(selected, |d| {
                d.child(div().size(px(8.)).rounded_full().bg(p.accent))
            })
    };
    kit::surface(id, p)
        .role(if check {
            Role::CheckBox
        } else {
            Role::RadioButton
        })
        .selected(selected)
        .w_full()
        .h(px(palette::CTL_MD))
        .px(px(palette::SP_2))
        .rounded(px(palette::R_CONTROL_SM))
        .flex()
        .items_center()
        .justify_start()
        .gap(px(palette::SP_2))
        .text_size(palette::fs(palette::FS_BASE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(if selected {
            p.text_primary
        } else {
            p.text_secondary
        })
        .child(mark)
        .child(div().flex_1().min_w_0().truncate().child(label.into()))
        .when_some(count, |d, n| {
            d.child(
                div()
                    .text_size(palette::fs(palette::FS_SM))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(p.text_tertiary)
                    .child(n.to_string()),
            )
        })
        .on_click(move |_, w, cx| on(w, cx))
}

fn section_title(p: &Palette, text: &'static str) -> Div {
    div()
        .text_size(palette::fs(palette::FS_SM))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(p.text_tertiary)
        .child(text)
}

impl NoteWarden {
    fn view_counts(&self) -> ViewCounts {
        let today = self.now_today();
        let week_end = self.week_end(today);
        let live: Vec<&model::Update> = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.is_live())
            .collect();
        let count = |f: &dyn Fn(&model::Update) -> bool| live.iter().filter(|u| f(u)).count();
        ViewCounts {
            status: Status::ALL.map(|s| count(&|u| u.status == s)),
            priority: [0u8, 1, 2, 3].map(|n| count(&|u| u.priority.min(3) == n)),
            date: [
                count(&|u| u.overdue(today)),
                count(&|u| u.planned_for == Some(today)),
                count(&|u| u.planned_for.is_some_and(|d| d >= today && d <= week_end)),
                count(&|u| {
                    u.planned_for
                        .is_some_and(|d| d >= today && d < today + Duration::days(7))
                }),
                count(&|u| u.planned_for.is_none()),
            ],
            projects: self
                .workspace
                .projects
                .iter()
                .filter(|p| !p.archived)
                .map(|p| (p.id, p.name.clone(), count(&|u| u.project_id == Some(p.id))))
                .collect(),
        }
    }

    pub fn active_filter_count(&self) -> usize {
        let ui = &self.workspace.settings.updates_ui;
        ui.statuses.len()
            + ui.priorities.len()
            + usize::from(ui.date_filter > 0)
            + usize::from(self.project_filter.is_some())
            + ui.tags.len()
    }

    fn view_customized(&self) -> bool {
        let ui = &self.workspace.settings.updates_ui;
        self.sort != SortKey::Edited || self.group != GroupBy::None || ui.compact || ui.reverse
    }

    pub fn reset_filters(&mut self, cx: &mut Context<Self>) {
        let ui = &mut self.workspace.settings.updates_ui;
        ui.statuses.clear();
        ui.priorities.clear();
        ui.date_filter = 0;
        ui.tags.clear();
        ui.range_start = None;
        ui.range_end = None;
        self.project_filter = None;
        self.save_updates_ui(cx);
    }

    fn reset_view(&mut self, cx: &mut Context<Self>) {
        let ui = &mut self.workspace.settings.updates_ui;
        ui.compact = false;
        ui.reverse = false;
        self.sort = SortKey::Edited;
        self.group = GroupBy::None;
        self.reset_filters(cx);
    }

    /// One icon button "Filter & Ansicht" (filter & view) with a popover (sorting, grouping, display, filter).
    pub fn view_popover(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let app = cx.entity();
        let active = self.active_filter_count();
        let customized = self.view_customized();
        let trigger =
            kit::toned_button("list-view", &p, kit::ButtonTone::Secondary, palette::CTL_MD)
                .w(px(palette::CTL_MD))
                .px(px(0.))
                .relative()
                .tooltip("Filter & Ansicht")
                .accessibility_label("Filter & Ansicht")
                .child(kit::icon(icons::FILTER_VIEW, palette::ICON_MD))
                .when(active > 0, |b| {
                    b.child(
                        div()
                            .absolute()
                            .top(px(-4.))
                            .right(px(-4.))
                            .min_w(px(16.))
                            .h(px(16.))
                            .px(px(4.))
                            .rounded_full()
                            .bg(p.accent)
                            .text_color(p.accent_contrast)
                            .text_size(px(10.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(active.to_string()),
                    )
                })
                .when(active == 0 && customized, |b| {
                    b.child(
                        div()
                            .absolute()
                            .top(px(4.))
                            .right(px(4.))
                            .size(px(8.))
                            .rounded_full()
                            .bg(p.accent),
                    )
                });
        Popover::new("view-popover")
            .anchor(Anchor::TopRight)
            .trigger(trigger)
            .w(px(320.))
            .content(move |_, window, cx| {
                let pop = cx.entity();
                let p = palette::get(cx);
                let (sort, group, ui, counts, open, customized, project) = {
                    let a = app.read(cx);
                    (
                        a.sort,
                        a.group,
                        a.workspace.settings.updates_ui.clone(),
                        a.view_counts(),
                        a.view_groups_open,
                        a.view_customized(),
                        a.project_filter,
                    )
                };
                let click = |f: StateFn| pop_click(&app, &pop, f);
                let mut body = div().flex().flex_col().gap(px(palette::SP_4)).p(px(4.));

                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(section_title(&p, "Sortieren nach"))
                        .children(SortKey::ALL.map(|key| {
                            option_row(
                                SharedString::from(format!("sort-{}", key as usize)),
                                &p,
                                key.label(),
                                None,
                                sort == key,
                                false,
                                click(Box::new(move |a, cx| {
                                    a.sort = key;
                                    if key == SortKey::Manual {
                                        if a.writable {
                                            a.workspace.ensure_manual_order();
                                        }
                                        a.group = GroupBy::None;
                                        a.workspace.settings.updates_ui.reverse = false;
                                    }
                                    a.save_updates_ui(cx);
                                })),
                            )
                        }))
                        .when(sort != SortKey::Manual, |d| {
                            d.child(div().mt(px(palette::SP_1)).child(kit::segment(
                                "sort-dir",
                                &p,
                                &[(None, "Aufsteigend"), (None, "Absteigend")],
                                usize::from(!ui.reverse),
                                true,
                                {
                                    let (app, pop) = (app.clone(), pop.clone());
                                    move |i, _, cx| {
                                        app.update(cx, |a, cx| {
                                            a.workspace.settings.updates_ui.reverse = i == 0;
                                            a.persist(cx);
                                            cx.notify();
                                        });
                                        pop.update(cx, |_, cx| cx.notify());
                                    }
                                },
                            )))
                        }),
                );
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(section_title(&p, "Gruppieren nach"))
                        .children(GROUP_ORDER.map(|(key, label)| {
                            option_row(
                                SharedString::from(format!("group-{}", key as usize)),
                                &p,
                                label,
                                None,
                                group == key,
                                false,
                                click(Box::new(move |a, cx| {
                                    a.group = key;
                                    a.save_updates_ui(cx);
                                })),
                            )
                        })),
                );
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(8.))
                        .child(section_title(&p, "Anzeige"))
                        .child(kit::segment(
                            "density",
                            &p,
                            &[(None, "Komfortabel"), (None, "Kompakt")],
                            usize::from(ui.compact),
                            true,
                            {
                                let (app, pop) = (app.clone(), pop.clone());
                                move |i, _, cx| {
                                    app.update(cx, |a, cx| {
                                        a.workspace.settings.updates_ui.compact = i == 1;
                                        a.persist(cx);
                                        cx.notify();
                                    });
                                    pop.update(cx, |_, cx| cx.notify());
                                }
                            },
                        )),
                );

                // Filter: vier einklappbare Gruppen.
                let mut filter = div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(section_title(&p, "Filter"));
                let titles = ["Status", "Projekt", "Priorität", "Datum"];
                let active_in = [
                    ui.statuses.len(),
                    usize::from(project.is_some()),
                    ui.priorities.len(),
                    usize::from(ui.date_filter > 0),
                ];
                for g in 0..4 {
                    let expanded = open[g];
                    let toggle = click(Box::new(move |a, cx| {
                        a.view_groups_open[g] = !a.view_groups_open[g];
                        cx.notify();
                    }));
                    filter = filter.child(
                        kit::surface(SharedString::from(format!("view-group-{g}")), &p)
                            .w_full()
                            .h(px(palette::CTL_MD))
                            .px(px(palette::SP_1))
                            .rounded(px(palette::R_CONTROL_SM))
                            .flex()
                            .items_center()
                            .justify_start()
                            .gap(px(4.))
                            .text_size(palette::fs(palette::FS_BASE))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text_primary)
                            .child(
                                kit::icon(
                                    if expanded {
                                        icons::EXPAND
                                    } else {
                                        icons::COLLAPSE
                                    },
                                    palette::ICON_SM,
                                )
                                .text_color(p.text_tertiary),
                            )
                            .child(div().flex_1().child(titles[g]))
                            .when(active_in[g] > 0, |b| {
                                b.child(
                                    div()
                                        .text_size(palette::fs(palette::FS_SM))
                                        .text_color(p.accent)
                                        .child(active_in[g].to_string()),
                                )
                            })
                            .on_click(move |_, w, cx| toggle(w, cx)),
                    );
                    if !expanded {
                        continue;
                    }
                    let mut list = div().pl(px(palette::SP_4)).flex().flex_col().gap(px(2.));
                    match g {
                        0 => {
                            for (i, s) in Status::ALL.into_iter().enumerate() {
                                let on = ui.statuses.contains(&s);
                                list = list.child(option_row(
                                    SharedString::from(format!("f-status-{i}")),
                                    &p,
                                    s.label(),
                                    Some(counts.status[i]),
                                    on,
                                    true,
                                    click(Box::new(move |a, cx| {
                                        let v = &mut a.workspace.settings.updates_ui.statuses;
                                        if v.contains(&s) {
                                            v.retain(|x| *x != s);
                                        } else {
                                            v.push(s);
                                        }
                                        a.persist(cx);
                                        cx.notify();
                                    })),
                                ));
                            }
                        }
                        1 => {
                            if counts.projects.is_empty() {
                                list = list.child(
                                    div()
                                        .px(px(palette::SP_2))
                                        .py(px(palette::SP_1))
                                        .text_size(palette::fs(palette::FS_SM))
                                        .text_color(p.text_tertiary)
                                        .child("Keine Projekte"),
                                );
                            }
                            for (id, name, n) in counts.projects.clone() {
                                list = list.child(option_row(
                                    SharedString::from(format!("f-project-{id}")),
                                    &p,
                                    name,
                                    Some(n),
                                    project == Some(id),
                                    true,
                                    click(Box::new(move |a, cx| {
                                        a.project_filter = if a.project_filter == Some(id) {
                                            None
                                        } else {
                                            Some(id)
                                        };
                                        a.save_updates_ui(cx);
                                    })),
                                ));
                            }
                        }
                        2 => {
                            for n in 0..4u8 {
                                let on = ui.priorities.contains(&n);
                                list = list.child(option_row(
                                    SharedString::from(format!("f-prio-{n}")),
                                    &p,
                                    PRIORITY_LABELS[n as usize],
                                    Some(counts.priority[n as usize]),
                                    on,
                                    true,
                                    click(Box::new(move |a, cx| {
                                        let v = &mut a.workspace.settings.updates_ui.priorities;
                                        if v.contains(&n) {
                                            v.retain(|x| *x != n);
                                        } else {
                                            v.push(n);
                                        }
                                        a.persist(cx);
                                        cx.notify();
                                    })),
                                ));
                            }
                        }
                        _ => {
                            for (i, label) in DATE_LABELS.into_iter().enumerate() {
                                let value = i + 1;
                                list = list.child(option_row(
                                    SharedString::from(format!("f-date-{value}")),
                                    &p,
                                    label,
                                    Some(counts.date[i]),
                                    ui.date_filter == value,
                                    true,
                                    click(Box::new(move |a, cx| {
                                        let ui = &mut a.workspace.settings.updates_ui;
                                        ui.date_filter =
                                            if ui.date_filter == value { 0 } else { value };
                                        a.persist(cx);
                                        cx.notify();
                                    })),
                                ));
                            }
                        }
                    }
                    filter = filter.child(list);
                }
                body = body.child(filter);

                let any_filter = !ui.statuses.is_empty()
                    || !ui.priorities.is_empty()
                    || ui.date_filter > 0
                    || project.is_some();
                let reset = click(Box::new(|a, cx| a.reset_view(cx)));
                let save = click(Box::new(|a, cx| a.save_current_view(cx)));
                let max_h = window.bounds().size.height - px(140.);
                div()
                    .w_full()
                    .max_h(max_h)
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("view-pop-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .child(body),
                    )
                    .child(
                        div()
                            .flex_none()
                            .pt(px(palette::SP_2))
                            .mt(px(palette::SP_2))
                            .border_t_1()
                            .border_color(p.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                kit::button("view-reset")
                                    .ghost()
                                    .label("Zurücksetzen")
                                    .disabled(!customized && !any_filter)
                                    .on_click(move |_, w, cx| reset(w, cx)),
                            )
                            .child(
                                kit::toned_button(
                                    "view-save",
                                    &p,
                                    kit::ButtonTone::Secondary,
                                    palette::CTL_MD,
                                )
                                .label("Als Ansicht speichern …")
                                .on_click(move |_, w, cx| save(w, cx)),
                            ),
                    )
            })
    }

    fn save_current_view(&mut self, cx: &mut Context<Self>) {
        let mut state = self.workspace.settings.updates_ui.clone();
        state.presets.clear();
        state.sort = self.sort;
        state.group = self.group;
        state.filter = self.filter;
        state.project = self.project_filter;
        let number = self.workspace.settings.updates_ui.presets.len() + 1;
        self.workspace
            .settings
            .updates_ui
            .presets
            .push(model::ViewPreset {
                name: format!("Ansicht {number}"),
                state: Box::new(state),
            });
        self.persist(cx);
        self.toast(format!("Ansicht {number} gespeichert"), None, cx);
    }

    /// Single-line row of active filters under the search; without active filters it does not exist.
    pub fn filter_chips(&self, cx: &mut Context<Self>) -> Option<Div> {
        let p = palette::get(cx);
        let ui = &self.workspace.settings.updates_ui;
        let mut items: Vec<(String, StateFn)> = Vec::new();
        for s in ui.statuses.clone() {
            items.push((
                format!("Status: {}", s.label()),
                Box::new(move |a, cx| {
                    a.workspace.settings.updates_ui.statuses.retain(|x| *x != s);
                    a.persist(cx);
                    cx.notify();
                }),
            ));
        }
        for n in ui.priorities.clone() {
            items.push((
                format!("Priorität: {}", PRIORITY_LABELS[n.min(3) as usize]),
                Box::new(move |a, cx| {
                    a.workspace
                        .settings
                        .updates_ui
                        .priorities
                        .retain(|x| *x != n);
                    a.persist(cx);
                    cx.notify();
                }),
            ));
        }
        for tag in ui.tags.clone() {
            items.push((
                format!("Tag: #{tag}"),
                Box::new(move |a, cx| {
                    a.workspace
                        .settings
                        .updates_ui
                        .tags
                        .retain(|t| t.to_lowercase() != tag.to_lowercase());
                    a.save_updates_ui(cx);
                }),
            ));
        }
        if ui.date_filter > 0 {
            let label = if ui.date_filter <= 5 {
                DATE_LABELS[ui.date_filter - 1]
            } else {
                "Zeitraum"
            };
            items.push((
                format!("Datum: {label}"),
                Box::new(|a, cx| {
                    a.workspace.settings.updates_ui.date_filter = 0;
                    a.persist(cx);
                    cx.notify();
                }),
            ));
        }
        if let Some(name) = self
            .project_filter
            .and_then(|id| self.workspace.projects.iter().find(|p| p.id == id))
            .map(|p| p.name.clone())
        {
            items.push((
                format!("Projekt: {name}"),
                Box::new(|a, cx| {
                    a.project_filter = None;
                    a.save_updates_ui(cx);
                }),
            ));
        }
        if items.is_empty() {
            return None;
        }
        let clear = cx.entity();
        let chips: Vec<_> = items
            .into_iter()
            .enumerate()
            .map(|(n, (label, remove))| {
                let app = cx.entity();
                kit::surface(("filter-chip", n), &p)
                    .flex_none()
                    .h(px(palette::CHIP_MD))
                    .px(px(palette::CONTROL_PAD_SM))
                    .rounded_full()
                    .border_1()
                    .border_color(p.border_subtle)
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_1))
                    .text_size(palette::fs(palette::FS_CONTROL_SM))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text_secondary)
                    .accessibility_label(format!("Filter entfernen: {label}"))
                    .child(label)
                    .child(kit::icon(icons::CLOSE, palette::ICON_XS).text_color(p.text_tertiary))
                    .on_click(move |_, _, cx| app.update(cx, |a, cx| remove(a, cx)))
            })
            .collect();
        Some(
            div()
                .px(px(palette::SP_3))
                .pb(px(palette::SP_2))
                .flex()
                .items_center()
                .gap(px(palette::SP_2))
                .child(
                    div()
                        .id("filter-chips")
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_1))
                        .overflow_x_scroll()
                        .children(chips),
                )
                .child(
                    kit::button("filters-clear")
                        .ghost()
                        .with_size(gpui_kit::component::Size::Small)
                        .h(px(palette::CTL_SM))
                        .flex_none()
                        .label("Alle löschen")
                        .text_color(p.text_secondary)
                        .on_click(move |_, _, cx| clear.update(cx, |a, cx| a.reset_filters(cx))),
                ),
        )
    }
}

impl NoteWarden {
    fn apply_view(&mut self, state: &model::UpdatesUi, cx: &mut Context<Self>) {
        let ui = &mut self.workspace.settings.updates_ui;
        ui.statuses = state.statuses.clone();
        ui.priorities = state.priorities.clone();
        ui.tags = state.tags.clone();
        ui.date_filter = state.date_filter;
        ui.range_start = state.range_start;
        ui.range_end = state.range_end;
        ui.reverse = state.reverse;
        self.sort = state.sort;
        self.group = state.group;
        self.filter = state.filter;
        self.project_filter = state.project;
        self.save_updates_ui(cx);
    }

    fn view_matches(&self, state: &model::UpdatesUi) -> bool {
        let ui = &self.workspace.settings.updates_ui;
        let same =
            |a: &[Status], b: &[Status]| a.len() == b.len() && a.iter().all(|s| b.contains(s));
        self.sort == state.sort
            && self.group == state.group
            && same(&ui.statuses, &state.statuses)
            && ui.priorities.len() == state.priorities.len()
            && ui.priorities.iter().all(|n| state.priorities.contains(n))
            && ui.date_filter == state.date_filter
            && self.filter == state.filter
            && self.project_filter == state.project
            && ui.tags == state.tags
    }

    /// Saved views for the "Ansichten" (views) sidebar group.
    pub fn saved_view_items(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let p = palette::get(cx);
        self.workspace
            .settings
            .updates_ui
            .presets
            .iter()
            .enumerate()
            .map(|(n, preset)| {
                let (apply, remove) = (cx.entity(), cx.entity());
                let state = (*preset.state).clone();
                let active = self.view == View::Updates && self.view_matches(&state);
                div()
                    .id(("saved-view", n))
                    .child(
                        kit::surface(("saved-view-item", n), &p)
                            .w_full()
                            .h(px(palette::CTL_MD))
                            .px(px(palette::CONTROL_PAD_SM))
                            .rounded(px(palette::R_INPUT))
                            .flex()
                            .items_center()
                            .justify_start()
                            .gap(px(palette::SP_2))
                            .text_size(palette::fs(palette::FS_BASE))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if active {
                                p.text_primary
                            } else {
                                p.text_secondary
                            })
                            .when(active, |b| b.bg(p.bg_active))
                            .child(
                                kit::icon(icons::FILTER, palette::ICON_MD)
                                    .text_color(p.text_secondary),
                            )
                            .child(div().truncate().child(preset.name.clone()))
                            .on_click(move |_, _, cx| {
                                apply.update(cx, |a, cx| {
                                    a.view = View::Updates;
                                    a.apply_view(&state, cx);
                                })
                            }),
                    )
                    .context_menu(move |menu, _, _| {
                        let remove = remove.clone();
                        menu.item(PopupMenuItem::new("Löschen").on_click(move |_, _, cx| {
                            remove.update(cx, |a, cx| {
                                if n < a.workspace.settings.updates_ui.presets.len() {
                                    a.workspace.settings.updates_ui.presets.remove(n);
                                }
                                a.persist(cx);
                                cx.notify();
                            })
                        }))
                    })
                    .into_any_element()
            })
            .collect()
    }
}

impl NoteWarden {
    pub fn export_changelog(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(u) = self.workspace.update(id) {
            let text = md::format_changelog(None, &u.tasks, self.changelog_done_only);
            self.save_with_dialog(
                "CHANGELOG.md".into(),
                text.into_bytes(),
                "Changelog gespeichert",
                cx,
            );
        }
    }
}

impl NoteWarden {
    fn keyboard_options(&self) -> Vec<String> {
        match self.keyboard_menu.map(|(_, n)| n) {
            Some(0) => Status::ALL.iter().map(|s| s.label().to_owned()).collect(),
            Some(1) => ["Keine Priorität", "Niedrig", "Mittel", "Hoch"]
                .map(str::to_owned)
                .to_vec(),
            Some(2) => std::iter::once("Kein Projekt".to_owned())
                .chain(
                    self.workspace
                        .projects
                        .iter()
                        .filter(|p| !p.archived)
                        .map(|p| p.name.clone()),
                )
                .collect(),
            _ => vec![],
        }
    }
    fn keyboard_choose(&mut self, n: usize, w: &mut Window, cx: &mut Context<Self>) {
        let Some((id, kind)) = self.keyboard_menu.take() else {
            return;
        };
        if self.writable {
            match kind {
                0 => self.set_status(id, Status::ALL[n.min(3)], cx),
                1 => {
                    if let Some(u) = self.workspace.update_mut(id) {
                        u.priority = n.min(3) as u8;
                        u.touch();
                    }
                    self.persist(cx);
                }
                _ => {
                    let project = if n == 0 {
                        None
                    } else {
                        self.workspace
                            .projects
                            .iter()
                            .filter(|p| !p.archived)
                            .nth(n - 1)
                            .map(|p| p.id)
                    };
                    if let Some(u) = self.workspace.update_mut(id) {
                        u.project_id = project;
                        u.touch();
                    }
                    self.persist(cx);
                }
            }
        }
        self.focus_handle.focus(w, cx);
        cx.notify();
    }
    pub fn keyboard_popup(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.keyboard_menu.is_none() {
            return div().into_any_element();
        }
        let p = palette::get(cx);
        let options = self.keyboard_options();
        div()
            .id("update-keyboard-menu")
            .role(Role::Menu)
            .track_focus(&self.overlay_focus)
            .absolute()
            .top(px(96.))
            .left(px(240.))
            .w(px(240.))
            .p(px(palette::SP_2))
            .rounded(px(palette::MENU_RADIUS))
            .bg(p.bg_elevated)
            .border_1()
            .border_color(p.border_subtle)
            .shadow_lg()
            .flex()
            .flex_col()
            .on_key_down(cx.listener(|a, ev: &KeyDownEvent, w, cx| {
                let n = a.keyboard_options().len();
                match ev.keystroke.key.as_str() {
                    "down" => a.keyboard_index = (a.keyboard_index + 1) % n,
                    "up" => a.keyboard_index = (a.keyboard_index + n - 1) % n,
                    "enter" => a.keyboard_choose(a.keyboard_index, w, cx),
                    "1" | "2" | "3" | "4" => {
                        a.keyboard_choose(ev.keystroke.key.parse::<usize>().unwrap() - 1, w, cx)
                    }
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .children(options.into_iter().enumerate().map(|(n, label)| {
                let e = cx.entity();
                kit::button(("keyboard-option", n))
                    .ghost()
                    .role(Role::MenuItem)
                    .w_full()
                    .justify_start()
                    .label(label)
                    .when(n == self.keyboard_index, |b| b.bg(p.bg_active))
                    .on_click(move |_, w, cx| e.update(cx, |a, cx| a.keyboard_choose(n, w, cx)))
            }))
            .into_any_element()
    }
}
