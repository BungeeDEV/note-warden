use super::*;
use chrono::Datelike as _;
use gpui_kit::base::FocusTrapElement as _;
use gpui_kit::component::input::{
    Enter, Escape, MoveDown, MoveLeft, MovePageDown, MovePageUp, MoveRight, MoveUp,
};

/// Fuzzy-Treffer: alle Zeichen der Eingabe in Reihenfolge, Bonus für zusammenhängende
/// Treffer und Wortanfänge. `None` = kein Treffer.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let query: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if query.is_empty() {
        return Some(0);
    }
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut last: Option<usize> = None;
    for (i, c) in text.iter().enumerate() {
        if qi < query.len() && *c == query[qi] {
            score += 10;
            if last == Some(i.wrapping_sub(1)) {
                score += 15;
            }
            if i == 0 || !text[i - 1].is_alphanumeric() {
                score += 12;
            }
            if let Some(l) = last {
                score -= (i - l - 1).min(8) as i32;
            }
            last = Some(i);
            qi += 1;
        }
    }
    (qi == query.len()).then_some(score - text.len() as i32 / 8)
}

#[derive(Clone)]
pub enum PaletteAction {
    OpenUpdate(u64),
    QuickAdd(String),
    NewUpdate,
    GoToday,
    GoUpdates,
    GoCalendar,
    GoInbox,
    GoBoard,
    ToggleTheme,
    ToggleSidebar,
    ExportBackup,
    ImportBackup,
    Settings,
    Shortcuts,
    GoReleases,
    Publish,
    BackupNow,
    CheckUpdates,
}

pub struct PaletteItem {
    pub label: String,
    pub hint: String,
    pub action: PaletteAction,
}

impl NoteWarden {
    pub fn palette_items(&self, query: &str) -> Vec<PaletteItem> {
        let command = |label: &str, hint: &str, action| PaletteItem {
            label: label.into(),
            hint: hint.into(),
            action,
        };
        if let Some(text) = query.strip_prefix('+') {
            let parsed = quick::parse_quick_input(text, self.now_today(), &self.workspace.projects);
            return vec![command(
                &format!("Aufgabe anlegen: {}", parsed.title),
                "Enter",
                PaletteAction::QuickAdd(text.into()),
            )];
        }
        let mut items = vec![
            command("Neues Update", "Strg+N", PaletteAction::NewUpdate),
            command("Zur Heute-Ansicht", "Strg+1", PaletteAction::GoToday),
            command("Zu den Updates", "Strg+2", PaletteAction::GoUpdates),
            command("Zur Inbox", "", PaletteAction::GoInbox),
            command("Zum Board", "Strg+3", PaletteAction::GoBoard),
            command("Zum Kalender", "Strg+4", PaletteAction::GoCalendar),
            command(
                "Theme wechseln",
                "System · Dunkel · Hell",
                PaletteAction::ToggleTheme,
            ),
            command(
                "Seitenleiste ein-/ausklappen",
                "Strg+B",
                PaletteAction::ToggleSidebar,
            ),
            command("Gehe zu Releases", "", PaletteAction::GoReleases),
            command("Jetzt sichern", "Backup", PaletteAction::BackupNow),
            command("Nach Updates suchen", "", PaletteAction::CheckUpdates),
            command("Backup exportieren", "JSON", PaletteAction::ExportBackup),
            command("Backup importieren", "JSON", PaletteAction::ImportBackup),
            command("Einstellungen", "", PaletteAction::Settings),
            command("Tastaturkürzel", "?", PaletteAction::Shortcuts),
        ];
        if self
            .selected()
            .is_some_and(|u| u.is_live() && u.release.is_none())
        {
            items.insert(
                1,
                command("Veröffentlichen …", "Release", PaletteAction::Publish),
            );
        }
        for update in self.workspace.updates.iter().filter(|u| u.is_live()) {
            items.push(PaletteItem {
                label: update.title.clone(),
                hint: format!("Update · {}", update.status.label()),
                action: PaletteAction::OpenUpdate(update.id),
            });
            for task in &update.tasks {
                items.push(PaletteItem {
                    label: task.title.clone(),
                    hint: format!("Aufgabe in {}", update.title),
                    action: PaletteAction::OpenUpdate(update.id),
                });
            }
        }
        if query.trim().is_empty() {
            items.truncate(10);
            return items;
        }
        let mut scored: Vec<(i32, PaletteItem)> = items
            .into_iter()
            .filter_map(|item| fuzzy_score(query, &item.label).map(|s| (s, item)))
            .collect();
        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
        scored.into_iter().take(12).map(|(_, i)| i).collect()
    }

    fn run_palette(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.palette_input.read(cx).value().to_string();
        let Some(item) = self.palette_items(&query).into_iter().nth(index) else {
            return;
        };
        self.palette_open = false;
        self.focus_handle.focus(window, cx);
        match item.action {
            PaletteAction::QuickAdd(text) => {
                self.quick_input
                    .update(cx, |input, cx| input.set_value(text, window, cx));
                self.commit_quick_add(window, cx);
            }
            PaletteAction::OpenUpdate(id) => self.select_update(id, window, cx),
            PaletteAction::NewUpdate => self.create_update(None, None, window, cx),
            PaletteAction::GoToday => self.set_view(View::Today, cx),
            PaletteAction::GoUpdates => self.set_view(View::Updates, cx),
            PaletteAction::GoInbox => self.set_view(View::Inbox, cx),
            PaletteAction::GoBoard => self.set_view(View::Board, cx),
            PaletteAction::GoCalendar => self.set_view(View::Calendar, cx),
            PaletteAction::ToggleTheme => self.cycle_theme(cx),
            PaletteAction::ToggleSidebar => self.toggle_sidebar(cx),
            PaletteAction::ExportBackup => self.export_backup(cx),
            PaletteAction::ImportBackup => self.pick_import(cx),
            PaletteAction::Settings => self.open_settings(None, window, cx),
            PaletteAction::Shortcuts => self.open_dialog(Dialog::Shortcuts, window, cx),
            PaletteAction::GoReleases => self.set_view(View::Releases, cx),
            PaletteAction::Publish => {
                if let Some(id) = self.selected_update {
                    self.open_publish(id, window, cx);
                }
            }
            PaletteAction::BackupNow => {
                if self.backup_now(true) {
                    self.toast("Backup angelegt", None, cx);
                }
            }
            PaletteAction::CheckUpdates => self.check_updates(true, cx),
        }
        cx.notify();
    }

    pub(super) fn command_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let query = self.palette_input.read(cx).value().to_string();
        let items = self.palette_items(&query);
        let selected = self.palette_index.min(items.len().saturating_sub(1));
        let count = items.len();
        kit::surface("palette-backdrop", &p)
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .pt(px(96.))
            .bg(gpui_kit::black().opacity(0.4))
            .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx)))
            .capture_action(cx.listener(move |this, _: &MoveDown, _, cx| {
                this.palette_index = (this.palette_index + 1).min(count.saturating_sub(1));
                cx.stop_propagation();
                cx.notify();
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, _, cx| {
                this.palette_index = this.palette_index.saturating_sub(1);
                cx.stop_propagation();
                cx.notify();
            }))
            .capture_action(cx.listener(move |this, _: &Enter, window, cx| {
                let index = this.palette_index.min(count.saturating_sub(1));
                this.run_palette(index, window, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                this.close_overlay(window, cx);
                cx.stop_propagation();
            }))
            .child(
                div()
                    .id("palette")
                    .role(Role::Group)
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w(px(560.))
                    .max_h(px(440.))
                    .flex()
                    .flex_col()
                    .rounded(px(palette::R_DIALOG))
                    .bg(p.bg_elevated)
                    .border_1()
                    .border_color(p.border_strong)
                    .shadow_lg()
                    .overflow_hidden()
                    .child(
                        div().p(px(palette::SP_3)).child(
                            kit::input(&self.palette_input)
                                .aria_label("Befehlspalette")
                                .prefix(ico(icons::SEARCH, p.text_tertiary))
                                .w_full(),
                        ),
                    )
                    .child(
                        div()
                            .id("palette-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .pl(px(palette::SP_2))
                            .pr(px(palette::SCROLL_GUTTER))
                            .pb(px(palette::SP_2))
                            .when(items.is_empty(), |d| {
                                d.child(
                                    div().p(px(12.)).child(text_secondary(&p, "Keine Treffer.")),
                                )
                            })
                            .children(items.into_iter().enumerate().map(|(i, item)| {
                                let entity = cx.entity();
                                kit::surface(("palette-item", i), &p)
                                    .h(px(34.))
                                    .px(px(palette::SP_3))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(12.))
                                    .rounded(px(palette::R_INPUT))
                                    .cursor_pointer()
                                    .when(i == selected, |d| d.bg(p.bg_active))
                                    .when(i != selected, |d| d)
                                    .child(div().min_w_0().truncate().child(item.label))
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(palette::fs(palette::FS_SM))
                                            .text_color(p.text_tertiary)
                                            .child(item.hint),
                                    )
                                    .on_click(move |_, window, cx| {
                                        entity.update(cx, |app, cx| app.run_palette(i, window, cx))
                                    })
                            })),
                    ),
            )
    }

    pub(super) fn date_popover(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let pop = self.date_pop.as_ref().expect("date popover");
        let today = self.now_today();
        let current = self.target_date(pop.target);
        let week_start = self.workspace.settings.week_start;
        let first = pop.month;
        let start = first - Duration::days(week_start.offset(first.weekday()));
        let cursor = pop.cursor;
        let anchor = pop.anchor;
        let quick: [(&'static str, Option<NaiveDate>); 5] = [
            ("Heute", Some(today)),
            ("Morgen", Some(today + Duration::days(1))),
            ("Nächsten Montag", Some(dates::next_monday(today))),
            ("In 1 Woche", Some(today + Duration::days(7))),
            ("Kein Datum", None),
        ];
        let nav = |id: &'static str,
                   icon: IconName,
                   tip: &'static str,
                   delta: i32,
                   cx: &mut Context<Self>| {
            let entity = cx.entity();
            icon_btn(id, icon, tip).on_click(move |_, _, cx| {
                entity.update(cx, |app, cx| {
                    if let Some(pop) = &mut app.date_pop {
                        pop.month = model::shift_month(pop.month, delta);
                        cx.notify();
                    }
                })
            })
        };
        let empty_input = |this: &Self, cx: &App| this.date_input.read(cx).value().is_empty();
        let _ = empty_input;

        let panel = div()
            .id("date-popover")
            .max_h(px(540.))
            .overflow_y_scrollbar()
            .occlude()
            .w(px(296.))
            .p(px(12.))
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .rounded(px(palette::R_DIALOG))
            .bg(p.bg_elevated)
            .border_1()
            .border_color(p.border_strong)
            .shadow_lg()
            .capture_action(cx.listener(|this, _: &MoveLeft, _, cx| {
                if this.date_input.read(cx).value().is_empty() {
                    this.move_date_cursor(-1, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveRight, _, cx| {
                if this.date_input.read(cx).value().is_empty() {
                    this.move_date_cursor(1, cx);
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, _, cx| {
                this.move_date_cursor(-7, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &MoveDown, _, cx| {
                this.move_date_cursor(7, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &MovePageUp, _, cx| {
                if let Some(pop) = &mut this.date_pop {
                    pop.cursor = model::shift_month(pop.cursor, -1)
                        .with_day(1)
                        .unwrap_or(pop.cursor);
                    pop.month = model::month_start(pop.cursor);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .capture_action(cx.listener(|this, _: &MovePageDown, _, cx| {
                if let Some(pop) = &mut this.date_pop {
                    pop.cursor = model::shift_month(pop.cursor, 1)
                        .with_day(1)
                        .unwrap_or(pop.cursor);
                    pop.month = model::month_start(pop.cursor);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                this.close_overlay(window, cx);
                cx.stop_propagation();
            }))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(4.))
                    .children(quick.map(|(label, date)| {
                        let entity = cx.entity();
                        kit::button(label)
                            .outline()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .label(label)
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |app, cx| app.choose_date(date, window, cx))
                            })
                    })),
            )
            .child(
                kit::input(&self.date_input)
                    .aria_label("Datum eingeben")
                    .w_full(),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(format!(
                        "{} {}",
                        dates::MONTHS[first.month0() as usize],
                        first.year()
                    )))
                    .child(
                        div()
                            .flex()
                            .child(nav(
                                "pop-prev",
                                IconName::ChevronLeft,
                                "Vorheriger Monat · Bild↑",
                                -1,
                                cx,
                            ))
                            .child(nav(
                                "pop-next",
                                icons::COLLAPSE,
                                "Nächster Monat · Bild↓",
                                1,
                                cx,
                            )),
                    ),
            )
            .child(div().flex().children(week_start.names().map(|name| {
                div()
                    .flex_1()
                    .text_size(px(11.))
                    .text_color(p.text_tertiary)
                    .flex()
                    .justify_center()
                    .child(name)
            })))
            .children((0..6).map(|week| {
                div().flex().children((0..7).map(|day| {
                    let date = start + Duration::days((week * 7 + day) as i64);
                    let entity = cx.entity();
                    let in_month = date.month() == first.month();
                    let selected = current == Some(date);
                    kit::surface(("pick", date.num_days_from_ce() as usize), &p)
                        .flex_1()
                        .h(px(30.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(palette::R_CONTROL_SM))
                        .cursor_pointer()
                        .text_size(palette::fs(palette::FS_BASE))
                        .when(selected, |d| {
                            d.bg(p.accent)
                                .text_color(p.accent_contrast)
                                .font_weight(FontWeight::SEMIBOLD)
                        })
                        .when(!selected, |d| {
                            d.text_color(if in_month {
                                p.text_primary
                            } else {
                                p.text_tertiary
                            })
                            .when(date == today, |d| d.border_1().border_color(p.accent))
                            .when(date == cursor, |d| d.bg(p.bg_active))
                        })
                        .child(date.day().to_string())
                        .on_click(move |_, window, cx| {
                            entity.update(cx, |app, cx| app.choose_date(Some(date), window, cx))
                        })
                }))
            }))
            .children(self.reminder_row(pop.target, &p, cx))
            .child(text_secondary(
                &p,
                "Enter übernimmt · Pfeile wählen · Esc schließt",
            ));

        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .id("date-backdrop")
                    .absolute()
                    .inset_0()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                    ),
            )
            .child(deferred(
                anchored()
                    .position(anchor)
                    .snap_to_window_with_margin(px(8.))
                    .child(panel),
            ))
    }

    /// Zeile "Erinnerung" im Datums-Popover (nur für Updates, Aufgaben und Inbox-Einträge).
    fn reminder_row(
        &self,
        date_target: DateTarget,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let target = reminders::Target::from_date_target(date_target)?;
        let current = self.remind_of(target).and_then(|r| r.at);
        let now = chrono::Local::now().naive_local();
        let today = self.now_today();
        let mut chips: Vec<(&'static str, Option<chrono::NaiveDateTime>)> = vec![("Keine", None)];
        if let Some(due) = self
            .target_date(date_target)
            .map(reminders::morning)
            .filter(|at| *at > now)
        {
            chips.push(("Zur Fälligkeit (9:00)", Some(due)));
        }
        if let Some(evening) = today.and_hms_opt(18, 0, 0).filter(|at| *at > now) {
            chips.push(("Heute 18:00", Some(evening)));
        }
        chips.push((
            "Morgen 9:00",
            Some(reminders::morning(today + Duration::days(1))),
        ));
        let custom_active = current.is_some() && !chips.iter().any(|(_, at)| *at == current);
        let mut row = div().flex().flex_col().gap(px(palette::SP_2)).child(
            div()
                .text_size(palette::fs(palette::FS_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_secondary)
                .child("Erinnerung"),
        );
        row = row.child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.))
                .children(chips.into_iter().map(|(label, at)| {
                    let entity = cx.entity();
                    let selected = at == current;
                    kit::button(SharedString::from(format!("remind-{label}")))
                        .outline()
                        .toggled(selected)
                        .with_size(gpui_kit::component::Size::Small)
                        .h(px(palette::CTL_SM))
                        .label(label)
                        .on_click(move |_, _, cx| {
                            entity.update(cx, |app, cx| app.set_remind(target, at, cx))
                        })
                }))
                .child({
                    let entity = cx.entity();
                    kit::button("remind-custom")
                        .outline()
                        .toggled(custom_active || self.remind_custom)
                        .with_size(gpui_kit::component::Size::Small)
                        .h(px(palette::CTL_SM))
                        .label("Eigene Zeit …")
                        .on_click(move |_, window, cx| {
                            entity.update(cx, |app, cx| {
                                app.remind_custom = !app.remind_custom;
                                if app.remind_custom {
                                    app.remind_time_input
                                        .update(cx, |input, cx| input.focus(window, cx));
                                }
                                cx.notify();
                            })
                        })
                }),
        );
        if self.remind_custom {
            row = row.child(
                kit::input(&self.remind_time_input)
                    .aria_label("Uhrzeit der Erinnerung")
                    .w_full(),
            );
        }
        if let Some(at) = current {
            row = row.child(text_secondary(
                p,
                format!("Erinnerung: {}", self.remind_label(at)),
            ));
        }
        Some(row.into_any_element())
    }

    fn dialog_frame(
        &self,
        title: &'static str,
        width: f32,
        body: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        kit::surface("dialog-backdrop", &p)
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui_kit::black().opacity(0.5))
            .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx)))
            .child(
                div()
                    .id("dialog")
                    .role(Role::Dialog)
                    .track_focus(&self.overlay_focus)
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w(px(width))
                    .max_w_full()
                    .max_h(relative(0.9))
                    .flex()
                    .flex_col()
                    .rounded(px(palette::R_DIALOG))
                    .bg(p.bg_elevated)
                    .border_1()
                    .border_color(p.border_strong)
                    .shadow_lg()
                    .child(
                        div()
                            .px(px(palette::SP_6))
                            .pt(px(16.))
                            .pb(px(palette::SP_3))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_LG))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                icon_btn("dialog-close", icons::CLOSE, "Schließen · Esc").on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.close_overlay(window, cx)
                                    }),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .id("dialog-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .px(px(palette::SP_6))
                            .pb(px(palette::SP_6))
                            .child(body),
                    )
                    .focus_trap("dialog-trap", &self.overlay_focus),
            )
    }

    fn shortcuts_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        div().flex().flex_col().gap(px(palette::SP_3)).children(
            super::settings::SHORTCUTS.iter().map(|(group, rows)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .pb(px(4.))
                            .text_size(palette::fs(palette::FS_SM))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text_secondary)
                            .child(*group),
                    )
                    .children(rows.iter().map(|(keys, what)| {
                        div()
                            .py(px(4.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .child(div().child(*what))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.))
                                    .children(keys.split(" / ").map(|alt| {
                                        super::kit::keycaps(&p, alt).into_any_element()
                                    })),
                            )
                    }))
            }),
        )
    }

    /// Dezente Leiste: Update verfügbar, Fortschritt oder bereit zum Neustart.
    pub(super) fn update_banner(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let p = palette::get(cx);
        let state = self
            .update_state
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let bar = |text: String, extra: Vec<AnyElement>| {
            div()
                .px(px(palette::SP_4))
                .py(px(palette::SP_2))
                .bg(p.bg_hover)
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().text_size(palette::fs(palette::FS_SM)).child(text))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_2))
                        .children(extra),
                )
        };
        let small = |id: &'static str| {
            kit::button(id)
                .with_size(gpui_kit::component::Size::Small)
                .h(px(palette::CTL_SM))
        };
        match state {
            crate::updater::State::Available(info) if !self.update_dismissed => {
                let (now, later, notes) = (cx.entity(), cx.entity(), cx.entity());
                let target = info.clone();
                Some(
                    bar(
                        format!("Version {} ist verfügbar", info.version),
                        vec![
                            small("update-now")
                                .primary()
                                .label("Jetzt aktualisieren")
                                .on_click(move |_, _, cx| {
                                    let info = target.clone();
                                    now.update(cx, |a, _| {
                                        crate::updater::download(&a.update_state, info)
                                    })
                                })
                                .into_any_element(),
                            small("update-later")
                                .ghost()
                                .label("Später")
                                .on_click(move |_, _, cx| {
                                    later.update(cx, |a, cx| {
                                        a.update_dismissed = true;
                                        cx.notify();
                                    })
                                })
                                .into_any_element(),
                            small("update-notes")
                                .outline()
                                .label("Neuigkeiten")
                                .on_click(move |_, window, cx| {
                                    notes.update(cx, |a, cx| {
                                        a.open_dialog(Dialog::UpdateNotes, window, cx)
                                    })
                                })
                                .into_any_element(),
                        ],
                    )
                    .into_any_element(),
                )
            }
            crate::updater::State::Downloading(info, progress) => Some(
                bar(
                    format!("Version {} wird geladen …", info.version),
                    vec![
                        div()
                            .w(px(160.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(p.border_subtle)
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(p.accent)
                                    .w(relative(progress.clamp(0., 1.))),
                            )
                            .into_any_element(),
                    ],
                )
                .into_any_element(),
            ),
            crate::updater::State::Ready(info, path) => {
                let install = cx.entity();
                Some(
                    bar(
                        format!("Version {} ist bereit", info.version),
                        vec![
                            small("update-restart")
                                .primary()
                                .label("Neu starten, um zu aktualisieren")
                                .on_click(move |_, _, cx| {
                                    let path = path.clone();
                                    install.update(cx, |a, cx| a.install_update(path, cx))
                                })
                                .into_any_element(),
                        ],
                    )
                    .into_any_element(),
                )
            }
            _ => None,
        }
    }

    /// Ruhiger Hinweis nach automatischer Wiederherstellung (schließbar).
    pub(super) fn restore_banner(&self, at: i64, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let when = dates::full_time(at).unwrap_or_default();
        div()
            .px(px(palette::SP_4))
            .py(px(palette::SP_2))
            .bg(p.bg_hover)
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(div().text_size(palette::fs(palette::FS_SM)).child(format!(
                "Daten wurden aus einer Sicherung vom {when} wiederhergestellt."
            )))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::button("restore-details")
                            .outline()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .label("Details")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_dialog(Dialog::Recovered(at), window, cx)
                            })),
                    )
                    .child(
                        kit::button("restore-folder")
                            .outline()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .label("Datenordner öffnen")
                            .on_click(cx.listener(|this, _, _, cx| this.open_data_dir(cx))),
                    )
                    .child(
                        icon_btn("restore-dismiss", icons::CLOSE, "Hinweis schließen").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.restore_notice = None;
                                cx.notify();
                            }),
                        ),
                    ),
            )
    }

    pub(super) fn dialog_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let dialog = self.dialog.clone().expect("dialog");
        match dialog {
            Dialog::Template => {
                let body = self.template_dialog_body(cx).into_any_element();
                self.dialog_frame("Vorlage bearbeiten", 520., body, cx)
                    .into_any_element()
            }
            Dialog::AppChangelog => {
                let body = self.app_changelog_body(cx).into_any_element();
                self.dialog_frame("Änderungsprotokoll", 560., body, cx)
                    .into_any_element()
            }
            Dialog::Recovered(at) => {
                let when = dates::full_time(at).unwrap_or_default();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(&p, format!("Die Datendatei war nicht lesbar. Note Warden hat die neueste gültige Sicherung vom {when} geladen. Änderungen seitdem fehlen eventuell.")))
                    .child(text_secondary(&p, "Die beschädigte Datei liegt unverändert als „workspace.json.corrupt-…“ im Datenordner."))
                    .child(
                        div().flex().justify_end().gap(px(8.))
                            .child(kit::button("recovered-folder").outline().label("Datenordner öffnen")
                                .on_click(cx.listener(|this, _, _, cx| this.open_data_dir(cx))))
                            .child(kit::button("recovered-close").primary().label("Schließen")
                                .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx)))),
                    );
                self.dialog_frame("Daten wiederhergestellt", 480., body, cx)
                    .into_any_element()
            }
            Dialog::ConfirmRestore(at) => {
                let when = dates::full_time(at).unwrap_or_default();
                let entity = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(&p, format!("Der Stand vom {when} ersetzt alle Updates und Aufgaben. Der aktuelle Stand wird vorher gesichert. Einstellungen bleiben erhalten.")))
                    .child(
                        div().flex().justify_end().gap(px(8.))
                            .child(kit::button("restore-cancel").outline().label("Abbrechen")
                                .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx))))
                            .child(kit::button("restore-confirm").primary().label("Wiederherstellen")
                                .on_click(move |_, window, cx| entity.update(cx, |app, cx| app.restore_from_backup(at, window, cx)))),
                    );
                self.dialog_frame("Backup wiederherstellen?", 460., body, cx)
                    .into_any_element()
            }
            Dialog::Publish(id) => {
                let editing = self
                    .workspace
                    .update(id)
                    .is_some_and(|u| u.release.is_some());
                let body = self.publish_body(id, cx).into_any_element();
                self.dialog_frame(
                    if editing {
                        "Release bearbeiten"
                    } else {
                        "Veröffentlichen"
                    },
                    560.,
                    body,
                    cx,
                )
                .into_any_element()
            }
            Dialog::UpdateNotes => {
                let notes = match self
                    .update_state
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default()
                {
                    crate::updater::State::Available(i)
                    | crate::updater::State::Downloading(i, _)
                    | crate::updater::State::Ready(i, _) => i.notes,
                    _ => String::new(),
                };
                let body = div()
                    .p(px(12.))
                    .rounded(px(palette::R_INPUT))
                    .bg(p.bg_app)
                    .border_1()
                    .border_color(p.border_subtle)
                    .font_family(palette::FONT_MONO)
                    .text_size(palette::fs(palette::FS_SM))
                    .children(
                        notes
                            .lines()
                            .map(|l| div().min_h(px(16.)).child(l.to_owned()))
                            .collect::<Vec<_>>(),
                    );
                self.dialog_frame("Neuigkeiten", 520., body, cx)
                    .into_any_element()
            }
            Dialog::Licenses => {
                let body = self.licenses_body(cx).into_any_element();
                self.dialog_frame("Drittbibliotheken", 520., body, cx)
                    .into_any_element()
            }
            Dialog::ConfirmTrash(id) => {
                let name = self
                    .workspace
                    .update(id)
                    .map(|u| u.title.clone())
                    .unwrap_or_default();
                let entity = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(
                        &p,
                        format!("„{name}“ wird in den Papierkorb verschoben und kann dort wiederhergestellt werden."),
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(kit::button("trash-cancel").outline().label("Abbrechen").on_click(
                                cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                            ))
                            .child(kit::button("trash-confirm").danger().label("In den Papierkorb").on_click(
                                move |_, window, cx| {
                                    entity.update(cx, |app, cx| {
                                        app.dialog = None;
                                        app.trash_update(id, window, cx)
                                    })
                                },
                            )),
                    );
                self.dialog_frame("Update löschen?", 440., body, cx)
                    .into_any_element()
            }
            Dialog::ConfirmEmptyTrash => {
                let count = self
                    .workspace
                    .updates
                    .iter()
                    .filter(|u| u.deleted_on.is_some())
                    .count();
                let entity = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(
                        &p,
                        format!(
                            "{} werden dauerhaft entfernt. Das kann nicht rückgängig gemacht werden.",
                            model::plural(count, "Update", "Updates")
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(kit::button("empty-cancel").outline().label("Abbrechen").on_click(
                                cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                            ))
                            .child(kit::button("empty-confirm").danger().label("Papierkorb leeren").on_click(
                                move |_, _, cx| entity.update(cx, |app, cx| app.empty_trash(cx)),
                            )),
                    );
                self.dialog_frame("Papierkorb leeren?", 440., body, cx)
                    .into_any_element()
            }
            Dialog::ConfirmWipe => {
                let typed = self.wipe_input.read(cx).value().to_string();
                let armed = typed.trim() == "LÖSCHEN";
                let entity = cx.entity();
                let entity2 = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(
                        &p,
                        "Alle Updates, Aufgaben und der Papierkorb werden gelöscht. Vorher legt Note Warden automatisch eine Kopie neben der Datendatei an. Einstellungen bleiben erhalten.",
                    ))
                    .child(text_secondary(&p, "Tippe LÖSCHEN, um zu bestätigen:"))
                    .child(
                        kit::input(&self.wipe_input)
                            .aria_label("Bestätigung")
                            .w_full(),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .gap(px(8.))
                            .child(kit::button("wipe-backup").outline().label("Zuerst Backup exportieren").on_click(
                                move |_, _, cx| entity2.update(cx, |app, cx| app.export_backup(cx)),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .child(kit::button("wipe-cancel").outline().label("Abbrechen").on_click(
                                        cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                                    ))
                                    .child(
                                        kit::button("wipe-confirm")
                                            .danger()
                                            .label("Alles löschen")
                                            .disabled(!armed)
                                            .on_click(move |_, window, cx| {
                                                entity.update(cx, |app, cx| app.wipe_all(window, cx))
                                            }),
                                    ),
                            ),
                    );
                self.dialog_frame("Alle Daten löschen?", 480., body, cx)
                    .into_any_element()
            }
            Dialog::Shortcuts => {
                let body = self.shortcuts_body(cx).into_any_element();
                self.dialog_frame("Tastaturkürzel", 520., body, cx)
                    .into_any_element()
            }
            Dialog::ConfirmPurge(id) => {
                let name = self
                    .workspace
                    .update(id)
                    .map(|u| u.title.clone())
                    .unwrap_or_default();
                let entity = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_4))
                    .child(text_secondary(&p, format!("„{name}“ wird dauerhaft entfernt. Das kann nicht rückgängig gemacht werden.")))
                    .child(
                        div().flex().justify_end().gap(px(8.))
                            .child(kit::button("purge-cancel").outline().label("Abbrechen").on_click(
                                cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                            ))
                            .child(kit::button("purge-confirm").danger().label("Endgültig löschen").on_click(
                                move |_, window, cx| {
                                    entity.update(cx, |app, cx| app.purge_update(id, window, cx))
                                },
                            )),
                    );
                self.dialog_frame("Update endgültig löschen?", 440., body, cx)
                    .into_any_element()
            }
            Dialog::Import => {
                let (new_updates, duplicates, tasks) = self
                    .import_preview
                    .as_ref()
                    .map(|i| (i.new_updates, i.duplicates, i.tasks))
                    .unwrap_or_default();
                let merge = cx.entity();
                let replace = cx.entity();
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(div().child(format!(
                        "{} neu, {} bereits vorhanden · {} insgesamt in der Datei.",
                        model::plural(new_updates, "Update", "Updates"),
                        duplicates,
                        model::plural(tasks, "Aufgabe", "Aufgaben"),
                    )))
                    .child(text_secondary(
                        &p,
                        "Zusammenführen ergänzt nur neue Updates. Ersetzen tauscht alle Updates aus; vorher wird automatisch eine Kopie des aktuellen Stands neben der Datenbank abgelegt.",
                    ))
                    .child(
                        div().flex().justify_end().gap(px(8.))
                            .child(kit::button("import-cancel").outline().label("Abbrechen").on_click(
                                cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                            ))
                            .child(kit::button("import-replace").outline().label("Ersetzen").on_click(
                                move |_, window, cx| {
                                    replace.update(cx, |app, cx| app.apply_import(true, window, cx))
                                },
                            ))
                            .child(kit::button("import-merge").primary().label("Zusammenführen").on_click(
                                move |_, window, cx| {
                                    merge.update(cx, |app, cx| app.apply_import(false, window, cx))
                                },
                            )),
                    );
                self.dialog_frame("Backup importieren", 480., body, cx)
                    .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fuzzy_score;

    #[::core::prelude::v1::test]
    fn fuzzy_prefers_prefix_and_requires_order() {
        assert!(fuzzy_score("nu", "Neues Update").is_some());
        assert!(fuzzy_score("xyz", "Neues Update").is_none());
        assert!(fuzzy_score("heu", "Zur Heute-Ansicht") > fuzzy_score("hte", "Zur Heute-Ansicht"));
        assert!(
            fuzzy_score("ba", "Backup exportieren")
                > fuzzy_score("ba", "Seitenleiste ein-/ausklappen")
        );
    }
}
