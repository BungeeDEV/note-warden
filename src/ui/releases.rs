use super::*;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use model::{ChangeKind, Release, Status};

const PREVIEW_LINES: usize = 6;

impl NoteWarden {
    // ---------- Aktionen ----------

    /// Öffnet den Release-Dialog; bei vorhandenem Release wird dieser bearbeitet.
    pub fn open_publish(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(update) = self.workspace.update(id) else {
            return;
        };
        let (version, text) = match &update.release {
            Some(release) => (release.version.clone(), release.changelog.clone()),
            None => {
                let versions: Vec<&str> = self
                    .workspace
                    .updates
                    .iter()
                    .filter(|other| other.project_id == update.project_id)
                    .filter_map(|other| other.release.as_ref().map(|r| r.version.as_str()))
                    .collect();
                let minor = update.tasks.iter().any(|t| t.kind == ChangeKind::Added);
                let version = md::suggest_version(&versions, minor)
                    .or_else(|| {
                        md::is_version(&update.version).then(|| update.version.trim().to_owned())
                    })
                    .unwrap_or_default();
                (version, md::format_changelog(None, &update.tasks, true))
            }
        };
        self.release_generated = text.clone();
        self.release_close_open = false;
        self.release_version_input
            .update(cx, |input, cx| input.set_value(version, window, cx));
        self.release_text_input
            .update(cx, |input, cx| input.set_value(text, window, cx));
        self.open_dialog(Dialog::Publish(id), window, cx);
        self.release_version_input
            .update(cx, |input, cx| input.focus(window, cx));
    }

    /// "Offene Aufgaben als erledigt markieren": erzeugter Text wird mitgeführt, solange er nicht bearbeitet wurde.
    pub fn set_release_close_open(
        &mut self,
        on: bool,
        id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.release_close_open = on;
        let current = self.release_text_input.read(cx).value().to_string();
        if current.trim() == self.release_generated.trim()
            && let Some(update) = self.workspace.update(id)
        {
            let text = md::format_changelog(None, &update.tasks, !on);
            self.release_generated = text.clone();
            self.release_text_input
                .update(cx, |input, cx| input.set_value(text, window, cx));
        }
        cx.notify();
    }

    fn release_draft(&self, cx: &App) -> (String, String) {
        (
            self.release_version_input
                .read(cx)
                .value()
                .trim()
                .to_owned(),
            self.release_text_input.read(cx).value().trim().to_owned(),
        )
    }

    pub fn copy_release_draft(&mut self, cx: &mut Context<Self>) {
        let (version, text) = self.release_draft(cx);
        self.copy_text(md::release_block(&version, &text), "Changelog kopiert", cx);
    }

    pub fn publish_update(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let (version, changelog) = self.release_draft(cx);
        if !self.writable || !md::is_version(&version) {
            return;
        }
        let close_open = self.release_close_open;
        let Some(update) = self.workspace.update_mut(id) else {
            return;
        };
        let previous = Box::new(update.clone());
        let editing = update.release.is_some();
        let previous_status = update
            .release
            .as_ref()
            .map_or(update.status, |r| r.previous_status);
        if close_open {
            update.tasks.iter_mut().for_each(|t| t.done = true);
        }
        update.version = version.clone();
        update.status = Status::Erledigt;
        update.release = Some(Release {
            version: version.clone(),
            changelog,
            at: previous
                .release
                .as_ref()
                .map_or_else(model::now_millis, |r| r.at),
            previous_status,
        });
        update.touch();
        self.dialog = None;
        self.sync_fields(window, cx);
        self.persist(cx);
        crate::log::info("Release veröffentlicht");
        let message = if editing {
            format!("Release {version} gespeichert")
        } else {
            format!("Version {version} veröffentlicht")
        };
        self.toast(message, Some(Undo::Update(previous)), cx);
        self.focus_handle.focus(window, cx);
    }

    pub fn unpublish_update(&mut self, id: u64, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        let Some(update) = self.workspace.update_mut(id) else {
            return;
        };
        let Some(release) = update.release.take() else {
            return;
        };
        let previous = {
            let mut copy = update.clone();
            copy.release = Some(release.clone());
            Box::new(copy)
        };
        update.status = release.previous_status;
        update.touch();
        self.persist(cx);
        self.toast(
            format!("Veröffentlichung von {} zurückgenommen", release.version),
            Some(Undo::Update(previous)),
            cx,
        );
        cx.notify();
    }

    /// Alle Releases des gewählten Projekts (oder aller), neueste zuerst.
    fn releases(&self) -> Vec<(&model::Update, &Release)> {
        let mut list: Vec<_> = self
            .workspace
            .updates
            .iter()
            .filter(|u| {
                u.deleted_on.is_none()
                    && self
                        .releases_project
                        .is_none_or(|p| u.project_id == Some(p))
            })
            .filter_map(|u| u.release.as_ref().map(|r| (u, r)))
            .collect();
        list.sort_by_key(|entry| std::cmp::Reverse(entry.1.at));
        list
    }

    pub fn export_changelog_file(&mut self, cx: &mut Context<Self>) {
        let releases = self.releases();
        if releases.is_empty() {
            self.toast("Noch keine Releases zum Exportieren", None, cx);
            return;
        }
        let pairs: Vec<(&str, &str)> = releases
            .iter()
            .map(|(_, r)| (r.version.as_str(), r.changelog.as_str()))
            .collect();
        let text = md::changelog_file(&pairs);
        self.save_with_dialog(
            "CHANGELOG.md".into(),
            text.into_bytes(),
            "CHANGELOG.md gespeichert",
            cx,
        );
    }

    // ---------- Dialog ----------

    pub(super) fn publish_body(&self, id: u64, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let (version, _) = self.release_draft(cx);
        let valid = md::is_version(&version);
        let editing = self
            .workspace
            .update(id)
            .is_some_and(|u| u.release.is_some());
        let open = self
            .workspace
            .update(id)
            .map_or(0, |u| u.tasks.iter().filter(|t| !t.done).count());
        let entity = cx.entity();
        let copy = cx.entity();
        let toggle = cx.entity();
        let label = |text: &'static str| {
            div()
                .text_size(palette::fs(palette::FS_SM))
                .text_color(p.text_secondary)
                .child(text)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(label("Version"))
            .child(
                kit::input(&self.release_version_input)
                    .aria_label("Version")
                    .w_full(),
            )
            .when(!version.is_empty() && !valid, |d| {
                d.child(
                    div()
                        .text_size(palette::fs(palette::FS_SM))
                        .text_color(p.danger)
                        .child("Format: x.y.z, zum Beispiel 1.2.0"),
                )
            })
            .child(label("Changelog"))
            .child(
                div()
                    .font_family(palette::FONT_MONO)
                    .rounded(px(palette::R_INPUT))
                    .border_1()
                    .border_color(p.border_subtle)
                    .bg(p.bg_app)
                    .child(
                        Textarea::new(&self.release_text_input)
                            .aria_label("Changelog-Text")
                            .appearance(false)
                            .bordered(false)
                            .w_full(),
                    ),
            )
            .when(open > 0 && !editing, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(palette::SP_3))
                        .child(text_secondary(
                            &p,
                            format!("{} noch offen", model::plural(open, "Aufgabe", "Aufgaben")),
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(palette::SP_2))
                                .child(text_secondary(&p, "Offene Aufgaben als erledigt markieren"))
                                .child(kit::switch(
                                    "release-close-open",
                                    &p,
                                    self.release_close_open,
                                    "Offene Aufgaben als erledigt markieren",
                                    kit::click_fn(move |window, cx| {
                                        toggle.update(cx, |a, cx| {
                                            let on = !a.release_close_open;
                                            a.set_release_close_open(on, id, window, cx)
                                        })
                                    }),
                                )),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::toned_button(
                            "release-cancel",
                            &p,
                            kit::ButtonTone::Ghost,
                            palette::CTL_MD,
                        )
                        .label("Abbrechen")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                        ),
                    )
                    .child(
                        kit::toned_button(
                            "release-copy",
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_MD,
                        )
                        .label("Nur kopieren")
                        .on_click(move |_, _, cx| {
                            copy.update(cx, |a, cx| a.copy_release_draft(cx))
                        }),
                    )
                    .child(
                        kit::toned_button(
                            "release-publish",
                            &p,
                            kit::ButtonTone::Primary,
                            palette::CTL_MD,
                        )
                        .label(if editing {
                            "Speichern"
                        } else {
                            "Veröffentlichen"
                        })
                        .disabled(!valid)
                        .on_click(move |_, window, cx| {
                            entity.update(cx, |a, cx| a.publish_update(id, window, cx))
                        }),
                    ),
            )
    }

    // ---------- Ansicht ----------

    pub fn releases_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let releases = self.releases();
        let project_label = self
            .releases_project
            .and_then(|id| self.workspace.projects.iter().find(|pr| pr.id == id))
            .map_or("Alle Projekte".to_owned(), |pr| pr.name.clone());
        let projects: Vec<(u64, String)> = self
            .workspace
            .projects
            .iter()
            .filter(|pr| !pr.archived)
            .map(|pr| (pr.id, pr.name.clone()))
            .collect();
        let current = self.releases_project;
        let entity = cx.entity();
        let export = cx.entity();
        let header = div()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap(px(palette::SP_3))
            .child(
                div()
                    .text_size(palette::fs(palette::FS_XL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Releases"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::toned_button(
                            "releases-project",
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_SM,
                        )
                        .label(project_label)
                        .icon(kit::icon(icons::PROJECT, palette::ICON_MD))
                        .accessibility_label("Projekt filtern")
                        .dropdown_menu(move |menu, _, _| {
                            let set = |pid: Option<u64>| {
                                let e = entity.clone();
                                move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                                    e.update(cx, |a, cx| {
                                        a.releases_project = pid;
                                        cx.notify();
                                    })
                                }
                            };
                            let menu = menu.item(
                                PopupMenuItem::new("Alle Projekte")
                                    .checked(current.is_none())
                                    .on_click(set(None)),
                            );
                            projects.iter().fold(menu, |menu, (pid, name)| {
                                menu.item(
                                    PopupMenuItem::new(name.clone())
                                        .checked(current == Some(*pid))
                                        .on_click(set(Some(*pid))),
                                )
                            })
                        }),
                    )
                    .child(
                        kit::toned_button(
                            "releases-export",
                            &p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_SM,
                        )
                        .icon(kit::icon(icons::DOWNLOAD, palette::ICON_MD))
                        .label("Als CHANGELOG.md exportieren")
                        .on_click(move |_, _, cx| {
                            export.update(cx, |a, cx| a.export_changelog_file(cx))
                        }),
                    ),
            );
        let mut body = div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(palette::SP_4))
            .p(px(palette::SP_6))
            .max_w(px(820.))
            .child(header);
        if releases.is_empty() {
            body = body.child(kit::empty_state(
                &p,
                "Noch keine Releases",
                "Ein Update über „…“ → „Veröffentlichen …“ abschließen, dann erscheint es hier.",
                None,
            ));
        }
        for (update, release) in releases {
            body = body.child(self.release_card(update, release, &p, cx));
        }
        div()
            .id("releases-scroll")
            .size_full()
            .min_h_0()
            .overflow_y_scrollbar()
            .pr(px(palette::SCROLL_GUTTER))
            .child(body)
    }

    fn release_card(
        &self,
        update: &model::Update,
        release: &Release,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = update.id;
        let expanded = self.releases_expanded.contains(&id);
        let lines: Vec<&str> = release.changelog.lines().collect();
        let long = lines.len() > PREVIEW_LINES;
        let shown = if expanded || !long {
            lines.len()
        } else {
            PREVIEW_LINES
        };
        let project = update
            .project_id
            .and_then(|pid| self.workspace.projects.iter().find(|pr| pr.id == pid));
        let block = md::release_block(&release.version, &release.changelog);
        let (toggle, copy, open, edit, undo) = (
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
            cx.entity(),
        );
        let menu_edit = edit.clone();
        kit::card(p)
            .p(px(palette::SP_4))
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .flex_wrap()
                    .gap(px(palette::SP_3))
                    .child(
                        div()
                            .text_size(palette::fs(palette::FS_XL))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(release.version.clone()),
                    )
                    .when_some(project, |d, project| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(palette::SP_1))
                                .text_size(palette::fs(palette::FS_SM))
                                .text_color(p.text_secondary)
                                .child(
                                    div()
                                        .size(px(8.))
                                        .rounded_full()
                                        .bg(palette::project_color(project, p)),
                                )
                                .child(project.name.clone()),
                        )
                    })
                    .child(
                        div()
                            .text_size(palette::fs(palette::FS_BASE))
                            .text_color(p.text_secondary)
                            .child(if update.title.trim().is_empty() {
                                "Ohne Titel".to_owned()
                            } else {
                                update.title.clone()
                            }),
                    )
                    .child(
                        div()
                            .id(("release-at", id))
                            .text_size(palette::fs(palette::FS_SM))
                            .text_color(p.text_tertiary)
                            .tooltip(tip(dates::full_time(release.at).unwrap_or_default()))
                            .child(dates::elapsed_at(release.at, model::now_millis())),
                    ),
            )
            .child(
                div()
                    .font_family(palette::FONT_MONO)
                    .text_size(palette::fs(palette::FS_SM))
                    .text_color(p.text_primary)
                    .flex()
                    .flex_col()
                    .children(
                        lines
                            .iter()
                            .take(shown)
                            .map(|line| div().min_h(px(16.)).child((*line).to_owned())),
                    ),
            )
            .when(long, |d| {
                d.child(
                    kit::link_button(
                        ("release-more", id),
                        p,
                        if expanded {
                            "Weniger anzeigen"
                        } else {
                            "Mehr anzeigen"
                        },
                    )
                    .on_click(move |_, _, cx| {
                        toggle.update(cx, |a, cx| {
                            if !a.releases_expanded.remove(&id) {
                                a.releases_expanded.insert(id);
                            }
                            cx.notify();
                        })
                    }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::toned_button(
                            ("release-copy-card", id),
                            p,
                            kit::ButtonTone::Secondary,
                            palette::CTL_SM,
                        )
                        .icon(kit::icon(icons::COPY, palette::ICON_MD))
                        .label("Kopieren")
                        .on_click(move |_, _, cx| {
                            let block = block.clone();
                            copy.update(cx, |a, cx| a.copy_text(block, "Changelog kopiert", cx))
                        }),
                    )
                    .child(
                        kit::toned_button(
                            ("release-open", id),
                            p,
                            kit::ButtonTone::Ghost,
                            palette::CTL_SM,
                        )
                        .label("Öffnen")
                        .on_click(move |_, window, cx| {
                            open.update(cx, |a, cx| {
                                a.set_view(View::Updates, cx);
                                a.select_update(id, window, cx);
                                a.detail_open = true;
                            })
                        }),
                    )
                    .child(
                        icon_btn(("release-more-menu", id), icons::MORE, "Weitere Aktionen")
                            .dropdown_menu(move |menu, _, _| {
                                let (edit, undo) = (menu_edit.clone(), undo.clone());
                                menu.item(PopupMenuItem::new("Bearbeiten …").on_click(
                                    move |_, window, cx| {
                                        edit.update(cx, |a, cx| a.open_publish(id, window, cx))
                                    },
                                ))
                                .item(
                                    PopupMenuItem::new("Veröffentlichung zurücknehmen").on_click(
                                        move |_, _, cx| {
                                            undo.update(cx, |a, cx| a.unpublish_update(id, cx))
                                        },
                                    ),
                                )
                            }),
                    ),
            )
    }
}
