use super::kit::*;
use super::*;
use chrono::Datelike as _;
use model::WeekStart;

struct Entry {
    update_id: u64,
    task_id: Option<u64>,
    title: String,
    color: Hsla,
    status: Status,
    done: bool,
    progress: (usize, usize),
}

impl Entry {
    fn item(&self) -> CalItem {
        match self.task_id {
            Some(t) => CalItem::Task(self.update_id, t, self.title.clone()),
            None => CalItem::Update(self.update_id, self.title.clone()),
        }
    }
}

impl NoteWarden {
    fn entries_for(&self, date: Option<NaiveDate>, p: &Palette) -> Vec<Entry> {
        let show_done = self.workspace.settings.show_done_in_calendar;
        let mut out = Vec::new();
        for update in self.workspace.updates.iter().filter(|u| {
            u.is_live()
                && self
                    .project_filter
                    .is_none_or(|id| u.project_id == Some(id))
        }) {
            if update.planned_for == date && (show_done || update.status != Status::Erledigt) {
                out.push(Entry {
                    update_id: update.id,
                    task_id: None,
                    title: update.title.clone(),
                    color: p.status(update.status),
                    status: update.status,
                    done: update.status == Status::Erledigt,
                    progress: update.task_progress(),
                });
            }
        }
        for update in self.workspace.updates.iter().filter(|u| {
            u.is_live()
                && self
                    .project_filter
                    .is_none_or(|id| u.project_id == Some(id))
        }) {
            for task in update.tasks.iter().filter(|t| t.planned_for == date) {
                if (date.is_none() || !show_done) && task.done {
                    continue;
                }
                out.push(Entry {
                    update_id: update.id,
                    task_id: Some(task.id),
                    title: task.title.clone(),
                    color: p.text_secondary,
                    status: update.status,
                    done: task.done,
                    progress: (0, 0),
                });
            }
        }
        for task in self.workspace.standalone_tasks.iter().filter(|t| {
            t.date == date
                && (show_done || !t.done)
                && self
                    .project_filter
                    .is_none_or(|id| t.project_id == Some(id))
        }) {
            out.push(Entry {
                update_id: 0,
                task_id: Some(task.id),
                title: task.title.clone(),
                color: p.text_secondary,
                status: if task.done {
                    Status::Erledigt
                } else {
                    Status::Geplant
                },
                done: task.done,
                progress: (0, 0),
            });
        }
        out
    }

    /// Eintrag als Pill (22 px, Statusbalken links) bzw. in der Wochenansicht als Karte.
    fn pill(
        &self,
        entry: &Entry,
        key: String,
        p: &Palette,
        card_style: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let uid = entry.update_id;
        let tid = entry.task_id;
        let is_task = tid.is_some();
        let item = entry.item();
        let entity = cx.entity();
        let menu = cx.entity();
        let toggle = cx.entity();
        let p = *p;
        let color = entry.color;
        div()
            .id(SharedString::from(key.clone()))
            .relative()
            .w_full()
            .min_h(px(if card_style { 44. } else { 22. }))
            .pl(px(8.))
            .pr(px(palette::SP_2))
            .py(px(if card_style { 6. } else { 0. }))
            .flex()
            .items_center()
            .gap(px(palette::SP_2))
            .rounded(px(palette::R_CONTROL_SM))
            .overflow_hidden()
            .cursor_pointer()
            .text_size(palette::fs(if is_task && !card_style { 11. } else { 12. }))
            .bg(if is_task {
                p.bg_hover
            } else {
                color.opacity(0.14)
            })
            .text_color(if is_task {
                p.text_secondary
            } else {
                p.text_primary
            })
            .hover(move |s| s.bg(color.opacity(0.24)))
            .when(entry.done, |d| d.opacity(0.55))
            // 3-px-Statusbalken links
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(3.))
                    .bg(if is_task { p.border_strong } else { color }),
            )
            .when(is_task, |d| {
                d.child(
                    kit::button(SharedString::from(format!("{key}-chk")))
                        .ghost()
                        .size(px(palette::ICON_LG))
                        .accessibility_label(if entry.done {
                            "Aufgabe wieder öffnen"
                        } else {
                            "Aufgabe erledigen"
                        })
                        .role(Role::CheckBox)
                        .toggled(entry.done)
                        .flex_none()
                        .rounded_full()
                        .border_1()
                        .border_color(p.text_tertiary)
                        .when(entry.done, |d| d.bg(p.done).border_color(p.done))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            if let Some(tid) = tid {
                                toggle.update(cx, |app, cx| app.toggle_task(uid, tid, cx))
                            }
                        }),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .when(entry.done, |d| d.line_through())
                            .child(entry.title.clone()),
                    )
                    .when(card_style, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(palette::SP_2))
                                .text_size(palette::fs(11.))
                                .text_color(p.text_tertiary)
                                .child(dot(p.status(entry.status)).size(px(6.)))
                                .child(entry.status.label())
                                .when(entry.progress.1 > 0, |d| {
                                    d.child(format!("· {}/{}", entry.progress.0, entry.progress.1))
                                }),
                        )
                    }),
            )
            .tooltip(tip(entry.title.clone()))
            .on_drag(item, |item, _, _, cx| {
                let label = item.label().to_owned();
                cx.new(|_| DragPreview { label })
            })
            .on_mouse_down(MouseButton::Right, move |ev, _, cx| {
                cx.stop_propagation();
                let anchor = ev.position;
                menu.update(cx, |app, cx| {
                    app.pill_menu = Some(PillMenu {
                        update_id: uid,
                        task_id: tid,
                        anchor,
                    });
                    cx.notify();
                })
            })
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                entity.update(cx, |app, cx| {
                    if uid == 0 {
                        app.set_view(View::Inbox, cx);
                    } else {
                        app.select_update(uid, window, cx);
                    }
                })
            })
    }

    fn open_inline(date: NaiveDate, window: &mut Window, cx: &mut Context<Self>, app: &mut Self) {
        app.cal_inline = Some(date);
        app.cal_focus = Some(date);
        app.cal_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn inline_editor(&self, p: &Palette) -> impl IntoElement {
        let selected_title = self.selected().map(|u| u.title.clone());
        div()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                kit::input(&self.cal_input)
                    .aria_label("Neuer Eintrag")
                    .w_full(),
            )
            .child(div().text_size(px(10.)).text_color(p.text_tertiary).child(
                match selected_title {
                    Some(t) => format!("Enter: Update · Strg+Enter: Aufgabe in „{t}“"),
                    None => "Enter: Update anlegen".to_owned(),
                },
            ))
    }

    fn add_button(
        &self,
        date: NaiveDate,
        group: SharedString,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entity = cx.entity();
        let p = *p;
        div().invisible().group_hover(group, |s| s.visible()).child(
            kit::surface(("cal-add", date.num_days_from_ce() as usize), &p)
                .size(px(20.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .cursor_pointer()
                .text_color(p.text_secondary)
                .tooltip(tip("Eintrag hinzufügen"))
                .child(ico(icons::ADD, p.text_secondary))
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    entity.update(cx, |app, cx| Self::open_inline(date, window, cx, app))
                }),
        )
    }

    fn day_cell(
        &self,
        date: NaiveDate,
        month: u32,
        limit: usize,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let today = self.now_today();
        let st = &self.workspace.settings;
        let is_today = date == today;
        let weekend = st.highlight_weekends
            && matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
        let entries = self.entries_for(Some(date), p);
        let total = entries.len();
        let hidden = total.saturating_sub(limit);
        let entity = cx.entity();
        let inline = self.cal_inline == Some(date);
        let focused = self.cal_focus == Some(date);
        let key = date.num_days_from_ce();
        let p = *p;
        let group = SharedString::from(format!("cell-{key}"));
        let label = if date.day() == 1 {
            format!("1. {}", dates::month_short(date))
        } else {
            date.day().to_string()
        };
        div()
            .id(("day", key as usize))
            .group(group.clone())
            .flex_1()
            .min_w_0()
            .min_h(px(96.))
            .overflow_hidden()
            .p(px(palette::SP_2))
            .flex()
            .flex_col()
            .gap(px(palette::SP_1))
            .border_r_1()
            .border_b_1()
            .border_color(p.border_subtle.opacity(0.6))
            .bg(if is_today {
                p.accent.opacity(0.06)
            } else if weekend {
                p.bg_sidebar
            } else {
                p.bg_panel
            })
            .when(focused, |d| d.border_color(p.accent))
            .drag_over::<CalItem>(move |s, _, _, _| {
                s.bg(p.accent.opacity(0.12)).border_color(p.accent)
            })
            .on_drop(cx.listener(move |this, item: &CalItem, _, cx| {
                this.drop_on_day(item, Some(date), cx)
            }))
            .on_click({
                let entity = entity.clone();
                move |ev, window, cx| {
                    entity.update(cx, |app, cx| {
                        app.cal_focus = Some(date);
                        // Doppelklick auf eine leere Fläche öffnet die Eingabe.
                        if ev.click_count() >= 2 {
                            Self::open_inline(date, window, cx, app);
                        } else {
                            cx.notify();
                        }
                    })
                }
            })
            .child(
                div()
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .min_w(px(22.))
                            .h(px(22.))
                            .px(px(4.))
                            .flex()
                            .items_center()
                            .rounded_full()
                            .text_size(palette::fs(palette::FS_SM))
                            .when(is_today, |d| {
                                d.bg(p.accent)
                                    .text_color(p.accent_contrast)
                                    .font_weight(FontWeight::SEMIBOLD)
                            })
                            .when(!is_today, |d| {
                                d.text_color(if date.month() == month {
                                    p.text_primary
                                } else {
                                    p.text_tertiary
                                })
                                .when(date.day() == 1, |d| d.font_weight(FontWeight::MEDIUM))
                            })
                            .child(label),
                    )
                    .child(self.add_button(date, group, &p, cx)),
            )
            .children(entries.iter().take(limit).enumerate().map(|(i, e)| {
                self.pill(e, format!("pill-{key}-{i}"), &p, false, cx)
                    .into_any_element()
            }))
            .when(hidden > 0, |d| {
                let entity = entity.clone();
                d.child(
                    kit::surface(("more", key as usize), &p)
                        .px(px(palette::SP_2))
                        .text_size(palette::fs(11.))
                        .text_color(p.text_secondary)
                        .cursor_pointer()
                        .child(format!("+{hidden} weitere"))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            entity.update(cx, |app, cx| {
                                app.cal_more = Some(date);
                                cx.notify();
                            })
                        }),
                )
            })
            .when(inline, |d| d.child(self.inline_editor(&p)))
    }

    /// Spalte der Wochenansicht: Tagesüberschrift und scrollbare Kartenliste.
    fn week_column(
        &self,
        date: NaiveDate,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let today = self.now_today();
        let st = &self.workspace.settings;
        let is_today = date == today;
        let weekend = st.highlight_weekends
            && matches!(date.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
        let entries = self.entries_for(Some(date), p);
        let key = date.num_days_from_ce();
        let inline = self.cal_inline == Some(date);
        let focused = self.cal_focus == Some(date);
        let p = *p;
        let group = SharedString::from(format!("wcol-{key}"));
        let empty = entries.is_empty();
        div()
            .id(("week-col", key as usize))
            .group(group.clone())
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(p.border_subtle.opacity(0.6))
            .bg(if is_today {
                p.accent.opacity(0.06)
            } else if weekend {
                p.bg_sidebar
            } else {
                p.bg_panel
            })
            .when(focused, |d| d.border_color(p.accent))
            .drag_over::<CalItem>(move |s, _, _, _| {
                s.bg(p.accent.opacity(0.12)).border_color(p.accent)
            })
            .on_drop(cx.listener(move |this, item: &CalItem, _, cx| {
                this.drop_on_day(item, Some(date), cx)
            }))
            .child(
                div()
                    .h(px(36.))
                    .px(px(palette::SP_2))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(p.border_subtle.opacity(0.6))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_SM))
                                    .text_color(p.text_secondary)
                                    .child(
                                        dates::weekday_short(date.weekday())
                                            .trim_end_matches('.')
                                            .to_owned(),
                                    ),
                            )
                            .child(
                                div()
                                    .min_w(px(22.))
                                    .h(px(22.))
                                    .px(px(4.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_full()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .when(is_today, |d| {
                                        d.bg(p.accent).text_color(p.accent_contrast)
                                    })
                                    .child(date.day().to_string()),
                            ),
                    )
                    .child(self.add_button(date, group.clone(), &p, cx)),
            )
            .child(
                div()
                    .id(("week-col-scroll", key as usize))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p(px(palette::SP_2))
                    .pr(px(palette::SCROLL_GUTTER))
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .children(entries.iter().enumerate().map(|(i, e)| {
                        self.pill(e, format!("wpill-{key}-{i}"), &p, true, cx)
                            .into_any_element()
                    }))
                    .when(inline, |d| d.child(self.inline_editor(&p)))
                    .when(empty && !inline, |d| {
                        let entity = cx.entity();
                        d.child(
                            div().invisible().group_hover(group, |s| s.visible()).child(
                                kit::surface(("week-add", key as usize), &p)
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .gap(px(4.))
                                    .rounded(px(palette::R_CONTROL_SM))
                                    .cursor_pointer()
                                    .text_size(palette::fs(palette::FS_SM))
                                    .text_color(p.text_secondary)
                                    .child(ico(icons::ADD, p.text_secondary))
                                    .child("Hinzufügen")
                                    .on_click(move |_, window, cx| {
                                        entity.update(cx, |app, cx| {
                                            Self::open_inline(date, window, cx, app)
                                        })
                                    }),
                            ),
                        )
                    }),
            )
    }

    fn unscheduled_panel(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.panel_search.read(cx).value().to_lowercase();
        let all = self.entries_for(None, p);
        let total = all.len();
        let entries: Vec<&Entry> = all
            .iter()
            .filter(|e| query.trim().is_empty() || e.title.to_lowercase().contains(query.trim()))
            .collect();
        let p = *p;
        let width = self.workspace.settings.cal_panel_width.clamp(220., 420.);
        let show_hint = !self.workspace.settings.cal_hint_dismissed || self.cal_hint_show;
        let hint_entity = cx.entity();
        let resize_entity = cx.entity();
        div()
            .flex()
            .h_full()
            .flex_none()
            .child(
                div()
                    .id("panel-resize")
                    .w(px(5.))
                    .h_full()
                    .flex_none()
                    .cursor_col_resize()
                    .hover(move |s| s.bg(p.accent.opacity(0.35)))
                    .when(self.resizing_panel, |d| d.bg(p.accent.opacity(0.5)))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        resize_entity.update(cx, |app, cx| {
                            app.resizing_panel = true;
                            cx.notify();
                        })
                    }),
            )
            .child(
                div()
                    .id("unscheduled")
                    .w(px(width))
                    .h_full()
                    .flex()
                    .flex_col()
                    .border_l_1()
                    .border_color(p.border_subtle)
                    .bg(p.bg_sidebar)
                    .drag_over::<CalItem>(move |s, _, _, _| s.bg(p.bg_active))
                    .on_drop(
                        cx.listener(|this, item: &CalItem, _, cx| this.drop_on_day(item, None, cx)),
                    )
                    .child(
                        div()
                            .h(px(48.))
                            .px(px(palette::SP_3))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!("Ohne Termin · {total}")),
                            )
                            .child(
                                icon_btn("panel-help", IconName::Info, "Hinweis anzeigen")
                                    .on_click(move |_, _, cx| {
                                        hint_entity.update(cx, |app, cx| {
                                            app.cal_hint_show = !app.cal_hint_show;
                                            cx.notify();
                                        })
                                    }),
                            ),
                    )
                    .when(show_hint, |d| {
                        d.child(
                            div()
                                .mx(px(palette::SP_3))
                                .mb(px(palette::SP_2))
                                .px(px(palette::SP_2))
                                .py(px(palette::SP_2))
                                .rounded(px(palette::R_INPUT))
                                .bg(p.bg_active)
                                .text_size(palette::fs(palette::FS_SM))
                                .text_color(p.text_secondary)
                                .child("Auf einen Tag ziehen, um einen Termin zu setzen"),
                        )
                    })
                    .child(
                        div().px(px(palette::SP_3)).pb(px(palette::SP_2)).child(
                            kit::input(&self.panel_search)
                                .aria_label("Ohne Termin durchsuchen")
                                .w_full(),
                        ),
                    )
                    .child(
                        div()
                            .id("unscheduled-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .pl(px(palette::SP_2))
                            .pr(px(palette::SCROLL_GUTTER))
                            .pb(px(palette::SP_2))
                            .flex()
                            .flex_col()
                            .gap(px(4.))
                            .when(entries.is_empty(), |d| {
                                d.child(div().py(px(palette::SP_6)).flex().justify_center().child(
                                    text_secondary(
                                        &p,
                                        if total == 0 {
                                            "Alles hat einen Termin."
                                        } else {
                                            "Keine Treffer."
                                        },
                                    ),
                                ))
                            })
                            .children(entries.iter().enumerate().map(|(i, e)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.))
                                    .child(ico(icons::DRAG, p.text_tertiary))
                                    .child(div().flex_1().min_w_0().child(self.pill(
                                        e,
                                        format!("un-{i}"),
                                        &p,
                                        false,
                                        cx,
                                    )))
                                    .into_any_element()
                            })),
                    ),
            )
    }

    fn more_overlay(
        &self,
        date: NaiveDate,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let entries = self.entries_for(Some(date), p);
        let p = *p;
        let format = self.workspace.settings.date_format;
        kit::surface("more-backdrop", &p)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui_kit::black().opacity(0.35))
            .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx)))
            .child(
                div()
                    .id("more-card")
                    .role(Role::Group)
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w(px(320.))
                    .max_h(px(420.))
                    .p(px(palette::SP_4))
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_2))
                    .rounded(px(palette::R_DIALOG))
                    .bg(p.bg_elevated)
                    .border_1()
                    .border_color(p.border_strong)
                    .shadow_lg()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .pb(px(4.))
                            .child(dates::full_with_weekday(date, format)),
                    )
                    .when(entries.is_empty(), |d| {
                        d.child(text_secondary(&p, "Für diesen Tag ist nichts geplant."))
                    })
                    .children(entries.iter().enumerate().map(|(i, e)| {
                        self.pill(e, format!("more-pill-{i}"), &p, false, cx)
                            .into_any_element()
                    }))
                    .child({
                        let entity = cx.entity();
                        kit::button("more-add")
                            .outline()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .icon(kit::icon(icons::ADD, palette::ICON_MD))
                            .label("Eintrag hinzufügen")
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |app, cx| {
                                    app.cal_more = None;
                                    Self::open_inline(date, window, cx, app)
                                })
                            })
                    }),
            )
    }

    /// Kontextmenü (Rechtsklick): Öffnen, Datum entfernen, Status ändern.
    pub(super) fn pill_menu_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let menu = self.pill_menu.expect("menu");
        let item = |id: &'static str, label: String, dot_color: Option<Hsla>| {
            div()
                .id(SharedString::from(format!("pm-{id}")))
                .h(px(palette::CTL_SM))
                .px(px(palette::SP_2))
                .flex()
                .items_center()
                .gap(px(palette::SP_2))
                .rounded(px(palette::R_CONTROL_SM))
                .cursor_pointer()
                .text_size(palette::fs(palette::FS_BASE))
                .hover(move |s| s.bg(p.bg_hover))
                .when_some(dot_color, |d, c| d.child(dot(c)))
                .child(label)
        };
        let uid = menu.update_id;
        let tid = menu.task_id;
        let e_open = cx.entity();
        let e_clear = cx.entity();
        let panel = div()
            .id("pill-menu")
            .min_w(px(180.))
            .p(px(4.))
            .flex()
            .flex_col()
            .rounded(px(palette::R_CARD))
            .bg(p.bg_elevated)
            .border_1()
            .border_color(p.border_subtle)
            .shadow_lg()
            .child(
                item("open", "Öffnen".into(), None).on_click(move |_, window, cx| {
                    e_open.update(cx, |app, cx| {
                        app.pill_menu = None;
                        app.select_update(uid, window, cx)
                    })
                }),
            )
            .child(
                item("clear", "Datum entfernen".into(), None).on_click(move |_, _, cx| {
                    e_clear.update(cx, |app, cx| {
                        app.pill_menu = None;
                        let target = match tid {
                            Some(t) => DateTarget::Task(uid, t),
                            None => DateTarget::Update(uid),
                        };
                        app.set_date(target, None, cx);
                    })
                }),
            )
            .when(tid.is_none(), |d| {
                d.child(div().h(px(1.)).mx(px(4.)).my(px(4.)).bg(p.border_subtle))
                    .children(Status::ALL.map(|s| {
                        let entity = cx.entity();
                        item(
                            s.label(),
                            format!("Status: {}", s.label()),
                            Some(p.status(s)),
                        )
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| {
                                app.pill_menu = None;
                                app.set_status(uid, s, cx);
                            })
                        })
                    }))
            });
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .id("pill-menu-backdrop")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                    ),
            )
            .child(deferred(
                anchored()
                    .position(menu.anchor)
                    .snap_to_window_with_margin(px(8.))
                    .child(panel),
            ))
    }

    pub(super) fn calendar_view(&self, narrow: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let st = &self.workspace.settings;
        let week_start: WeekStart = st.week_start;
        let anchor = self.cal_anchor;
        let month_mode = self.cal_mode == CalMode::Month;
        let first = model::month_start(anchor);
        let (start, rows) = if month_mode {
            let lead = week_start.offset(first.weekday());
            let days = (model::shift_month(first, 1) - first).num_days();
            (
                first - Duration::days(lead),
                ((lead + days + 6) / 7) as usize,
            )
        } else {
            (
                anchor - Duration::days(week_start.offset(anchor.weekday())),
                1,
            )
        };
        let title = if month_mode {
            format!(
                "{} {}",
                dates::MONTHS[first.month0() as usize],
                first.year()
            )
        } else {
            let end = start + Duration::days(6);
            format!(
                "{}. {} – {}. {} {}",
                start.day(),
                dates::month_short(start).trim_end_matches('.'),
                end.day(),
                dates::month_short(end).trim_end_matches('.'),
                end.year()
            )
        };
        let limit = (st.max_pills.clamp(2, 4) as usize)
            .saturating_sub(narrow as usize)
            .max(2);
        let show_weeks = st.week_numbers && month_mode;
        let panel_open = self.cal_unscheduled;
        let entity = cx.entity();
        let shift = |dir: i32| {
            let entity = cx.entity();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                entity.update(cx, |app, cx| app.shift_calendar(dir, cx))
            }
        };
        let mode_entity = cx.entity();
        let panel_entity = cx.entity();

        div()
            .size_full()
            .flex()
            .bg(p.bg_panel)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    // Sticky Kopfzeile: liegt außerhalb des scrollenden Rasters.
                    .child(
                        div()
                            .min_h(px(64.))
                            .py(px(palette::SP_2))
                            .px(px(palette::SP_6))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .flex_wrap()
                            .gap(px(palette::SP_3))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(palette::SP_3))
                                    .flex_wrap()
                                    .child(
                                        div()
                                            .text_size(palette::fs(palette::FS_XL))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(
                                                icon_btn(
                                                    "calendar-prev",
                                                    IconName::ChevronLeft,
                                                    "Zurück · ←",
                                                )
                                                .on_click(shift(-1)),
                                            )
                                            .child(
                                                icon_btn(
                                                    "calendar-next",
                                                    icons::COLLAPSE,
                                                    "Weiter",
                                                )
                                                .on_click(shift(1)),
                                            )
                                            .child(
                                                kit::button("calendar-today")
                                                    .outline()
                                                    .with_size(gpui_kit::component::Size::Small)
                                                    .h(px(palette::CTL_SM))
                                                    .label("Heute")
                                                    .tooltip("Heute · T")
                                                    .on_click(move |_, _, cx| {
                                                        entity.update(cx, |app, cx| {
                                                            app.cal_anchor = model::today();
                                                            cx.notify();
                                                        })
                                                    }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(palette::SP_2))
                                    .child(segment(
                                        "cal-mode",
                                        &p,
                                        &[
                                            (Some(icons::DATE), "Monat"),
                                            (Some(IconName::CalendarDays), "Woche"),
                                        ],
                                        self.cal_mode as usize,
                                        true,
                                        move |i, _, cx| {
                                            mode_entity.update(cx, |app, cx| {
                                                app.cal_mode = [CalMode::Month, CalMode::Week][i];
                                                cx.notify();
                                            })
                                        },
                                    ))
                                    .child(
                                        icon_btn(
                                            "calendar-unscheduled",
                                            if panel_open {
                                                IconName::PanelRightClose
                                            } else {
                                                IconName::PanelRightOpen
                                            },
                                            "Ohne Termin ein-/ausblenden",
                                        )
                                        .on_click(
                                            move |_, _, cx| {
                                                panel_entity.update(cx, |app, cx| {
                                                    app.cal_unscheduled = !app.cal_unscheduled;
                                                    app.workspace.settings.cal_panel_open =
                                                        app.cal_unscheduled;
                                                    app.persist(cx);
                                                    cx.notify();
                                                })
                                            },
                                        ),
                                    ),
                            ),
                    )
                    // Wochenkopf linksbündig mit denselben Einzügen wie die Tageszahlen.
                    .when(month_mode, |d| {
                        d.child(
                            div()
                                .h(px(28.))
                                .flex_none()
                                .flex()
                                .border_t_1()
                                .border_b_1()
                                .border_color(p.border_subtle.opacity(0.6))
                                .when(show_weeks, |d| d.child(div().w(px(32.)).flex_none()))
                                .children(week_start.names().map(|name| {
                                    div()
                                        .flex_1()
                                        .pl(px(palette::SP_3))
                                        .flex()
                                        .items_center()
                                        .text_size(palette::fs(palette::FS_SM))
                                        .text_color(p.text_secondary)
                                        .child(name)
                                })),
                        )
                    })
                    .child(
                        div()
                            .id("calendar-grid")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .flex()
                            .flex_col()
                            .children((0..rows).map(|week| {
                                let week_first = start + Duration::days((week * 7) as i64);
                                div()
                                    .flex_1()
                                    .min_h(px(if month_mode { 96. } else { 280. }))
                                    .flex()
                                    .when(show_weeks, |d| {
                                        d.child(
                                            div()
                                                .w(px(32.))
                                                .flex_none()
                                                .pt(px(8.))
                                                .flex()
                                                .justify_center()
                                                .text_size(palette::fs(11.))
                                                .text_color(p.text_tertiary)
                                                .border_r_1()
                                                .border_b_1()
                                                .border_color(p.border_subtle.opacity(0.6))
                                                .child(week_first.iso_week().week().to_string()),
                                        )
                                    })
                                    .children((0..7).map(|day| {
                                        let date = start + Duration::days((week * 7 + day) as i64);
                                        if month_mode {
                                            self.day_cell(date, first.month(), limit, &p, cx)
                                                .into_any_element()
                                        } else {
                                            self.week_column(date, &p, cx).into_any_element()
                                        }
                                    }))
                            })),
                    ),
            )
            .when(panel_open && !narrow, |d| {
                d.child(self.unscheduled_panel(&p, cx))
            })
            .when_some(self.cal_more, |d, date| {
                d.child(self.more_overlay(date, &p, cx))
            })
    }
}
