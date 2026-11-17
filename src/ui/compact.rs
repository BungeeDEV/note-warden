//! A small presentation of the same updates, editor, tasks and autosave pipeline.
use super::list::update_menu;
use super::*;
use crate::compact::StickyStartView;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenuItem};

impl NoteWarden {
    fn compact_view_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let prefs = &self.workspace.settings.compact;
        let hide = prefs.hide_completed;
        let compact = prefs.compact_cards;
        let start_view = prefs.start_view;
        let entity = cx.entity();
        icon_btn(
            "compact-view-options",
            icons::FILTER,
            "Sticky-Ansicht einstellen",
        )
        .toggled(hide || compact)
        .disabled(!self.writable)
        .dropdown_menu(move |menu, window, cx| {
            let toggle = entity.clone();
            let density = entity.clone();
            let start = entity.clone();
            menu.item(
                PopupMenuItem::new("Erledigte Updates ausblenden")
                    .checked(hide)
                    .on_click(move |_, _, cx| {
                        toggle.update(cx, |app, cx| {
                            app.update_settings(cx, |s| {
                                s.compact.hide_completed = !s.compact.hide_completed
                            })
                        })
                    }),
            )
            .separator()
            .submenu("Kartenansicht", window, cx, move |menu, _, _| {
                [(false, "Mit Details"), (true, "Nur Titel")]
                    .into_iter()
                    .fold(menu, |menu, (value, label)| {
                        let e = density.clone();
                        menu.item(
                            PopupMenuItem::new(label)
                                .checked(value == compact)
                                .on_click(move |_, _, cx| {
                                    e.update(cx, |app, cx| {
                                        app.update_settings(cx, |s| s.compact.compact_cards = value)
                                    })
                                }),
                        )
                    })
            })
            .submenu("Beim Öffnen", window, cx, move |menu, _, _| {
                [
                    (StickyStartView::Overview, "Übersicht"),
                    (StickyStartView::LastUpdate, "Zuletzt geöffnetes Update"),
                ]
                .into_iter()
                .fold(menu, |menu, (value, label)| {
                    let e = start.clone();
                    menu.item(
                        PopupMenuItem::new(label)
                            .checked(value == start_view)
                            .on_click(move |_, _, cx| {
                                e.update(cx, |app, cx| {
                                    app.update_settings(cx, |s| s.compact.start_view = value)
                                })
                            }),
                    )
                })
            })
        })
    }

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
        if self.writable {
            self.workspace.ensure_manual_order();
        }
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
        self.sort = self.workspace.settings.updates_ui.sort;
        self.group = self.workspace.settings.updates_ui.group;
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
        let _ = cx.update_window(handle, move |_, window, cx| {
            system::show_window(window);
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
        self.workspace.settings.compact.last_update = Some(id);
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
            .child(self.compact_view_options(cx))
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
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap(px(palette::SP_2))
                        .child(self.date_prop_chip(update, cx))
                        .child(self.priority_chip(id, update.priority, cx))
                        .child(self.version_chip(update, cx)),
                )
                .child(self.tags_row(update, cx))
                .child(self.editor(update, cx));
            content = content.child(self.task_section(update, cx));
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
    /// Updates and post-its share one scroll area, with no reserved empty sections.
    fn compact_list_body(&self, p: &Palette, cx: &mut Context<Self>) -> Div {
        let accent = p.accent;
        let updates = compact_updates(&self.workspace, &self.search_query);
        let compact = self.workspace.settings.compact.compact_cards;
        let query = self.search_query.trim().to_lowercase();
        let hidden_count = self
            .workspace
            .updates
            .iter()
            .filter(|u| {
                self.workspace.settings.compact.hide_completed
                    && u.is_live()
                    && u.status == Status::Erledigt
                    && matches_compact_query(u, &query)
            })
            .count();
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
                if hidden_count > 0 && self.search_query.is_empty() {
                    "Alles erledigt · abgeschlossene Updates sind ausgeblendet"
                } else if self.search_query.is_empty() {
                    "Noch keine Updates · mit + oben anlegen"
                } else {
                    "Keine passenden Updates"
                },
            ));
        }
        for update in &updates {
            let id = update.id;
            let open = cx.entity();
            let meta = self.meta_parts(update, p);
            let label = note_title(update);
            cards = cards.child(
                kit::surface(("compact-note", id), p)
                    .w_full()
                    .min_w_0()
                    .min_h(px(if compact {
                        palette::ROW_COMPACT
                    } else {
                        palette::CTL_LG + palette::SP_1
                    }))
                    .px(px(palette::SP_3))
                    .rounded(px(palette::R_INPUT))
                    .border_1()
                    .border_color(p.border_subtle)
                    .bg(p.bg_panel)
                    .py(px(if compact {
                        palette::SP_1
                    } else {
                        palette::SP_2
                    }))
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .when(self.writable, |row| {
                        row.on_drag(
                            UpdateDrag {
                                id,
                                label: label.clone(),
                            },
                            |d, _, _, cx| {
                                let label = d.label.clone();
                                cx.new(|_| DragPreview { label })
                            },
                        )
                        .drag_over::<UpdateDrag>(move |s, _, _, _| {
                            s.border_t_2().border_color(accent)
                        })
                        .on_drop(cx.listener(
                            move |app, d: &UpdateDrag, _, cx| {
                                app.reorder_update(d.id, Some(id), cx);
                            },
                        ))
                    })
                    .child(div().flex_none().cursor_grab().child(
                        kit::icon(icons::DRAG, palette::ICON_SM).text_color(p.text_tertiary),
                    ))
                    .child(dot(p.status(update.status)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(palette::SP_1))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(palette::fs(palette::FS_MD))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(label),
                            )
                            .when(!compact, |d| {
                                d.child(
                                    kit::meta_line(p, {
                                        let mut parts = vec![
                                            div().child(update.status.label()).into_any_element(),
                                        ];
                                        parts.extend(meta);
                                        if let Some(release) = &update.release {
                                            parts.push(
                                                div()
                                                    .text_color(p.done)
                                                    .child(format!("Release {}", release.version))
                                                    .into_any_element(),
                                            );
                                        } else if !update.version.trim().is_empty() {
                                            parts.push(
                                                div()
                                                    .child(format!("Version {}", update.version))
                                                    .into_any_element(),
                                            );
                                        }
                                        parts
                                    })
                                    .flex_wrap(),
                                )
                            }),
                    )
                    .when(update.priority > 0, |d| {
                        d.child(kit::priority_icon(
                            ("compact-priority", id),
                            p,
                            update.priority,
                        ))
                    })
                    .tooltip(tip(compact_card_tooltip(
                        update,
                        self.workspace.settings.date_format,
                    )))
                    .on_click(move |_, window, cx| {
                        open.update(cx, |app, cx| {
                            app.select_update(id, window, cx);
                            app.compact_list = false;
                            app.compact_preview = false;
                            cx.notify();
                        })
                    })
                    .context_menu({
                        let build = update_menu(
                            cx.entity(),
                            id,
                            false,
                            false,
                            false,
                            update.release.is_some(),
                        );
                        move |m, w, cx| build(m, w, cx)
                    }),
            );
        }
        cards = cards.child(
            div()
                .id("compact-update-end")
                .h(px(palette::SP_3))
                .border_t_2()
                .border_color(gpui_kit::transparent_black())
                .drag_over::<UpdateDrag>(move |s, _, _, _| s.border_color(accent))
                .on_drop(
                    cx.listener(|app, d: &UpdateDrag, _, cx| app.reorder_update(d.id, None, cx)),
                ),
        );
        let grid = self.sticky_grid(&stickies, 2, true, cx);
        let post_header =
            section("Post-its", stickies.len()).children(self.sticky_tag.clone().map(|tag| {
                let clear = cx.entity();
                let color = palette::tag_color(&tag, p);
                let cleared = tag.clone();
                kit::surface("sticky-tag-clear", p)
                    .h(px(palette::CTL_SM))
                    .px(px(palette::SP_2))
                    .rounded_full()
                    .bg(color.opacity(0.16))
                    .text_color(color)
                    .tooltip(tip("Filter aufheben"))
                    .child(format!("#{tag} ×"))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        clear.update(cx, |app, cx| app.toggle_sticky_tag(&cleared, cx))
                    })
            }));
        let stack = div().w_full().min_w_0().flex().flex_col().gap(px(palette::SP_3))
            .child(section("Updates", updates.len()).child(div().flex_1()).child(
                div().id("compact-order-hint").text_size(palette::fs(palette::FS_SM))
                    .tooltip(tip("Am Griff ziehen, um die Reihenfolge zu ändern. Weitere Aktionen per Rechtsklick."))
                    .child("Manuell")))
            .when(hidden_count > 0, |d| {
                let show = cx.entity();
                d.child(kit::button("compact-show-completed").ghost()
                    .with_size(gpui_kit::component::Size::Small)
                    .label(format!("{} · anzeigen", model::plural(hidden_count, "erledigtes Update", "erledigte Updates")))
                    .accessibility_label("Erledigte Updates wieder anzeigen")
                    .on_click(move |_, _, cx| show.update(cx, |app, cx| {
                        app.update_settings(cx, |s| s.compact.hide_completed = false)
                    })))
            })
            .child(cards)
            .child(post_header)
            .child(grid);
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
            .child(
                div()
                    .id("compact-list-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(stack),
            )
    }
}

fn note_title(update: &model::Update) -> String {
    update
        .display_title()
        .unwrap_or_else(|| "Neue Notiz".into())
}

pub(super) fn compact_updates<'a>(workspace: &'a Workspace, query: &str) -> Vec<&'a model::Update> {
    let query = query.trim().to_lowercase();
    let updates: Vec<_> = workspace
        .manual_updates()
        .into_iter()
        .filter(|u| {
            u.is_live()
                && (!workspace.settings.compact.hide_completed || u.status != Status::Erledigt)
                && matches_compact_query(u, &query)
        })
        .collect();
    updates
}

fn matches_compact_query(update: &model::Update, query: &str) -> bool {
    query.is_empty()
        || update.title.to_lowercase().contains(query)
        || update.notes.to_lowercase().contains(query)
        || update
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(query.trim_start_matches('#')))
        || update
            .tasks
            .iter()
            .any(|t| t.title.to_lowercase().contains(query))
}

fn compact_card_tooltip(update: &model::Update, format: model::DateFormat) -> String {
    let mut parts = vec![note_title(update), update.status.label().to_owned()];
    let (done, total) = update.task_progress();
    if total > 0 {
        parts.push(format!("{done}/{total} Aufgaben"));
    }
    if let Some(date) = update.planned_for {
        parts.push(dates::full_with_weekday(date, format));
    }
    if let Some(release) = &update.release {
        parts.push(format!("Release {}", release.version));
    } else if !update.version.trim().is_empty() {
        parts.push(format!("Version {}", update.version));
    }
    if update.priority > 0 {
        parts.push(format!(
            "Priorität: {}",
            ["Keine", "Niedrig", "Mittel", "Hoch"][update.priority.min(3) as usize]
        ));
    }
    parts.extend(update.tags.iter().map(|tag| format!("#{tag}")));
    parts.join(" · ")
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

    #[::core::prelude::v1::test]
    fn compact_search_preserves_manual_order() {
        let mut ws = Workspace::default();
        let a = ws.new_update(None);
        let b = ws.new_update(None);
        let c = ws.new_update(None);
        for id in [a, b, c] {
            ws.update_mut(id).unwrap().title = "Release".into();
        }
        ws.move_update(a, Some(c));
        ws.update_mut(b).unwrap().archived = true;
        assert_eq!(
            compact_updates(&ws, "release")
                .iter()
                .map(|u| u.id)
                .collect::<Vec<_>>(),
            vec![a, c]
        );
        ws.update_mut(c).unwrap().touch();
        assert_eq!(
            compact_updates(&ws, "")
                .iter()
                .map(|u| u.id)
                .collect::<Vec<_>>(),
            vec![a, c]
        );
    }

    #[::core::prelude::v1::test]
    fn hiding_completed_preserves_search_order_and_workspace_entries() {
        let mut ws = Workspace::default();
        let a = ws.new_update(None);
        let b = ws.new_update(None);
        let archived = ws.new_update(None);
        for id in [a, b, archived] {
            ws.update_mut(id).unwrap().title = "Release".into();
        }
        ws.update_mut(b).unwrap().status = Status::Erledigt;
        ws.update_mut(archived).unwrap().archived = true;
        ws.move_update(a, Some(b));
        let ids = |ws: &Workspace, query| {
            compact_updates(ws, query)
                .iter()
                .map(|u| u.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&ws, "Release"), vec![a, b]);
        ws.settings.compact.hide_completed = true;
        assert_eq!(ids(&ws, "RELEASE"), vec![a]);
        assert!(ids(&ws, "missing").is_empty());
        assert_eq!(ws.updates.len(), 3);
        ws.update_mut(a).unwrap().status = Status::Erledigt;
        assert!(ids(&ws, "").is_empty());
        ws.settings.compact.hide_completed = false;
        assert_eq!(ids(&ws, ""), vec![a, b]);
        ws.settings.compact.hide_completed = true;
        ws.update_mut(b).unwrap().status = Status::InArbeit;
        assert_eq!(ids(&ws, ""), vec![b]);
    }

    #[::core::prelude::v1::test]
    fn compact_tooltips_keep_hidden_card_metadata() {
        let mut ws = Workspace::default();
        let id = ws.new_update(Some(model::today()));
        let update = ws.update_mut(id).unwrap();
        update.title = "Release planen".into();
        update.version = "1.2.2".into();
        update.priority = 3;
        update.tags.push("Server".into());
        let text = compact_card_tooltip(update, Default::default());
        for expected in [
            "Release planen",
            "Geplant",
            "Version 1.2.2",
            "Priorität: Hoch",
            "#Server",
        ] {
            assert!(text.contains(expected));
        }
        assert!(!text.contains("0/0"));
    }
}
