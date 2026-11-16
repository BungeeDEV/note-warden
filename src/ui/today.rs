use super::kit::*;
use super::*;
use chrono::Timelike as _;

type RecentItem = (u64, String, Status, i64, (usize, usize));
use gpui_kit::component::menu::DropdownMenu as _;

#[derive(Clone, Copy)]
pub enum DueItem<'a> {
    Update(&'a model::Update),
    Task(&'a model::Update, &'a model::Task),
    Inbox(&'a model::StandaloneTask),
}

impl DueItem<'_> {
    pub fn date(&self) -> Option<NaiveDate> {
        match self {
            Self::Update(u) => u.planned_for,
            Self::Task(_, t) => t.planned_for,
            Self::Inbox(t) => t.date,
        }
    }

    pub fn key(&self) -> (u64, Option<u64>) {
        match self {
            Self::Update(u) => (u.id, None),
            Self::Task(u, t) => (u.id, Some(t.id)),
            Self::Inbox(t) => (0, Some(t.id)),
        }
    }
}

pub struct Due<'a> {
    pub overdue: Vec<DueItem<'a>>,
    pub today: Vec<DueItem<'a>>,
    pub week: Vec<DueItem<'a>>,
    pub later: Vec<DueItem<'a>>,
}

// Gruppen-Indizes in `today_collapsed`.
const G_OVERDUE: usize = 0;
const G_TODAY: usize = 1;
const G_WEEK: usize = 2;
const G_LATER: usize = 3;
const G_DONE: usize = 4;

impl NoteWarden {
    pub fn record_completion(&mut self, uid: u64, tid: Option<u64>, done: bool) {
        self.workspace
            .settings
            .home_completions
            .retain(|(u, t, _)| (*u, *t) != (uid, tid));
        if done {
            let day = self.now_today();
            self.workspace
                .settings
                .home_completions
                .push((uid, tid, day));
        }
    }
    fn completion_is_live(&self, uid: u64, tid: Option<u64>) -> bool {
        if uid == 0 {
            return self
                .workspace
                .standalone_tasks
                .iter()
                .any(|t| Some(t.id) == tid && t.done);
        }
        self.workspace.update(uid).is_some_and(|u| {
            u.is_live()
                && match tid {
                    Some(id) => u.tasks.iter().any(|t| t.id == id && t.done),
                    None => u.status == Status::Erledigt,
                }
        })
    }
    pub fn toggle_inbox(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(t) = self
            .workspace
            .standalone_tasks
            .iter_mut()
            .find(|t| t.id == id)
        {
            t.done = !t.done;
            let done = t.done;
            self.record_completion(0, Some(id), done);
            self.persist(cx);
            cx.notify();
        }
    }
    pub fn due_items(&self) -> Due<'_> {
        let today = self.now_today();
        let week_end = today + Duration::days(7);
        let mut due = Due {
            overdue: vec![],
            today: vec![],
            week: vec![],
            later: vec![],
        };
        let completing = |key: (u64, Option<u64>)| self.completing.contains(&key);
        for update in self.workspace.updates.iter().filter(|u| {
            u.is_live()
                && self
                    .project_filter
                    .is_none_or(|id| u.project_id == Some(id))
        }) {
            if (update.status != Status::Erledigt || completing((update.id, None)))
                && let Some(d) = update.planned_for
            {
                bucket(DueItem::Update(update), d, today, week_end, &mut due);
            }
            for task in update
                .tasks
                .iter()
                .filter(|t| !t.done || completing((update.id, Some(t.id))))
            {
                if let Some(d) = task.planned_for {
                    bucket(DueItem::Task(update, task), d, today, week_end, &mut due);
                }
            }
        }
        for task in self.workspace.standalone_tasks.iter().filter(|t| {
            (!t.done || completing((0, Some(t.id))))
                && self
                    .project_filter
                    .is_none_or(|id| t.project_id == Some(id))
        }) {
            if let Some(date) = task.date {
                bucket(DueItem::Inbox(task), date, today, week_end, &mut due);
            }
        }
        for list in [
            &mut due.overdue,
            &mut due.today,
            &mut due.week,
            &mut due.later,
        ] {
            list.sort_by_key(|i| {
                (
                    std::cmp::Reverse(match i {
                        DueItem::Update(u) => u.priority,
                        DueItem::Task(_, t) => t.priority,
                        DueItem::Inbox(t) => t.priority,
                    }),
                    i.date(),
                )
            });
        }
        due
    }

    pub fn due_count(&self) -> usize {
        let due = self.due_items();
        due.overdue.len() + due.today.len()
    }

    /// Abhaken in der Heute-Ansicht: Zeile bleibt 800 ms (durchgestrichen) und wechselt dann
    /// in „Erledigt heute“.
    pub fn complete_item(&mut self, uid: u64, tid: Option<u64>, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let previous = self.workspace.update(uid).map(|u| u.status);
        match tid {
            Some(tid) if uid == 0 => {
                if let Some(t) = self
                    .workspace
                    .standalone_tasks
                    .iter_mut()
                    .find(|t| t.id == tid)
                {
                    t.done = !t.done;
                }
                self.persist(cx);
            }
            Some(tid) => self.toggle_task(uid, tid, cx),
            None => {
                if let Some(u) = self.workspace.update_mut(uid) {
                    u.status = Status::Erledigt;
                    u.touch();
                }
            }
        }
        self.workspace
            .settings
            .home_completions
            .retain(|(u, t, _)| (*u, *t) != (uid, tid));
        let today = self.now_today();
        self.workspace
            .settings
            .home_completions
            .push((uid, tid, today));
        self.persist(cx);
        self.toast("Erledigt", Some(Undo::Completed(uid, tid, previous)), cx);
        let key = (uid, tid);
        if !self.completing.contains(&key) {
            self.completing.push(key);
        }
        let delay = if self.workspace.settings.motion == model::Motion::Reduced {
            0
        } else {
            800
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(StdDuration::from_millis(delay))
                .await;
            this.update(cx, |this, cx| {
                this.completing.retain(|k| *k != key);
                if !this.done_today.contains(&key) {
                    this.done_today.push(key);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn reopen_item(&mut self, uid: u64, tid: Option<u64>, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        match tid {
            Some(tid) if uid == 0 => {
                if let Some(t) = self
                    .workspace
                    .standalone_tasks
                    .iter_mut()
                    .find(|t| t.id == tid)
                {
                    t.done = !t.done;
                }
                self.persist(cx);
            }
            Some(tid) => self.toggle_task(uid, tid, cx),
            None => self.set_status(uid, Status::Geplant, cx),
        }
        self.done_today.retain(|k| *k != (uid, tid));
        self.workspace
            .settings
            .home_completions
            .retain(|(u, t, _)| (*u, *t) != (uid, tid));
        self.persist(cx);
        cx.notify();
    }

    /// Enter erfasst eine Inbox-Aufgabe; Strg+Enter erstellt ein Update.
    pub fn commit_quick_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_capture(self.quick_input.clone(), window, cx);
    }
    pub fn commit_capture(
        &mut self,
        input: Entity<InputState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let raw = input.read(cx).value().to_string();
        let parsed = quick::parse_quick_input(&raw, self.now_today(), &self.workspace.projects);
        if parsed.title.is_empty() || !self.writable {
            return;
        }
        let date = parsed.date.or(self.quick_date);
        if window.modifiers().secondary(cx) {
            let id = self.workspace.new_update(date);
            if let Some(update) = self.workspace.update_mut(id) {
                update.title = parsed.title;
                update.priority = parsed.priority;
                update.project_id = parsed.project_id;
            }
        } else {
            let id = self.workspace.alloc_id();
            self.workspace.standalone_tasks.push(model::StandaloneTask {
                id,
                title: parsed.title,
                done: false,
                date,
                project_id: parsed.project_id,
                priority: parsed.priority,
                remind: Default::default(),
                created_at: model::now_millis(),
            });
        }
        input.update(cx, |input, cx| input.set_value("", window, cx));
        self.quick_date = None;
        self.persist(cx);
        self.toast("Eintrag angelegt", None, cx);
        cx.notify();
    }

    pub fn focus_quick_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view != View::Today {
            self.set_view(View::Today, cx);
        }
        self.quick_input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    fn progress_bar(p: &Palette, done: usize, total: usize) -> Div {
        let frac = if total == 0 {
            0.
        } else {
            done as f32 / total as f32
        };
        div()
            .flex()
            .items_center()
            .gap(px(palette::SP_2))
            .child(
                div()
                    .w(px(28.))
                    .h(px(4.))
                    .rounded_full()
                    .bg(p.border_subtle)
                    .child(div().h_full().w(relative(frac)).rounded_full().bg(
                        if done == total && total > 0 {
                            p.done
                        } else {
                            p.accent
                        },
                    )),
            )
            .child(
                div()
                    .text_size(palette::fs(11.))
                    .text_color(p.text_tertiary)
                    .child(format!("{done}/{total}")),
            )
    }

    fn due_row(
        &self,
        item: DueItem<'_>,
        scope: &'static str,
        checked: bool,
        reopen: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let entity = cx.entity();
        let open = cx.entity();
        let open_btn = cx.entity();
        let key_entity = cx.entity();
        let (uid, tid) = item.key();
        let narrow = matches!(scope, "week" | "inbox");
        let (project, priority) = match item {
            DueItem::Update(u) => (u.project_id, u.priority),
            DueItem::Task(u, t) => (u.project_id, t.priority),
            DueItem::Inbox(t) => (t.project_id, t.priority),
        };
        let project = self
            .workspace
            .projects
            .iter()
            .find(|p| Some(p.id) == project)
            .map(|p| p.name.clone());
        let (title, sub, status, date, target, progress) = match item {
            DueItem::Update(u) => (
                u.title.clone(),
                None,
                u.status,
                u.planned_for,
                DateTarget::Update(u.id),
                Some(u.task_progress()).filter(|(_, n)| *n > 0),
            ),
            DueItem::Inbox(t) => (
                t.title.clone(),
                Some("Inbox".into()),
                Status::Geplant,
                t.date,
                DateTarget::Inbox(t.id),
                None,
            ),
            DueItem::Task(u, t) => (
                t.title.clone(),
                Some(u.title.clone()),
                u.status,
                t.planned_for,
                DateTarget::Task(u.id, t.id),
                None,
            ),
        };
        let row_id = SharedString::from(format!("due-{scope}-{uid}-{}", tid.unwrap_or(0)));
        let group = SharedString::from(format!("grp-{scope}-{uid}-{}", tid.unwrap_or(0)));
        kit::surface(row_id.clone(), &p)
            .group(group.clone())
            .tooltip(tip(if title.trim().is_empty() {
                "Ohne Titel".to_owned()
            } else {
                title.clone()
            }))
            .tab_stop(true)
            .on_key_down(move |ev, window, cx| match ev.keystroke.key.as_str() {
                "up" => {
                    window.focus_prev(cx);
                    cx.stop_propagation();
                }
                "down" => {
                    window.focus_next(cx);
                    cx.stop_propagation();
                }
                "space" => {
                    key_entity.update(cx, |a, cx| {
                        if reopen {
                            a.reopen_item(uid, tid, cx)
                        } else if !checked {
                            a.complete_item(uid, tid, cx)
                        }
                    });
                    cx.stop_propagation();
                }
                "enter" => {
                    key_entity.update(cx, |a, cx| {
                        if uid == 0 {
                            a.set_view(View::Inbox, cx)
                        } else {
                            a.select_update(uid, window, cx)
                        }
                    });
                    cx.stop_propagation();
                }
                _ => {}
            })
            .min_h(palette::dens(44.))
            .px(px(palette::SP_2))
            .py(px(4.))
            .relative()
            .flex()
            .items_center()
            .gap(px(palette::SP_2))
            .rounded(px(palette::R_INPUT))
            .cursor_pointer()
            .on_click(move |_, window, cx| {
                open.update(cx, |app, cx| {
                    if uid == 0 {
                        app.set_view(View::Inbox, cx)
                    } else {
                        app.select_update(uid, window, cx)
                    }
                })
            })
            .child(kit::row_leading(if tid.is_none() {
                self.status_control(self.workspace.update(uid).unwrap(), cx)
                    .into_any_element()
            } else {
                kit::task_checkbox(SharedString::from(format!("{row_id}-check")), &p, checked)
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        entity.update(cx, |app, cx| {
                            if reopen {
                                app.reopen_item(uid, tid, cx)
                            } else if !checked {
                                app.complete_item(uid, tid, cx)
                            }
                        })
                    })
                    .into_any_element()
            }))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(1.))
                    .child(
                        div()
                            .truncate()
                            .text_size(palette::fs(palette::FS_MD))
                            .when(checked, |d| d.line_through().text_color(p.text_tertiary))
                            .child(if title.trim().is_empty() {
                                "Ohne Titel".to_owned()
                            } else {
                                title.clone()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .text_size(palette::fs(palette::FS_SM))
                            .text_color(p.text_tertiary)
                            .when_some(sub, |d, sub| {
                                d.child(div().truncate().max_w(px(220.)).child(sub))
                                    .child("·")
                            })
                            .child(dot(p.status(status)).size(px(6.)))
                            .child(status.label()),
                    ),
            )
            .when_some(project.filter(|_| !narrow), |d, name| {
                d.child(
                    div()
                        .max_w(px(110.))
                        .truncate()
                        .px(px(palette::SP_2))
                        .py(px(2.))
                        .rounded(px(palette::R_XS))
                        .bg(p.bg_hover)
                        .text_size(palette::fs(12.))
                        .child(name),
                )
            })
            .when(scope != "week", |d| {
                d.child(
                    self.date_chip(format!("{row_id}-date"), target, date, checked, cx)
                        .when(date.is_none(), |b| b.label("Termin setzen")),
                )
            })
            .children(self.remind_bell(target, cx))
            .when(priority > 0, |d| {
                d.child(kit::priority_icon(
                    SharedString::from(format!("{row_id}-priority")),
                    &p,
                    priority,
                ))
            })
            .child(
                kit::row_actions(SharedString::from(format!("{row_id}-actions")), group).child(
                    icon_btn(
                        SharedString::from(format!("{row_id}-open")),
                        IconName::ExternalLink,
                        "Öffnen",
                    )
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        open_btn.update(cx, |app, cx| {
                            if uid == 0 {
                                app.set_view(View::Inbox, cx)
                            } else {
                                app.select_update(uid, window, cx)
                            }
                        })
                    }),
                ),
            )
            .when_some(progress.filter(|_| !narrow), |d, (done, total)| {
                d.child(Self::progress_bar(&p, done, total))
            })
    }

    fn due_group(
        &self,
        index: usize,
        label: &'static str,
        color: Hsla,
        items: &[DueItem<'_>],
        hint_when_empty: Option<&'static str>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if items.is_empty() && hint_when_empty.is_none() {
            return None;
        }
        let p = palette::get(cx);
        let entity = cx.entity();
        let collapsed = self.today_collapsed[index];
        let done_group = index == G_DONE;
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    kit::surface(("today-group", index), &p)
                        .h(px(palette::CTL_MD))
                        .px(px(palette::SP_2))
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_2))
                        .cursor_pointer()
                        .rounded(px(palette::R_INPUT))
                        .child(dot(color))
                        .child(
                            div()
                                .text_size(palette::fs(palette::FS_BASE))
                                .text_color(p.text_secondary)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(label),
                        )
                        .child(text_secondary(&p, items.len().to_string()))
                        .child(div().flex_1())
                        .child(ico(
                            if collapsed {
                                icons::COLLAPSE
                            } else {
                                icons::EXPAND
                            },
                            p.text_tertiary,
                        ))
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| {
                                app.today_collapsed[index] = !app.today_collapsed[index];
                                cx.notify();
                            })
                        }),
                )
                .when(!collapsed, |d| {
                    d.children(items.iter().map(|item| {
                        let checked = done_group || self.completing.contains(&item.key());
                        self.due_row(*item, label, checked, done_group, cx)
                            .into_any_element()
                    }))
                    .when(items.is_empty(), |d| {
                        d.child(
                            div()
                                .px(px(palette::SP_2))
                                .py(px(4.))
                                .child(text_secondary(&p, hint_when_empty.unwrap_or_default())),
                        )
                    })
                })
                .into_any_element(),
        )
    }

    fn week_card(&self, cx: &mut Context<Self>) -> Div {
        use chrono::Datelike as _;
        let p = palette::get(cx);
        let today = self.now_today();
        // Ein einziger Anker (`home_day`): Streifen, Auswahl und Navigation bleiben konsistent.
        let week_start = self.workspace.settings.week_start;
        let start = dates::week_start(self.home_day, week_start);
        let off_week = start != dates::week_start(today, week_start);
        let due = self.due_items();
        let all: Vec<_> = due
            .overdue
            .iter()
            .chain(&due.today)
            .chain(&due.week)
            .chain(&due.later)
            .collect();
        let mut days = div().flex().gap(px(4.));
        for i in 0..7 {
            let date = start + Duration::days(i);
            let count = all.iter().filter(|item| item.date() == Some(date)).count();
            let e = cx.entity();
            days = days.child(
                kit::surface(("week-day", i as usize), &p)
                    .tab_stop(true)
                    .flex_1()
                    .min_w_0()
                    .py(px(8.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .rounded(px(palette::R_INPUT))
                    .bg(
                        if matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun) {
                            p.bg_hover
                        } else {
                            p.bg_panel
                        },
                    )
                    .cursor_pointer()
                    .child(text_secondary(&p, week_start.names()[i as usize]))
                    .child(
                        div()
                            .size(px(28.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(date == today, |d| {
                                d.bg(p.accent).text_color(p.accent_contrast)
                            })
                            .child(date.day().to_string()),
                    )
                    .child(
                        div()
                            .text_color(if date < today {
                                p.danger
                            } else {
                                p.text_secondary
                            })
                            .child("•".repeat(count.min(3))),
                    )
                    .on_click(move |_, _, cx| {
                        e.update(cx, |a, cx| {
                            a.home_day = date;
                            cx.notify();
                        })
                    }),
            );
        }
        let selected: Vec<_> = all
            .iter()
            .filter(|i| i.date() == Some(self.home_day))
            .take(4)
            .collect();
        let mut day_list = div().flex().flex_col().gap(px(4.)).child(text_secondary(
            &p,
            format!(
                "{}: {}",
                dates::full_with_weekday(self.home_day, self.workspace.settings.date_format),
                model::plural(
                    all.iter()
                        .filter(|i| i.date() == Some(self.home_day))
                        .count(),
                    "Eintrag",
                    "Einträge"
                )
            ),
        ));
        for item in selected {
            day_list = day_list.child(self.due_row(**item, "week", false, false, cx));
        }
        let prev = cx.entity();
        let next = cx.entity();
        let month = cx.entity();
        card(&p)
            .p(px(16.))
            .gap(px(12.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Woche"),
                    )
                    .when(off_week, |d| {
                        let reset = cx.entity();
                        d.child(
                            kit::button("week-today")
                                .ghost()
                                .with_size(gpui_kit::component::Size::Small)
                                .h(px(palette::CTL_SM))
                                .label("Diese Woche")
                                .tooltip("Zur aktuellen Woche")
                                .on_click(move |_, _, cx| {
                                    reset.update(cx, |a, cx| {
                                        a.home_day = a.now_today();
                                        cx.notify();
                                    })
                                }),
                        )
                    })
                    .child(
                        icon_btn("week-prev", IconName::ChevronLeft, "Vorige Woche").on_click(
                            move |_, _, cx| {
                                prev.update(cx, |a, cx| {
                                    a.home_day -= Duration::days(7);
                                    cx.notify();
                                })
                            },
                        ),
                    )
                    .child(
                        icon_btn("week-next", icons::COLLAPSE, "Nächste Woche").on_click(
                            move |_, _, cx| {
                                next.update(cx, |a, cx| {
                                    a.home_day += Duration::days(7);
                                    cx.notify();
                                })
                            },
                        ),
                    ),
            )
            .child(days)
            .child(day_list)
            .child(
                kit::link_button("week-month", &p, "Monat anzeigen").on_click(move |_, _, cx| {
                    month.update(cx, |a, cx| a.set_view(View::Calendar, cx))
                }),
            )
    }

    fn recent_card(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let mut recent: Vec<&model::Update> = self
            .workspace
            .updates
            .iter()
            .filter(|u| {
                u.is_live()
                    && self
                        .project_filter
                        .is_none_or(|id| u.project_id == Some(id))
            })
            .collect();
        recent.sort_by_key(|u| std::cmp::Reverse(u.edited_at));
        let items: Vec<RecentItem> = recent
            .iter()
            .take(5)
            .map(|u| {
                (
                    u.id,
                    u.title.clone(),
                    u.status,
                    u.edited_at,
                    u.task_progress(),
                )
            })
            .collect();
        card(&p)
            .p(px(palette::SP_3))
            .gap(px(palette::SP_1))
            .child(
                div()
                    .pb(px(palette::SP_1))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Zuletzt bearbeitet"),
            )
            .when(items.is_empty(), |d| {
                d.child(text_secondary(&p, "Noch keine Updates."))
            })
            .children(
                items
                    .into_iter()
                    .map(|(id, title, status, edited, (done, total))| {
                        let entity = cx.entity();
                        kit::surface(("recent", id as usize), &p)
                            .min_h(px(32.))
                            .px(px(palette::SP_2))
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .rounded(px(palette::R_INPUT))
                            .cursor_pointer()
                            .child(dot(p.status(status)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().truncate().child(title))
                                    .child(text_secondary(
                                        &p,
                                        format!("{} · {}", status.label(), elapsed_label(edited)),
                                    )),
                            )
                            .child(progress_ring(&p, done, total))
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |a, cx| a.select_update(id, window, cx))
                            })
                    }),
            )
    }

    pub(super) fn today_view(
        &self,
        content_width: f32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let narrow = content_width < 900.;
        let wide = content_width >= 900.;
        let three = content_width >= 1700.;
        let p = palette::get(cx);
        let today = self.now_today();
        let due = self.due_items();
        let cards = self.workspace.settings.home_cards;
        let greeting = match chrono::Local::now().hour() {
            5..=10 => "Guten Morgen",
            11..=17 => "Guten Tag",
            _ => "Guten Abend",
        };
        let mut header = div().flex().flex_wrap().gap(px(16.)).items_center().child(
            div()
                .flex_1()
                .child(
                    div()
                        .text_size(palette::fs(palette::FS_TITLE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(greeting),
                )
                .child(text_secondary(
                    &p,
                    dates::full_with_weekday(today, self.workspace.settings.date_format),
                ))
                .when(!cards[6], |d| {
                    d.child(text_secondary(
                        &p,
                        if due.today.is_empty() && due.overdue.is_empty() {
                            "Nichts fällig.".to_owned()
                        } else {
                            format!(
                                "{} heute · {} überfällig",
                                model::plural(due.today.len(), "Eintrag", "Einträge"),
                                due.overdue.len()
                            )
                        },
                    ))
                }),
        );
        if cards[6] {
            for (i, label, count, view) in [
                (0, "Überfällig", due.overdue.len(), View::Today),
                (1, "Heute", due.today.len(), View::Today),
                (2, "Diese Woche", due.week.len(), View::Calendar),
                (
                    3,
                    "Inbox",
                    self.workspace
                        .standalone_tasks
                        .iter()
                        .filter(|t| !t.done)
                        .count(),
                    View::Inbox,
                ),
            ] {
                let e = cx.entity();
                header = header.child(
                    stat_chip(
                        ("metric", i),
                        &p,
                        [icons::DATE, icons::TODAY, icons::DATE, icons::INBOX][i],
                        count,
                        label,
                        i == 0,
                    )
                    .on_click(move |_, _, cx| e.update(cx, |a, cx| a.set_view(view, cx))),
                );
            }
        }
        let mut focus = card(&p).p(px(16.)).gap(px(12.)).child(self.home_header(
            "focus-all",
            "Fokus",
            Filter::All,
            cx,
        ));
        if due.today.is_empty() && due.overdue.is_empty() {
            focus = focus.child(text_secondary(
                &p,
                "Heute ist frei · Plane etwas Neues oder wähle den nächsten Schritt.",
            ));
        }
        focus = focus
            .children(self.due_group(G_OVERDUE, "Überfällig", p.danger, &due.overdue, None, cx))
            .children(self.due_group(G_TODAY, "Heute", p.text_primary, &due.today, None, cx));
        let mut upcoming: Vec<_> = due.week.iter().chain(&due.later).copied().collect();
        upcoming.sort_by_key(|i| i.date());
        if due.today.is_empty() && due.overdue.is_empty() {
            focus = focus.children(self.due_group(
                G_WEEK,
                "Als Nächstes",
                p.text_secondary,
                &upcoming.into_iter().take(5).collect::<Vec<_>>(),
                None,
                cx,
            ));
        } else {
            focus = focus.children(self.due_group(
                G_WEEK,
                "Diese Woche",
                p.text_secondary,
                &due.week,
                None,
                cx,
            ));
        }
        let undated: Vec<_> = self
            .workspace
            .updates
            .iter()
            .filter(|u| {
                u.is_live()
                    && !matches!(u.status, Status::Erledigt | Status::InArbeit)
                    && u.planned_for.is_none()
            })
            .take(3)
            .map(DueItem::Update)
            .collect();
        let mut undated = undated;
        undated.extend(
            self.workspace
                .standalone_tasks
                .iter()
                .filter(|t| !t.done && t.date.is_none())
                .take(3usize.saturating_sub(undated.len()))
                .map(DueItem::Inbox),
        );
        focus = focus.children(self.due_group(
            G_LATER,
            "Noch ohne Termin",
            p.text_secondary,
            &undated,
            None,
            cx,
        ));
        let done: Vec<_> = self
            .done_today
            .iter()
            .filter_map(|(u, t)| {
                if *u == 0 {
                    return self
                        .workspace
                        .standalone_tasks
                        .iter()
                        .find(|task| Some(task.id) == *t && task.done)
                        .map(DueItem::Inbox);
                }
                let update = self.workspace.update(*u).filter(|u| u.is_live())?;
                match t {
                    Some(t) => update
                        .tasks
                        .iter()
                        .find(|task| task.id == *t && task.done)
                        .map(|t| DueItem::Task(update, t)),
                    None => (update.status == Status::Erledigt).then_some(DueItem::Update(update)),
                }
            })
            .collect();
        focus = focus.children(self.due_group(G_DONE, "Heute erledigt", p.done, &done, None, cx));
        let mut work = card(&p).p(px(16.)).gap(px(8.)).child(self.home_header(
            "work-all",
            "In Arbeit",
            Filter::Status(Status::InArbeit),
            cx,
        ));
        let active: Vec<_> = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.is_live() && u.status == Status::InArbeit)
            .take(4)
            .collect();
        if active.is_empty() {
            let e = cx.entity();
            let planned: Vec<_> = self
                .workspace
                .updates
                .iter()
                .filter(|u| u.is_live() && u.status == Status::Geplant)
                .map(|u| (u.id, u.title.clone()))
                .collect();
            work = work
                .child(text_secondary(&p, "Noch kein Update in Arbeit."))
                .child(
                    kit::button("start-update")
                        .outline()
                        .label("Update starten")
                        .dropdown_menu(move |menu, _, _| {
                            planned.iter().fold(menu, |menu, (id, title)| {
                                let e = e.clone();
                                let id = *id;
                                menu.item(
                                    gpui_kit::component::menu::PopupMenuItem::new(title.clone())
                                        .on_click(move |_, _, cx| {
                                            e.update(cx, |a, cx| {
                                                a.set_status(id, Status::InArbeit, cx)
                                            })
                                        }),
                                )
                            })
                        }),
                );
        }
        for u in active {
            work = work.child(self.due_row(DueItem::Update(u), "work", false, false, cx));
        }

        let mut columns: Vec<Div> = (0..if three {
            3
        } else if wide {
            2
        } else {
            1
        })
            .map(|i| {
                div()
                    .flex_1()
                    .when(wide && !three && i == 0, |d| d.flex_grow(2.))
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
            })
            .collect();
        let mut add = |column: usize, index: usize, element: AnyElement| {
            let wrapped = div().id(("home-card", index)).w_full().child(element);
            let wrapped = if self.workspace.settings.motion != model::Motion::Full {
                wrapped.into_any_element()
            } else {
                wrapped
                    .with_animation(
                        ("home-fade", index),
                        Animation::new(palette::motion(
                            palette::MOTION_FAST_MS + index as u64 * 35,
                        )),
                        move |el, t| {
                            el.opacity(
                                ((t * (120. + index as f32 * 35.) - index as f32 * 35.) / 120.)
                                    .clamp(0., 1.),
                            )
                        },
                    )
                    .into_any_element()
            };
            let old = std::mem::replace(&mut columns[column], div());
            columns[column] = old.child(wrapped);
        };
        if cards[0] {
            add(0, 0, focus.into_any_element());
        }
        if !wide && cards[2] {
            add(0, 2, self.week_card(cx).into_any_element());
        }
        if cards[1] {
            add(if three { 1 } else { 0 }, 1, work.into_any_element());
        }
        if wide && cards[2] {
            add(
                if three { 2 } else { 1 },
                2,
                self.week_card(cx).into_any_element(),
            );
        }
        if cards[3] {
            add(
                if wide { 1 } else { 0 },
                3,
                self.home_inbox_link(cx).into_any_element(),
            );
        }
        if cards[4] {
            add(
                if three {
                    2
                } else if wide {
                    1
                } else {
                    0
                },
                4,
                self.recent_card(cx).into_any_element(),
            );
        }
        if three && cards[5] {
            add(2, 5, self.home_activity(cx).into_any_element());
        }
        let grid = div().flex().items_start().gap(px(16.)).children(
            columns.into_iter().enumerate().filter_map(|(i, col)| {
                let occupied = if !wide {
                    cards[..5].iter().any(|v| *v)
                } else if three {
                    match i {
                        0 => cards[0],
                        1 => cards[1] || cards[3],
                        _ => cards[2] || cards[4] || cards[5],
                    }
                } else {
                    if i == 0 {
                        cards[0] || cards[1]
                    } else {
                        cards[2] || cards[3] || cards[4]
                    }
                };
                occupied.then_some(col)
            }),
        );
        let focused = self
            .quick_input
            .read(cx)
            .focus_handle(cx)
            .is_focused(window);
        let parsed = quick::parse_quick_input(
            &self.quick_input.read(cx).value(),
            today,
            &self.workspace.projects,
        );
        let preview = format!(
            "Aufgabe · {} · {} · {}",
            parsed
                .date
                .or(self.quick_date)
                .map(|d| dates::relative(d, today, self.workspace.settings.date_format))
                .unwrap_or_else(|| "Ohne Termin".into()),
            self.workspace
                .projects
                .iter()
                .find(|p| Some(p.id) == parsed.project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("Kein Projekt"),
            ["Keine Priorität", "Niedrig", "Mittel", "Hoch"][parsed.priority.min(3) as usize]
        );
        div()
            .id("today-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .child(
                div()
                    .w_full()
                    .max_w(px(1400.))
                    .mx_auto()
                    .p(px(if narrow { 20. } else { 32. }))
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .child(header)
                    .child(self.home_quick_field(cx))
                    .when(focused, |d| {
                        d.child(
                            div().flex().flex_wrap().gap(px(palette::SP_2)).children(
                                [
                                    "#Projekt",
                                    "@Datum",
                                    "!Priorität",
                                    "Enter = Inbox",
                                    "Strg+Enter = als Update",
                                ]
                                .map(|t| keycap(&p, t)),
                            ),
                        )
                        .child(text_secondary(&p, preview))
                    })
                    .children(self.reminders_card(&p, cx))
                    .when(self.splash != Splash::Showing, |d| d.child(grid))
                    .when(self.splash == Splash::Showing, |d| {
                        d.child(div().flex().gap(px(16.)).children(
                            (0..if wide { 2 } else { 1 }).map(|_| {
                                card(&p)
                                    .flex_1()
                                    .p(px(palette::SP_6))
                                    .gap(px(16.))
                                    .children((0..4).map(|_| {
                                        div()
                                            .h(px(24.))
                                            .rounded(px(palette::R_CONTROL_SM))
                                            .bg(p.bg_hover)
                                    }))
                            }),
                        ))
                    }),
            )
    }
    /// Karte "Erinnerungen": heutige Erinnerungen; ohne Einträge gibt es keine Karte.
    fn reminders_card(&self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let items = self.reminders_today();
        if items.is_empty() {
            return None;
        }
        let now = chrono::Local::now().naive_local();
        let rows = items.into_iter().take(5).map(|(target, at)| {
            let entity = cx.entity();
            let past = at < now;
            kit::button(SharedString::from(format!("remind-row-{target:?}")))
                .ghost()
                .w_full()
                .min_h(px(palette::MIN_HIT))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_3))
                        .child(
                            kit::icon(icons::BELL, palette::ICON_MD).text_color(if past {
                                p.text_tertiary
                            } else {
                                p.accent
                            }),
                        )
                        .child(
                            div()
                                .min_w(px(44.))
                                .text_size(palette::fs(palette::FS_SM))
                                .text_color(p.text_secondary)
                                .child(format!("{:02}:{:02}", at.hour(), at.minute())),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(if past {
                                    p.text_secondary
                                } else {
                                    p.text_primary
                                })
                                .child(self.target_title(target)),
                        ),
                )
                .on_click(move |_, window, cx| {
                    entity.update(cx, |a, cx| a.open_target(target, window, cx))
                })
        });
        Some(
            card(p)
                .p(px(16.))
                .gap(px(8.))
                .child(
                    div()
                        .text_size(palette::fs(14.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Erinnerungen"),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn home_header(
        &self,
        id: &'static str,
        title: &'static str,
        filter: Filter,
        cx: &mut Context<Self>,
    ) -> Div {
        let e = cx.entity();
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(palette::fs(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .child(
                kit::link_button(id, &palette::get(cx), "Alle ansehen").on_click(
                    move |_, _, cx| {
                        e.update(cx, |a, cx| {
                            a.filter = filter;
                            a.set_view(View::Updates, cx);
                        })
                    },
                ),
            )
    }
    pub fn quick_options(&self, cx: &App) -> Vec<(String, String)> {
        if !self.quick_suggestions_open {
            return vec![];
        }
        let input = self.quick_input.read(cx);
        let raw = input.value().to_string();
        let cursor = input.selected_range().end.min(raw.len());
        let start = raw[..cursor]
            .rfind(char::is_whitespace)
            .map_or(0, |i| i + 1);
        let word = &raw[start..cursor];
        let options = if word.starts_with('@') {
            vec![
                ("@heute".into(), "Heute".into()),
                ("@morgen".into(), "Morgen".into()),
                ("@übermorgen".into(), "Übermorgen".into()),
                ("@nächsten montag".into(), "Nächsten Montag".into()),
                ("@in 7 tagen".into(), "In einer Woche".into()),
            ]
        } else if word.starts_with('#') {
            self.workspace
                .projects
                .iter()
                .filter(|p| !p.archived)
                .map(|p| {
                    (
                        format!("#{}", p.name.split_whitespace().collect::<String>()),
                        p.name.clone(),
                    )
                })
                .collect()
        } else if word.starts_with('!') {
            vec![
                ("!niedrig".into(), "Niedrig".into()),
                ("!mittel".into(), "Mittel".into()),
                ("!hoch".into(), "Hoch".into()),
            ]
        } else {
            return vec![];
        };
        options
            .into_iter()
            .filter(|(value, _)| value.to_lowercase().starts_with(&word.to_lowercase()))
            .take(8)
            .collect()
    }
    pub fn accept_quick_option(
        &mut self,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.quick_input.read(cx);
        let raw = input.value().to_string();
        let cursor = input.selected_range().end.min(raw.len());
        let start = raw[..cursor]
            .rfind(char::is_whitespace)
            .map_or(0, |i| i + 1);
        let text = format!("{}{} {}", &raw[..start], value, &raw[cursor..]);
        let cursor = start + value.len() + 1;
        self.quick_input.update(cx, |i, cx| {
            i.set_value(text, window, cx);
            i.set_selected_range(cursor..cursor, cx);
            i.focus(window, cx);
        });
        self.quick_suggestions_open = false;
        self.decorate_quick(window, cx);
        cx.notify();
    }
    pub fn decorate_quick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.quick_input.read(cx).value().to_string();
        let parsed = quick::parse_quick_input(&raw, self.now_today(), &self.workspace.projects);
        let ranges: Vec<_> = parsed
            .tokens
            .into_iter()
            .filter(|r| {
                r.end < raw.len()
                    && raw[r.end..].starts_with(char::is_whitespace)
                    && !self
                        .quick_input
                        .read(cx)
                        .tokens()
                        .iter()
                        .any(|t| t.range().start < r.end && r.start < t.range().end)
            })
            .collect();
        if ranges.is_empty() {
            return;
        }
        self.quick_input.update(cx, |i, cx| {
            let selection = i.selected_range();
            for r in ranges.into_iter().rev() {
                let token = gpui_kit::component::input::InlineToken::new(
                    format!("quick-{}", r.start),
                    raw[r.clone()].to_string(),
                );
                let _ = i.replace_range_with_token(r, token, window, cx);
            }
            i.set_selected_range(selection, cx);
        });
    }
    fn home_quick_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let bounds_entity = cx.entity();
        let date_entity = cx.entity();
        div()
            .id("home-quick-field")
            .relative()
            .w_full()
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                let options = this.quick_options(cx);
                if options.is_empty() {
                    return;
                }
                match ev.keystroke.key.as_str() {
                    "down" => {
                        this.quick_suggestion = (this.quick_suggestion + 1) % options.len();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" => {
                        this.quick_suggestion =
                            (this.quick_suggestion + options.len() - 1) % options.len();
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "enter" | "tab" if !ev.keystroke.modifiers.control => {
                        let value = options[this.quick_suggestion.min(options.len() - 1)]
                            .0
                            .clone();
                        this.accept_quick_option(value, window, cx);
                        cx.stop_propagation();
                    }
                    "escape" => {
                        this.quick_suggestions_open = false;
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ => {}
                }
            }))
            .child(
                kit::input(&self.quick_input)
                    .prefix(ico(icons::ADD, p.text_secondary))
                    .aria_label("Aufgabe hinzufügen · Enter: Inbox · Strg+Enter: Update")
                    .suffix(
                        div()
                            .flex()
                            .items_center()
                            .child(
                                icon_btn("home-quick-date", icons::DATE, "Datum wählen").on_click(
                                    move |_, w, cx| {
                                        date_entity.update(cx, |a, cx| {
                                            a.open_date_pop(
                                                DateTarget::Quick,
                                                w.mouse_position(),
                                                w,
                                                cx,
                                            )
                                        })
                                    },
                                ),
                            )
                            .child(keycap(&p, "N")),
                    )
                    .h(px(palette::CTL_LG))
                    .w_full(),
            )
            .child(
                canvas(
                    |bounds, _, _| bounds,
                    move |bounds, _, _, cx| {
                        bounds_entity.update(cx, |a, _| a.quick_bounds = Some(bounds));
                    },
                )
                .absolute()
                .inset_0(),
            )
    }
    pub(super) fn quick_options_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let bounds = self.quick_bounds.unwrap_or_default();
        let options = self.quick_options(cx);
        deferred(
            anchored()
                .position(point(bounds.origin.x, bounds.bottom()))
                .snap_to_window_with_margin(px(8.))
                .child(
                    div()
                        .id("quick-options")
                        .occlude()
                        .w(bounds.size.width)
                        .max_h(window.viewport_size().height - px(80.))
                        .overflow_y_scrollbar()
                        .p(px(4.))
                        .rounded(px(palette::R_INPUT))
                        .bg(p.bg_elevated)
                        .border_1()
                        .border_color(p.border_strong)
                        .shadow_lg()
                        .children(options.into_iter().enumerate().map(
                            |(index, (value, label))| {
                                let e = cx.entity();
                                div()
                                    .id(("quick-option", index))
                                    .min_h(px(palette::ROW_COMPACT))
                                    .px(px(12.))
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .when(index == self.quick_suggestion, |d| d.bg(p.bg_active))
                                    .hover(|s| s.bg(p.bg_hover))
                                    .child(label)
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        cx.stop_propagation();
                                        e.update(cx, |a, cx| {
                                            a.accept_quick_option(value.clone(), window, cx)
                                        });
                                    })
                            },
                        )),
                ),
        )
    }

    fn home_inbox_link(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let e = cx.entity();
        let tasks: Vec<_> = self
            .workspace
            .standalone_tasks
            .iter()
            .rev()
            .filter(|t| !t.done)
            .take(5)
            .collect();
        let mut body = card(&p).p(px(16.)).gap(px(8.)).child(
            div()
                .flex()
                .justify_between()
                .child(div().font_weight(FontWeight::SEMIBOLD).child("Inbox"))
                .child(
                    kit::link_button("home-inbox-open", &p, "Alle ansehen").on_click(
                        move |_, _, cx| e.update(cx, |a, cx| a.set_view(View::Inbox, cx)),
                    ),
                ),
        );
        if tasks.is_empty() {
            body = body.child(text_secondary(&p, "Inbox ist leer"));
        }
        for t in tasks {
            body = body.child(self.due_row(DueItem::Inbox(t), "inbox", false, false, cx));
        }
        body.child(
            kit::input(&self.inbox_quick_input)
                .aria_label("Schnell erfassen")
                .w_full(),
        )
    }
    fn home_activity(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let today = self.now_today();
        let counts: Vec<_> = (0..7)
            .map(|i| {
                let day = today - Duration::days(6 - i);
                let count = self
                    .workspace
                    .settings
                    .home_completions
                    .iter()
                    .filter(|(u, t, d)| t.is_some() && *d == day && self.completion_is_live(*u, *t))
                    .count();
                (day, count)
            })
            .collect();
        let max = counts.iter().map(|(_, c)| *c).max().unwrap_or(1).max(1);
        card(&p)
            .w_full()
            .p(px(16.))
            .gap(px(12.))
            .child("Diese Woche geschafft")
            .child(
                div()
                    .text_size(palette::fs(32.))
                    .child(counts.iter().map(|(_, c)| c).sum::<usize>().to_string()),
            )
            .child(text_secondary(
                &p,
                "Erfasste Abschlüsse der letzten sieben Tage.",
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(8.))
                    .children(counts.into_iter().map(|(day, count)| {
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .id(SharedString::from(format!("activity-{day}")))
                                    .h(px(4. + count as f32 / max as f32 * 56.))
                                    .bg(p.accent)
                                    .tooltip(tip(format!(
                                        "{day}: {}",
                                        model::plural(count, "Aufgabe", "Aufgaben")
                                    ))),
                            )
                            .child(text_secondary(
                                &p,
                                ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]
                                    [chrono::Datelike::weekday(&day).num_days_from_monday()
                                        as usize],
                            ))
                    })),
            )
    }
}

fn bucket<'a>(
    item: DueItem<'a>,
    date: NaiveDate,
    today: NaiveDate,
    week_end: NaiveDate,
    due: &mut Due<'a>,
) {
    if date < today {
        due.overdue.push(item);
    } else if date == today {
        due.today.push(item);
    } else if date <= week_end {
        due.week.push(item);
    } else {
        due.later.push(item);
    }
}

fn elapsed_label(edited: i64) -> String {
    dates::elapsed(edited)
}
