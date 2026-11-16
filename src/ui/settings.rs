//! Settings: a large view with categories, cards and rows. Changes apply immediately.

use super::kit::*;
use super::*;
use crate::model::{
    Accent, CalView, DateFormat, Density, EditorFont, FontScale, Motion, Settings, StartView,
    WeekStart,
};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};

/// Builds a section (heading + card of rows); rows filter themselves through the search.
struct Section {
    p: Palette,
    q: String,
    title: &'static str,
    rows: Vec<AnyElement>,
    reset: Option<ClickFn>,
    danger: bool,
    navigation: Option<(Entity<NoteWarden>, SettingsCat, Option<String>, FocusHandle)>,
}

fn matches_query(q: &str, title: &str, desc: &str) -> bool {
    let q = q.trim().to_lowercase();
    q.is_empty() || title.to_lowercase().contains(&q) || desc.to_lowercase().contains(&q)
}

impl Section {
    fn new(p: &Palette, q: &str, title: &'static str) -> Self {
        Self {
            p: *p,
            q: q.to_owned(),
            title,
            rows: Vec::new(),
            reset: None,
            danger: false,
            navigation: None,
        }
    }

    fn navigation(
        mut self,
        entity: Entity<NoteWarden>,
        cat: SettingsCat,
        target: Option<&str>,
        focus: &FocusHandle,
    ) -> Self {
        self.navigation = Some((entity, cat, target.map(str::to_owned), focus.clone()));
        self
    }
    fn add_row(&mut self, title: String, desc: Option<SharedString>, control: AnyElement) {
        let mut label = div()
            .id(SharedString::from(format!("setting-label-{title}")))
            .child(highlighted(&title, &self.q, &self.p));
        if !self.q.trim().is_empty()
            && let Some((entity, cat, _, _)) = &self.navigation
        {
            let e = entity.clone();
            let cat = *cat;
            let target = title.clone();
            label = label
                .cursor_pointer()
                .tab_stop(true)
                .hover(|s| s.underline())
                .on_click(move |_, window, cx| {
                    e.update(cx, |a, cx| {
                        a.settings_cat = cat;
                        a.workspace.settings.last_settings_category =
                            SettingsCat::ALL.iter().position(|c| *c == cat).unwrap_or(0);
                        a.settings_jump = Some(target.clone());
                        a.settings_jump_pending = true;
                        a.settings_search
                            .update(cx, |i, cx| i.set_value("", window, cx));
                        a.persist(cx);
                        cx.defer_in(window, |a, window, cx| {
                            a.settings_target_focus.focus(window, cx)
                        });
                        cx.notify();
                    })
                });
        }
        let mut row = settings_row(&self.p, self.rows.is_empty(), label, desc, control)
            .id(SharedString::from(format!("setting-row-{title}")));
        if let Some((entity, _, Some(target), focus)) = &self.navigation
            && target == &title
        {
            let e = entity.clone();
            row = row.relative().child(
                canvas(
                    |bounds, _, _| bounds,
                    move |bounds, _, _, cx| {
                        e.update(cx, |a, cx| {
                            if a.settings_jump_pending
                                && let Some(view) = a.settings_bounds
                            {
                                a.settings_jump_pending = false;
                                let offset = a.settings_scroll.offset();
                                a.settings_scroll.set_offset(point(
                                    px(0.),
                                    (offset.y + view.origin.y + px(12.) - bounds.origin.y)
                                        .min(px(0.)),
                                ));
                                cx.notify();
                            }
                        })
                    },
                )
                .absolute()
                .inset_0(),
            );
            row = row
                .bg(self.p.bg_active)
                .track_focus(focus)
                .tab_stop(true)
                .focus_visible(|s| s.border_1().border_color(self.p.accent));
        }
        self.rows.push(row.into_any_element());
    }
    fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// "Auf Standard zurücksetzen" (reset to default) only appears if something differs from the default.
    fn reset(mut self, differs: bool, f: ClickFn) -> Self {
        if differs {
            self.reset = Some(f);
        }
        self
    }

    fn row(&mut self, title: &'static str, desc: &'static str, control: impl IntoElement) {
        if !matches_query(&self.q, title, desc) {
            return;
        }
        self.add_row(
            title.to_owned(),
            (!desc.is_empty()).then(|| SharedString::from(desc)),
            control.into_any_element(),
        );
    }

    /// Row with a dynamic description (path, numbers).
    fn row_text(&mut self, title: &'static str, desc: String, control: impl IntoElement) {
        if !matches_query(&self.q, title, &desc) {
            return;
        }
        self.add_row(
            title.to_owned(),
            Some(desc.into()),
            control.into_any_element(),
        );
    }

    fn build(self) -> Option<AnyElement> {
        if self.rows.is_empty() {
            return None;
        }
        let p = self.p;
        let mut card = card(&p);
        if self.danger {
            card = card.border_color(p.danger.opacity(0.6));
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(palette::SP_2))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(palette::fs(palette::FS_BASE))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(if self.danger {
                                    p.danger
                                } else {
                                    p.text_secondary
                                })
                                .child(self.title),
                        )
                        .when_some(self.reset, |d, f| {
                            d.child(
                                kit::surface(
                                    SharedString::from(format!("reset-{}", self.title)),
                                    &p,
                                )
                                .cursor_pointer()
                                .text_size(palette::fs(palette::FS_SM))
                                .text_color(p.text_secondary)
                                .child("Auf Standard zurücksetzen")
                                .on_click(move |_, window, cx| f(window, cx)),
                            )
                        }),
                )
                .child(card.children(self.rows))
                .into_any_element(),
        )
    }
}

/// Shortens long paths in the middle ("C:\\Users\\…\\Note Warden").
fn ellipsize_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    let keep = max.saturating_sub(1);
    let head: String = chars[..keep / 2].iter().collect();
    let tail: String = chars[chars.len() - (keep - keep / 2)..].iter().collect();
    format!("{head}…{tail}")
}

fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.)
    } else {
        format!("{:.1} MB", bytes as f64 / 1024. / 1024.)
    }
}

pub const SHORTCUTS: &[(&str, &[(&str, &str)])] = &[
    (
        "Allgemein",
        &[
            ("Strg+N", "Neues Update"),
            ("Strg+Umschalt+M", "Notizzettel öffnen oder schließen"),
            ("Strg+K", "Befehlspalette"),
            ("Strg+,", "Einstellungen öffnen"),
            ("Strg+F", "Suche fokussieren"),
            ("Strg+B", "Seitenleiste ein-/ausklappen"),
            ("N", "Heute: neue Aufgabe eingeben"),
            (
                "Strg+Alt+N",
                "Quick-Capture aus jedem Programm (einstellbar unter System)",
            ),
            ("Entf", "Ausgewähltes Update löschen (mit Rückgängig)"),
            ("Esc", "Dialog, Popover oder Einstellungen schließen"),
            ("?", "Tastenkürzel-Übersicht"),
        ],
    ),
    (
        "Updates",
        &[
            ("↑ / ↓ / J / K", "Update auswählen"),
            ("Enter", "Titel bearbeiten"),
            ("1 / 2 / 3 / 4", "Status setzen"),
            ("Leertaste", "Update erledigt setzen oder wieder öffnen"),
            ("D", "Datum setzen"),
            ("Strg+D", "Update duplizieren"),
            ("Strg+Z", "Änderung rückgängig machen"),
            ("Strg+Y / Strg+Umschalt+Z", "Änderung wiederholen"),
            ("Alt+↑ / Alt+↓", "Vorheriges/nächstes Update"),
            ("Strg+Umschalt+F", "Fokus-Modus umschalten"),
            ("Strg+A", "Alle sichtbaren Updates auswählen"),
            ("X", "Auswahl umschalten"),
        ],
    ),
    (
        "Navigation",
        &[
            ("Strg+1", "Heute"),
            ("Strg+2", "Updates"),
            ("Strg+3", "Board"),
            ("Strg+4", "Kalender"),
        ],
    ),
    (
        "Editor",
        &[
            ("Strg+B", "Fett"),
            ("Strg+I", "Kursiv"),
            ("Strg+Umschalt+K", "Link"),
            ("Tab", "Liste einrücken"),
            ("Umschalt+Tab", "Liste ausrücken"),
            ("Leertaste", "Fokussierte Aufgabe abhaken"),
            ("Enter", "Fokussierte Aufgabe umbenennen"),
            ("Alt+↑ / Alt+↓", "Fokussierte Aufgabe verschieben"),
        ],
    ),
    (
        "Kalender",
        &[
            ("← / →", "Zeitraum wechseln"),
            ("T", "Zu heute springen"),
            ("M / W", "Monats- bzw. Wochenansicht"),
            ("Umschalt+← / Umschalt+→", "Fokus einen Tag zurück/vor"),
            ("↑ / ↓", "Fokus eine Woche zurück/vor"),
            ("Enter", "Fokussierten Tag öffnen"),
        ],
    ),
];

impl NoteWarden {
    // ---------- Actions of the data category ----------

    pub fn reveal_data_file(&mut self, cx: &mut Context<Self>) {
        if !crate::system::reveal(&self.path) {
            self.toast("Ordner konnte nicht geöffnet werden", None, cx);
        }
    }

    pub fn delete_backup(&mut self, at: i64, cx: &mut Context<Self>) {
        if let Some((info, _)) = self.backups.iter().find(|(b, _)| b.at == at && b.pinned) {
            let _ = std::fs::remove_file(&info.path);
            self.refresh_backups();
            self.toast("Backup gelöscht", None, cx);
        }
    }

    /// Restores a backup; the current state is backed up first, settings are kept.
    pub fn restore_from_backup(&mut self, at: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.dialog = None;
        let Some(path) = self
            .backups
            .iter()
            .find(|(b, _)| b.at == at)
            .map(|(b, _)| b.path.clone())
        else {
            return;
        };
        let restored = match storage::read_backup_file(&path) {
            Ok(workspace) => workspace,
            Err(_) => {
                self.toast("Backup nicht lesbar", None, cx);
                return;
            }
        };
        if !self.backup_now(false) {
            self.toast(
                "Wiederherstellung abgebrochen: Backup nicht möglich",
                None,
                cx,
            );
            return;
        }
        let settings = self.workspace.settings.clone();
        self.workspace = restored;
        self.workspace.settings = settings;
        self.workspace.repair_ids();
        self.selected_update = self
            .workspace
            .updates
            .iter()
            .rev()
            .find(|u| u.is_live())
            .map(|u| u.id);
        self.sync_fields(window, cx);
        self.persist(cx);
        crate::log::info("Backup wiederhergestellt");
        self.toast("Backup wiederhergestellt", None, cx);
        cx.notify();
    }

    pub fn toggle_autostart(&mut self, cx: &mut Context<Self>) {
        let on = !self.workspace.settings.autostart;
        match crate::system::set_autostart(on) {
            Ok(()) => self.update_settings(cx, |s| s.autostart = on),
            Err(error) => {
                crate::log::error(&format!("Autostart nicht änderbar: {error}"));
                self.toast("Autostart konnte nicht geändert werden", None, cx);
            }
        }
    }

    /// Registers a new hotkey; on a conflict the old one stays active and the row explains it.
    pub fn apply_hotkey(&mut self, text: &str, cx: &mut Context<Self>) {
        self.hotkey_capture = false;
        match crate::system::set_hotkey(text) {
            Ok(()) => {
                self.hotkey_error = None;
                let text = text.to_owned();
                self.update_settings(cx, |s| s.hotkey = text);
            }
            Err(crate::system::HotkeyError::Taken) => {
                self.hotkey_error = Some("Dieses Kürzel ist bereits belegt.")
            }
            Err(crate::system::HotkeyError::Invalid) => {
                self.hotkey_error = Some("Dieses Kürzel ist ungültig.")
            }
        }
        cx.notify();
    }

    pub fn apply_system_defaults(&mut self, cx: &mut Context<Self>) {
        let _ = crate::system::set_autostart(false);
        self.apply_hotkey("Strg+Alt+N", cx);
        self.update_settings(cx, |s| {
            s.autostart = false;
            s.tray_on_close = true;
        });
    }

    /// Takes the next key combination as the hotkey (Esc cancels).
    pub fn capture_hotkey(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.hotkey_capture {
            return;
        }
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        cx.stop_propagation();
        if key == "escape" {
            self.hotkey_capture = false;
            self.focus_handle.focus(window, cx);
            cx.notify();
            return;
        }
        let single = key.chars().count() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric());
        let function = key
            .strip_prefix('f')
            .is_some_and(|n| n.parse::<u8>().is_ok_and(|n| (1..=12).contains(&n)));
        if !(single || function) {
            return;
        }
        if !(modifiers.control || modifiers.alt || modifiers.platform) {
            self.hotkey_error = Some("Kombiniere Strg, Alt oder Win mit einer Taste.");
            cx.notify();
            return;
        }
        let mut parts = Vec::new();
        if modifiers.control {
            parts.push("Strg".to_owned());
        }
        if modifiers.alt {
            parts.push("Alt".to_owned());
        }
        if modifiers.shift {
            parts.push("Umschalt".to_owned());
        }
        if modifiers.platform {
            parts.push("Win".to_owned());
        }
        parts.push(key.to_uppercase());
        self.focus_handle.focus(window, cx);
        self.apply_hotkey(&parts.join("+"), cx);
    }

    pub fn open_data_dir(&mut self, cx: &mut Context<Self>) {
        if !crate::system::open_path(&storage::data_dir(&self.path)) {
            self.toast("Ordner konnte nicht geöffnet werden", None, cx);
        }
    }

    pub fn open_log(&mut self, cx: &mut Context<Self>) {
        if !crate::log::file().is_some_and(|file| crate::system::open_path(&file)) {
            self.toast("Logdatei konnte nicht geöffnet werden", None, cx);
        }
    }

    pub fn empty_trash(&mut self, cx: &mut Context<Self>) {
        let before = self.workspace.updates.len();
        self.workspace.updates.retain(|u| u.deleted_on.is_none());
        let removed = before - self.workspace.updates.len();
        self.dialog = None;
        self.persist(cx);
        self.toast(
            format!(
                "Papierkorb geleert ({})",
                model::plural(removed, "Update", "Updates")
            ),
            None,
            cx,
        );
        cx.notify();
    }

    pub fn wipe_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        // Pinned backup before anything is deleted.
        if !self.backup_now(true) {
            self.toast(
                "Sicherheitskopie fehlgeschlagen – nichts gelöscht",
                None,
                cx,
            );
            return;
        }
        self.workspace.updates.clear();
        self.selected_update = None;
        self.detail_open = false;
        self.dialog = None;
        self.sync_fields(window, cx);
        self.persist(cx);
        self.toast(
            "Alle Daten gelöscht · Backup liegt unter Daten & Backups",
            None,
            cx,
        );
        cx.notify();
    }

    pub fn export_all_markdown(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Zielordner wählen".into()),
        });
        let files: Vec<(String, String)> = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.deleted_on.is_none())
            .map(|u| (md::file_stem(&u.title), md::export_update(u)))
            .collect();
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(dir) = paths.into_iter().next()
            {
                let mut used = std::collections::HashSet::new();
                let mut written = 0;
                for (stem, content) in files {
                    let mut name = stem.clone();
                    let mut n = 2;
                    while !used.insert(name.to_lowercase()) {
                        name = format!("{stem}-{n}");
                        n += 1;
                    }
                    if storage::write_atomic(&dir.join(format!("{name}.md")), content.as_bytes())
                        .is_ok()
                    {
                        written += 1;
                    }
                }
                this.update(cx, |this, cx| {
                    this.toast(
                        format!(
                            "{} als Markdown exportiert",
                            model::plural(written, "Update", "Updates")
                        ),
                        None,
                        cx,
                    )
                })
                .ok();
            }
        })
        .detach();
    }

    pub fn open_template_editor(
        &mut self,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (name, notes, tasks) =
            match index.and_then(|i| self.workspace.settings.templates.get(i)) {
                Some(t) => (t.name.clone(), t.notes.clone(), t.tasks.join("\n")),
                None => (String::new(), String::new(), String::new()),
            };
        self.tpl_edit = Some(index);
        self.dialog = Some(Dialog::Template);
        self.template_name_input
            .update(cx, |i, cx| i.set_value(name, window, cx));
        self.tpl_notes_input
            .update(cx, |i, cx| i.set_value(notes, window, cx));
        self.tpl_tasks_input
            .update(cx, |i, cx| i.set_value(tasks, window, cx));
        self.overlay_focus.focus(window, cx);
        cx.notify();
    }

    pub fn save_template(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.template_name_input.read(cx).value().trim().to_owned();
        if name.is_empty() {
            self.toast("Bitte einen Namen eingeben", None, cx);
            return;
        }
        let template = model::Template {
            name,
            notes: self.tpl_notes_input.read(cx).value().to_string(),
            tasks: self
                .tpl_tasks_input
                .read(cx)
                .value()
                .lines()
                .map(|l| l.trim().to_owned())
                .filter(|l| !l.is_empty())
                .collect(),
        };
        let target = self.tpl_edit.flatten();
        self.update_settings(cx, |s| match target {
            Some(i) if i < s.templates.len() => s.templates[i] = template,
            _ => s.templates.push(template),
        });
        self.tpl_edit = None;
        self.dialog = None;
        self.overlay_focus.focus(window, cx);
        cx.notify();
    }

    pub fn template_from_selected(&mut self, cx: &mut Context<Self>) {
        let Some(update) = self.selected() else {
            self.toast("Wähle zuerst ein Update als Grundlage", None, cx);
            return;
        };
        let template = model::Template {
            name: format!("{} (Vorlage)", update.title),
            notes: update.notes.clone(),
            tasks: update.tasks.iter().map(|t| t.title.clone()).collect(),
        };
        self.update_settings(cx, |s| s.templates.push(template));
    }

    // ---------- Rendering ----------

    fn theme_card(
        &self,
        id: &'static str,
        label: &'static str,
        choice: model::ThemeChoice,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = palette::get(cx);
        let active = self.workspace.settings.theme == choice;
        let (d, l) = (Palette::dark(), Palette::light());
        let preview = |pal: Palette| {
            div()
                .flex_1()
                .h_full()
                .bg(pal.bg_app)
                .flex()
                .child(div().w(px(14.)).h_full().bg(pal.bg_sidebar))
                .child(
                    div()
                        .flex_1()
                        .p(px(palette::SP_2))
                        .flex()
                        .flex_col()
                        .gap(px(4.))
                        .child(
                            div()
                                .h(px(4.))
                                .w(px(34.))
                                .rounded_full()
                                .bg(pal.text_primary),
                        )
                        .child(
                            div()
                                .h(px(3.))
                                .w(px(24.))
                                .rounded_full()
                                .bg(pal.text_tertiary),
                        )
                        .child(
                            div()
                                .h(px(8.))
                                .rounded(px(2.))
                                .bg(pal.bg_active)
                                .border_l_2()
                                .border_color(pal.accent),
                        ),
                )
        };
        let entity = cx.entity();
        let keyboard = entity.clone();
        kit::surface(id, &p)
            .tab_stop(true)
            .on_key_down(move |ev, _, cx| {
                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                    keyboard.update(cx, |a, cx| a.update_settings(cx, |s| s.theme = choice));
                    cx.stop_propagation();
                }
            })
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(palette::SP_2))
            .p(px(palette::SP_2))
            .rounded(px(palette::R_CARD))
            .when(!active, |d| d.border_1().border_color(p.border_subtle))
            .when(active, |d| {
                d.bg(p.bg_active).border_l_2().border_color(p.accent)
            })
            .cursor_pointer()
            .child(
                div()
                    .h(px(64.))
                    .rounded(px(palette::R_CONTROL_SM))
                    .overflow_hidden()
                    .border_1()
                    .border_color(p.border_subtle)
                    .flex()
                    .map(|el| match choice {
                        model::ThemeChoice::Dark => el.child(preview(d)),
                        model::ThemeChoice::Light => el.child(preview(l)),
                        model::ThemeChoice::System => el.child(preview(d)).child(preview(l)),
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(palette::SP_2))
                    .text_size(palette::fs(palette::FS_BASE))
                    .text_color(p.text_primary)
                    .font_weight(FontWeight::MEDIUM)
                    .child(label),
            )
            .on_click(move |_, _, cx| {
                entity.update(cx, |app, cx| app.update_settings(cx, |s| s.theme = choice))
            })
    }

    #[allow(clippy::too_many_lines)]
    fn category_sections(
        &self,
        cat: SettingsCat,
        q: &str,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let p = palette::get(cx);
        let s = &self.workspace.settings;
        let today = self.now_today();
        let entity = cx.entity();

        // Small builder helpers: every switch/segment writes through `update_settings`.
        let sw = |id: &'static str, on: bool, label: &'static str, apply: fn(&mut Settings)| {
            let e = entity.clone();
            switch(
                id,
                &p,
                on,
                label,
                click_fn(move |_, cx| e.update(cx, |a, cx| a.update_settings(cx, apply))),
            )
        };
        let seg = |id: &'static str,
                   labels: &[&'static str],
                   active: usize,
                   apply: fn(&mut Settings, usize)| {
            let e = entity.clone();
            let items: Vec<_> = labels.iter().map(|l| (None, *l)).collect();
            segment(id, &p, &items, active, true, move |i, _, cx| {
                e.update(cx, |a, cx| a.update_settings(cx, |s| apply(s, i)))
            })
        };
        let reset = |apply: fn(&mut Settings, &Settings)| {
            let e = entity.clone();
            click_fn(move |_, cx| {
                e.update(cx, |a, cx| {
                    a.update_settings(cx, |s| apply(s, &Settings::default()))
                })
            })
        };
        let action =
            |id: &'static str,
             label: &'static str,
             f: fn(&mut NoteWarden, &mut Window, &mut Context<NoteWarden>)| {
                let e = entity.clone();
                kit::button(id)
                    .outline()
                    .with_size(gpui_kit::component::Size::Small)
                    .h(px(palette::CTL_SM))
                    .label(label)
                    .on_click(move |_, window, cx| e.update(cx, |a, cx| f(a, window, cx)))
            };
        let d = Settings::default();
        let mut out: Vec<Option<AnyElement>> = Vec::new();

        match cat {
            SettingsCat::Projects => {
                let mut sec = Section::new(&p, q, "Projekte").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row(
                    "Projekt anlegen",
                    "Namen in der Inbox mit #Projekt eingeben und dort das Projekt anlegen.",
                    action("projects-inbox", "Zur Inbox", |a, w, cx| {
                        a.close_overlay(w, cx);
                        a.set_view(View::Inbox, cx);
                    }),
                );
                for project in &self.workspace.projects {
                    if !matches_query(q, &project.name, "Projekt archivieren wiederherstellen") {
                        continue;
                    }
                    let id = project.id;
                    let e = entity.clone();
                    sec.rows.push(
                        settings_row(
                            &p,
                            sec.rows.is_empty(),
                            project.name.clone(),
                            Some(
                                if project.archived {
                                    "Archiviert"
                                } else {
                                    "Aktiv"
                                }
                                .into(),
                            ),
                            kit::button(("project-archive", id))
                                .outline()
                                .label(if project.archived {
                                    "Wiederherstellen"
                                } else {
                                    "Archivieren"
                                })
                                .on_click(move |_, _, cx| {
                                    e.update(cx, |a, cx| {
                                        if !a.writable {
                                            return;
                                        }
                                        if let Some(p) =
                                            a.workspace.projects.iter_mut().find(|p| p.id == id)
                                        {
                                            p.archived = !p.archived;
                                        }
                                        a.persist(cx);
                                        cx.notify();
                                    })
                                }),
                        )
                        .into_any_element(),
                    );
                }
                out.push(sec.build());
            }
            SettingsCat::Home => {
                let mut sec = Section::new(&p, q, "Startseite")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.home_cards != d.home_cards,
                        reset(|s, d| s.home_cards = d.home_cards),
                    );
                for (i, title) in [
                    "Fokus",
                    "In Arbeit",
                    "Woche",
                    "Inbox",
                    "Zuletzt bearbeitet",
                    "Diese Woche geschafft",
                    "Kennzahlen oben anzeigen",
                ]
                .into_iter()
                .enumerate()
                {
                    let e = entity.clone();
                    sec.row(
                        title,
                        "",
                        switch(
                            SharedString::from(format!("home-switch-{i}")),
                            &p,
                            s.home_cards[i],
                            title,
                            click_fn(move |_, cx| {
                                e.update(cx, |a, cx| {
                                    a.update_settings(cx, |s| s.home_cards[i] = !s.home_cards[i])
                                })
                            }),
                        ),
                    );
                }
                out.push(sec.build());
            }
            SettingsCat::General => {
                let mut sec = Section::new(&p, q, "Start")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.start_view != d.start_view,
                        reset(|s, d| s.start_view = d.start_view),
                    );
                sec.row(
                    "Startansicht",
                    "Was beim Öffnen von Note Warden angezeigt wird.",
                    seg(
                        "start-view",
                        &["Heute", "Updates", "Kalender", "Zuletzt"],
                        s.start_view as usize,
                        |s, i| {
                            s.start_view = [
                                StartView::Today,
                                StartView::Updates,
                                StartView::Calendar,
                                StartView::Last,
                            ][i]
                        },
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Bestätigungen")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.undo_instead_of_dialog != d.undo_instead_of_dialog,
                        reset(|s, d| s.undo_instead_of_dialog = d.undo_instead_of_dialog),
                    );
                sec.row(
                    "Löschen mit Rückgängig-Hinweis statt Dialog",
                    "Updates wandern sofort in den Papierkorb; „Rückgängig“ erscheint als Hinweis.",
                    sw(
                        "sw-undo",
                        s.undo_instead_of_dialog,
                        "Rückgängig statt Dialog",
                        |s| s.undo_instead_of_dialog = !s.undo_instead_of_dialog,
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Papierkorb")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.trash_days != d.trash_days,
                        reset(|s, d| s.trash_days = d.trash_days),
                    );
                sec.row(
                    "Aufbewahrungsdauer",
                    "Gelöschte Updates werden beim Start nach dieser Zeit endgültig entfernt.",
                    seg(
                        "trash-days",
                        &["7 Tage", "30 Tage", "90 Tage", "Nie"],
                        match s.trash_days {
                            7 => 0,
                            30 => 1,
                            90 => 2,
                            0 => 3,
                            _ => 1,
                        },
                        |s, i| s.trash_days = [7, 30, 90, 0][i],
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Fenster")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.sidebar_collapsed != d.sidebar_collapsed
                            || s.remember_window != d.remember_window,
                        reset(|s, d| {
                            s.sidebar_collapsed = d.sidebar_collapsed;
                            s.remember_window = d.remember_window;
                        }),
                    );
                sec.row(
                    "Seitenleiste eingeklappt",
                    "Zeigt nur Icons; Strg+B klappt sie wieder aus.",
                    sw(
                        "sw-sidebar",
                        s.sidebar_collapsed,
                        "Seitenleiste eingeklappt",
                        |s| s.sidebar_collapsed = !s.sidebar_collapsed,
                    ),
                );
                sec.row(
                    "Fensterposition und -größe merken",
                    "Die Fenstergröße wird beim nächsten Start wiederhergestellt.",
                    sw(
                        "sw-window",
                        s.remember_window,
                        "Fenstergröße merken",
                        |s| s.remember_window = !s.remember_window,
                    ),
                );
                out.push(sec.build());
            }

            SettingsCat::Appearance => {
                let mut sec = Section::new(&p, q, "Theme")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(s.theme != d.theme, reset(|s, d| s.theme = d.theme));
                if matches_query(q, "Theme", "System Dunkel Hell") {
                    let first = sec.rows.is_empty();
                    let cards = div()
                        .p(px(palette::SP_3))
                        .flex()
                        .gap(px(palette::SP_3))
                        .child(self.theme_card(
                            "theme-system",
                            "System",
                            model::ThemeChoice::System,
                            cx,
                        ))
                        .child(self.theme_card(
                            "theme-dark",
                            "Dunkel",
                            model::ThemeChoice::Dark,
                            cx,
                        ))
                        .child(self.theme_card(
                            "theme-light",
                            "Hell",
                            model::ThemeChoice::Light,
                            cx,
                        ));
                    let _ = first;
                    sec.rows.push(cards.into_any_element());
                }
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Akzentfarbe")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(s.accent != d.accent, reset(|s, d| s.accent = d.accent));
                let swatches =
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(px(palette::SP_1))
                        .children(Accent::ALL.map(|choice| {
                            let e = entity.clone();
                            // Preview color: as the accent is resolved in the active theme.
                            let color = Palette::build(p.dark, choice).accent;
                            swatch(
                                SharedString::from(format!("accent-{}", choice.label())),
                                &p,
                                color,
                                s.accent == choice,
                                choice.label(),
                                click_fn(move |_, cx| {
                                    e.update(cx, |a, cx| {
                                        a.update_settings(cx, |s| s.accent = choice)
                                    })
                                }),
                            )
                        }));
                sec.row(
                    "Akzentfarbe",
                    "Der Kontrasttext auf Akzentflächen wird automatisch so gewählt, dass er lesbar bleibt.",
                    swatches,
                );
                let hue = p.accent.h;
                let similar = [
                    ("Idee", p.idea),
                    ("Geplant", p.planned),
                    ("In Arbeit", p.progress),
                    ("Erledigt", p.done),
                ]
                .into_iter()
                .find(|(_, c)| {
                    let d = (hue - c.h).abs();
                    d.min(1. - d) < 0.07
                });
                if let Some((name, _)) = similar {
                    sec.row_text("Hinweis",format!("Ähnelt der Statusfarbe '{name}'. Status wird zusätzlich als Text angezeigt."),div());
                }
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Größe & Dichte")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.density != d.density || s.font_scale != d.font_scale,
                        reset(|s, d| {
                            s.density = d.density;
                            s.font_scale = d.font_scale;
                        }),
                    );
                sec.row(
                    "Dichte",
                    "Kompakt verkleinert Zeilenhöhen und Abstände.",
                    seg(
                        "density",
                        &["Komfortabel", "Kompakt"],
                        s.density as usize,
                        |s, i| s.density = [Density::Comfortable, Density::Compact][i],
                    ),
                );
                sec.row(
                    "Schriftgröße",
                    "Skaliert alle Texte: Klein 12, Standard 13, Groß 15 Pixel Basis.",
                    seg(
                        "font-scale",
                        &["Klein", "Standard", "Groß"],
                        s.font_scale as usize,
                        |s, i| {
                            s.font_scale =
                                [FontScale::Small, FontScale::Standard, FontScale::Large][i]
                        },
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Bewegung")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(s.motion != d.motion, reset(|s, d| s.motion = d.motion));
                sec.row(
                    "Animationen reduzieren",
                    "„An“ blendet den Splash ohne Übergang aus und verzichtet auf Bewegung.",
                    seg(
                        "motion",
                        &["System", "An", "Aus"],
                        s.motion as usize,
                        |s, i| s.motion = [Motion::System, Motion::Reduced, Motion::Full][i],
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Seitenleiste").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row(
                    "Inbox",
                    "Zeigt die Inbox in der Seitenleiste.",
                    sw("sw-nav-inbox", !s.hide_inbox, "Inbox", |s| {
                        s.hide_inbox = !s.hide_inbox
                    }),
                );
                sec.row(
                    "Board",
                    "Zeigt das Kanban-Board in der Seitenleiste.",
                    sw("sw-nav-board", !s.hide_board, "Board", |s| {
                        s.hide_board = !s.hide_board
                    }),
                );
                sec.row(
                    "Post-its",
                    "Zeigt die Post-its in der Seitenleiste. Im Notizzettel bleiben sie verfügbar.",
                    sw("sw-nav-stickies", !s.hide_stickies, "Post-its", |s| {
                        s.hide_stickies = !s.hide_stickies
                    }),
                );
                sec.row(
                    "Releases",
                    "Zeigt die Releases in der Seitenleiste.",
                    sw("sw-nav-releases", !s.hide_releases, "Releases", |s| {
                        s.hide_releases = !s.hide_releases
                    }),
                );
                sec.row(
                    "Kalender",
                    "Zeigt den Kalender in der Seitenleiste.",
                    sw("sw-nav-calendar", !s.hide_calendar, "Kalender", |s| {
                        s.hide_calendar = !s.hide_calendar
                    }),
                );
                out.push(sec.build());
            }

            SettingsCat::Editor => {
                let mut sec = Section::new(&p, q, "Schreiben")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.editor_mode != d.editor_mode
                            || s.editor_font != d.editor_font
                            || s.toolbar != d.toolbar
                            || s.word_count != d.word_count,
                        reset(|s, d| {
                            s.editor_mode = d.editor_mode;
                            s.editor_font = d.editor_font;
                            s.toolbar = d.toolbar;
                            s.word_count = d.word_count;
                        }),
                    );
                sec.row(
                    "Standardmodus",
                    "Ansicht beim Öffnen eines Updates.",
                    seg(
                        "editor-mode",
                        &["Schreiben", "Vorschau", "Geteilt"],
                        s.editor_mode as usize,
                        |s, i| {
                            s.editor_mode =
                                [EditorMode::Write, EditorMode::Preview, EditorMode::Split][i]
                        },
                    ),
                );
                sec.row(
                    "Schriftart im Editor",
                    "Monospace eignet sich für Tabellen und Code.",
                    seg(
                        "editor-font",
                        &["Proportional", "Monospace"],
                        s.editor_font as usize,
                        |s, i| s.editor_font = [EditorFont::Proportional, EditorFont::Mono][i],
                    ),
                );
                sec.row(
                    "Formatierungsleiste anzeigen",
                    "Fett, Kursiv, Listen und mehr über dem Editor.",
                    sw("sw-toolbar", s.toolbar, "Formatierungsleiste", |s| {
                        s.toolbar = !s.toolbar
                    }),
                );
                sec.row(
                    "Wortzähler anzeigen",
                    "",
                    sw("sw-words", s.word_count, "Wortzähler", |s| {
                        s.word_count = !s.word_count
                    }),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Speichern")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.autosave_ms != d.autosave_ms,
                        reset(|s, d| s.autosave_ms = d.autosave_ms),
                    );
                sec.row(
                    "Autosave-Verzögerung",
                    "Wie lange nach der letzten Eingabe gespeichert wird.",
                    seg(
                        "autosave",
                        &["Schnell", "Standard", "Langsam"],
                        match s.autosave_ms {
                            ..=299 => 0,
                            300..=699 => 1,
                            _ => 2,
                        },
                        |s, i| s.autosave_ms = [200, 400, 1000][i],
                    ),
                );
                out.push(sec.build());
            }

            SettingsCat::Calendar => {
                let mut sec = Section::new(&p, q, "Datum")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.week_start != d.week_start
                            || s.date_format != d.date_format
                            || s.relative_dates != d.relative_dates,
                        reset(|s, d| {
                            s.week_start = d.week_start;
                            s.date_format = d.date_format;
                            s.relative_dates = d.relative_dates;
                        }),
                    );
                sec.row(
                    "Wochenstart",
                    "",
                    seg(
                        "week-start",
                        &["Montag", "Sonntag"],
                        s.week_start as usize,
                        |s, i| s.week_start = [WeekStart::Monday, WeekStart::Sunday][i],
                    ),
                );
                let formats =
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(palette::SP_1))
                        .children(DateFormat::ALL.map(|format| {
                            let e = entity.clone();
                            let keyboard = e.clone();
                            let active = s.date_format == format;
                            kit::surface(
                                SharedString::from(format!("dfmt-{}", dates::full(today, format))),
                                &p,
                            )
                            .h(px(palette::CTL_SM))
                            .min_w(px(150.))
                            .px(px(palette::SP_3))
                            .flex()
                            .items_center()
                            .gap(px(palette::SP_2))
                            .rounded(px(palette::R_INPUT))
                            .when(!active, |d| d.border_1().border_color(p.border_subtle))
                            .cursor_pointer()
                            .text_size(palette::fs(palette::FS_BASE))
                            .text_color(p.text_primary)
                            .tab_stop(true)
                            .when(active, |d| {
                                d.bg(p.bg_active).border_l_2().border_color(p.accent)
                            })
                            .when(!active, |d| d)
                            .on_key_down(move |ev, _, cx| {
                                if matches!(ev.keystroke.key.as_str(), "enter" | "space") {
                                    keyboard.update(cx, |a, cx| {
                                        a.update_settings(cx, |s| s.date_format = format)
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .child(dates::full(today, format))
                            .on_click(move |_, _, cx| {
                                e.update(cx, |a, cx| {
                                    a.update_settings(cx, |s| s.date_format = format)
                                })
                            })
                        }));
                sec.row(
                    "Datumsformat",
                    "Die Vorschau zeigt das heutige Datum im jeweiligen Format.",
                    formats,
                );
                sec.row(
                    "Relative Datumsangaben",
                    "„Morgen“ und „In 3 Tagen“ statt des Datums.",
                    sw(
                        "sw-relative",
                        s.relative_dates,
                        "Relative Datumsangaben",
                        |s| s.relative_dates = !s.relative_dates,
                    ),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Kalenderansicht")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.cal_view != d.cal_view
                            || s.highlight_weekends != d.highlight_weekends
                            || s.week_numbers != d.week_numbers
                            || s.show_done_in_calendar != d.show_done_in_calendar
                            || s.max_pills != d.max_pills,
                        reset(|s, d| {
                            s.cal_view = d.cal_view;
                            s.highlight_weekends = d.highlight_weekends;
                            s.week_numbers = d.week_numbers;
                            s.show_done_in_calendar = d.show_done_in_calendar;
                            s.max_pills = d.max_pills;
                        }),
                    );
                sec.row(
                    "Standard-Kalenderansicht",
                    "",
                    seg(
                        "cal-view",
                        &["Monat", "Woche"],
                        s.cal_view as usize,
                        |s, i| s.cal_view = [CalView::Month, CalView::Week][i],
                    ),
                );
                sec.row(
                    "Wochenenden hervorheben",
                    "Samstag und Sonntag mit leicht abgesetzter Fläche.",
                    sw(
                        "sw-weekend",
                        s.highlight_weekends,
                        "Wochenenden hervorheben",
                        |s| s.highlight_weekends = !s.highlight_weekends,
                    ),
                );
                sec.row(
                    "Kalenderwochen anzeigen",
                    "Nummer der Kalenderwoche links neben jeder Zeile.",
                    sw("sw-weeknum", s.week_numbers, "Kalenderwochen", |s| {
                        s.week_numbers = !s.week_numbers
                    }),
                );
                sec.row(
                    "Erledigte Einträge im Kalender anzeigen",
                    "",
                    sw(
                        "sw-done-cal",
                        s.show_done_in_calendar,
                        "Erledigte anzeigen",
                        |s| s.show_done_in_calendar = !s.show_done_in_calendar,
                    ),
                );
                sec.row(
                    "Einträge pro Tag",
                    "Maximale Anzahl sichtbarer Einträge pro Tag in der Monatsansicht.",
                    {
                        let e = entity.clone();
                        let current = s.max_pills;
                        kit::button("max-pills-select")
                            .outline()
                            .label(format!("{current} Einträge"))
                            .min_w(px(200.))
                            .dropdown_menu(move |menu, _, _| {
                                [2, 3, 4].into_iter().fold(menu, |menu, count| {
                                    let e = e.clone();
                                    menu.item(
                                        PopupMenuItem::new(format!("{count} Einträge"))
                                            .checked(current == count)
                                            .on_click(move |_, _, cx| {
                                                e.update(cx, |a, cx| {
                                                    a.update_settings(cx, |s| s.max_pills = count)
                                                })
                                            }),
                                    )
                                })
                            })
                    },
                );
                out.push(sec.build());
            }

            SettingsCat::Shortcuts => {
                let query = if q.trim().is_empty() {
                    self.shortcut_search.read(cx).value().to_string()
                } else {
                    q.to_owned()
                };
                if q.trim().is_empty() {
                    out.push(Some(
                        div()
                            .child(
                                kit::input(&self.shortcut_search)
                                    .aria_label("Kürzel suchen")
                                    .w_full(),
                            )
                            .into_any_element(),
                    ));
                }
                for (group, rows) in SHORTCUTS {
                    let mut sec = Section::new(&p, &query, group);
                    for (keys, what) in rows.iter() {
                        let e = keys.to_string();
                        if !matches_query(&query, what, keys) {
                            continue;
                        }
                        let caps = div().flex().items_center().gap(px(palette::SP_2)).children(
                            e.split(" / ")
                                .map(|alt| keycaps(&p, alt).into_any_element()),
                        );
                        let first = sec.rows.is_empty();
                        sec.rows.push(
                            settings_row(&p, first, highlighted(what, &query, &p), None, caps)
                                .into_any_element(),
                        );
                    }
                    out.push(sec.build());
                }
                if q.trim().is_empty() {
                    out.push(Some(
                        text_secondary(
                            &p,
                            "Anpassen ist in dieser Version nicht möglich; die Kürzel sind fest.",
                        )
                        .into_any_element(),
                    ));
                }
            }

            SettingsCat::Data => {
                let size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
                let live = self
                    .workspace
                    .updates
                    .iter()
                    .filter(|u| u.deleted_on.is_none());
                let (n_updates, n_tasks) =
                    live.fold((0usize, 0usize), |(u, t), up| (u + 1, t + up.tasks.len()));
                let dir = storage::data_dir(&self.path);
                let path_text = dir.display().to_string();

                let mut sec = Section::new(&p, q, "Speicherort").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row_text(
                    if crate::storage::is_portable() {
                        "Datenordner (portabel)"
                    } else {
                        "Datenordner"
                    },
                    ellipsize_middle(&path_text, 52),
                    div()
                        .id("data-path")
                        .tooltip(tip(path_text.clone()))
                        .flex()
                        .gap(px(palette::SP_2))
                        .child({
                            let e = entity.clone();
                            let text = path_text.clone();
                            kit::button("copy-path")
                                .outline()
                                .with_size(gpui_kit::component::Size::Small)
                                .h(px(palette::CTL_SM))
                                .icon(kit::icon(icons::COPY, palette::ICON_MD))
                                .label("Kopieren")
                                .on_click(move |_, _, cx| {
                                    let text = text.clone();
                                    e.update(cx, |a, cx| a.copy_text(text, "Pfad kopiert", cx))
                                })
                        })
                        .child(action("reveal-path", "Ordner öffnen", |a, _, cx| {
                            a.open_data_dir(cx)
                        })),
                );
                sec.row_text(
                    "Umfang",
                    format!(
                        "{} · {} · {}",
                        fmt_size(size),
                        model::plural(n_updates, "Update", "Updates"),
                        model::plural(n_tasks, "Aufgabe", "Aufgaben")
                    ),
                    div(),
                );
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Backups").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row(
                    "Jetzt sichern",
                    "Manuelle Backups werden nie automatisch gelöscht. Automatisch gesichert wird beim Start und alle 30 Minuten bei Änderungen.",
                    action("backup-now", "Jetzt sichern", |a, _, cx| {
                        if a.backup_now(true) {
                            a.toast("Backup angelegt", None, cx);
                        }
                        cx.notify();
                    }),
                );
                if matches_query(q, "Backups", "Wiederherstellen Sicherung") {
                    if self.backups.is_empty() {
                        sec.rows.push(
                            settings_row(
                                &p,
                                sec.rows.is_empty(),
                                "Noch keine Backups",
                                None,
                                div(),
                            )
                            .into_any_element(),
                        );
                    }
                    for (info, count) in self.backups.iter().take(10) {
                        let at = info.at;
                        let when = dates::full_time(at).unwrap_or_default();
                        let relative = dates::elapsed_at(at, model::now_millis());
                        let (restore, reveal, remove) =
                            (entity.clone(), entity.clone(), entity.clone());
                        let path = info.path.clone();
                        let pinned = info.pinned;
                        sec.rows.push(
                            settings_row(
                                &p,
                                sec.rows.is_empty(),
                                div()
                                    .id(("backup-title", at as u64))
                                    .tooltip(tip(when.clone()))
                                    .flex()
                                    .items_center()
                                    .gap(px(palette::SP_2))
                                    .child(relative)
                                    .when(pinned, |d| {
                                        d.child(
                                            div()
                                                .px(px(palette::SP_2))
                                                .rounded(px(palette::R_XS))
                                                .bg(p.bg_hover)
                                                .text_size(palette::fs(palette::FS_SM))
                                                .text_color(p.text_secondary)
                                                .child("Manuell"),
                                        )
                                    }),
                                Some(
                                    format!(
                                        "{when} · {} · {}",
                                        model::plural(*count, "Update", "Updates"),
                                        fmt_size(info.size)
                                    )
                                    .into(),
                                ),
                                div()
                                    .flex()
                                    .gap(px(palette::SP_2))
                                    .child(
                                        kit::button(("backup-restore", at as u64))
                                            .outline()
                                            .with_size(gpui_kit::component::Size::Small)
                                            .h(px(palette::CTL_SM))
                                            .label("Wiederherstellen")
                                            .on_click(move |_, window, cx| {
                                                restore.update(cx, |a, cx| {
                                                    a.open_dialog(
                                                        Dialog::ConfirmRestore(at),
                                                        window,
                                                        cx,
                                                    )
                                                })
                                            }),
                                    )
                                    .child(
                                        kit::button(("backup-reveal", at as u64))
                                            .ghost()
                                            .with_size(gpui_kit::component::Size::Small)
                                            .h(px(palette::CTL_SM))
                                            .label("Im Ordner zeigen")
                                            .on_click(move |_, _, cx| {
                                                let path = path.clone();
                                                reveal.update(cx, |a, cx| {
                                                    if !crate::system::reveal(&path) {
                                                        a.toast(
                                                            "Ordner konnte nicht geöffnet werden",
                                                            None,
                                                            cx,
                                                        );
                                                    }
                                                })
                                            }),
                                    )
                                    .when(pinned, |d| {
                                        d.child(
                                            kit::button(("backup-delete", at as u64))
                                                .ghost()
                                                .with_size(gpui_kit::component::Size::Small)
                                                .h(px(palette::CTL_SM))
                                                .label("Löschen")
                                                .on_click(move |_, _, cx| {
                                                    remove
                                                        .update(cx, |a, cx| a.delete_backup(at, cx))
                                                }),
                                        )
                                    }),
                            )
                            .into_any_element(),
                        );
                    }
                }
                out.push(sec.build());

                let mut sec = Section::new(&p, q, "Export und Import").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row(
                    "Alles exportieren (JSON)",
                    "Alle Updates, Aufgaben und Einstellungen in einer Datei mit Datum im Namen.",
                    action("backup-export", "Exportieren …", |a, _, cx| {
                        a.export_backup(cx)
                    }),
                );
                sec.row(
                    "Notizen als Markdown exportieren",
                    "Ein Ordner mit je einer .md-Datei pro Update.",
                    action("md-export", "Exportieren …", |a, _, cx| {
                        a.export_all_markdown(cx)
                    }),
                );
                sec.row(
                    "Importieren",
                    "Zeigt zuerst eine Vorschau; Zusammenführen oder Ersetzen. Vorher wird automatisch gesichert.",
                    action("backup-import", "Importieren …", |a, _, cx| a.pick_import(cx)),
                );
                out.push(sec.build());

                let trashed = self
                    .workspace
                    .updates
                    .iter()
                    .filter(|u| u.deleted_on.is_some())
                    .count();
                let mut sec = Section::new(&p, q, "Gefahrenzone")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .danger();
                let danger_btn = |id: &'static str, label: &'static str, dialog: Dialog| {
                    let e = entity.clone();
                    kit::button(id)
                        .danger()
                        .with_size(gpui_kit::component::Size::Small)
                        .h(px(palette::CTL_SM))
                        .label(label)
                        .on_click(move |_, window, cx| {
                            let dialog = dialog.clone();
                            e.update(cx, |a, cx| a.open_dialog(dialog, window, cx))
                        })
                };
                sec.row_text(
                    "Papierkorb jetzt leeren",
                    format!(
                        "{} werden endgültig gelöscht.",
                        model::plural(trashed, "Update", "Updates")
                    ),
                    danger_btn("empty-trash", "Leeren …", Dialog::ConfirmEmptyTrash),
                );
                sec.row(
                    "Alle Daten löschen",
                    "Entfernt alle Updates. Vorher wird automatisch eine Kopie angelegt.",
                    danger_btn("wipe-all", "Alles löschen …", Dialog::ConfirmWipe),
                );
                out.push(sec.build());
            }

            SettingsCat::Templates => {
                let mut sec = Section::new(&p, q, "Vorlagen").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                for (i, t) in s.templates.iter().enumerate() {
                    let is_default = s.default_template == Some(i);
                    let first_line = t
                        .notes
                        .lines()
                        .map(|l| l.trim_start_matches('#').trim())
                        .find(|l| !l.is_empty())
                        .unwrap_or("Ohne Notiz");
                    let desc = format!(
                        "{} · {}",
                        model::plural(t.tasks.len(), "Aufgabe", "Aufgaben"),
                        first_line
                    );
                    if !matches_query(q, &t.name, &desc) {
                        continue;
                    }
                    let e1 = entity.clone();
                    let e2 = entity.clone();
                    let e3 = entity.clone();
                    let e4 = entity.clone();
                    let controls = div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(
                            kit::button(("tpl-default", i))
                                .ghost()
                                .with_size(gpui_kit::component::Size::Small)
                                .h(px(palette::CTL_SM))
                                .label(if is_default {
                                    "Standard"
                                } else {
                                    "Als Standard"
                                })
                                .when(is_default, |b| b.text_color(p.accent))
                                .when(!is_default, |b| b.text_color(p.text_secondary))
                                .on_click(move |_, _, cx| {
                                    e4.update(cx, |a, cx| {
                                        a.update_settings(cx, |s| {
                                            s.default_template =
                                                if is_default { None } else { Some(i) }
                                        })
                                    })
                                }),
                        )
                        .child(
                            icon_btn(("tpl-edit", i), icons::EDIT, "Bearbeiten").on_click(
                                move |_, window, cx| {
                                    e1.update(cx, |a, cx| {
                                        a.open_template_editor(Some(i), window, cx)
                                    })
                                },
                            ),
                        )
                        .child(
                            icon_btn(("tpl-dup", i), icons::COPY, "Duplizieren").on_click(
                                move |_, _, cx| {
                                    e2.update(cx, |a, cx| {
                                        a.update_settings(cx, |s| {
                                            if let Some(t) = s.templates.get(i).cloned() {
                                                s.templates.push(model::Template {
                                                    name: format!("{} (Kopie)", t.name),
                                                    ..t
                                                });
                                            }
                                        })
                                    })
                                },
                            ),
                        )
                        .child(icon_btn(("tpl-del", i), icons::DELETE, "Löschen").on_click(
                            move |_, _, cx| {
                                e3.update(cx, |a, cx| {
                                    a.update_settings(cx, |s| {
                                        if i < s.templates.len() {
                                            s.templates.remove(i);
                                            s.default_template = match s.default_template {
                                                Some(x) if x == i => None,
                                                Some(x) if x > i => Some(x - 1),
                                                other => other,
                                            };
                                        }
                                    })
                                })
                            },
                        ));
                    let first = sec.rows.is_empty();
                    sec.rows.push(
                        settings_row(
                            &p,
                            first,
                            highlighted(&t.name, q, &p),
                            Some(desc.into()),
                            controls,
                        )
                        .into_any_element(),
                    );
                }
                let actions = div()
                    .px(px(palette::SP_4))
                    .py(px(palette::SP_3))
                    .flex()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::button("tpl-new")
                            .primary()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .icon(kit::icon(icons::ADD, palette::ICON_MD))
                            .label("Neue Vorlage")
                            .on_click({
                                let e = entity.clone();
                                move |_, window, cx| {
                                    e.update(cx, |a, cx| a.open_template_editor(None, window, cx))
                                }
                            }),
                    )
                    .child(
                        kit::button("tpl-from-update")
                            .outline()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .label("Aus aktuellem Update")
                            .tooltip("Notizen und Aufgaben des gewählten Updates übernehmen")
                            .on_click({
                                let e = entity.clone();
                                move |_, _, cx| e.update(cx, |a, cx| a.template_from_selected(cx))
                            }),
                    );
                if q.trim().is_empty() {
                    if sec.rows.is_empty() {
                        sec.rows.push(
                            div()
                                .p(px(palette::SP_4))
                                .child(text_secondary(&p, "Noch keine Vorlagen."))
                                .into_any_element(),
                        );
                    }
                    sec.rows.push(
                        actions
                            .border_t_1()
                            .border_color(p.border_subtle)
                            .into_any_element(),
                    );
                }
                out.push(sec.build());
            }

            SettingsCat::Notifications => {
                let sec_new = |title: &'static str| {
                    Section::new(&p, q, title).navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                };
                // Uhrzeit in 30-Minuten-Schritten: [−] hh:mm [+]
                let stepper = |id: &'static str, minutes: u32, apply: fn(&mut Settings, i32)| {
                    let (down, up) = (entity.clone(), entity.clone());
                    div()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_1))
                        .child(
                            icon_btn((id, 0usize), IconName::Minus, "30 Minuten früher").on_click(
                                move |_, _, cx| {
                                    down.update(cx, |a, cx| {
                                        a.update_settings(cx, |s| apply(s, -30))
                                    })
                                },
                            ),
                        )
                        .child(
                            div()
                                .min_w(px(48.))
                                .flex()
                                .justify_center()
                                .text_size(palette::fs(palette::FS_MD))
                                .child(format!("{:02}:{:02}", minutes / 60, minutes % 60)),
                        )
                        .child(
                            icon_btn((id, 1usize), icons::ADD, "30 Minuten später").on_click(
                                move |_, _, cx| {
                                    up.update(cx, |a, cx| a.update_settings(cx, |s| apply(s, 30)))
                                },
                            ),
                        )
                };
                fn shift(minutes: u32, delta: i32) -> u32 {
                    (minutes as i32 + delta).rem_euclid(24 * 60) as u32
                }
                let mut sec = sec_new("Erinnerungen").reset(
                    s.reminders_on != d.reminders_on
                        || s.daily_on != d.daily_on
                        || s.daily_time != d.daily_time
                        || s.quiet_on != d.quiet_on
                        || s.quiet_from != d.quiet_from
                        || s.quiet_to != d.quiet_to,
                    reset(|s, d| {
                        s.reminders_on = d.reminders_on;
                        s.daily_on = d.daily_on;
                        s.daily_time = d.daily_time;
                        s.quiet_on = d.quiet_on;
                        s.quiet_from = d.quiet_from;
                        s.quiet_to = d.quiet_to;
                    }),
                );
                sec.row(
                    "Erinnerungen aktivieren",
                    "Meldet gesetzte Erinnerungen zur gewählten Zeit.",
                    sw(
                        "sw-reminders",
                        s.reminders_on,
                        "Erinnerungen aktivieren",
                        |s| s.reminders_on = !s.reminders_on,
                    ),
                );
                sec.row(
                    "Tägliche Übersicht",
                    "Eine Meldung wie „Heute fällig: 3 · Überfällig: 2“, nur wenn etwas zutrifft.",
                    div()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_3))
                        .when(s.daily_on, |d| {
                            d.child(stepper("daily-time", s.daily_time, |s, delta| {
                                s.daily_time = shift(s.daily_time, delta)
                            }))
                        })
                        .child(sw("sw-daily", s.daily_on, "Tägliche Übersicht", |s| {
                            s.daily_on = !s.daily_on
                        })),
                );
                sec.row(
                    "Ruhige Zeiten",
                    "In dieser Zeit kommt nichts; danach folgt eine gesammelte Meldung.",
                    div()
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_3))
                        .when(s.quiet_on, |d| {
                            d.child(stepper("quiet-from", s.quiet_from, |s, delta| {
                                s.quiet_from = shift(s.quiet_from, delta)
                            }))
                            .child(text_secondary(&p, "bis"))
                            .child(stepper(
                                "quiet-to",
                                s.quiet_to,
                                |s, delta| s.quiet_to = shift(s.quiet_to, delta),
                            ))
                        })
                        .child(sw("sw-quiet", s.quiet_on, "Ruhige Zeiten", |s| {
                            s.quiet_on = !s.quiet_on
                        })),
                );
                out.push(sec.build());
                let mut sec = sec_new("Test");
                sec.row(
                    "Testbenachrichtigung senden",
                    "Erinnerungen kommen nur an, solange Note Warden läuft (siehe System).",
                    action("notice-test", "Senden", |a, _, cx| {
                        a.notice_failed = !crate::system::notify(
                            "Note Warden",
                            "So melden sich Erinnerungen.",
                            None,
                        );
                        cx.notify();
                    }),
                );
                if self.notice_failed {
                    sec.row("Benachrichtigungen blockiert", "Windows hat die Meldung nicht angezeigt. Erlaube Benachrichtigungen für Note Warden unter Windows-Einstellungen > System > Benachrichtigungen.", div());
                }
                out.push(sec.build());
            }
            SettingsCat::System => {
                let mut sec = Section::new(&p, q, "Windows")
                    .navigation(
                        entity.clone(),
                        cat,
                        self.settings_jump.as_deref(),
                        &self.settings_target_focus,
                    )
                    .reset(
                        s.autostart != d.autostart
                            || s.tray_on_close != d.tray_on_close
                            || s.hotkey != d.hotkey,
                        {
                            let e = entity.clone();
                            click_fn(move |_, cx| e.update(cx, |a, cx| a.apply_system_defaults(cx)))
                        },
                    );
                sec.row(
                    "Beim Windows-Start starten",
                    "Startet minimiert im Tray; das Fenster öffnet sich nicht von selbst.",
                    {
                        let e = entity.clone();
                        switch(
                            "sw-autostart",
                            &p,
                            s.autostart,
                            "Beim Windows-Start starten",
                            click_fn(move |_, cx| e.update(cx, |a, cx| a.toggle_autostart(cx))),
                        )
                    },
                );
                sec.row("Beim Schließen im Tray weiterlaufen", "Nötig, damit Erinnerungen ankommen. „Beenden“ im Tray-Menü beendet Note Warden wirklich.", sw("sw-tray", s.tray_on_close, "Beim Schließen im Tray weiterlaufen", |s| s.tray_on_close = !s.tray_on_close));
                let capture = self.hotkey_capture;
                let shown = if capture {
                    "Taste drücken …".to_owned()
                } else {
                    s.hotkey.clone()
                };
                let hotkey_error = self.hotkey_error;
                let (start, reset_key) = (entity.clone(), entity.clone());
                let key_entity = entity.clone();
                sec.row_text("Hotkey für Quick-Capture", hotkey_error.unwrap_or("Öffnet Note Warden mit dem Eingabefeld, auch aus anderen Programmen.").to_owned(), div().flex().items_center().gap(px(palette::SP_2))
                    .child(
                        div().id("hotkey-field").track_focus(&self.hotkey_focus)
                            .on_key_down(move |event, window, cx| key_entity.update(cx, |a, cx| a.capture_hotkey(event, window, cx)))
                            .child(kit::button("hotkey-capture").outline().with_size(gpui_kit::component::Size::Small).h(px(palette::CTL_SM)).label(shown)
                                .accessibility_label("Hotkey ändern: Klicken, dann Tastenkombination drücken")
                                .on_click(move |_, window, cx| start.update(cx, |a, cx| {
                                    a.hotkey_capture = true;
                                    a.hotkey_error = None;
                                    a.hotkey_focus.focus(window, cx);
                                    cx.notify();
                                }))),
                    )
                    .child(kit::button("hotkey-reset").ghost().with_size(gpui_kit::component::Size::Small).h(px(palette::CTL_SM)).label("Zurücksetzen")
                        .on_click(move |_, _, cx| reset_key.update(cx, |a, cx| a.apply_hotkey("Strg+Alt+N", cx)))));
                out.push(sec.build());
            }
            SettingsCat::About => {
                let e1 = entity.clone();
                let e2 = entity.clone();
                out.push(Some(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(palette::SP_2))
                        .py(px(palette::SP_6))
                        .child(crate::logo::shield(crate::logo::LogoStyle::new(4.)))
                        .child(
                            div()
                                .mt(px(palette::SP_2))
                                .text_size(palette::fs(palette::FS_XL))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Note Warden"),
                        )
                        .child(text_secondary(&p, format!("Version {VERSION}")))
                        .child(text_secondary(
                            &p,
                            format!(
                                "{} · Schema {}",
                                if cfg!(debug_assertions) {
                                    "Debug-Build"
                                } else {
                                    "Release-Build"
                                },
                                model::SCHEMA_VERSION
                            ),
                        ))
                        .child(
                            div()
                                .mt(px(palette::SP_3))
                                .flex()
                                .gap(px(palette::SP_2))
                                .child(
                                    kit::button("show-changelog")
                                        .outline()
                                        .label("Änderungsprotokoll anzeigen")
                                        .on_click(move |_, window, cx| {
                                            e1.update(cx, |a, cx| {
                                                a.open_dialog(Dialog::AppChangelog, window, cx)
                                            })
                                        }),
                                )
                                .child(
                                    kit::button("show-licenses")
                                        .outline()
                                        .label("Drittbibliotheken")
                                        .on_click(move |_, window, cx| {
                                            e2.update(cx, |a, cx| {
                                                a.open_dialog(Dialog::Licenses, window, cx)
                                            })
                                        }),
                                ),
                        )
                        .child(div().mt(px(palette::SP_3)).child(text_secondary(
                            &p,
                            "Note Warden sendet keine Nutzungsdaten. Alle Daten bleiben lokal auf diesem Gerät.",
                        )))
                        .into_any_element(),
                ));
                let status = self
                    .update_state
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let mut sec = Section::new(&p, q, "Updates").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                if crate::updater::configured() {
                    let text = match &status {
                        crate::updater::State::Checking => "Suche läuft …".to_owned(),
                        crate::updater::State::UpToDate => "Note Warden ist aktuell.".to_owned(),
                        crate::updater::State::Failed => {
                            "Update-Prüfung gerade nicht möglich.".to_owned()
                        }
                        crate::updater::State::Available(i)
                        | crate::updater::State::Downloading(i, _)
                        | crate::updater::State::Ready(i, _) => {
                            format!("Version {} ist verfügbar.", i.version)
                        }
                        crate::updater::State::Idle => format!("Installiert: Version {VERSION}."),
                    };
                    sec.row_text(
                        "Nach Updates suchen",
                        text,
                        action("check-updates", "Suchen", |a, _, cx| {
                            a.check_updates(true, cx)
                        }),
                    );
                } else {
                    sec.row(
                        "Automatische Updates",
                        "Automatische Updates sind nicht eingerichtet.",
                        div(),
                    );
                }
                out.push(sec.build());
                let mut sec = Section::new(&p, q, "Dateien").navigation(
                    entity.clone(),
                    cat,
                    self.settings_jump.as_deref(),
                    &self.settings_target_focus,
                );
                sec.row(
                    "Datenordner",
                    "Daten, Backups und Versionsverlauf.",
                    action("about-data-dir", "Öffnen", |a, _, cx| a.open_data_dir(cx)),
                );
                sec.row(
                    "Logdatei",
                    "Nur Ereignisse und Fehler, keine Inhalte.",
                    action("about-log", "Öffnen", |a, _, cx| a.open_log(cx)),
                );
                out.push(sec.build());
            }
        }
        out.into_iter().flatten().collect()
    }

    fn settings_nav_item(&self, cat: SettingsCat, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let active = self.settings_cat == cat;
        let entity = cx.entity();
        let icon = match cat {
            SettingsCat::General => icons::FILTER,
            SettingsCat::Appearance => IconName::Palette,
            SettingsCat::Home => icons::TODAY,
            SettingsCat::Editor => icons::EDIT,
            SettingsCat::Calendar => icons::DATE,
            SettingsCat::Shortcuts => IconName::Keyboard,
            SettingsCat::Data => IconName::Database,
            SettingsCat::Notifications => icons::BELL,
            SettingsCat::System => IconName::Monitor,
            SettingsCat::Templates => IconName::LayoutDashboard,
            SettingsCat::Projects => icons::PROJECT,
            SettingsCat::About => IconName::Info,
        };
        kit::surface(SharedString::from(format!("set-cat-{}", cat.title())), &p)
            .relative()
            .min_h(palette::fs(palette::CTL_MD))
            .flex_none()
            .px(px(palette::SP_3))
            .flex()
            .items_center()
            .justify_start()
            .gap(px(palette::SP_3))
            .rounded(px(palette::R_INPUT))
            .cursor_pointer()
            .text_size(palette::fs(palette::FS_BASE))
            .text_color(if active {
                p.text_primary
            } else {
                p.text_secondary
            })
            .tab_stop(true)
            .border_l_2()
            .border_color(if active {
                p.accent
            } else {
                gpui_kit::transparent_black()
            })
            .when(active, |d| d.bg(p.bg_active))
            .child(ico(icon, p.text_primary))
            .child(cat.title())
            .on_click(move |_, window, cx| {
                entity.update(cx, |a, cx| {
                    a.settings_cat = cat;
                    a.settings_jump = None;
                    a.settings_scroll.set_offset(point(px(0.), px(0.)));
                    a.workspace.settings.last_settings_category =
                        SettingsCat::ALL.iter().position(|c| *c == cat).unwrap_or(0);
                    a.persist(cx);
                    a.settings_search
                        .update(cx, |i, cx| i.set_value("", window, cx));
                    cx.notify();
                })
            })
    }

    pub(super) fn settings_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let q = self.settings_search.read(cx).value().to_string();
        let searching = !q.trim().is_empty();
        let cats: Vec<SettingsCat> = if searching {
            SettingsCat::ALL.to_vec()
        } else {
            vec![self.settings_cat]
        };
        let mut body: Vec<AnyElement> = Vec::new();
        for cat in cats {
            let sections = self.category_sections(cat, &q, cx);
            if sections.is_empty() {
                continue;
            }
            if searching {
                body.push(
                    div()
                        .pt(px(palette::SP_2))
                        .text_size(palette::fs(palette::FS_LG))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(cat.title())
                        .into_any_element(),
                );
            }
            body.extend(sections);
        }
        if searching && body.is_empty() {
            body.push(
                empty_state(
                    &p,
                    "Keine Einstellung gefunden",
                    "Versuche einen anderen Suchbegriff.",
                    None,
                )
                .into_any_element(),
            );
        }
        let saved = self
            .settings_saved_at
            .is_some_and(|t| t.elapsed() < StdDuration::from_millis(1700));
        let (title, blurb) = if searching {
            ("Suchergebnisse", "Treffer aus allen Kategorien.")
        } else {
            (self.settings_cat.title(), self.settings_cat.blurb())
        };

        kit::surface("settings-backdrop", &p)
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(p.bg_app)
            .on_click(cx.listener(|this, _, window, cx| this.close_overlay(window, cx)))
            .child(
                div()
                    .id("settings-panel")
                    .role(Role::Group)
                    .track_focus(&self.overlay_focus)
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .size_full()
                    .max_w_full()
                    .max_h_full()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .overflow_hidden()
                    .bg(p.bg_app)
                    .border_1()
                    .border_color(p.border_strong)
                    .child(
                        div()
                            .w(px(220.))
                            .h_full()
                            .min_h_0()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _, cx| {
                                let k = &ev.keystroke;
                                if k.modifiers.control || k.modifiers.alt || k.modifiers.shift {
                                    return;
                                }
                                let step = match k.key.as_str() {
                                    "down" => 1isize,
                                    "up" => -1,
                                    _ => return,
                                };
                                let all = SettingsCat::ALL;
                                let i = all
                                    .iter()
                                    .position(|c| *c == this.settings_cat)
                                    .unwrap_or(0);
                                let next =
                                    (i as isize + step).rem_euclid(all.len() as isize) as usize;
                                this.settings_cat = all[next];
                                this.settings_jump = None;
                                this.workspace.settings.last_settings_category = next;
                                this.settings_scroll.set_offset(point(px(0.), px(0.)));
                                this.persist(cx);
                                cx.stop_propagation();
                                cx.notify();
                            }))
                            .gap(px(palette::SP_1))
                            .p(px(palette::SP_3))
                            .bg(p.bg_sidebar)
                            .border_r_1()
                            .border_color(p.border_subtle)
                            .child(
                                kit::button("settings-back")
                                    .ghost()
                                    .self_start()
                                    .icon(kit::icon(icons::BACK, palette::ICON_MD))
                                    .label("Zurück")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_overlay(window, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_XL))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Einstellungen"),
                            )
                            .child(
                                div().pb(px(palette::SP_2)).child(
                                    kit::input(&self.settings_search)
                                        .aria_label("Einstellungen durchsuchen")
                                        .w_full(),
                                ),
                            )
                            .child(
                                div()
                                    .id("settings-nav-scroll")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scrollbar()
                                    .pr(px(palette::SCROLL_GUTTER))
                                    .flex()
                                    .flex_col()
                                    .gap(px(palette::SP_1))
                                    .children(SettingsCat::ALL.map(|cat| {
                                        self.settings_nav_item(cat, cx).into_any_element()
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex_none()
                                    .px(px(palette::SP_8))
                                    .pt(px(if self.settings_scroll.offset().y < px(-4.) {
                                        8.
                                    } else {
                                        palette::SP_6
                                    }))
                                    .pb(px(palette::SP_3))
                                    .when(self.settings_scroll.offset().y < px(-4.), |d| {
                                        d.shadow_sm()
                                    })
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .border_b_1()
                                    .border_color(p.border_subtle)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.))
                                            .child(
                                                div()
                                                    .text_size(palette::fs(palette::FS_XL))
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .child(title),
                                            )
                                            .when(
                                                self.settings_scroll.offset().y >= px(-4.),
                                                |d| d.child(text_secondary(&p, blurb)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(palette::SP_2))
                                            .when(saved, |d| {
                                                d.child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap(px(4.))
                                                        .text_size(palette::fs(palette::FS_SM))
                                                        .text_color(p.done)
                                                        .child(ico(icons::CONFIRM, p.done))
                                                        .child("Gespeichert"),
                                                )
                                            })
                                            .child(
                                                icon_btn(
                                                    "settings-close",
                                                    icons::CLOSE,
                                                    "Schließen · Esc",
                                                )
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.close_overlay(window, cx)
                                                })),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .id(("settings-scroll", self.settings_cat as usize))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .on_scroll(cx.listener(|_, _, _, cx| cx.notify()))
                                    .track_scroll(&self.settings_scroll)
                                    .vertical_scrollbar(&self.settings_scroll)
                                    .relative()
                                    .child({
                                        let e = cx.entity();
                                        canvas(
                                            |bounds, _, _| bounds,
                                            move |bounds, _, _, cx| {
                                                e.update(cx, |a, _| {
                                                    a.settings_bounds = Some(bounds)
                                                })
                                            },
                                        )
                                        .absolute()
                                        .inset_0()
                                    })
                                    .flex()
                                    .items_start()
                                    .justify_center()
                                    .child(
                                        div()
                                            .w_full()
                                            .flex_none()
                                            .max_w(px(720.))
                                            .px(px(palette::SP_8))
                                            .py(px(palette::SP_6))
                                            .flex()
                                            .flex_col()
                                            .gap(px(palette::SP_8))
                                            .children(body),
                                    ),
                            ),
                    ),
            )
    }

    pub(super) fn template_dialog_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let e = cx.entity();
        let label = |t: &'static str| {
            div()
                .text_size(palette::fs(palette::FS_SM))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_secondary)
                .child(t)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(label("Name"))
            .child(
                kit::input(&self.template_name_input)
                    .aria_label("Name der Vorlage")
                    .w_full(),
            )
            .child(label("Notiz-Text (Markdown)"))
            .child(
                gpui_kit::component::input::Textarea::new(&self.tpl_notes_input)
                    .aria_label("Notiz-Text der Vorlage")
                    .w_full(),
            )
            .child(label("Vorbefüllte Aufgaben (eine pro Zeile)"))
            .child(
                gpui_kit::component::input::Textarea::new(&self.tpl_tasks_input)
                    .aria_label("Aufgaben der Vorlage")
                    .w_full(),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::button("tpl-cancel")
                            .outline()
                            .label("Abbrechen")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.tpl_edit = None;
                                this.dialog = None;
                                this.overlay_focus.focus(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        kit::button("tpl-save")
                            .primary()
                            .label("Speichern")
                            .on_click(move |_, window, cx| {
                                e.update(cx, |a, cx| a.save_template(window, cx))
                            }),
                    ),
            )
    }

    pub(super) fn app_changelog_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        div().flex().flex_col().gap(px(2.)).children(
            include_str!("../../CHANGELOG.md").lines().map(|line| {
                if let Some(h) = line.strip_prefix("## ") {
                    div()
                        .pt(px(palette::SP_3))
                        .text_size(palette::fs(palette::FS_LG))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(h.to_owned())
                } else if let Some(h) = line.strip_prefix("### ") {
                    div()
                        .pt(px(palette::SP_2))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(p.accent)
                        .child(h.to_owned())
                } else if let Some(item) = line.strip_prefix("* ") {
                    div()
                        .pl(px(palette::SP_3))
                        .text_color(p.text_secondary)
                        .child(format!("• {item}"))
                } else {
                    div().h(px(4.))
                }
            }),
        )
    }

    pub(super) fn licenses_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let deps = [
            ("GPUI / GPUI Kit", "Apache-2.0"),
            ("chrono", "MIT OR Apache-2.0"),
            ("directories", "MIT OR Apache-2.0"),
            ("serde, serde_json", "MIT OR Apache-2.0"),
            ("winresource (Build)", "MIT"),
            ("Lucide Icons", "ISC"),
        ];
        div()
            .flex()
            .flex_col()
            .gap(px(palette::SP_2))
            .child(text_secondary(
                &p,
                "Direkte Abhängigkeiten von Note Warden. Vollständige Lizenztexte liegen den jeweiligen Paketen bei.",
            ))
            .child(
                card(&p).children(deps.iter().enumerate().map(|(i, (name, license))| {
                    settings_row(&p, i == 0, *name, None, text_secondary(&p, *license))
                })),
            )
    }
}
