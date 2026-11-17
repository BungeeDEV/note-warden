//! Project management: create, rename, recolor, archive and delete projects.
use super::*;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};

impl NoteWarden {
    /// Opens the project dialog: `id` edits that project, `None` creates one. A new project is
    /// assigned to the update `assign_to` once it is saved.
    pub fn open_project_dialog(
        &mut self,
        id: Option<u64>,
        assign_to: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.writable {
            return;
        }
        let existing = id.and_then(|id| self.workspace.projects.iter().find(|p| p.id == id));
        let (name, color) = match existing {
            Some(project) => (project.name.clone(), effective_color(project)),
            None => (String::new(), self.next_free_project_color()),
        };
        self.project_color = color;
        self.project_error = None;
        self.project_assign = assign_to;
        self.open_dialog(Dialog::Project(id), window, cx);
        self.project_name_input.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
    }

    /// First palette color no project uses yet, so new projects are easy to tell apart.
    fn next_free_project_color(&self) -> u32 {
        let used: Vec<u32> = self
            .workspace
            .projects
            .iter()
            .map(effective_color)
            .collect();
        palette::PROJECT_PALETTE
            .iter()
            .copied()
            .find(|color| !used.contains(color))
            .unwrap_or(palette::PROJECT_PALETTE[0])
    }

    pub fn save_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(Dialog::Project(id)) = self.dialog.clone() else {
            return;
        };
        if !self.writable {
            return;
        }
        let raw = self.project_name_input.read(cx).value().to_string();
        let Some(name) = model::clean_project_name(&raw) else {
            self.project_error = Some("Bitte einen Namen eingeben.");
            cx.notify();
            return;
        };
        if self.workspace.project_name_taken(&name, id) {
            self.project_error = Some("Ein Projekt mit diesem Namen gibt es schon.");
            cx.notify();
            return;
        }
        let color = self.project_color;
        let message = match id {
            Some(id) => {
                if let Some(project) = self.workspace.projects.iter_mut().find(|p| p.id == id) {
                    project.name = name;
                    project.color = color;
                }
                "Projekt gespeichert"
            }
            None => {
                let new_id = self.workspace.alloc_id();
                self.workspace.projects.push(model::Project {
                    id: new_id,
                    name,
                    color,
                    archived: false,
                    created_at: model::now_millis(),
                });
                if let Some(update) = self
                    .project_assign
                    .take()
                    .and_then(|uid| self.workspace.update_mut(uid))
                {
                    update.project_id = Some(new_id);
                    update.touch();
                }
                "Projekt angelegt"
            }
        };
        self.project_assign = None;
        self.dialog = None;
        self.overlay_focus.focus(window, cx);
        self.persist(cx);
        self.toast(message, None, cx);
        cx.notify();
    }

    pub fn set_project_archived(&mut self, id: u64, archived: bool, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(project) = self.workspace.projects.iter_mut().find(|p| p.id == id) {
            project.archived = archived;
        }
        if archived && self.project_filter == Some(id) {
            self.project_filter = None;
            self.save_updates_ui(cx);
        }
        self.persist(cx);
        cx.notify();
    }

    /// Deletes a project. Its updates and tasks stay and only lose the assignment; undo restores it.
    pub fn delete_project(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let Some(removed) = self.workspace.remove_project(id) else {
            return;
        };
        if self.project_filter == Some(id) {
            self.project_filter = None;
        }
        let ui = &mut self.workspace.settings.updates_ui;
        if ui.project == Some(id) {
            ui.project = None;
        }
        for preset in &mut ui.presets {
            if preset.state.project == Some(id) {
                preset.state.project = None;
            }
        }
        let name = removed.project.name.clone();
        self.save_updates_ui(cx);
        self.toast(
            format!("Projekt „{name}“ gelöscht"),
            Some(Undo::Project(Box::new(removed))),
            cx,
        );
    }

    /// "Updates · Aufgaben" summary for a project, used in the settings and the dialog.
    pub fn project_usage_text(&self, id: u64) -> String {
        let (updates, tasks) = self.workspace.project_usage(id);
        format!(
            "{} · {}",
            model::plural(updates, "Update", "Updates"),
            model::plural(tasks, "offene Aufgabe", "offene Aufgaben")
        )
    }

    pub(super) fn project_dialog_body(
        &self,
        id: Option<u64>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let e = cx.entity();
        let label = |t: &'static str| {
            div()
                .text_size(palette::fs(palette::FS_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_secondary)
                .child(t)
        };
        let swatches = palette::PROJECT_PALETTE
            .iter()
            .copied()
            .zip(palette::PROJECT_COLOR_NAMES)
            .map(|(hex, name)| {
                let pick = cx.entity();
                kit::swatch(
                    SharedString::from(format!("project-color-{hex:x}")),
                    &p,
                    palette::swatch_color(hex),
                    self.project_color == hex,
                    name,
                    std::rc::Rc::new(move |_, cx| {
                        pick.update(cx, |a, cx| {
                            a.project_color = hex;
                            cx.notify();
                        })
                    }),
                )
            });
        let existing = id.and_then(|id| self.workspace.projects.iter().find(|p| p.id == id));
        let mut footer_left = div().flex().items_center().gap(px(palette::SP_2));
        if let Some(project) = existing {
            let pid = project.id;
            let archived = project.archived;
            let archive = cx.entity();
            let delete = cx.entity();
            footer_left = footer_left
                .child(
                    kit::button("project-dialog-delete")
                        .outline()
                        .text_color(p.danger)
                        .label("Löschen")
                        .on_click(move |_, window, cx| {
                            delete.update(cx, |a, cx| {
                                a.dialog = None;
                                a.overlay_focus.focus(window, cx);
                                a.delete_project(pid, cx);
                            })
                        }),
                )
                .child(
                    kit::button("project-dialog-archive")
                        .outline()
                        .label(if archived {
                            "Wiederherstellen"
                        } else {
                            "Archivieren"
                        })
                        .on_click(move |_, window, cx| {
                            archive.update(cx, |a, cx| {
                                a.dialog = None;
                                a.overlay_focus.focus(window, cx);
                                a.set_project_archived(pid, !archived, cx);
                            })
                        }),
                );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(label("Name"))
            .child(
                kit::input(&self.project_name_input)
                    .aria_label("Projektname")
                    .w_full(),
            )
            .children(self.project_error.map(|error| {
                div()
                    .text_size(palette::fs(palette::FS_SM))
                    .text_color(p.danger)
                    .child(error)
            }))
            .child(label("Farbe"))
            .child(div().flex().flex_wrap().gap(px(palette::SP_1)).children(swatches))
            .children(existing.map(|project| {
                text_secondary(
                    &p,
                    format!(
                        "{}. Beim Löschen behalten sie ihren Inhalt und verlieren nur die Zuordnung.",
                        self.project_usage_text(project.id)
                    ),
                )
            }))
            .child(
                div()
                    .mt(px(palette::SP_2))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(palette::SP_2))
                    .child(footer_left)
                    .child(
                        div()
                            .flex()
                            .gap(px(palette::SP_2))
                            .child(kit::button("project-dialog-cancel").outline().label("Abbrechen").on_click(
                                cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                            ))
                            .child(
                                kit::button("project-dialog-save")
                                    .primary()
                                    .label(if id.is_some() { "Speichern" } else { "Anlegen" })
                                    .on_click(move |_, window, cx| {
                                        e.update(cx, |a, cx| a.save_project(window, cx))
                                    }),
                            ),
                    ),
            )
    }

    /// Sidebar entry of a project: click filters by it, the "…" menu manages it.
    pub fn project_sidebar_row(&self, project: &model::Project, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let id = project.id;
        let active = self.project_filter == Some(id);
        let group = SharedString::from(format!("project-row-{id}"));
        let count = self.workspace.project_usage(id).0;
        let filter = cx.entity();
        let edit = cx.entity();
        let archive = cx.entity();
        let delete = cx.entity();
        div()
            .group(group.clone())
            .w_full()
            .flex()
            .items_center()
            .gap(px(palette::SP_1))
            .child(
                kit::surface(SharedString::from(format!("project-{id}")), &p)
                    .flex_1()
                    .min_w_0()
                    .h(px(palette::CTL_MD))
                    .px(px(palette::SP_3))
                    .rounded(px(palette::R_INPUT))
                    .flex()
                    .items_center()
                    .justify_start()
                    .gap(px(palette::SP_3))
                    .text_size(palette::fs(palette::FS_BASE))
                    .text_color(if active {
                        p.text_primary
                    } else {
                        p.text_secondary
                    })
                    .when(active, |b| b.bg(p.bg_active))
                    .child(
                        div()
                            .size(px(palette::ICON_LG))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(dot(palette::project_color(project, &p))),
                    )
                    .tooltip(tip(project.name.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(project.name.clone()),
                    )
                    .when(count > 0, |d| {
                        d.child(
                            div()
                                .text_size(palette::fs(11.))
                                .text_color(p.text_secondary)
                                .child(count.to_string()),
                        )
                    })
                    .on_click(move |_, _, cx| {
                        filter.update(cx, |app, cx| {
                            app.project_filter = if app.project_filter == Some(id) {
                                None
                            } else {
                                Some(id)
                            };
                            app.save_updates_ui(cx);
                        })
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .opacity(0.5)
                    .group_hover(group, |s| s.opacity(1.))
                    .child(
                        icon_btn(("project-menu", id), icons::MORE, "Projekt verwalten")
                            .size(px(palette::CTL_SM))
                            .dropdown_menu(move |menu, _, _| {
                                let edit = edit.clone();
                                let archive = archive.clone();
                                let delete = delete.clone();
                                menu.item(PopupMenuItem::new("Bearbeiten …").on_click(
                                    move |_, window, cx| {
                                        edit.update(cx, |a, cx| {
                                            a.open_project_dialog(Some(id), None, window, cx)
                                        })
                                    },
                                ))
                                .item(PopupMenuItem::new("Archivieren").on_click(
                                    move |_, _, cx| {
                                        archive.update(cx, |a, cx| {
                                            a.set_project_archived(id, true, cx)
                                        })
                                    },
                                ))
                                .item(PopupMenuItem::separator())
                                .item(
                                    PopupMenuItem::new("Löschen").on_click(move |_, _, cx| {
                                        delete.update(cx, |a, cx| a.delete_project(id, cx))
                                    }),
                                )
                            }),
                    ),
            )
    }

    /// "+ Neues Projekt" entry at the end of the sidebar's project group.
    pub fn new_project_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let create = cx.entity();
        kit::surface("project-new", &p)
            .w_full()
            .h(px(palette::CTL_MD))
            .px(px(palette::SP_3))
            .rounded(px(palette::R_INPUT))
            .flex()
            .items_center()
            .justify_start()
            .gap(px(palette::SP_3))
            .text_size(palette::fs(palette::FS_BASE))
            .text_color(p.text_tertiary)
            .child(
                div()
                    .size(px(palette::ICON_LG))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(kit::icon(icons::ADD, palette::ICON_SM).text_color(p.text_tertiary)),
            )
            .child("Neues Projekt")
            .on_click(move |_, window, cx| {
                create.update(cx, |a, cx| a.open_project_dialog(None, None, window, cx))
            })
    }
}

fn effective_color(project: &model::Project) -> u32 {
    if project.color == 0 {
        palette::default_project_color(project.id)
    } else {
        project.color
    }
}
