use super::*;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};

type MenuFn = dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu;

/// Menü für ein Update (Kontextmenü, "…" in der Liste und im Detail).
pub fn update_menu(
    entity: Entity<NoteWarden>,
    id: u64,
    archived: bool,
    trashed: bool,
    detail: bool,
    released: bool,
) -> Box<MenuFn> {
    Box::new(move |menu, window, cx| {
        let item =
            |label: &'static str,
             f: fn(&mut NoteWarden, u64, &mut Window, &mut Context<NoteWarden>)| {
                let entity = entity.clone();
                PopupMenuItem::new(label).on_click(move |_, window, cx| {
                    entity.update(cx, |app, cx| f(app, id, window, cx))
                })
            };
        if trashed {
            return menu
                .item(item("Wiederherstellen", |app, id, _, cx| {
                    app.restore_update(id, cx)
                }))
                .item(item("Endgültig löschen …", |app, id, window, cx| {
                    app.open_dialog(Dialog::ConfirmPurge(id), window, cx)
                }));
        }
        if archived {
            return menu
                .item(item("Wiederherstellen", |app, id, _, cx| {
                    app.restore_update(id, cx)
                }))
                .item(item("Duplizieren", |app, id, window, cx| {
                    app.duplicate_update(id, window, cx)
                }))
                .item(item("Löschen", |app, id, window, cx| {
                    app.request_trash(id, window, cx)
                }));
        }
        let status_entity = entity.clone();
        let mut menu = menu
            .item(item("Duplizieren", |app, id, window, cx| {
                app.duplicate_update(id, window, cx)
            }))
            .submenu("Status ändern", window, cx, move |sub, _, _| {
                Status::ALL.iter().fold(sub, |sub, status| {
                    let entity = status_entity.clone();
                    let status = *status;
                    sub.item(
                        PopupMenuItem::new(status.label()).on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| app.set_status(id, status, cx))
                        }),
                    )
                })
            })
            .item(item("Datum setzen …", |app, id, window, cx| {
                let pos = window.mouse_position();
                app.open_date_pop(DateTarget::Update(id), pos, window, cx)
            }));
        if detail {
            menu = menu.item(item("Als Markdown exportieren …", |app, id, _, cx| {
                app.export_update_md(id, cx)
            }));
        }
        menu = if released {
            menu.item(item("Veröffentlichung zurücknehmen", |app, id, _, cx| {
                app.unpublish_update(id, cx)
            }))
        } else {
            menu.item(item("Veröffentlichen …", |app, id, window, cx| {
                app.open_publish(id, window, cx)
            }))
        };
        menu.item(item("Versionsverlauf …", |app, id, window, cx| {
            app.open_history(id, window, cx)
        }))
        .item(item("Archivieren", |app, id, window, cx| {
            app.archive_update(id, window, cx)
        }))
        .item(item("Löschen", |app, id, window, cx| {
            app.request_trash(id, window, cx)
        }))
    })
}

#[derive(Clone)]
enum Row<'a> {
    Header(String, usize),
    Card(&'a model::Update),
}

impl NoteWarden {
    fn matches_query(update: &model::Update, query: &str) -> bool {
        query.is_empty()
            || fuzzy_matches(&update.title, query)
            || update.notes.to_lowercase().contains(query)
            || update.tasks.iter().any(|t| {
                t.title.to_lowercase().contains(query)
                    || t.description.to_lowercase().contains(query)
            })
            || update
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(query.trim_start_matches('#')))
    }

    /// Gehört das Update zur Sidebar-Ansicht (ohne Suche und Filter aus dem Popover)?
    fn in_scope(&self, u: &model::Update) -> bool {
        let today = self.now_today();
        let week_end = self.week_end(today);
        match self.filter {
            Filter::All => u.is_live(),
            Filter::Status(s) => u.is_live() && u.status == s,
            Filter::Overdue => u.is_live() && u.overdue(today),
            Filter::NoDate => u.is_live() && u.planned_for.is_none(),
            Filter::ThisWeek => {
                u.is_live() && u.planned_for.is_some_and(|d| d >= today && d <= week_end)
            }
            Filter::Archive => u.archived && u.deleted_on.is_none(),
            Filter::Trash => u.deleted_on.is_some(),
        }
    }

    pub fn visible_updates(&self) -> Vec<&model::Update> {
        let today = self.now_today();
        let week_end = self.week_end(today);
        let query = self.search_query.trim().to_lowercase();
        let mut list: Vec<&model::Update> = self
            .workspace
            .updates
            .iter()
            .filter(|u| {
                self.project_filter
                    .is_none_or(|id| u.project_id == Some(id))
            })
            .filter(|u| self.passes_tag_filter(u))
            .filter(|u| self.in_scope(u))
            .filter(|u| Self::matches_query(u, &query))
            .filter(|u| {
                let ui = &self.workspace.settings.updates_ui;
                (ui.statuses.is_empty() || ui.statuses.contains(&u.status))
                    && (ui.priorities.is_empty() || ui.priorities.contains(&u.priority))
                    && match ui.date_filter {
                        1 => u.overdue(today),
                        2 => u.planned_for == Some(today),
                        3 => u.planned_for.is_some_and(|d| d >= today && d <= week_end),
                        4 => u
                            .planned_for
                            .is_some_and(|d| d >= today && d < today + Duration::days(7)),
                        5 => u.planned_for.is_none(),
                        6 => u.planned_for.is_some_and(|d| {
                            ui.range_start.is_none_or(|a| d >= a)
                                && ui.range_end.is_none_or(|b| d <= b)
                        }),
                        _ => true,
                    }
            })
            .collect();
        match self.sort {
            SortKey::Edited => list.sort_by_key(|u| std::cmp::Reverse((u.edited_at, u.id))),
            SortKey::Date => list.sort_by_key(|u| (u.planned_for.is_none(), u.planned_for, u.id)),
            SortKey::Status => {
                list.sort_by_key(|u| (u.status.order(), u.planned_for.is_none(), u.planned_for))
            }
            SortKey::Priority => list.sort_by_key(|u| std::cmp::Reverse(u.priority)),
            SortKey::Title => list.sort_by_key(|u| u.title.to_lowercase()),
        }
        if self.workspace.settings.updates_ui.reverse {
            list.reverse();
        }
        list
    }

    fn period_label(&self, update: &model::Update, today: NaiveDate) -> &'static str {
        match update.planned_for {
            None => "Ohne Termin",
            Some(d) if d < today && update.status != Status::Erledigt => "Überfällig",
            Some(d) if d == today => "Heute",
            Some(d) if d <= self.week_end(today) && d > today => "Diese Woche",
            Some(d) if d < today => "Früher",
            Some(_) => "Später",
        }
    }

    fn rows<'a>(&self, list: Vec<&'a model::Update>) -> Vec<Row<'a>> {
        let today = self.now_today();
        let mut rows = Vec::new();
        match self.group {
            GroupBy::None => rows.extend(list.into_iter().map(Row::Card)),
            GroupBy::Status => {
                for status in [
                    Status::InArbeit,
                    Status::Geplant,
                    Status::Idee,
                    Status::Erledigt,
                ] {
                    let part: Vec<_> = list
                        .iter()
                        .copied()
                        .filter(|u| u.status == status)
                        .collect();
                    if !part.is_empty() {
                        rows.push(Row::Header(status.label().into(), part.len()));
                        rows.extend(part.into_iter().map(Row::Card));
                    }
                }
            }
            GroupBy::Period => {
                for label in [
                    "Überfällig",
                    "Heute",
                    "Diese Woche",
                    "Später",
                    "Früher",
                    "Ohne Termin",
                ] {
                    let part: Vec<_> = list
                        .iter()
                        .copied()
                        .filter(|u| self.period_label(u, today) == label)
                        .collect();
                    if !part.is_empty() {
                        rows.push(Row::Header(label.into(), part.len()));
                        rows.extend(part.into_iter().map(Row::Card));
                    }
                }
            }
            GroupBy::Project | GroupBy::Priority => {
                let mut groups: std::collections::BTreeMap<String, Vec<&model::Update>> =
                    Default::default();
                for u in list {
                    let label = if self.group == GroupBy::Project {
                        u.project_id
                            .and_then(|id| self.workspace.projects.iter().find(|p| p.id == id))
                            .map(|p| p.name.clone())
                            .unwrap_or("Kein Projekt".into())
                    } else {
                        ["Keine Priorität", "Niedrig", "Mittel", "Hoch"][u.priority.min(3) as usize]
                            .into()
                    };
                    groups.entry(label).or_default().push(u);
                }
                for (label, group) in groups {
                    rows.push(Row::Header(label, group.len()));
                    rows.extend(group.into_iter().map(Row::Card));
                }
            }
        }
        let mut collapsed = false;
        rows.retain(|row| match row {
            Row::Header(label, _) => {
                collapsed = self.workspace.settings.updates_ui.collapsed.contains(label);
                true
            }
            Row::Card(_) => !collapsed,
        });
        rows
    }

    /// Meta-Teile einer Update-Zeile oder -Karte in fester Reihenfolge: Projekt · Fortschritt · Datum.
    pub(super) fn meta_parts(&self, update: &model::Update, p: &Palette) -> Vec<AnyElement> {
        let (done, total) = update.task_progress();
        let today = self.now_today();
        let format = self.workspace.settings.date_format;
        let overdue = update
            .planned_for
            .is_some_and(|d| d < today && update.status != Status::Erledigt);
        let mut meta: Vec<AnyElement> = Vec::new();
        if let Some(pj) = update
            .project_id
            .and_then(|pid| self.workspace.projects.iter().find(|p| p.id == pid))
        {
            meta.push(
                div()
                    .id(("meta-project", update.id))
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_1))
                    .tooltip(tip(pj.name.clone()))
                    .child(dot(palette::project_color(pj, p)).size(px(6.)))
                    .child(div().truncate().child(pj.name.clone()))
                    .into_any_element(),
            );
        }
        if total > 0 {
            meta.push(
                div()
                    .flex_none()
                    .child(format!("{done}/{total}"))
                    .into_any_element(),
            );
        }
        if let Some(d) = update.planned_for {
            let text = if self.workspace.settings.relative_dates {
                dates::relative(d, today, format)
            } else {
                dates::absolute(d, today, format)
            };
            meta.push(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_1))
                    .when(overdue, |d| {
                        d.text_color(p.danger)
                            .child(kit::icon(icons::DATE, palette::ICON_XS))
                    })
                    .child(text)
                    .into_any_element(),
            );
        }
        for tag in update.tags.iter().take(3) {
            meta.push(
                div()
                    .flex_none()
                    .text_color(palette::tag_color(tag, p))
                    .child(format!("#{tag}"))
                    .into_any_element(),
            );
        }
        meta
    }

    fn card(&self, update: &model::Update, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let id = update.id;
        let e = cx.entity();
        let compact = self.workspace.settings.updates_ui.compact;
        let selected = self.selected_update == Some(id) || self.multi.contains(&id);
        let group: SharedString = format!("row-{id}").into();
        let (_, total) = update.task_progress();
        let project = update
            .project_id
            .and_then(|pid| self.workspace.projects.iter().find(|p| p.id == pid));
        // Meta-Zeile als Fließtext: Projekt · Fortschritt · Datum (ohne Chips).
        let mut meta = self.meta_parts(update, &p);
        if !self.search_query.is_empty() && !update.notes.trim().is_empty() {
            meta.push(
                div()
                    .min_w_0()
                    .truncate()
                    .child(
                        update
                            .notes
                            .chars()
                            .take(60)
                            .collect::<String>()
                            .replace('\n', " "),
                    )
                    .into_any_element(),
            );
        }
        let meta_line = kit::meta_line(&p, meta);
        let has_meta = project.is_some()
            || total > 0
            || update.planned_for.is_some()
            || !update.tags.is_empty();
        let title = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(palette::SP_2))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(palette::fs(palette::FS_MD))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(p.text_primary)
                    .child(match update.display_title() {
                        Some(title) => {
                            highlighted(&title, &self.search_query, &p).into_any_element()
                        }
                        None => div()
                            .text_color(p.text_tertiary)
                            .child("Ohne Titel")
                            .into_any_element(),
                    }),
            )
            .when(update.priority > 0, |d| {
                d.child(kit::priority_icon(
                    ("row-priority", id),
                    &p,
                    update.priority,
                ))
            });
        let row_bg = if selected { p.bg_active } else { p.bg_app };
        kit::surface(("update-row", id), &p)
            .w_full()
            .h(palette::fs(if compact {
                palette::ROW_COMPACT
            } else {
                palette::ROW_COMFORTABLE
            }))
            .role(Role::ListBoxOption)
            .tab_stop(selected)
            .selected(selected)
            .group(group.clone())
            .relative()
            .rounded(px(palette::R_INPUT))
            .px(px(palette::SP_2))
            .flex()
            .items_center()
            .gap(px(palette::SP_2))
            .bg(row_bg)
            .border_l_2()
            .border_color(if selected {
                p.accent
            } else {
                gpui_kit::transparent_black()
            })
            .accessibility_label(format!(
                "{} · {}",
                update
                    .display_title()
                    .unwrap_or_else(|| "Ohne Titel".into()),
                update.status.label()
            ))
            .tooltip(tip({
                let mut parts = vec![
                    update
                        .display_title()
                        .unwrap_or_else(|| "Ohne Titel".into()),
                    update.status.label().to_owned(),
                ];
                if let Some(project) = project {
                    parts.push(project.name.clone());
                }
                if total > 0 {
                    let (done, total) = update.task_progress();
                    parts.push(format!("{done}/{total} Aufgaben"));
                }
                if let Some(date) = update.planned_for {
                    parts.push(dates::full_with_weekday(
                        date,
                        self.workspace.settings.date_format,
                    ));
                }
                parts.join(" · ")
            }))
            .on_click(move |ev, w, cx| {
                e.update(cx, |a, cx| {
                    a.click_card(id, ev.modifiers().control, ev.modifiers().shift, w, cx)
                })
            })
            .child(kit::row_leading(self.status_control(update, cx)))
            .child(if compact {
                div().flex_1().min_w_0().child(title)
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(title)
                    .when(has_meta || !self.search_query.is_empty(), |d| {
                        d.child(meta_line)
                    })
            })
            .child(
                kit::row_actions(("row-actions", id), group.clone())
                    .shown(selected)
                    .child({
                        let entity = cx.entity();
                        icon_btn(("row-date", id), icons::DATE_ADD, "Datum setzen").on_click(
                            move |_, window, cx| {
                                let pos = window.mouse_position();
                                entity.update(cx, |app, cx| {
                                    app.open_date_pop(DateTarget::Update(id), pos, window, cx)
                                });
                            },
                        )
                    })
                    .child(
                        icon_btn(("row-more", id), icons::MORE, "Weitere Aktionen").dropdown_menu(
                            {
                                let build = update_menu(
                                    cx.entity(),
                                    id,
                                    update.archived,
                                    update.deleted_on.is_some(),
                                    false,
                                    update.release.is_some(),
                                );
                                move |m, w, cx| build(m, w, cx)
                            },
                        ),
                    ),
            )
            .context_menu({
                let build = update_menu(
                    cx.entity(),
                    id,
                    update.archived,
                    update.deleted_on.is_some(),
                    false,
                    update.release.is_some(),
                );
                move |m, w, cx| build(m, w, cx)
            })
    }

    fn render_row(&self, row: Row<'_>, uniform: bool, cx: &mut Context<Self>) -> AnyElement {
        match row {
            Row::Header(label, n) => {
                let p = palette::get(cx);
                let closed = self
                    .workspace
                    .settings
                    .updates_ui
                    .collapsed
                    .contains(&label);
                let e = cx.entity();
                let name = label.clone();
                kit::surface(SharedString::from(format!("group-{label}")), &p)
                    .w_full()
                    .h(if uniform {
                        palette::fs(if self.workspace.settings.updates_ui.compact {
                            palette::ROW_COMPACT
                        } else {
                            palette::ROW_COMFORTABLE
                        })
                    } else {
                        px(palette::CTL_SM)
                    })
                    .px(px(palette::SP_2))
                    .rounded(px(palette::R_CONTROL_SM))
                    .flex()
                    .items_center()
                    .justify_start()
                    .gap(px(palette::SP_1))
                    .text_size(palette::fs(palette::FS_SM))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(p.text_tertiary)
                    .child(
                        kit::icon(
                            if closed {
                                icons::COLLAPSE
                            } else {
                                icons::EXPAND
                            },
                            palette::ICON_SM,
                        )
                        .text_color(p.text_tertiary),
                    )
                    .child(label)
                    .child(div().font_weight(FontWeight::NORMAL).child(n.to_string()))
                    .on_click(move |_, _, cx| {
                        e.update(cx, |a, cx| {
                            let groups = &mut a.workspace.settings.updates_ui.collapsed;
                            if groups.contains(&name) {
                                groups.retain(|s| s != &name);
                            } else {
                                groups.push(name.clone());
                            }
                            a.persist(cx);
                            cx.notify();
                        })
                    })
                    .into_any_element()
            }
            Row::Card(update) => self.card(update, cx).into_any_element(),
        }
    }

    pub(super) fn update_list(
        &self,
        narrow: bool,
        width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let list = self.visible_updates();
        let count = list.len();
        let any_updates = self.workspace.updates.iter().any(|u| u.is_live());
        let searching = !self.search_query.trim().is_empty();
        let rows = self.rows(list);
        let entity = cx.entity();
        let template_entity = cx.entity();
        let templates: Vec<String> = self
            .workspace
            .settings
            .templates
            .iter()
            .map(|t| t.name.clone())
            .collect();
        let filter = self.filter;
        let scope_total = self
            .workspace
            .updates
            .iter()
            .filter(|u| self.in_scope(u))
            .count();

        div()
            .id("update-list")
            .role(Role::ListBox)
            .aria_label("Updates")
            .key_context("UpdatesList")
            .tab_stop(true)
            .on_key_down(cx.listener(Self::update_keys))
            .when(narrow, |d| d.flex_1())
            .when(!narrow, |d| d.w(px(width)).flex_none())
            .h_full()
            .flex()
            .flex_col()
            .bg(p.bg_app)
            .child(
                div()
                    .px(px(palette::SP_3))
                    .pt(px(palette::SP_4))
                    .pb(px(palette::SP_3))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .pl(px(palette::SP_2))
                            .flex()
                            .items_baseline()
                            .gap(px(palette::SP_2))
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_XL))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.text_primary)
                                    .child(filter.title()),
                            )
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_BASE))
                                    .text_color(p.text_tertiary)
                                    .child(if count == scope_total {
                                        count.to_string()
                                    } else {
                                        format!("{count} von {scope_total}")
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .rounded(px(palette::R_INPUT))
                            .flex_none()
                            .bg(p.accent)
                            .child(
                                kit::button("list-new")
                                    .primary()
                                    .rounded_none()
                                    .rounded_tl(px(palette::R_INPUT))
                                    .rounded_bl(px(palette::R_INPUT))
                                    .icon(kit::icon(icons::ADD, palette::ICON_MD))
                                    .label("Neu")
                                    .tooltip("Neues Update · Strg+N")
                                    .on_click({
                                        let entity = entity.clone();
                                        move |_, window, cx| {
                                            entity.update(cx, |app, cx| {
                                                app.create_update(None, None, window, cx)
                                            })
                                        }
                                    }),
                            )
                            .child(
                                kit::button("list-new-more")
                                    .primary()
                                    .rounded_none()
                                    .rounded_tr(px(palette::R_INPUT))
                                    .rounded_br(px(palette::R_INPUT))
                                    .w(px(palette::CTL_MD))
                                    .px(px(0.))
                                    .border_l_1()
                                    .border_color(p.accent_contrast.opacity(0.3))
                                    .child(
                                        kit::icon(icons::EXPAND, palette::ICON_SM)
                                            .text_color(p.accent_contrast),
                                    )
                                    .tooltip("Weitere Optionen zum Anlegen")
                                    .accessibility_label("Weitere Optionen zum Anlegen")
                                    .dropdown_menu(move |menu, _, _| {
                                        let entity = template_entity.clone();
                                        let mut menu = menu.item(
                                            PopupMenuItem::new("Neues Update").on_click({
                                                let entity = entity.clone();
                                                move |_, window, cx| {
                                                    entity.update(cx, |app, cx| {
                                                        app.create_update(None, None, window, cx)
                                                    })
                                                }
                                            }),
                                        );
                                        for (index, name) in templates.iter().enumerate() {
                                            let entity = entity.clone();
                                            menu = menu.item(
                                                PopupMenuItem::new(format!("Aus Vorlage: {name}"))
                                                    .on_click(move |_, window, cx| {
                                                        entity.update(cx, |app, cx| {
                                                            app.create_update(
                                                                None,
                                                                Some(index),
                                                                window,
                                                                cx,
                                                            )
                                                        })
                                                    }),
                                            );
                                        }
                                        let inbox = entity.clone();
                                        menu.item(PopupMenuItem::new("Aufgabe in Inbox").on_click(
                                            move |_, window, cx| {
                                                inbox.update(cx, |a, cx| {
                                                    a.set_view(View::Inbox, cx);
                                                    a.quick_input
                                                        .update(cx, |i, cx| i.focus(window, cx));
                                                })
                                            },
                                        ))
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .px(px(palette::SP_3))
                    .pb(px(palette::SP_3))
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .child(
                        div().flex_1().min_w_0().child(
                            kit::input(&self.search_input)
                                .aria_label("Updates durchsuchen")
                                .prefix(ico(icons::SEARCH, p.text_tertiary))
                                .cleanable(true)
                                .w_full(),
                        ),
                    )
                    .child(self.view_popover(cx)),
            )
            .when_some(self.filter_chips(cx), |d, chips| d.child(chips))
            .when(filter == Filter::Trash, |d| {
                d.child(div().px(px(16.)).pb(px(8.)).child(text_secondary(
                    &p,
                    "Einträge werden nach Ablauf der Aufbewahrungsdauer endgültig gelöscht.",
                )))
            })
            .child(
                div()
                    .id("list-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.list_scroll)
                    .vertical_scrollbar(&self.list_scroll)
                    .pl(px(palette::SP_3))
                    .pr(px(palette::SCROLL_GUTTER))
                    .pb(px(palette::SP_4))
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .when(rows.is_empty(), |d| {
                        let entity = entity.clone();
                        d.child(
                            div()
                                .mt(px(40.))
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(palette::SP_3))
                                .child(ico(
                                    if searching {
                                        icons::SEARCH
                                    } else {
                                        icons::INBOX
                                    },
                                    p.text_tertiary,
                                ))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(p.text_primary)
                                        .child(if searching || scope_total > 0 {
                                            "Nichts gefunden"
                                        } else if filter == Filter::Archive {
                                            "Archiv ist leer"
                                        } else if filter == Filter::Trash {
                                            "Papierkorb ist leer"
                                        } else if !any_updates && filter == Filter::All {
                                            "Noch keine Updates"
                                        } else {
                                            "Keine Updates passen zu den Filtern"
                                        }),
                                )
                                .child(text_secondary(
                                    &p,
                                    if searching {
                                        "Versuche einen anderen Suchbegriff."
                                    } else if scope_total > 0 {
                                        "Passe die aktiven Filter an."
                                    } else if filter == Filter::Archive {
                                        "Archivierte Updates erscheinen hier."
                                    } else if filter == Filter::Trash {
                                        "Gelöschte Updates kannst du hier wiederherstellen."
                                    } else {
                                        "Hier erscheinen passende Updates."
                                    },
                                ))
                                .when(
                                    searching
                                        || self.active_filter_count() > 0
                                        || (scope_total == 0
                                            && !matches!(
                                                filter,
                                                Filter::All | Filter::Archive | Filter::Trash
                                            )),
                                    |d| {
                                        d.child(
                                            kit::button("clear-list-filters")
                                                .ghost()
                                                .label(if searching {
                                                    "Suche leeren"
                                                } else {
                                                    "Filter zurücksetzen"
                                                })
                                                .on_click({
                                                    let e = cx.entity();
                                                    move |_, w, cx| {
                                                        e.update(cx, |a, cx| {
                                                            if searching {
                                                                a.search_query.clear();
                                                                a.search_input
                                                                    .update(cx, |i, cx| {
                                                                        i.set_value("", w, cx)
                                                                    });
                                                            } else {
                                                                a.reset_filters(cx);
                                                                if !matches!(
                                                                    a.filter,
                                                                    Filter::Archive | Filter::Trash
                                                                ) {
                                                                    a.filter = Filter::All;
                                                                }
                                                            }
                                                            a.save_updates_ui(cx);
                                                        })
                                                    }
                                                }),
                                        )
                                    },
                                )
                                .when(!any_updates && filter == Filter::All && !searching, |d| {
                                    d.child(
                                        kit::button("first-update")
                                            .primary()
                                            .label("Erstes Update anlegen")
                                            .on_click(move |_, window, cx| {
                                                entity.update(cx, |app, cx| {
                                                    app.create_update(None, None, window, cx)
                                                })
                                            }),
                                    )
                                }),
                        )
                    })
                    .when(count < 100, |d| {
                        d.children(rows.into_iter().map(|row| self.render_row(row, false, cx)))
                    })
                    .when(count >= 100, |d| {
                        let e = cx.entity();
                        let row_count = self.rows(self.visible_updates()).len();
                        d.overflow_hidden().child(
                            uniform_list("virtual-updates", row_count, move |range, _, cx| {
                                e.update(cx, |a, cx| {
                                    let rows = a.rows(a.visible_updates());
                                    range
                                        .filter_map(|i| rows.get(i).cloned())
                                        .map(|row| a.render_row(row, true, cx))
                                        .collect::<Vec<_>>()
                                })
                            })
                            .track_scroll(&self.virtual_scroll)
                            .size_full(),
                        )
                    }),
            )
            .when(!self.multi.is_empty(), |d| d.child(self.bulk_bar(cx)))
    }

    /// Klick auf eine Karte: Strg schaltet um, Umschalt wählt einen Bereich, sonst normal öffnen.
    pub fn click_card(
        &mut self,
        id: u64,
        ctrl: bool,
        shift: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ctrl {
            if self.multi.is_empty()
                && let Some(current) = self.selected_update
            {
                self.multi.insert(current);
            }
            if !self.multi.remove(&id) {
                self.multi.insert(id);
            }
            cx.notify();
        } else if shift {
            let order: Vec<u64> = self.visible_updates().iter().map(|u| u.id).collect();
            let anchor = self.selected_update.unwrap_or(id);
            if let (Some(a), Some(b)) = (
                order.iter().position(|x| *x == anchor),
                order.iter().position(|x| *x == id),
            ) {
                let (from, to) = (a.min(b), a.max(b));
                self.multi = order[from..=to].iter().copied().collect();
            }
            cx.notify();
        } else {
            self.multi.clear();
            self.select_update(id, window, cx);
        }
    }

    pub fn request_trash(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.settings.undo_instead_of_dialog {
            self.trash_update(id, window, cx);
        } else {
            self.open_dialog(Dialog::ConfirmTrash(id), window, cx);
        }
    }

    fn bulk_apply(&mut self, cx: &mut Context<Self>, f: impl Fn(&mut model::Update)) -> usize {
        if !self.writable {
            return 0;
        }
        let ids: Vec<u64> = self.multi.iter().copied().collect();
        for id in &ids {
            if let Some(update) = self.workspace.update_mut(*id) {
                f(update);
                update.touch();
            }
        }
        self.persist(cx);
        cx.notify();
        ids.len()
    }

    pub fn bulk_set_status(&mut self, status: Status, cx: &mut Context<Self>) {
        let n = self.bulk_apply(cx, |u| u.status = status);
        self.toast(
            format!(
                "Status gesetzt für {}",
                model::plural(n, "Update", "Updates")
            ),
            None,
            cx,
        );
    }

    pub fn bulk_set_date(&mut self, date: Option<NaiveDate>, cx: &mut Context<Self>) {
        let n = self.bulk_apply(cx, |u| {
            u.planned_for = date;
            if date.is_some() && u.status == Status::Idee {
                u.status = Status::Geplant;
            }
        });
        self.toast(
            format!(
                "Datum gesetzt für {}",
                model::plural(n, "Update", "Updates")
            ),
            None,
            cx,
        );
    }

    pub fn bulk_archive_or_trash(
        &mut self,
        trash: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let today = model::today();
        let n = self.bulk_apply(cx, |u| {
            if trash {
                u.deleted_on = Some(today);
            } else {
                u.archived = true;
            }
        });
        self.multi.clear();
        if self
            .selected_update
            .is_some_and(|id| !self.workspace.update(id).is_some_and(|u| u.is_live()))
        {
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
        self.toast(
            format!(
                "{} {}",
                model::plural(n, "Update", "Updates"),
                if trash {
                    "in den Papierkorb verschoben"
                } else {
                    "archiviert"
                }
            ),
            None,
            cx,
        );
    }

    /// Aktionsleiste am unteren Rand bei Mehrfachauswahl.
    fn bulk_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let n = self.multi.len();
        let status_entity = cx.entity();
        let date_entity = cx.entity();
        let archive = cx.entity();
        let trash = cx.entity();
        let close = cx.entity();
        div()
            .flex_none()
            .min_h(px(52.))
            .py(px(palette::SP_2))
            .px(px(palette::SP_3))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(palette::SP_2))
            .bg(p.bg_elevated)
            .border_t_1()
            .border_color(p.border_subtle)
            .child(
                div()
                    .flex_1()
                    .text_size(palette::fs(palette::FS_BASE))
                    .child(format!("{n} ausgewählt")),
            )
            .child(
                kit::button("bulk-status")
                    .outline()
                    .with_size(gpui_kit::component::Size::Small)
                    .h(px(palette::CTL_SM))
                    .label("Status")
                    .dropdown_menu(move |menu, _, _| {
                        Status::ALL.iter().fold(menu, |menu, s| {
                            let entity = status_entity.clone();
                            let s = *s;
                            menu.item(PopupMenuItem::new(s.label()).on_click(move |_, _, cx| {
                                entity.update(cx, |app, cx| app.bulk_set_status(s, cx))
                            }))
                        })
                    }),
            )
            .child(
                kit::button("bulk-date")
                    .outline()
                    .with_size(gpui_kit::component::Size::Small)
                    .h(px(palette::CTL_SM))
                    .label("Datum")
                    .dropdown_menu(move |menu, _, _| {
                        let today = model::today();
                        [
                            ("Heute", Some(today)),
                            ("Morgen", Some(today + Duration::days(1))),
                            ("In einer Woche", Some(today + Duration::days(7))),
                            ("Datum entfernen", None),
                        ]
                        .into_iter()
                        .fold(menu, |menu, (label, date)| {
                            let entity = date_entity.clone();
                            menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                                entity.update(cx, |app, cx| app.bulk_set_date(date, cx))
                            }))
                        })
                    }),
            )
            .child(
                kit::button("bulk-archive")
                    .outline()
                    .with_size(gpui_kit::component::Size::Small)
                    .h(px(palette::CTL_SM))
                    .label("Archivieren")
                    .on_click(move |_, window, cx| {
                        archive.update(cx, |app, cx| app.bulk_archive_or_trash(false, window, cx))
                    }),
            )
            .child(
                kit::button("bulk-trash")
                    .danger()
                    .with_size(gpui_kit::component::Size::Small)
                    .h(px(palette::CTL_SM))
                    .label("Löschen")
                    .on_click(move |_, window, cx| {
                        trash.update(cx, |app, cx| app.bulk_archive_or_trash(true, window, cx))
                    }),
            )
            .child(
                icon_btn("bulk-close", icons::CLOSE, "Auswahl aufheben").on_click(
                    move |_, _, cx| {
                        close.update(cx, |app, cx| {
                            app.multi.clear();
                            cx.notify();
                        })
                    },
                ),
            )
    }
}

fn fuzzy_matches(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    let mut chars = text.chars();
    query
        .chars()
        .all(|wanted| chars.by_ref().any(|c| c == wanted))
}
