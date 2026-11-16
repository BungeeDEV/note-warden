use super::*;
impl NoteWarden {
    pub fn inbox_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let header = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(
                div()
                    .text_size(palette::fs(palette::FS_XL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Inbox"),
            )
            .child(
                kit::input(&self.quick_input)
                    .aria_label("Aufgabe hinzufügen")
                    .w_full(),
            );
        let mut body = div().w_full().flex().flex_col().gap(px(palette::SP_2));
        if self.workspace.standalone_tasks.is_empty() {
            body = body.child(kit::empty_state(
                &p,
                "Inbox ist leer",
                "Hier landen schnelle Gedanken. Füge oben eine Aufgabe hinzu.",
                None,
            ));
        }
        let parsed = quick::parse_quick_input(
            &self.quick_input.read(cx).value(),
            self.now_today(),
            &self.workspace.projects,
        );
        if let Some(name) = parsed.new_project_name {
            let entity = cx.entity();
            body = body.child(
                kit::button("create-project")
                    .outline()
                    .label(format!("Projekt '{}' anlegen", name))
                    .on_click(move |_, _, cx| {
                        entity.update(cx, |app, cx| {
                            if !app.writable {
                                return;
                            }
                            let id = app.workspace.alloc_id();
                            app.workspace.projects.push(model::Project {
                                id,
                                name: name.clone(),
                                color: palette::default_project_color(id),
                                archived: false,
                                created_at: model::now_millis(),
                            });
                            app.persist(cx);
                            cx.notify();
                        });
                    }),
            );
        }
        for task in &self.workspace.standalone_tasks {
            let id = task.id;
            let entity = cx.entity();
            let convert = cx.entity();
            let target = DateTarget::Inbox(id);
            body = body.child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .p(px(palette::SP_2))
                    .rounded(px(palette::R_INPUT))
                    .hover(|s| s.bg(p.bg_hover))
                    .child(kit::row_leading(
                        kit::task_checkbox(("inbox-check", id), &p, task.done).on_click(
                            move |_, _, cx| {
                                entity.update(cx, |app, cx| app.toggle_inbox(id, cx));
                            },
                        ),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(160.))
                            .text_size(palette::fs(palette::FS_MD))
                            .when(task.done, |d| d.line_through().text_color(p.text_tertiary))
                            .child(task.title.clone()),
                    )
                    .when(task.priority > 0, |d| {
                        d.child(kit::priority_icon(
                            ("inbox-priority", id),
                            &p,
                            task.priority,
                        ))
                    })
                    .child(self.date_chip(
                        format!("inbox-date-{id}"),
                        target,
                        task.date,
                        task.done,
                        cx,
                    ))
                    .children(self.remind_bell(target, cx))
                    .child(
                        kit::toned_button(
                            ("inbox-convert", id),
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_MD,
                        )
                        .label("Als Update anlegen")
                        .on_click(move |_, _, cx| {
                            convert.update(cx, |app, cx| {
                                if !app.writable {
                                    return;
                                }
                                if let Some(index) = app
                                    .workspace
                                    .standalone_tasks
                                    .iter()
                                    .position(|t| t.id == id)
                                {
                                    let t = app.workspace.standalone_tasks.remove(index);
                                    let uid = app.workspace.new_update(t.date);
                                    if let Some(u) = app.workspace.update_mut(uid) {
                                        u.title = t.title;
                                        u.priority = t.priority;
                                        u.project_id = t.project_id;
                                        if t.done {
                                            u.status = Status::Erledigt;
                                        }
                                    }
                                    app.persist(cx);
                                    cx.notify();
                                }
                            });
                        }),
                    ),
            );
        }
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p(px(palette::SP_6))
            .gap(px(palette::SP_4))
            .child(header)
            .child(
                div()
                    .id("inbox-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(body),
            )
    }
    pub fn board_view(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let mut columns = div()
            .w_full()
            .flex()
            .gap(px(palette::SP_4))
            .items_start()
            .pb(px(palette::SCROLL_GUTTER));
        for status in Status::ALL {
            let updates: Vec<_> = self
                .workspace
                .updates
                .iter()
                .filter(|u| {
                    u.is_live()
                        && u.status == status
                        && self
                            .project_filter
                            .is_none_or(|id| u.project_id == Some(id))
                })
                .collect();
            let mut column = div()
                .id(("board-column", status.order() as u64))
                .flex_1()
                .flex_basis(px(0.))
                .min_w(px(220.))
                .flex()
                .flex_col()
                .gap(px(palette::SP_3))
                .child(
                    div()
                        .text_size(palette::fs(palette::FS_BASE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(p.text_secondary)
                        .child(format!("{} · {}", status.label(), updates.len())),
                )
                .on_drop(cx.listener(move |this, d: &UpdateDrag, _, cx| {
                    this.set_status(d.id, status, cx)
                }));
            if updates.is_empty() {
                column = column.child(
                    div()
                        .id(("board-empty", status.order() as u64))
                        .min_h(px(96.))
                        .px(px(palette::SP_3))
                        .rounded(px(palette::R_CARD))
                        .border_1()
                        .border_dashed()
                        .border_color(p.border_subtle)
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(palette::fs(palette::FS_SM))
                        .text_color(p.text_tertiary)
                        .drag_over::<UpdateDrag>(move |s, _, _, _| {
                            s.border_2().border_color(p.accent)
                        })
                        .child(format!("Keine Updates in {}", status.label())),
                );
            }
            for u in updates {
                let id = u.id;
                let entity = cx.entity();
                let title: SharedString = if u.title.trim().is_empty() {
                    "Ohne Titel".into()
                } else {
                    u.title.clone().into()
                };
                let mut moves = div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(palette::SP_1))
                    .child(
                        div()
                            .w_full()
                            .text_size(palette::fs(palette::FS_SM))
                            .text_color(p.text_tertiary)
                            .child("Verschieben nach"),
                    );
                for target in Status::ALL.into_iter().filter(|s| *s != status) {
                    let entity = cx.entity();
                    moves = moves.child(
                        kit::toned_button(
                            SharedString::from(format!("move-{id}-{}", target.order())),
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_SM,
                        )
                        .label(target.label())
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| app.set_status(id, target, cx));
                        }),
                    );
                }
                let meta = self.meta_parts(u, &p);
                let label = title.to_string();
                let card = div()
                    .id(("board-card", id))
                    .w_full()
                    .min_w_0()
                    .on_drag(UpdateDrag { id, label }, |d, _, _, cx| {
                        let label = d.label.clone();
                        cx.new(|_| DragPreview { label })
                    })
                    .p(px(palette::SP_3))
                    .rounded(px(palette::R_CARD))
                    .bg(p.bg_elevated)
                    .border_1()
                    .border_color(p.border_subtle)
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::surface(("board-open", id), &p)
                            .w_full()
                            .min_h(px(palette::CTL_MD))
                            .px(px(palette::SP_2))
                            .rounded(px(palette::R_INPUT))
                            .flex()
                            .items_center()
                            .justify_start()
                            .text_size(palette::fs(palette::FS_MD))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.text_primary)
                            .tooltip(tip(title.clone()))
                            .child(div().min_w_0().truncate().child(title))
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |app, cx| app.select_update(id, window, cx));
                            }),
                    )
                    .when(!meta.is_empty(), |d| {
                        d.child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .px(px(palette::SP_2))
                                .child(kit::meta_line(&p, meta)),
                        )
                    })
                    .child(moves);
                column = column.child(card);
            }
            columns = columns.child(column);
        }
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p(px(palette::SP_6))
            .gap(px(palette::SP_4))
            .child(
                div()
                    .flex_none()
                    .text_size(palette::fs(palette::FS_XL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Board"),
            )
            .child(
                div()
                    .id("board-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(columns),
            )
    }
}
