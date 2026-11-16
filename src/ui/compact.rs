//! A small presentation of the same updates, editor, tasks and autosave pipeline.
use super::*;
use gpui_kit::component::input::Textarea;

impl NoteWarden {
    /// Opens the "Notizzettel" window next to the main window, or brings it to the front.
    pub fn open_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup {
            return;
        }
        if let Some((handle, _)) = &self.popup_win {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            return;
        }
        self.snapshot_pending();
        self.workspace.settings.compact.enabled = true;
        let init = PopupInit {
            workspace: self.workspace.clone(),
            writable: self.writable,
            main: cx.entity().downgrade(),
            main_window: window.window_handle(),
        };
        let options = popup_options(&self.workspace.settings.compact, cx);
        match gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| NoteWarden::new(window, None, Some(init), cx))
        }) {
            Ok((handle, entity)) => {
                self.popup_win = Some((handle, entity));
                self.persist_local(cx);
            }
            Err(_) => {
                self.workspace.settings.compact.enabled = false;
                self.toast("Notizzettel konnte nicht geöffnet werden", None, cx);
            }
        }
        cx.notify();
    }

    /// Popup only: sticky chrome, pinning and the close hand-over to the main window.
    pub(crate) fn start_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(hwnd) = system::native_hwnd(window) {
            let (w, h) = self.workspace.settings.compact.size();
            let scale = window.scale_factor();
            cx.spawn(async move |_, _| system::enter_sticky(hwnd, w, h, scale))
                .detach();
        }
        if !system::pin_window(window, self.workspace.settings.compact.pinned) {
            self.workspace.settings.compact.pinned = false;
        }
        window.on_window_should_close(cx, {
            let entity = cx.entity();
            move |_, cx| {
                entity.update(cx, |popup, cx| popup.close_popup(cx));
                true
            }
        });
    }

    /// Popup only: hands the final data over and tells the main window the popup is gone.
    pub fn close_popup(&mut self, cx: &mut Context<Self>) {
        self.snapshot_pending();
        self.finish_sticky_edit(cx);
        self.push_to_peer(cx, true);
    }

    /// Opens or closes the popup from the keyboard.
    pub fn toggle_popup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.popup {
            self.close_popup(cx);
            window.remove_window();
        } else {
            self.open_popup(window, cx);
        }
    }

    /// Sends local changes to the other window once they are saved.
    pub fn sync_peer(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.peer_dirty) {
            self.push_to_peer(cx, false);
        }
    }

    fn peer_target(&self) -> Option<(WeakEntity<NoteWarden>, AnyWindowHandle)> {
        if self.popup {
            self.peer.clone()
        } else {
            self.popup_win
                .as_ref()
                .map(|(handle, entity)| (entity.downgrade(), *handle))
        }
    }

    fn push_to_peer(&mut self, cx: &mut Context<Self>, closing: bool) {
        let Some((peer, handle)) = self.peer_target() else {
            return;
        };
        self.peer_dirty = false;
        let workspace = self.workspace.clone();
        let _ = cx.update_window(handle, move |_, window, cx| {
            if let Some(peer) = peer.upgrade() {
                peer.update(cx, |peer, cx| {
                    peer.adopt_workspace(workspace, closing, window, cx)
                });
            }
        });
    }

    /// Takes over the other window's data; only the main window writes it to disk.
    pub fn adopt_workspace(
        &mut self,
        workspace: Workspace,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shown = |app: &Self| {
            app.selected()
                .map(|u| (u.title.clone(), u.notes.clone(), u.version.clone()))
        };
        let before = shown(self);
        self.workspace = workspace;
        if closing && !self.popup {
            self.popup_win = None;
            self.workspace.settings.compact.enabled = false;
        }
        if self.selected_update.is_some() && self.selected().is_none() {
            self.selected_update = None;
        }
        if shown(self) != before {
            self.sync_fields(window, cx);
        }
        if self
            .editing_sticky
            .is_some_and(|id| !self.workspace.stickies.iter().any(|s| s.id == id))
        {
            self.editing_sticky = None;
        }
        if !self.popup {
            self.persist_local(cx);
        }
        cx.notify();
    }

    /// Popup only: runs `f` in the main window (dialogs, settings, "open main window").
    pub fn forward_to_main(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut NoteWarden, &mut Window, &mut Context<NoteWarden>) + 'static,
    ) {
        let Some((main, handle)) = self.peer.clone() else {
            return;
        };
        if self.peer_dirty {
            self.push_to_peer(cx, false);
        }
        system::show_main_window();
        let _ = cx.update_window(handle, move |_, window, cx| {
            if let Some(main) = main.upgrade() {
                main.update(cx, |main, cx| f(main, window, cx));
            }
        });
    }

    pub fn remember_compact_bounds(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.workspace.settings.remember_window || window.is_maximized() {
            return;
        }
        let b = window.bounds();
        let current = (
            f32::from(b.origin.x).round(),
            f32::from(b.origin.y).round(),
            f32::from(b.size.width).round(),
            f32::from(b.size.height).round(),
        );
        if self.workspace.settings.compact.bounds != Some(current) {
            self.workspace.settings.compact.bounds = Some(current);
            self.persist(cx);
        }
    }

    pub fn create_compact_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        self.snapshot_pending();
        self.discard_untouched_drafts(None, cx);
        let id = self.workspace.new_update(None);
        self.drafts.insert(
            id,
            Self::draft_signature(self.workspace.update(id).unwrap()),
        );
        self.selected_update = Some(id);
        self.workspace.settings.last_update = Some(id);
        self.compact_list = false;
        self.compact_preview = false;
        self.editing_task = None;
        self.sync_fields(window, cx);
        self.persist(cx);
        self.notes_input
            .update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    pub fn compact_view(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let entity = cx.entity();
        let pin = cx.entity();
        let expand = cx.entity();
        let back = cx.entity();
        let pinned = self.workspace.settings.compact.pinned;
        let header = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(palette::SP_1))
            .h(px(palette::CTL_LG))
            .px(px(palette::SP_2))
            .border_b_1()
            .border_color(p.border_subtle)
            .when(!self.compact_list, |d| {
                d.child(
                    icon_btn("compact-back", icons::BACK, "Alle Notizzettel · Esc").on_click(
                        move |_, window, cx| {
                            back.update(cx, |app, cx| {
                                app.snapshot_pending();
                                app.discard_untouched_drafts(None, cx);
                                app.compact_list = true;
                                app.focus_handle.focus(window, cx);
                                cx.notify();
                            })
                        },
                    ),
                )
            })
            // Like Windows Sticky Notes: the free space of the bar drags the window.
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                icon_btn("compact-new", icons::ADD, "Neue Notiz · Strg+N")
                    .disabled(!self.writable)
                    .on_click(move |_, window, cx| {
                        entity.update(cx, |app, cx| app.create_compact_note(window, cx))
                    }),
            )
            .child(
                icon_btn(
                    "compact-pin",
                    IconName::Pin,
                    if pinned {
                        "Nicht mehr im Vordergrund halten"
                    } else {
                        "Immer im Vordergrund halten"
                    },
                )
                .toggled(pinned)
                .when(pinned, |b| b.text_color(p.accent))
                .on_click(move |_, window, cx| {
                    pin.update(cx, |app, cx| {
                        let next = !app.workspace.settings.compact.pinned;
                        if system::pin_window(window, next) {
                            app.workspace.settings.compact.pinned = next;
                            app.persist(cx);
                        } else {
                            app.toast("Fensterdarstellung konnte nicht geändert werden", None, cx);
                        }
                        cx.notify();
                    })
                }),
            )
            .child(
                icon_btn("compact-expand", IconName::Maximize2, "Hauptfenster öffnen").on_click(
                    move |_, _, cx| {
                        expand.update(cx, |app, cx| app.forward_to_main(cx, |_, _, _| {}))
                    },
                ),
            )
            .child(
                icon_btn("compact-close", icons::CLOSE, "Schließen")
                    .window_control_area(WindowControlArea::Close),
            );
        let mut content = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(palette::SP_4));
        let list_mode = self.compact_list || self.selected().is_none_or(|u| !u.is_live());
        if !list_mode && let Some(update) = self.selected() {
            let id = update.id;
            let mode = cx.entity();
            content = content
                .child(
                    Textarea::new(&self.title_input)
                        .aria_label("Titel der Notiz")
                        .appearance(false)
                        .bordered(false)
                        .disabled(!self.writable)
                        .w_full()
                        .text_size(palette::fs(palette::FS_XL))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .gap(px(palette::SP_2))
                        .child(kit::segment(
                            "compact-editor-mode",
                            &p,
                            &[
                                (Some(icons::EDIT), "Schreiben"),
                                (Some(icons::PREVIEW), "Vorschau"),
                            ],
                            usize::from(self.compact_preview),
                            true,
                            move |i, _, cx| {
                                mode.update(cx, |app, cx| {
                                    app.compact_preview = i == 1;
                                    cx.notify();
                                })
                            },
                        ))
                        .child(self.status_pill(update, cx)),
                )
                .child(self.tags_row(update, cx))
                .child(self.editor(update, cx));
            let (done, total) = update.task_progress();
            content = content.child(
                div()
                    .pt(px(palette::SP_2))
                    .border_t_1()
                    .border_color(p.border_subtle)
                    .text_size(palette::fs(palette::FS_BASE))
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("Aufgaben · {done}/{total}")),
            );
            for task in &update.tasks {
                let tid = task.id;
                let toggle = cx.entity();
                let edit = cx.entity();
                let remove = cx.entity();
                let row = div()
                    .flex()
                    .items_start()
                    .gap(px(palette::SP_2))
                    .min_w_0()
                    .when(task.sub, |d| d.pl(px(palette::SP_4)))
                    .child(
                        kit::task_checkbox(("compact-task-check", tid), &p, task.done)
                            .disabled(!self.writable)
                            .on_click(move |_, _, cx| {
                                toggle.update(cx, |app, cx| app.toggle_task(id, tid, cx))
                            }),
                    )
                    .child(if self.editing_task == Some(tid) {
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                kit::input(&self.task_edit_input)
                                    .aria_label("Aufgabe bearbeiten")
                                    .w_full(),
                            )
                            .into_any_element()
                    } else {
                        kit::surface(("compact-task-title", tid), &p)
                            .flex_1()
                            .min_w_0()
                            .min_h(px(palette::CTL_MD))
                            .justify_start()
                            .text_left()
                            .when(task.done, |b| b.line_through().text_color(p.text_tertiary))
                            .child(task.title.clone())
                            .disabled(!self.writable)
                            .on_click(move |_, window, cx| {
                                edit.update(cx, |app, cx| app.start_task_edit(tid, window, cx))
                            })
                            .into_any_element()
                    })
                    .child(
                        icon_btn(
                            ("compact-task-delete", tid),
                            icons::DELETE,
                            "Aufgabe löschen",
                        )
                        .disabled(!self.writable)
                        .on_click(move |_, _, cx| {
                            remove.update(cx, |app, cx| app.delete_task(id, tid, cx))
                        }),
                    );
                content = content.child(row);
            }
            let add = cx.entity();
            content = content.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::input(&self.task_input)
                            .aria_label("Aufgabe hinzufügen")
                            .disabled(!self.writable)
                            .w_full(),
                    )
                    .child(
                        icon_btn("compact-task-add", icons::ADD, "Aufgabe hinzufügen · Enter")
                            .disabled(!self.writable)
                            .on_click(move |_, window, cx| {
                                add.update(cx, |app, cx| app.add_task_from_input(window, cx))
                            }),
                    ),
            );
        }
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(p.bg_app)
            .child(header)
            .child(if list_mode {
                self.compact_list_body(&p, cx).into_any_element()
            } else {
                div()
                    .id("compact-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p(px(palette::SP_3))
                    .pr(px(palette::SP_3 + palette::SCROLL_GUTTER))
                    .child(content)
                    .into_any_element()
            })
            .child(
                div()
                    .flex_none()
                    .min_h(px(palette::CTL_MD))
                    .px(px(palette::SP_3))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(p.border_subtle)
                    .child(text_secondary(
                        &p,
                        if pinned {
                            "Angeheftet"
                        } else {
                            "Notizen & Aufgaben"
                        },
                    ))
                    .child(self.save_status(&p, cx)),
            )
    }
}

impl NoteWarden {
    /// Updates as slim cards on top, post-its below; each part scrolls on its own.
    fn compact_list_body(&self, p: &Palette, cx: &mut Context<Self>) -> Div {
        let updates = compact_updates(&self.workspace, &self.search_query);
        let stickies = self.visible_stickies(&self.search_query);
        let section = |label: &str, count: usize| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(palette::SP_1))
                .text_size(palette::fs(palette::FS_SM))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text_tertiary)
                .child(label.to_owned())
                .child(format!("{count}"))
        };
        let mut cards = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(palette::SP_1));
        if updates.is_empty() {
            cards = cards.child(text_secondary(
                p,
                if self.search_query.is_empty() {
                    "Noch keine Updates · mit + oben anlegen"
                } else {
                    "Keine passenden Updates"
                },
            ));
        }
        for update in &updates {
            let id = update.id;
            let open = cx.entity();
            let (done, total) = update.task_progress();
            cards = cards.child(
                kit::surface(("compact-note", id), p)
                    .w_full()
                    .min_w_0()
                    .min_h(px(palette::CTL_LG + palette::SP_1))
                    .px(px(palette::SP_3))
                    .rounded(px(palette::R_INPUT))
                    .border_1()
                    .border_color(p.border_subtle)
                    .bg(p.bg_panel)
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_3))
                    .child(dot(p.status(update.status)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(palette::fs(palette::FS_MD))
                            .font_weight(FontWeight::MEDIUM)
                            .child(note_title(update)),
                    )
                    .children(update.tags.iter().take(2).map(|tag| tags::tag_pill(p, tag)))
                    .when(total > 0, |d| {
                        d.child(text_secondary(p, format!("{done}/{total}")))
                    })
                    .on_click(move |_, window, cx| {
                        open.update(cx, |app, cx| {
                            app.select_update(id, window, cx);
                            app.compact_list = false;
                            app.compact_preview = false;
                            cx.notify();
                        })
                    }),
            );
        }
        let grid = self.sticky_grid(&stickies, 2, true, cx);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .p(px(palette::SP_3))
            .pt(px(palette::SP_1))
            .child(
                kit::input(&self.search_input)
                    .aria_label("Notizzettel suchen")
                    .w_full(),
            )
            .child(section("Updates", updates.len()))
            .child(
                div()
                    .id("compact-updates")
                    .flex_1()
                    .min_h(px(palette::CTL_LG))
                    .overflow_y_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(cards),
            )
            .child(
                section("Post-its", stickies.len()).children(self.sticky_tag.clone().map(|tag| {
                    let clear = cx.entity();
                    let color = palette::tag_color(&tag, p);
                    let cleared = tag.clone();
                    kit::surface("sticky-tag-clear", p)
                        .h(px(20.))
                        .px(px(palette::SP_2))
                        .rounded_full()
                        .bg(color.opacity(0.16))
                        .text_color(color)
                        .tooltip(tip("Filter aufheben"))
                        .child(format!("#{tag} ×"))
                        .on_click(move |_, _, cx| {
                            clear.update(cx, |app, cx| app.toggle_sticky_tag(&cleared, cx))
                        })
                })),
            )
            .child(
                div()
                    .id("compact-stickies")
                    .flex_none()
                    // About two rows of small notes; more scroll.
                    .max_h(px(2. * 76. + palette::SP_2))
                    .overflow_y_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(grid),
            )
    }
}

fn note_title(update: &model::Update) -> String {
    update
        .display_title()
        .unwrap_or_else(|| "Neue Notiz".into())
}

fn compact_updates<'a>(workspace: &'a Workspace, query: &str) -> Vec<&'a model::Update> {
    let query = query.trim().to_lowercase();
    let mut updates: Vec<_> = workspace
        .updates
        .iter()
        .filter(|u| {
            u.is_live()
                && (query.is_empty()
                    || u.title.to_lowercase().contains(&query)
                    || u.notes.to_lowercase().contains(&query)
                    || u.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(query.trim_start_matches('#')))
                    || u.tasks
                        .iter()
                        .any(|t| t.title.to_lowercase().contains(&query)))
        })
        .collect();
    updates.sort_by_key(|u| (std::cmp::Reverse(u.edited_at), std::cmp::Reverse(u.id)));
    updates
}

/// Window options for the popup: remembered size, remembered position if still on a monitor,
/// otherwise near the top right corner so it does not cover the main window's center.
fn popup_options(prefs: &crate::compact::CompactSettings, cx: &App) -> WindowOptions {
    let (w, h) = prefs.size();
    let displays: Vec<_> = cx
        .displays()
        .iter()
        .map(|display| {
            let b = display.bounds();
            (
                f32::from(b.origin.x),
                f32::from(b.origin.y),
                f32::from(b.size.width),
                f32::from(b.size.height),
            )
        })
        .collect();
    let mut bounds = Bounds::centered(None, size(px(w), px(h)), cx);
    if let Some((x, y)) = prefs.origin(&displays) {
        bounds.origin = point(px(x), px(y));
    } else if let Some(&(dx, dy, dw, _)) = displays.first() {
        bounds.origin = point(px(dx + dw - w - 48.), px(dy + 64.));
    }
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some("Notizzettel".into()),
            ..Default::default()
        }),
        window_min_size: Some(size(
            px(crate::compact::MIN_SIZE.0),
            px(crate::compact::MIN_SIZE.1),
        )),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn compact_notes_share_updates_and_find_notes_and_tasks_without_archived_entries() {
        let mut workspace = Workspace::default();
        let id = workspace.new_update(None);
        let note = workspace.update_mut(id).unwrap();
        note.notes = "- Veröffentlichung planen".into();
        assert_eq!(note_title(note), "Veröffentlichung planen");
        assert_eq!(compact_updates(&workspace, "VERÖFFENTLICHUNG")[0].id, id);
        workspace.update_mut(id).unwrap().archived = true;
        assert!(compact_updates(&workspace, "").is_empty());
    }
}
