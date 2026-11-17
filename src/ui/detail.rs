use super::list::update_menu;
use super::*;
use gpui_kit::component::input::{Enter, IndentInline, InputState, OutdentInline, Textarea};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::text::TextView;
use model::{ChangeKind, EditorMode};
use std::ops::Range;

type SlashChoices = (Range<usize>, Vec<(&'static str, &'static str)>);

impl NoteWarden {
    fn editor_state(&self, cx: &App) -> (String, Range<usize>) {
        let ta = self.notes_input.read(cx);
        (ta.value().to_string(), ta.selected_range())
    }

    pub fn apply_edit(&mut self, edit: md::Edit, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        self.notes_input.update(cx, |ta, cx| {
            ta.set_selected_range(edit.range.clone(), cx);
            ta.replace(edit.text.clone(), window, cx);
            ta.set_selected_range(edit.sel.clone(), cx);
        });
        let value = self.notes_input.read(cx).value().to_string();
        self.set_notes(value, cx);
    }

    fn format_notes(&mut self, format: md::Format, window: &mut Window, cx: &mut Context<Self>) {
        let (text, sel) = self.editor_state(cx);
        let edit = md::apply_format(&text, sel, format);
        self.apply_edit(edit, window, cx);
        self.notes_input.update(cx, |ta, cx| ta.focus(window, cx));
    }

    fn effective_mode(&self, update: &model::Update) -> EditorMode {
        if self.popup {
            return if self.compact_preview {
                EditorMode::Preview
            } else {
                EditorMode::Write
            };
        }
        let mode = self.workspace.settings.editor_mode;
        if update.status == Status::Erledigt
            && mode == EditorMode::Write
            && self.edit_forced != Some(update.id)
        {
            EditorMode::Preview
        } else {
            mode
        }
    }

    fn mode_switch(
        &self,
        mode: EditorMode,
        update_id: u64,
        show_text: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let entity = cx.entity();
        kit::segment(
            "editor-mode",
            &p,
            &[
                (Some(icons::EDIT), "Schreiben"),
                (Some(icons::PREVIEW), "Vorschau"),
                (Some(icons::SPLIT), "Geteilt"),
            ],
            mode as usize,
            show_text,
            move |i, _, cx| {
                let m = [EditorMode::Write, EditorMode::Preview, EditorMode::Split][i];
                entity.update(cx, |app, cx| {
                    if m == EditorMode::Write {
                        app.edit_forced = Some(update_id);
                    }
                    app.set_editor_mode(m, cx)
                })
            },
        )
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let tools: [(md::Format, IconName, &'static str); 8] = [
            (md::Format::Bold, IconName::Bold, "Fett · Strg+B"),
            (md::Format::Italic, IconName::Italic, "Kursiv · Strg+I"),
            (md::Format::Code, IconName::Code, "Code"),
            (md::Format::Heading, IconName::Heading2, "Überschrift"),
            (md::Format::List, IconName::List, "Liste"),
            (md::Format::TaskList, IconName::ListChecks, "Aufgabenliste"),
            (md::Format::Quote, IconName::TextQuote, "Zitat"),
            (md::Format::Link, IconName::Link, "Link · Strg+Umschalt+K"),
        ];
        div()
            .flex()
            .items_center()
            .gap(px(2.))
            .flex_wrap()
            .children(
                tools
                    .into_iter()
                    .enumerate()
                    .map(|(index, (format, icon, tip))| {
                        let entity = cx.entity();
                        div()
                            .flex()
                            .items_center()
                            .when(index == 3 || index == 7, |d| {
                                d.child(div().h(px(16.)).w(px(1.)).mx(px(4.)).bg(p.border_subtle))
                            })
                            .child(
                                icon_btn(("fmt", tip.len() + icon_index(icon)), icon, tip)
                                    .h(px(palette::CTL_SM))
                                    .w(px(palette::CTL_SM))
                                    .text_color(p.text_secondary)
                                    .on_click(move |_, window, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.format_notes(format, window, cx)
                                        })
                                    }),
                            )
                    }),
            )
    }

    fn preview(&self, notes: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        if notes.trim().is_empty() {
            return div()
                .py(px(12.))
                .child(text_secondary(&p, "Keine Notizen."))
                .into_any_element();
        }
        div()
            .id("notes-preview")
            .w_full()
            .min_h(px(120.))
            .p(px(4.))
            .child(
                TextView::markdown("notes-markdown", notes.to_owned())
                    .selectable(true)
                    .on_link_click(|url, _, _, cx| {
                        let url = url.to_string();
                        if url.starts_with("http://")
                            || url.starts_with("https://")
                            || url.starts_with("mailto:")
                        {
                            cx.open_url(&url);
                        }
                    })
                    .on_task_toggle(|event, window, cx| {
                        let marker = event.marker_range();
                        window.dispatch_action(
                            Box::new(NoteTaskToggle {
                                start: marker.start,
                                end: marker.end,
                                checked: event.checked(),
                            }),
                            cx,
                        );
                    })
                    .code_block_actions(|block, _, _| {
                        let code = block.code().to_string();
                        let id = code.len() + code.bytes().map(usize::from).sum::<usize>();
                        icon_btn(("copy-code", id), icons::COPY, "Kopieren").on_click(
                            move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(code.clone()))
                            },
                        )
                    }),
            )
            .into_any_element()
    }

    #[allow(clippy::let_and_return)]
    pub(super) fn editor(
        &self,
        update: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let mode = self.effective_mode(update);
        let writable = self.writable;
        let words = md::word_count(&update.notes);
        let height = update.notes_height;
        let write = div()
            .id("notes-editor")
            .key_context("Editor")
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .on_action(cx.listener(|this, _: &EditorBold, window, cx| {
                this.format_notes(md::Format::Bold, window, cx)
            }))
            .on_action(cx.listener(|this, _: &EditorItalic, window, cx| {
                this.format_notes(md::Format::Italic, window, cx)
            }))
            .on_action(cx.listener(|this, _: &EditorLink, window, cx| {
                this.format_notes(md::Format::Link, window, cx)
            }))
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if action.secondary {
                    return;
                }
                let (text, sel) = this.editor_state(cx);
                if let Some(edit) = md::list_enter(&text, sel) {
                    this.apply_edit(edit, window, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &IndentInline, window, cx| {
                let (text, sel) = this.editor_state(cx);
                if let Some(edit) = md::indent_lines(&text, sel, false) {
                    this.apply_edit(edit, window, cx);
                }
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &OutdentInline, window, cx| {
                let (text, sel) = this.editor_state(cx);
                if let Some(edit) = md::indent_lines(&text, sel, true) {
                    this.apply_edit(edit, window, cx);
                }
                cx.stop_propagation();
            }))
            .capture_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                if this.slash_key(ev, window, cx) {
                    return;
                }
                let k = &ev.keystroke;
                if k.modifiers.control || k.modifiers.alt || k.modifiers.platform {
                    return;
                }
                let Some(ch) = k.key_char.as_deref().and_then(|s| {
                    let mut chars = s.chars();
                    let first = chars.next()?;
                    chars.next().is_none().then_some(first)
                }) else {
                    return;
                };
                let (text, sel) = this.editor_state(cx);
                if let Some(edit) = md::auto_pair(&text, sel, ch) {
                    this.apply_edit(edit, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_h(px(if height == 0 {
                        72.
                    } else {
                        f32::from(height.clamp(72, 480))
                    }))
                    .rounded(px(palette::R_CARD))
                    .border_1()
                    .border_color(if self.notes_focused {
                        p.border_strong
                    } else {
                        p.border_subtle
                    })
                    .bg(p.bg_panel)
                    .when(self.workspace.settings.toolbar && self.notes_focused, |d| {
                        d.child(
                            div()
                                .px(px(palette::SP_2))
                                .pt(px(palette::SP_2))
                                .child(self.toolbar(cx)),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .when(
                                self.workspace.settings.editor_font == model::EditorFont::Mono,
                                |d| d.font_family(palette::FONT_MONO),
                            )
                            .child({
                                let entity = cx.entity();
                                Textarea::new(&self.notes_input)
                                    .aria_label("Notizen (Markdown)")
                                    .appearance(false)
                                    .bordered(false)
                                    .disabled(!writable)
                                    .w_full()
                                    .on_paste(move |item, window, cx| {
                                        let Some(clip) = item.text() else {
                                            return false;
                                        };
                                        let handled = entity.update(cx, |app, cx| {
                                            let (text, sel) = app.editor_state(cx);
                                            match md::paste_link(&text, sel, &clip) {
                                                Some(edit) => {
                                                    app.apply_edit(edit, window, cx);
                                                    true
                                                }
                                                None => false,
                                            }
                                        });
                                        handled
                                    })
                            }),
                    )
                    .child(self.slash_menu(cx))
                    .when(self.workspace.settings.word_count, |d| {
                        d.child(
                            div()
                                .flex()
                                .justify_end()
                                .px(px(palette::SP_3))
                                .pb(px(palette::SP_2))
                                .text_size(palette::fs(palette::FS_SM))
                                .text_color(p.text_tertiary)
                                .child(format!(
                                    "{} · Markdown",
                                    model::plural(words, "Wort", "Wörter")
                                )),
                        )
                    }),
            );
        let preview = self.preview(&update.notes, cx).into_any_element();
        let body = match mode {
            EditorMode::Write => write.into_any_element(),
            EditorMode::Preview => div().flex_1().min_w_0().child(preview).into_any_element(),
            EditorMode::Split => div()
                .flex()
                .gap(px(16.))
                .items_start()
                .child(write)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .pl(px(12.))
                        .border_l_1()
                        .border_color(p.border_subtle)
                        .child(preview),
                )
                .into_any_element(),
        };
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(palette::SP_2))
            .when(self.popup, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(text_secondary(&p, "Notizen"))
                        .child(self.notes_height_control(update, cx)),
                )
            })
            .child(body)
    }

    fn notes_height_control(
        &self,
        update: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = update.id;
        let height = update.notes_height;
        let resize = cx.entity();
        kit::button("notes-height")
            .ghost()
            .with_size(gpui_kit::component::Size::Small)
            .label(match height {
                0 => "Automatisch",
                120 => "Kompakt",
                320 => "Groß",
                _ => "Normal",
            })
            .icon(kit::icon(icons::EXPAND, palette::ICON_SM))
            .tooltip("Mindesthöhe der Notizen für dieses Update")
            .accessibility_label("Höhe des Notizfelds einstellen")
            .disabled(!self.writable)
            .dropdown_menu(move |menu, _, _| {
                [
                    (0, "Automatisch · passend zum Inhalt"),
                    (120, "Kompakt · 120 px"),
                    (200, "Normal · 200 px"),
                    (320, "Groß · 320 px"),
                ]
                .into_iter()
                .fold(menu, |menu, (value, label)| {
                    let e = resize.clone();
                    menu.item(PopupMenuItem::new(label).checked(height == value).on_click(
                        move |_, _, cx| {
                            e.update(cx, |app, cx| {
                                if !app.writable {
                                    return;
                                }
                                if let Some(u) = app.workspace.update_mut(id) {
                                    u.notes_height = value;
                                }
                                app.persist(cx);
                                cx.notify();
                            })
                        },
                    ))
                })
            })
    }

    fn project_chip(
        &self,
        id: u64,
        current: Option<u64>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let projects: Vec<_> = self
            .workspace
            .projects
            .iter()
            .filter(|p| !p.archived)
            .map(|p| (p.id, p.name.clone()))
            .collect();
        let selected = current.and_then(|c| self.workspace.projects.iter().find(|p| p.id == c));
        let lead = match selected {
            Some(pj) => dot(palette::project_color(pj, &p)).into_any_element(),
            None => kit::icon(icons::PROJECT, palette::ICON_SM)
                .text_color(p.text_tertiary)
                .into_any_element(),
        };
        let e = cx.entity();
        kit::prop_chip(
            "detail-project",
            &p,
            lead,
            selected
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "Projekt".into()),
            selected.is_some(),
        )
        .tooltip("Projekt")
        .accessibility_label("Projekt")
        .dropdown_menu(move |menu, _, _| {
            let set = |pid: Option<u64>| {
                let e = e.clone();
                move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                    e.update(cx, |a, cx| {
                        if !a.writable {
                            return;
                        }
                        if let Some(u) = a.workspace.update_mut(id) {
                            u.project_id = pid;
                            u.touch();
                        }
                        a.persist(cx);
                        cx.notify();
                    })
                }
            };
            let create = e.clone();
            let manage = e.clone();
            let menu = menu.item(
                PopupMenuItem::new("Kein Projekt")
                    .checked(current.is_none())
                    .on_click(set(None)),
            );
            projects
                .iter()
                .fold(menu, |menu, (pid, name)| {
                    menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(current == Some(*pid))
                            .on_click(set(Some(*pid))),
                    )
                })
                .item(PopupMenuItem::separator())
                .item(
                    PopupMenuItem::new("Neues Projekt …").on_click(move |_, window, cx| {
                        create.update(cx, |a, cx| {
                            a.open_project_dialog(None, Some(id), window, cx)
                        })
                    }),
                )
                .item(
                    PopupMenuItem::new("Projekte verwalten …").on_click(move |_, window, cx| {
                        manage.update(cx, |a, cx| {
                            a.open_settings(Some(SettingsCat::Projects), window, cx)
                        })
                    }),
                )
        })
    }

    pub(super) fn priority_chip(
        &self,
        id: u64,
        current: u8,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let e = cx.entity();
        let labels = ["Keine", "Niedrig", "Mittel", "Hoch"];
        let current = current.min(3);
        kit::prop_chip(
            "detail-priority",
            &p,
            kit::priority_icon("detail-priority-icon", &p, current),
            if current == 0 {
                "Priorität"
            } else {
                labels[current as usize]
            },
            current > 0,
        )
        .tooltip("Priorität")
        .accessibility_label("Priorität")
        .dropdown_menu(move |menu, _, _| {
            labels
                .into_iter()
                .enumerate()
                .fold(menu, |menu, (i, label)| {
                    let e = e.clone();
                    menu.item(
                        PopupMenuItem::new(label)
                            .checked(current == i as u8)
                            .on_click(move |_, _, cx| {
                                e.update(cx, |a, cx| {
                                    if !a.writable {
                                        return;
                                    }
                                    if let Some(u) = a.workspace.update_mut(id) {
                                        u.priority = i as u8;
                                        u.touch();
                                    }
                                    a.persist(cx);
                                    cx.notify();
                                })
                            }),
                    )
                })
        })
    }

    pub(super) fn date_prop_chip(
        &self,
        u: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let today = self.now_today();
        let format = self.workspace.settings.date_format;
        let id = u.id;
        let entity = cx.entity();
        let overdue = u
            .planned_for
            .is_some_and(|d| d < today && u.status != Status::Erledigt);
        let lead = kit::icon(icons::DATE, palette::ICON_SM).text_color(if overdue {
            p.danger
        } else if u.planned_for.is_some() {
            p.text_secondary
        } else {
            p.text_tertiary
        });
        let text = match u.planned_for {
            Some(d) if self.workspace.settings.relative_dates => dates::relative(d, today, format),
            Some(d) => dates::absolute(d, today, format),
            None => "Datum".to_owned(),
        };
        kit::prop_chip("detail-date", &p, lead, text, u.planned_for.is_some())
            .when(overdue, |b| b.text_color(p.danger))
            .tooltip(
                u.planned_for
                    .map(|d| dates::full_with_weekday(d, format))
                    .unwrap_or_else(|| "Datum setzen".into()),
            )
            .accessibility_label("Datum")
            .on_click(move |_, window, cx| {
                let pos = window.mouse_position();
                entity.update(cx, |app, cx| {
                    app.open_date_pop(DateTarget::Update(id), pos, window, cx)
                });
            })
    }

    pub(super) fn version_chip(
        &self,
        u: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let filled = !u.version.trim().is_empty();
        let lead = kit::icon(icons::VERSION, palette::ICON_SM).text_color(if filled {
            p.text_secondary
        } else {
            p.text_tertiary
        });
        let trigger = kit::prop_chip(
            "detail-version",
            &p,
            lead,
            if filled {
                u.version.clone()
            } else {
                "Version".into()
            },
            filled,
        )
        .tooltip("Version")
        .accessibility_label("Version");
        let input = self.version_input.clone();
        let writable = self.writable;
        Popover::new("detail-version-pop")
            .anchor(Anchor::TopLeft)
            .trigger(trigger)
            .content(move |_, _, cx| {
                let p = palette::get(cx);
                div()
                    .w(px(220.))
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_2))
                    .child(
                        div()
                            .text_size(palette::fs(palette::FS_SM))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_tertiary)
                            .child("Version"),
                    )
                    .child(
                        kit::input(&input)
                            .aria_label("Version des Updates")
                            .disabled(!writable),
                    )
            })
    }

    pub(super) fn status_pill(
        &self,
        u: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let id = u.id;
        let status = u.status;
        let color = p.status(status);
        let e = cx.entity();
        kit::button("detail-status")
            .ghost()
            .rounded_full()
            .with_size(gpui_kit::component::Size::Small)
            .h(px(palette::CHIP_MD))
            .px(px(palette::CONTROL_PAD_SM))
            .text_size(palette::fs(palette::FS_CONTROL_SM))
            .bg(color.opacity(0.14))
            .text_color(p.text_primary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(palette::ICON_GAP))
                    .child(kit::status_icon(&p, status, u.task_progress()))
                    .child(status.label())
                    .child(kit::icon(icons::EXPAND, palette::ICON_SM).text_color(p.text_secondary)),
            )
            .tooltip("Status ändern")
            .accessibility_label(format!("Status: {}", status.label()))
            .dropdown_menu(move |menu, _, _| {
                Status::ALL.into_iter().fold(menu, |menu, s| {
                    let e = e.clone();
                    menu.item(
                        PopupMenuItem::new(s.label()).checked(s == status).on_click(
                            move |_, _, cx| e.update(cx, |a, cx| a.set_status(id, s, cx)),
                        ),
                    )
                })
            })
    }

    fn task_row(
        &self,
        update_id: u64,
        update_date: Option<NaiveDate>,
        task: &model::Task,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.popup {
            return self.compact_task_row(update_id, task, cx);
        }
        let p = palette::get(cx);
        let task_id = task.id;
        let editing = self.editing_task == Some(task_id);
        let expanded = self.expanded_task == Some(task_id);
        let group_name: SharedString = format!("task-{task_id}").into();
        let drag = TaskDrag {
            update_id,
            task_id,
            label: task.title.clone(),
        };
        let late = task
            .planned_for
            .zip(update_date)
            .is_some_and(|(t, u)| t > u);
        let entity = cx.entity();
        let menu_entity = cx.entity();
        let sub = task.sub;
        let done = task.done;
        let kind = task.kind;
        let can_indent = !sub;

        div()
            .id(("task", task_id))
            .group(group_name.clone())
            .key_context("TaskRow")
            .focusable()
            .when(sub, |d| d.ml(px(palette::SP_6)))
            .rounded(px(palette::R_INPUT))
            .border_t_2()
            .border_color(gpui_kit::transparent_black())
            .focus_visible(|s| s.border_color(p.accent))
            .drag_over::<TaskDrag>(move |s, _, _, _| s.border_color(p.accent))
            .on_drop(cx.listener(move |this, d: &TaskDrag, _, cx| {
                if d.update_id == update_id {
                    this.move_task(update_id, d.task_id, Some(task_id), cx);
                }
            }))
            .on_drag(drag, |d, _, _, cx| {
                let label = d.label.clone();
                cx.new(|_| DragPreview { label })
            })
            .on_action(cx.listener(move |this, _: &TaskIndent, _, cx| {
                this.indent_task(update_id, task_id, true, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskOutdent, _, cx| {
                this.indent_task(update_id, task_id, false, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskMoveUp, _, cx| {
                this.nudge_task(update_id, task_id, true, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskMoveDown, _, cx| {
                this.nudge_task(update_id, task_id, false, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskToggleKey, _, cx| {
                this.toggle_task(update_id, task_id, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskEditKey, window, cx| {
                this.start_task_edit(task_id, window, cx)
            }))
            .on_action(cx.listener(move |this, _: &TaskDeleteKey, _, cx| {
                this.delete_task(update_id, task_id, cx)
            }))
            .child(
                div()
                    .min_h(px(palette::CTL_LG))
                    .px(px(palette::SP_1))
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .relative()
                    .rounded(px(palette::R_INPUT))
                    .hover(|s| s.bg(p.bg_hover))
                    .child(
                        div()
                            .w(px(14.))
                            .flex_none()
                            .opacity(0.)
                            .group_hover(group_name.clone(), |s| s.opacity(1.))
                            .cursor_grab()
                            .child(ico(icons::DRAG, p.text_tertiary)),
                    )
                    .child({
                        let entity = entity.clone();
                        kit::row_leading(
                            kit::task_checkbox(("check", task_id), &p, done)
                                .disabled(!self.writable)
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |app, cx| {
                                        app.toggle_task(update_id, task_id, cx)
                                    })
                                }),
                        )
                    })
                    .child(if editing {
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                kit::input(&self.task_edit_input)
                                    .aria_label("Aufgabe umbenennen")
                                    .w_full(),
                            )
                            .into_any_element()
                    } else {
                        let entity = entity.clone();
                        kit::surface(("task-title", task_id), &p)
                            .flex_1()
                            .min_w_0()
                            .cursor_text()
                            .justify_start()
                            .text_left()
                            .text_color(if done {
                                p.text_tertiary
                            } else {
                                p.text_primary
                            })
                            .when(done, |d| d.line_through())
                            .child(
                                div()
                                    .w_full()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .child(task.title.clone()),
                            )
                            .children(task.tags.iter().map(|tag| {
                                div().ml(px(palette::SP_2)).child(tags::tag_pill(&p, tag))
                            }))
                            .on_click(move |_, window, cx| {
                                entity
                                    .update(cx, |app, cx| app.start_task_edit(task_id, window, cx))
                            })
                            .into_any_element()
                    })
                    .when(late, |d| {
                        d.child(
                            div()
                                .text_size(px(11.))
                                .text_color(p.planned)
                                .child("nach Update-Termin"),
                        )
                    })
                    .child(self.date_chip(
                        format!("task-date-{task_id}"),
                        DateTarget::Task(update_id, task_id),
                        task.planned_for,
                        done,
                        cx,
                    ))
                    .children(self.remind_bell(DateTarget::Task(update_id, task_id), cx))
                    .child(
                        kit::row_actions(("task-actions", task_id), group_name)
                            .child({
                                let entity = entity.clone();
                                icon_btn(("task-del", task_id), icons::DELETE, "Aufgabe löschen")
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.delete_task(update_id, task_id, cx)
                                        })
                                    })
                            })
                            .child(
                                icon_btn(("task-more", task_id), icons::MORE, "Weitere Aktionen")
                                    .dropdown_menu(move |menu, _, _| {
                                        let e = menu_entity.clone();
                                        let kinds = ChangeKind::ALL.iter().fold(menu, |menu, k| {
                                            let e = e.clone();
                                            let k = *k;
                                            menu.item(
                                                PopupMenuItem::new(format!("Typ: {}", k.label()))
                                                    .checked(k == kind)
                                                    .on_click(move |_, _, cx| {
                                                        e.update(cx, |app, cx| {
                                                            app.set_task_kind(
                                                                update_id, task_id, k, cx,
                                                            )
                                                        })
                                                    }),
                                            )
                                        });
                                        let e1 = e.clone();
                                        let e2 = e.clone();
                                        kinds
                                            .item(
                                                PopupMenuItem::new("Erinnerung setzen …").on_click(
                                                    {
                                                        let e = e.clone();
                                                        move |_, window, cx| {
                                                            let pos = window.mouse_position();
                                                            e.update(cx, |app, cx| {
                                                                app.open_date_pop(
                                                                    DateTarget::Task(
                                                                        update_id, task_id,
                                                                    ),
                                                                    pos,
                                                                    window,
                                                                    cx,
                                                                )
                                                            })
                                                        }
                                                    },
                                                ),
                                            )
                                            .item(PopupMenuItem::new("Beschreibung").on_click(
                                                move |_, window, cx| {
                                                    e1.update(cx, |app, cx| {
                                                        app.toggle_description(task_id, window, cx)
                                                    })
                                                },
                                            ))
                                            .item(
                                                PopupMenuItem::new(if sub {
                                                    "Ausrücken"
                                                } else {
                                                    "Einrücken"
                                                })
                                                .disabled(!sub && !can_indent)
                                                .on_click(move |_, _, cx| {
                                                    e2.update(cx, |app, cx| {
                                                        app.indent_task(
                                                            update_id, task_id, !sub, cx,
                                                        )
                                                    })
                                                }),
                                            )
                                    }),
                            ),
                    ),
            )
            .when(!task.description.trim().is_empty() && !expanded, |d| {
                d.child(
                    div()
                        .pl(px(44.))
                        .pb(px(4.))
                        .truncate()
                        .child(text_secondary(
                            &p,
                            task.description.lines().next().unwrap_or("").to_owned(),
                        )),
                )
            })
            .when(expanded, |d| {
                d.child(
                    div().pl(px(44.)).pb(px(palette::SP_2)).child(
                        kit::input(&self.desc_input)
                            .aria_label("Beschreibung der Aufgabe")
                            .w_full(),
                    ),
                )
            })
            .into_any_element()
    }

    fn compact_task_row(&self, uid: u64, task: &model::Task, cx: &mut Context<Self>) -> AnyElement {
        let p = palette::get(cx);
        let tid = task.id;
        let done = task.done;
        let drag = TaskDrag {
            update_id: uid,
            task_id: tid,
            label: task.title.clone(),
        };
        let toggle = cx.entity();
        let edit = cx.entity();
        let remove = cx.entity();
        let menu = cx.entity();
        div()
            .id(("compact-task", tid))
            .key_context("TaskRow")
            .focusable()
            .min_w_0()
            .rounded(px(palette::R_INPUT))
            .bg(p.bg_panel)
            .p(px(palette::SP_1))
            .border_t_2()
            .border_color(gpui_kit::transparent_black())
            .focus_visible(|s| s.border_color(p.accent))
            .when(task.sub, |d| d.ml(px(palette::SP_4)))
            .hover(|s| s.bg(p.bg_hover))
            .when(self.writable, |d| {
                d.drag_over::<TaskDrag>(move |s, _, _, _| s.border_color(p.accent))
                    .on_drop(cx.listener(move |app, d: &TaskDrag, _, cx| {
                        if d.update_id == uid {
                            app.move_task(uid, d.task_id, Some(tid), cx);
                        }
                    }))
            })
            .on_action(
                cx.listener(move |app, _: &TaskMoveUp, _, cx| app.nudge_task(uid, tid, true, cx)),
            )
            .on_action(
                cx.listener(move |app, _: &TaskMoveDown, _, cx| {
                    app.nudge_task(uid, tid, false, cx)
                }),
            )
            .on_action(
                cx.listener(move |app, _: &TaskToggleKey, _, cx| app.toggle_task(uid, tid, cx)),
            )
            .on_action(
                cx.listener(move |app, _: &TaskEditKey, w, cx| app.start_task_edit(tid, w, cx)),
            )
            .on_action(
                cx.listener(move |app, _: &TaskDeleteKey, _, cx| app.delete_task(uid, tid, cx)),
            )
            .on_action(
                cx.listener(move |app, _: &TaskIndent, _, cx| app.indent_task(uid, tid, true, cx)),
            )
            .on_action(
                cx.listener(move |app, _: &TaskOutdent, _, cx| {
                    app.indent_task(uid, tid, false, cx)
                }),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .min_w_0()
                    .gap(px(palette::SP_1))
                    .child(
                        div()
                            .id(("compact-task-grip", tid))
                            .flex_none()
                            .w(px(palette::ICON_MD))
                            .h(px(palette::CTL_MD))
                            .flex()
                            .items_center()
                            .cursor_grab()
                            .child(
                                kit::icon(icons::DRAG, palette::ICON_SM)
                                    .text_color(p.text_tertiary),
                            )
                            .when(self.writable, |d| {
                                d.on_drag(drag, |d, _, _, cx| {
                                    let label = d.label.clone();
                                    cx.new(|_| DragPreview { label })
                                })
                            }),
                    )
                    .child(
                        kit::task_checkbox(("compact-check", tid), &p, done)
                            .h(px(palette::CTL_MD))
                            .disabled(!self.writable)
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |app, cx| app.toggle_task(uid, tid, cx))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(if self.editing_task == Some(tid) {
                                kit::input(&self.task_edit_input)
                                    .aria_label("Aufgabe bearbeiten")
                                    .w_full()
                                    .into_any_element()
                            } else {
                                kit::surface(("compact-task-title", tid), &p)
                                    .w_full()
                                    .min_w_0()
                                    .min_h(px(palette::CTL_MD))
                                    .py(px(palette::SP_2))
                                    .justify_start()
                                    .text_left()
                                    .when(done, |b| b.line_through().text_color(p.text_tertiary))
                                    .child(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .whitespace_normal()
                                            .child(task.title.clone()),
                                    )
                                    .disabled(!self.writable)
                                    .on_click(move |_, w, cx| {
                                        edit.update(cx, |app, cx| app.start_task_edit(tid, w, cx))
                                    })
                                    .into_any_element()
                            }),
                    )
                    .when(task.priority > 0, |d| {
                        d.child(kit::priority_icon(
                            ("compact-task-priority", tid),
                            &p,
                            task.priority,
                        ))
                    })
                    .child(
                        icon_btn(
                            ("compact-task-more", tid),
                            icons::MORE,
                            "Aufgabe verschieben oder planen",
                        )
                        .disabled(!self.writable)
                        .dropdown_menu(move |m, _, _| {
                            let up = menu.clone();
                            let down = menu.clone();
                            let date = menu.clone();
                            m.item(
                                PopupMenuItem::new("Nach oben verschieben · Alt+↑").on_click(
                                    move |_, _, cx| {
                                        up.update(cx, |a, cx| a.nudge_task(uid, tid, true, cx))
                                    },
                                ),
                            )
                            .item(
                                PopupMenuItem::new("Nach unten verschieben · Alt+↓").on_click(
                                    move |_, _, cx| {
                                        down.update(cx, |a, cx| a.nudge_task(uid, tid, false, cx))
                                    },
                                ),
                            )
                            .item(
                                PopupMenuItem::new("Termin & Erinnerung …").on_click(
                                    move |_, w, cx| {
                                        date.update(cx, |a, cx| {
                                            a.open_date_pop(
                                                DateTarget::Task(uid, tid),
                                                w.mouse_position(),
                                                w,
                                                cx,
                                            )
                                        })
                                    },
                                ),
                            )
                        }),
                    )
                    .child(
                        icon_btn(
                            ("compact-task-delete", tid),
                            icons::DELETE,
                            "Aufgabe löschen",
                        )
                        .disabled(!self.writable)
                        .on_click(move |_, _, cx| {
                            remove.update(cx, |app, cx| app.delete_task(uid, tid, cx))
                        }),
                    ),
            )
            .when(
                task.planned_for.is_some()
                    || !task.tags.is_empty()
                    || !task.description.trim().is_empty(),
                |d| {
                    d.child(
                        div()
                            .pl(px(palette::ICON_MD
                                + palette::ROW_LEADING_SLOT
                                + palette::SP_2))
                            .pb(px(palette::SP_2))
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap(px(palette::SP_1))
                            .when(task.planned_for.is_some(), |d| {
                                d.child(self.date_chip(
                                    format!("compact-task-date-{tid}"),
                                    DateTarget::Task(uid, tid),
                                    task.planned_for,
                                    done,
                                    cx,
                                ))
                            })
                            .children(task.tags.iter().map(|t| tags::tag_pill(&p, t)))
                            .when(!task.description.trim().is_empty(), |d| {
                                d.child(
                                    div()
                                        .w_full()
                                        .child(text_secondary(&p, task.description.clone())),
                                )
                            }),
                    )
                },
            )
            .into_any_element()
    }

    pub(super) fn task_section(
        &self,
        update: &model::Update,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let id = update.id;
        let (done, total) = update.task_progress();
        let paste_entity = cx.entity();
        let task_menu = cx.entity();
        let hide_done = self.hide_done;
        let mut tasks: Vec<&model::Task> = update
            .tasks
            .iter()
            .filter(|t| !(hide_done && t.done))
            .collect();
        if self.task_grouped {
            tasks.sort_by_key(|t| t.kind as usize);
        }
        let ratio = if total == 0 {
            0.
        } else {
            done as f32 / total as f32
        };
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(8.))
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_MD))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(p.text_primary)
                                    .child(if self.popup {
                                        "Aufgaben"
                                    } else {
                                        "Aufgaben & Features"
                                    }),
                            )
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_SM))
                                    .text_color(p.text_tertiary)
                                    .child(format!("{done}/{total}")),
                            ),
                    )
                    .child(
                        icon_btn("tasks-menu", icons::MORE, "Aufgaben-Aktionen").dropdown_menu(
                            move |m, _, _| {
                                let hide = task_menu.clone();
                                let grouped = task_menu.clone();
                                let complete = task_menu.clone();
                                let m =
                                    if done > 0 {
                                        m.item(
                                            PopupMenuItem::new(if hide_done {
                                                "Erledigte anzeigen"
                                            } else {
                                                "Erledigte ausblenden"
                                            })
                                            .on_click(move |_, _, cx| {
                                                hide.update(cx, |a, cx| {
                                                    a.hide_done = !a.hide_done;
                                                    cx.notify();
                                                })
                                            }),
                                        )
                                    } else {
                                        m
                                    };
                                m.item(PopupMenuItem::new("Nach Typ gruppieren").on_click(
                                    move |_, _, cx| {
                                        grouped.update(cx, |a, cx| {
                                            a.task_grouped = !a.task_grouped;
                                            cx.notify();
                                        })
                                    },
                                ))
                                .item(
                                    PopupMenuItem::new("Alle erledigen").on_click(
                                        move |_, _, cx| {
                                            complete.update(cx, |a, cx| {
                                                if !a.writable {
                                                    return;
                                                }
                                                if let Some(u) = a.workspace.update_mut(id) {
                                                    for t in &mut u.tasks {
                                                        t.done = true;
                                                    }
                                                    u.touch();
                                                }
                                                a.persist(cx);
                                                cx.notify();
                                            })
                                        },
                                    ),
                                )
                            },
                        ),
                    ),
            )
            .when(total > 0, |d| {
                d.child(
                    div()
                        .w_full()
                        .h(px(4.))
                        .rounded_full()
                        .bg(p.border_strong)
                        .overflow_hidden()
                        .child(
                            div()
                                .h_full()
                                .w(relative(ratio))
                                .rounded_full()
                                .bg(p.accent),
                        ),
                )
            })
            .when(update.tasks.is_empty(), |d| {
                d.child(text_secondary(
                    &p,
                    "Noch keine Aufgaben. Teile dieses Update in konkrete Schritte auf.",
                ))
            })
            .children(tasks.into_iter().map(|task| {
                self.task_row(id, update.planned_for, task, cx)
                    .into_any_element()
            }))
            .child(
                div()
                    .id("task-end-drop")
                    .h(px(4.))
                    .border_t_2()
                    .border_color(gpui_kit::transparent_black())
                    .drag_over::<TaskDrag>(move |s, _, _, _| s.border_color(p.accent))
                    .on_drop(cx.listener(move |this, d: &TaskDrag, _, cx| {
                        if d.update_id == id {
                            this.move_task(id, d.task_id, None, cx);
                        }
                    })),
            )
            .child(
                kit::input(&self.task_input)
                    .aria_label("Neue Aufgabe")
                    .prefix(ico(icons::ADD, p.text_tertiary))
                    .bordered(false)
                    .focus_bordered(true)
                    .disabled(!self.writable)
                    .w_full()
                    .on_paste(move |item, _, cx| {
                        let Some(text) = item.text() else {
                            return false;
                        };
                        if text.trim().lines().count() < 2 {
                            return false;
                        }
                        paste_entity.update(cx, |app, cx| app.add_tasks(&text, cx));
                        true
                    }),
            )
    }

    fn changelog_panel(&self, u: &model::Update, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let text = md::format_changelog(None, &u.tasks, self.changelog_done_only);
        let copy = text.clone();
        let (e, e2) = (cx.entity(), cx.entity());
        let id = u.id;
        div()
            .rounded(px(palette::R_CARD))
            .border_1()
            .border_color(p.border_subtle)
            .bg(p.bg_panel)
            .p(px(palette::SP_4))
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_end()
                    .gap(px(palette::SP_3))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .child(text_secondary(&p, "Nur erledigte"))
                            .child(kit::switch(
                                "changelog-done",
                                &p,
                                self.changelog_done_only,
                                "Nur erledigte Aufgaben",
                                kit::click_fn(move |_, cx| {
                                    e.update(cx, |a, cx| {
                                        a.changelog_done_only = !a.changelog_done_only;
                                        cx.notify();
                                    })
                                }),
                            )),
                    )
                    .child(
                        kit::button("changelog-save")
                            .ghost()
                            .label("Als Datei speichern")
                            .text_color(p.text_secondary)
                            .on_click({
                                let e = cx.entity();
                                move |_, _, cx| e.update(cx, |a, cx| a.export_changelog(id, cx))
                            }),
                    )
                    .child(
                        kit::toned_button(
                            "changelog-copy",
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_MD,
                        )
                        .icon(kit::icon(icons::COPY, palette::ICON_MD))
                        .label("Kopieren")
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));
                            e2.update(cx, |a, cx| a.toast("Changelog kopiert", None, cx));
                        }),
                    ),
            )
            .child(
                div()
                    .font_family(palette::FONT_MONO)
                    .text_size(palette::fs(palette::FS_BASE))
                    .text_color(p.text_primary)
                    .child(if text.is_empty() {
                        "Keine Einträge.".to_owned()
                    } else {
                        text
                    }),
            )
    }

    fn activity_panel(&self, u: &model::Update, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let icon_for = |kind: &str| match kind {
            "created" => icons::ADD,
            "date" => icons::DATE,
            "priority" => icons::PRIORITY,
            "project" => icons::PROJECT,
            "version" => icons::VERSION,
            "title" => icons::EDIT,
            _ => icons::CONFIRM,
        };
        div()
            .flex()
            .flex_col()
            .gap(px(palette::SP_1))
            .when(u.activity.is_empty(), |d| {
                d.child(text_secondary(&p, "Noch keine Aktivität."))
            })
            .children(u.activity.iter().rev().map(|event| {
                div()
                    .min_h(px(palette::CTL_MD))
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_3))
                    .child(
                        kit::icon(icon_for(&event.kind), palette::ICON_SM)
                            .text_color(p.text_tertiary),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(palette::fs(palette::FS_BASE))
                            .text_color(p.text_secondary)
                            .child(if event.count > 1 {
                                format!("{} · {}×", event.data, event.count)
                            } else {
                                event.data.clone()
                            }),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(palette::fs(palette::FS_SM))
                            .text_color(p.text_tertiary)
                            .child(dates::elapsed(event.at)),
                    )
            }))
    }

    pub(super) fn update_detail(&self, narrow: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let Some(u) = self.selected() else {
            return kit::empty_state(
                &p,
                "Wähle ein Update aus",
                "↑/↓ auswählen · Enter öffnen · Strg+N anlegen",
                None,
            )
            .flex_1()
            .into_any_element();
        };
        let id = u.id;
        let (back, prev, next, tabs, complete, dismiss, publish) = (
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
        );
        let (done, total) = u.task_progress();
        let mode = self.effective_mode(u);
        let scrolled = self.detail_scroll.offset().y < px(-4.);
        let detail_w = if narrow {
            self.content_width
        } else {
            self.content_width - self.workspace.settings.list_width
        };
        let animate = self.workspace.settings.motion != model::Motion::Reduced;
        let times: Vec<String> = [
            dates::elapsed_opt(u.created_at).map(|t| format!("Erstellt {t}")),
            dates::elapsed_opt(u.edited_at).map(|t| format!("Bearbeitet {t}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        let times_tip = [
            dates::full_time(u.created_at).map(|t| format!("Erstellt: {t}")),
            dates::full_time(u.edited_at).map(|t| format!("Bearbeitet: {t}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(p.bg_panel)
            .child(
                div()
                    .h(px(44.))
                    .px(px(palette::SP_4))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(scrolled, |d| d.shadow_sm())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .when(narrow, |d| {
                                d.child(
                                    kit::button("detail-back")
                                        .ghost()
                                        .icon(kit::icon(icons::BACK, palette::ICON_MD))
                                        .label("Zurück")
                                        .text_color(p.text_secondary)
                                        .on_click(move |_, _, cx| {
                                            back.update(cx, |a, cx| {
                                                a.discard_untouched_drafts(None, cx);
                                                a.detail_open = false;
                                                cx.notify();
                                            })
                                        }),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_1))
                            .child(self.save_status(&p, cx))
                            .child(
                                icon_btn(
                                    "detail-prev",
                                    IconName::ChevronLeft,
                                    "Vorheriges Update · Alt+↑",
                                )
                                .on_click(move |_, w, cx| {
                                    prev.update(cx, |a, cx| a.select_relative(-1, w, cx))
                                }),
                            )
                            .child(
                                icon_btn(
                                    "detail-next",
                                    IconName::ChevronRight,
                                    "Nächstes Update · Alt+↓",
                                )
                                .on_click(move |_, w, cx| {
                                    next.update(cx, |a, cx| a.select_relative(1, w, cx))
                                }),
                            )
                            .child(
                                icon_btn("detail-menu", icons::MORE, "Update-Aktionen")
                                    .dropdown_menu({
                                        let build = update_menu(
                                            cx.entity(),
                                            id,
                                            u.archived,
                                            u.deleted_on.is_some(),
                                            true,
                                            u.release.is_some(),
                                        );
                                        move |m, w, cx| build(m, w, cx)
                                    }),
                            ),
                    ),
            )
            .child(
                div()
                    .id("detail-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.detail_scroll)
                    .vertical_scrollbar(&self.detail_scroll)
                    .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
                    .px(px(palette::SP_8))
                    .flex()
                    .justify_center()
                    .items_start()
                    .child(
                        div()
                            .w_full()
                            .max_w(px(palette::DETAIL_MAX_W))
                            .pt(px(palette::SP_8))
                            .pb(px(96.))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .capture_action(cx.listener(|a, _: &Enter, w, cx| {
                                        a.notes_input.update(cx, |i, cx| i.focus(w, cx));
                                        cx.stop_propagation();
                                    }))
                                    .child(
                                        Textarea::new(&self.title_input)
                                            .appearance(false)
                                            .bordered(false)
                                            .aria_label("Titel")
                                            .text_size(palette::fs(palette::DETAIL_TITLE))
                                            .line_height(relative(1.2))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .px(px(0.))
                                            .w_full()
                                            .disabled(!self.writable),
                                    ),
                            )
                            .child(
                                div()
                                    .mt(px(palette::SP_3))
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .gap(px(palette::SP_2))
                                    .child(self.status_pill(u, cx))
                                    .child(self.date_prop_chip(u, cx))
                                    .children(self.remind_bell(DateTarget::Update(u.id), cx))
                                    .child(self.priority_chip(id, u.priority, cx))
                                    .child(self.project_chip(id, u.project_id, cx))
                                    .child(self.version_chip(u, cx)),
                            )
                            .child(self.tags_row(u, cx))
                            .when(!times.is_empty(), |d| {
                                d.child(
                                    div()
                                        .id("detail-times")
                                        .mt(px(palette::SP_2))
                                        .text_size(palette::fs(palette::FS_SM))
                                        .text_color(p.text_tertiary)
                                        .tooltip(tip(times_tip))
                                        .child(times.join(" · ")),
                                )
                            })
                            .child(div().mt(px(28.)).child(kit::tabs(
                                "detail-tabs",
                                &p,
                                &["Übersicht", "Changelog", "Aktivität"],
                                self.detail_tab,
                                animate,
                                move |tab, _, cx| {
                                    tabs.update(cx, |a, cx| {
                                        a.detail_tab = tab;
                                        cx.notify();
                                    })
                                },
                            )))
                            .child(
                                div()
                                    .mt(px(palette::SP_6))
                                    .flex()
                                    .flex_col()
                                    .gap(px(palette::SP_6))
                                    .when(self.detail_tab == 0, |d| {
                                        d.child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .gap(px(palette::SP_2))
                                                .child(kit::card_header(
                                                    &p,
                                                    "Notizen",
                                                    div()
                                                        .flex()
                                                        .flex_wrap()
                                                        .items_center()
                                                        .gap(px(palette::SP_2))
                                                        .child(self.mode_switch(
                                                            mode,
                                                            id,
                                                            narrow || detail_w >= 720.,
                                                            cx,
                                                        ))
                                                        .child(self.notes_height_control(u, cx)),
                                                ))
                                                .child(self.editor(u, cx)),
                                        )
                                        .child(self.task_section(u, cx))
                                        .when(
                                            total > 0
                                                && done == total
                                                && u.status != Status::Erledigt
                                                && self.completion_dismissed != Some(id),
                                            |d| {
                                                d.child(
                                                    div()
                                                        .min_h(px(40.))
                                                        .px(px(palette::SP_3))
                                                        .rounded(px(palette::R_INPUT))
                                                        .bg(p.done.opacity(0.14))
                                                        .flex()
                                                        .flex_wrap()
                                                        .items_center()
                                                        .gap(px(palette::SP_2))
                                                        .child(
                                                            kit::icon(
                                                                icons::CONFIRM,
                                                                palette::ICON_MD,
                                                            )
                                                            .text_color(p.done),
                                                        )
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .text_size(palette::fs(
                                                                    palette::FS_BASE,
                                                                ))
                                                                .text_color(p.text_secondary)
                                                                .child("Alle Aufgaben erledigt."),
                                                        )
                                                        .child(
                                                            kit::toned_button(
                                                                "finish-publish",
                                                                &p,
                                                                kit::ButtonTone::Success,
                                                                palette::CTL_SM,
                                                            )
                                                            .label("Veröffentlichen …")
                                                            .on_click(move |_, window, cx| {
                                                                publish.update(cx, |a, cx| {
                                                                    a.open_publish(id, window, cx)
                                                                })
                                                            }),
                                                        )
                                                        .child(
                                                            kit::toned_button(
                                                                "finish-update",
                                                                &p,
                                                                kit::ButtonTone::Secondary,
                                                                palette::CTL_SM,
                                                            )
                                                            .label("Auf Erledigt setzen")
                                                            .on_click(move |_, _, cx| {
                                                                complete.update(cx, |a, cx| {
                                                                    a.set_status(
                                                                        id,
                                                                        Status::Erledigt,
                                                                        cx,
                                                                    )
                                                                })
                                                            }),
                                                        )
                                                        .child(
                                                            kit::toned_button(
                                                                "finish-later",
                                                                &p,
                                                                kit::ButtonTone::Ghost,
                                                                palette::CTL_SM,
                                                            )
                                                            .label("Später")
                                                            .on_click(move |_, _, cx| {
                                                                dismiss.update(cx, |a, cx| {
                                                                    a.completion_dismissed =
                                                                        Some(id);
                                                                    cx.notify();
                                                                })
                                                            }),
                                                        ),
                                                )
                                            },
                                        )
                                    })
                                    .when(self.detail_tab == 1, |d| {
                                        d.child(self.changelog_panel(u, cx))
                                    })
                                    .when(self.detail_tab == 2, |d| {
                                        d.child(self.activity_panel(u, cx))
                                    }),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// Stable numeric key for toolbar button IDs.
fn icon_index(icon: IconName) -> usize {
    icon as usize
}

#[allow(dead_code)]
fn _assert_input_state(_: &InputState) {}

impl NoteWarden {
    fn slash_choices(&self, cx: &App) -> Option<SlashChoices> {
        if !self.notes_focused {
            return None;
        }
        let i = self.notes_input.read(cx);
        let text = i.value().to_string();
        let sel = i.selected_range();
        if !sel.is_empty() || sel.start > text.len() {
            return None;
        }
        let start = text[..sel.start].rfind('\n').map(|n| n + 1).unwrap_or(0);
        let query = text[start..sel.start].strip_prefix('/')?;
        let options = [
            ("Überschrift 1", "# "),
            ("Überschrift 2", "## "),
            ("Überschrift 3", "### "),
            ("Aufzählung", "- "),
            ("Nummerierung", "1. "),
            ("Aufgabenliste", "- [ ] "),
            ("Codeblock", "```\n\n```"),
            ("Zitat", "> "),
            ("Trennlinie", "---\n"),
            ("Tabelle", "| Spalte | Spalte |\n| --- | --- |\n|  |  |"),
        ]
        .into_iter()
        .filter(|(label, _)| label.to_lowercase().contains(&query.to_lowercase()))
        .collect::<Vec<_>>();
        (!options.is_empty()).then_some((start..sel.start, options))
    }
    fn insert_slash(&mut self, index: usize, w: &mut Window, cx: &mut Context<Self>) {
        if let Some((range, options)) = self.slash_choices(cx) {
            let text = options[index.min(options.len() - 1)].1.to_string();
            let cursor = range.start + text.len();
            self.apply_edit(
                md::Edit {
                    range,
                    text,
                    sel: cursor..cursor,
                },
                w,
                cx,
            );
            self.slash_index = 0;
        }
    }
    fn slash_key(&mut self, ev: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((_, options)) = self.slash_choices(cx) else {
            return false;
        };
        match ev.keystroke.key.as_str() {
            "down" => self.slash_index = (self.slash_index + 1) % options.len(),
            "up" => self.slash_index = (self.slash_index + options.len() - 1) % options.len(),
            "enter" | "tab" => self.insert_slash(self.slash_index, w, cx),
            _ => return false,
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    fn slash_menu(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let options = self.slash_choices(cx);
        div().when_some(options, |d, (_, options)| {
            d.p(px(palette::SP_2))
                .rounded(px(palette::MENU_RADIUS))
                .bg(p.bg_elevated)
                .border_1()
                .border_color(p.border_subtle)
                .children(options.into_iter().enumerate().map(|(n, (label, _))| {
                    let e = cx.entity();
                    kit::button(("slash", n))
                        .ghost()
                        .w_full()
                        .justify_start()
                        .label(label)
                        .when(n == self.slash_index, |b| b.bg(p.bg_active))
                        .on_click(move |_, w, cx| e.update(cx, |a, cx| a.insert_slash(n, w, cx)))
                }))
        })
    }
}
