mod calendar;
mod capture;
mod compact;
mod detail;
mod history;
mod icons;
pub mod kit;
mod list;
mod overlays;
mod releases;
mod settings;
mod stickies;
mod tags;
mod today;
pub mod workbench;

use crate::palette::{self, Palette};
use crate::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;

/// Draggable entry in the calendar.
#[derive(Clone)]
pub enum CalItem {
    Update(u64, String),
    Task(u64, u64, String),
}

impl CalItem {
    pub fn label(&self) -> &str {
        match self {
            Self::Update(_, l) | Self::Task(_, _, l) => l,
        }
    }
}

/// Task that is being reordered in the list.
#[derive(Clone)]
pub struct TaskDrag {
    pub update_id: u64,
    pub task_id: u64,
    pub label: String,
}

/// Update card that is dragged onto a status column in the board.
#[derive(Clone)]
pub struct UpdateDrag {
    pub id: u64,
    pub label: String,
}

pub struct DragPreview {
    pub label: String,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        div()
            .px(px(palette::SP_3))
            .py(px(palette::SP_2))
            .rounded(px(palette::R_INPUT))
            .bg(p.bg_elevated)
            .border_1()
            .border_color(p.accent)
            .text_color(p.text_primary)
            .text_size(palette::fs(palette::FS_BASE))
            .child(self.label.clone())
    }
}

pub fn ico(name: IconName, color: Hsla) -> Icon {
    kit::icon(name, palette::ICON_MD).text_color(color)
}

pub fn dot(color: Hsla) -> Div {
    div().size(px(8.)).flex_none().rounded_full().bg(color)
}

pub fn icon_btn(id: impl Into<ElementId>, name: IconName, tip: &'static str) -> Button {
    let b = kit::button(id)
        .ghost()
        .with_size(gpui_kit::component::Size::Medium)
        .h(px(palette::CTL_MD))
        .w(px(palette::CTL_MD))
        .p(px(0.))
        .icon(kit::icon(name, palette::ICON_MD))
        .tooltip(tip)
        .accessibility_label(tip);
    match name {
        icons::SEARCH => b.tooltip_with_action(tip, &OpenPalette, None),
        icons::SETTINGS => b.tooltip_with_action(tip, &OpenSettings, None),
        icons::UNDO => b.tooltip_with_action(tip, &HistoryUndo, None),
        icons::SIDEBAR_CLOSE | icons::SIDEBAR_OPEN => {
            b.tooltip_with_action(tip, &ToggleSidebar, Some("NoteWarden"))
        }
        _ => b,
    }
}

pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text: SharedString = text.into();
    move |window, cx| Tooltip::new(text.clone()).build(window, cx)
}

pub fn text_secondary(p: &Palette, text: impl Into<SharedString>) -> Div {
    div()
        .text_size(palette::fs(palette::FS_SM))
        .text_color(p.text_secondary)
        .child(text.into())
}

/// Text with the search matches highlighted (case-insensitive).
pub fn highlighted(text: &str, query: &str, p: &Palette) -> StyledText {
    let mut ranges = Vec::new();
    let query = query.trim().to_lowercase();
    if !query.is_empty() {
        let lower = text.to_lowercase();
        // The byte offsets only stay valid if the lengths match.
        if lower.len() == text.len() {
            let mut from = 0;
            while let Some(i) = lower[from..].find(&query) {
                let start = from + i;
                let end = start + query.len();
                if text.is_char_boundary(start) && text.is_char_boundary(end) {
                    ranges.push((
                        start..end,
                        HighlightStyle {
                            background_color: Some(p.accent.opacity(0.28)),
                            color: Some(p.text_primary),
                            ..Default::default()
                        },
                    ));
                }
                from = end;
            }
        }
    }
    StyledText::new(text.to_owned()).with_highlights(ranges)
}

impl NoteWarden {
    pub fn now_today(&self) -> NaiveDate {
        model::today()
    }

    /// Date chip: "+ Datum" or relative text. Opens the date popover.
    pub fn date_chip(
        &self,
        id: String,
        target: DateTarget,
        date: Option<NaiveDate>,
        done: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let p = palette::get(cx);
        let today = self.now_today();
        let format = self.workspace.settings.date_format;
        let entity = cx.entity();
        let button = kit::button(SharedString::from(id))
            .ghost()
            .compact()
            .with_size(gpui_kit::component::Size::Small)
            .h(px(palette::CTL_SM))
            .flex_none();
        let button = match date {
            Some(d) => {
                let overdue = d < today && !done;
                let (fg, bg) = if overdue {
                    (p.danger, p.danger.opacity(0.14))
                } else if d == today && !done {
                    (p.accent, p.accent.opacity(0.14))
                } else {
                    (p.text_secondary, p.bg_hover)
                };
                button
                    .icon(kit::icon(icons::DATE, palette::ICON_MD))
                    .label(if self.workspace.settings.relative_dates {
                        dates::relative(d, today, format)
                    } else {
                        dates::absolute(d, today, format)
                    })
                    .tooltip(dates::full_with_weekday(d, format))
                    .bg(bg)
                    .text_color(fg)
            }
            None => button
                .icon(kit::icon(icons::ADD, palette::ICON_MD))
                .label("Datum")
                .tooltip("Datum setzen")
                .text_color(p.text_tertiary),
        };
        button.on_click(move |_, window, cx| {
            let pos = window.mouse_position();
            entity.update(cx, |app, cx| app.open_date_pop(target, pos, window, cx));
        })
    }

    /// Small bell symbol (icon + tooltip only) when an open reminder is set.
    pub fn remind_bell(&self, target: DateTarget, cx: &mut Context<Self>) -> Option<AnyElement> {
        let target = reminders::Target::from_date_target(target)?;
        let remind = self.remind_of(target)?;
        let at = remind.due_at()?;
        let p = palette::get(cx);
        Some(
            div()
                .id(SharedString::from(format!("bell-{target:?}")))
                .flex()
                .items_center()
                .justify_center()
                .size(px(palette::CTL_SM))
                .tooltip(tip(format!("Erinnerung: {}", self.remind_label(at))))
                .child(kit::icon(icons::BELL, palette::ICON_SM).text_color(p.text_tertiary))
                .into_any_element(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn nav_item(
        &self,
        id: &'static str,
        icon: IconName,
        label: &'static str,
        shortcut: Option<&'static str>,
        badge: usize,
        badge_danger: bool,
        active: bool,
        icon_only: bool,
        on_click: impl Fn(&mut NoteWarden, &mut Window, &mut Context<NoteWarden>) + 'static,
        cx: &mut Context<Self>,
    ) -> gpui_kit::base::Button {
        let p = palette::get(cx);
        let entity = cx.entity();
        let tooltip = match shortcut {
            Some(keys) => format!("{label} · {keys}"),
            None => label.to_owned(),
        };
        kit::surface(id, &p)
            .relative()
            .h(palette::dens(34.))
            .px(px(palette::SP_3))
            .gap(px(palette::SP_3))
            .flex()
            .items_center()
            .rounded(px(palette::R_INPUT))
            .cursor_pointer()
            .text_size(palette::fs(palette::FS_BASE))
            .text_color(if active {
                p.text_primary
            } else {
                p.text_secondary
            })
            .when(active, |d| {
                d.bg(p.bg_active).child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(6.))
                        .bottom(px(6.))
                        .w(px(2.))
                        .rounded_full()
                        .bg(p.accent),
                )
            })
            .when(!active, |d| d)
            .child(kit::icon(icon, palette::ICON_LG).text_color(if active {
                p.text_primary
            } else {
                p.text_secondary
            }))
            .when(!icon_only, |d| {
                d.child(div().flex_1().child(label)).when(badge > 0, |d| {
                    d.child(
                        div()
                            .min_w(px(18.))
                            .h(px(18.))
                            .px(px(palette::SP_1))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .text_size(palette::fs(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .when(badge_danger, |d| d.bg(p.danger).text_color(p.bg_app))
                            .when(!badge_danger, |d| {
                                d.bg(p.bg_hover).text_color(p.text_secondary)
                            })
                            .child(badge.to_string()),
                    )
                })
            })
            .when(icon_only && badge > 0, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(6.))
                        .right(px(8.))
                        .size(px(7.))
                        .rounded_full()
                        .bg(if badge_danger { p.danger } else { p.accent }),
                )
            })
            .tooltip(tip(tooltip))
            .on_click(move |_, window, cx| entity.update(cx, |app, cx| on_click(app, window, cx)))
    }

    /// Entry of the "Status" and "Filter" groups and of the bottom shelf.
    #[allow(clippy::too_many_arguments)]
    fn view_item(
        &self,
        id: String,
        label: &'static str,
        lead: Result<Hsla, IconName>,
        count: usize,
        highlight: bool,
        filter: Filter,
        cx: &mut Context<Self>,
    ) -> gpui_kit::base::Button {
        let p = palette::get(cx);
        let entity = cx.entity();
        let active = self.view == View::Updates && self.filter == filter;
        kit::surface(SharedString::from(id), &p)
            .h(px(palette::CTL_MD))
            .px(px(palette::SP_3))
            .gap(px(palette::SP_3))
            .flex()
            .items_center()
            .rounded(px(palette::R_INPUT))
            .cursor_pointer()
            .text_size(palette::fs(palette::FS_BASE))
            .text_color(if active {
                p.text_primary
            } else {
                p.text_secondary
            })
            .when(active, |d| d.bg(p.bg_active))
            .when(!active, |d| d)
            // Same leading slot as in `nav_item`: dot and icon sit on the same center line.
            .child(
                div()
                    .size(px(palette::ICON_LG))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(match lead {
                        Ok(color) => dot(color).into_any_element(),
                        Err(icon) => kit::icon(icon, palette::ICON_MD)
                            .text_color(p.text_secondary)
                            .into_any_element(),
                    }),
            )
            .child(div().flex_1().child(label))
            // Zero is not shown; real numbers in text-secondary, overdue items highlighted.
            .when(count > 0, |d| {
                d.child(
                    div()
                        .text_size(palette::fs(11.))
                        .text_color(if highlight {
                            p.danger
                        } else {
                            p.text_secondary
                        })
                        .when(highlight, |d| d.font_weight(FontWeight::SEMIBOLD))
                        .child(count.to_string()),
                )
            })
            .on_click(move |_, _, cx| entity.update(cx, |app, cx| app.set_filter(filter, cx)))
    }

    fn group_header(
        &self,
        id: &'static str,
        label: &'static str,
        open: bool,
        toggle: fn(&mut model::Settings),
        cx: &mut Context<Self>,
    ) -> gpui_kit::base::Button {
        let p = palette::get(cx);
        let entity = cx.entity();
        kit::surface(id, &p)
            .mt(px(palette::SP_3))
            .w_full()
            .h(px(palette::CTL_MD))
            .px(px(palette::CONTROL_PAD_SM))
            .rounded(px(palette::R_INPUT))
            .flex()
            .items_center()
            .justify_start()
            .gap(px(4.))
            .cursor_pointer()
            .text_size(palette::fs(palette::FS_SM))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(p.text_tertiary)
            .child(
                kit::icon(
                    if open { icons::EXPAND } else { icons::COLLAPSE },
                    palette::ICON_SM,
                )
                .text_color(p.text_tertiary),
            )
            .child(label)
            .on_click(move |_, _, cx| {
                entity.update(cx, |app, cx| {
                    toggle(&mut app.workspace.settings);
                    app.persist(cx);
                    cx.notify();
                })
            })
    }

    fn sidebar(&self, icon_only: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let today = self.now_today();
        let live: Vec<&model::Update> = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.is_live())
            .collect();
        let due = self.due_count();
        let overdue = live.iter().filter(|u| u.overdue(today)).count();
        let overdue_items = self.due_items().overdue.len();
        let count_status = |s: Status| live.iter().filter(|u| u.status == s).count();
        let no_date = live.iter().filter(|u| u.planned_for.is_none()).count();
        let archived = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.archived && u.deleted_on.is_none())
            .count();
        let trashed = self
            .workspace
            .updates
            .iter()
            .filter(|u| u.deleted_on.is_some())
            .count();
        let entity = cx.entity();
        let status_open = self.workspace.settings.status_group_open;
        let filter_open = self.workspace.settings.filter_group_open;
        let sidebar_w = if icon_only { 56. } else { 220. };

        div()
            .id("sidebar")
            .w(px(sidebar_w))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(p.bg_sidebar)
            .border_r_1()
            .border_color(p.border_subtle)
            .p(px(8.))
            .gap(px(2.))
            .child(
                div()
                    .h(px(palette::CTL_LG))
                    .px(px(4.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        div()
                            .w(px(24.))
                            .h(px(24.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(crate::logo::shield(crate::logo::LogoStyle::new(1.))),
                    )
                    .when(!icon_only, |d| {
                        d.child(
                            div()
                                .flex_1()
                                .text_size(palette::fs(palette::FS_MD))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.text_primary)
                                .child("Note Warden"),
                        )
                        .child(
                            icon_btn("palette-open", icons::SEARCH, "Befehlspalette · Strg+K")
                                .on_click({
                                    let entity = entity.clone();
                                    move |_, window, cx| {
                                        entity.update(cx, |app, cx| app.open_palette(window, cx))
                                    }
                                }),
                        )
                    }),
            )
            .child(self.nav_item(
                "nav-today",
                icons::TODAY,
                "Heute",
                Some("Strg+1"),
                due,
                overdue_items > 0,
                self.view == View::Today,
                icon_only,
                |app, _, cx| app.set_view(View::Today, cx),
                cx,
            ))
            .when(!self.workspace.settings.hide_inbox, |d| {
                d.child(
                    self.nav_item(
                        "nav-inbox",
                        icons::INBOX,
                        "Inbox",
                        None,
                        self.workspace
                            .standalone_tasks
                            .iter()
                            .filter(|t| !t.done)
                            .count(),
                        false,
                        self.view == View::Inbox,
                        icon_only,
                        |app, _, cx| app.set_view(View::Inbox, cx),
                        cx,
                    ),
                )
            })
            .child(self.nav_item(
                "nav-updates",
                icons::UPDATES,
                "Updates",
                Some("Strg+2"),
                0,
                false,
                self.view == View::Updates && self.filter == Filter::All,
                icon_only,
                |app, _, cx| {
                    app.filter = Filter::All;
                    app.set_view(View::Updates, cx)
                },
                cx,
            ))
            .when(!self.workspace.settings.hide_board, |d| {
                d.child(self.nav_item(
                    "nav-board",
                    IconName::LayoutDashboard,
                    "Board",
                    Some("Strg+3"),
                    0,
                    false,
                    self.view == View::Board,
                    icon_only,
                    |app, _, cx| app.set_view(View::Board, cx),
                    cx,
                ))
            })
            .when(!self.workspace.settings.hide_stickies, |d| {
                d.child(self.nav_item(
                    "nav-stickies",
                    IconName::StickyNote,
                    "Post-its",
                    None,
                    0,
                    false,
                    self.view == View::Stickies,
                    icon_only,
                    |app, _, cx| app.set_view(View::Stickies, cx),
                    cx,
                ))
            })
            .when(!self.workspace.settings.hide_releases, |d| {
                d.child(self.nav_item(
                    "nav-releases",
                    icons::VERSION,
                    "Releases",
                    None,
                    0,
                    false,
                    self.view == View::Releases,
                    icon_only,
                    |app, _, cx| app.set_view(View::Releases, cx),
                    cx,
                ))
            })
            .when(!self.workspace.settings.hide_calendar, |d| {
                d.child(self.nav_item(
                    "nav-calendar",
                    icons::DATE,
                    "Kalender",
                    Some("Strg+4"),
                    0,
                    false,
                    self.view == View::Calendar,
                    icon_only,
                    |app, _, cx| app.set_view(View::Calendar, cx),
                    cx,
                ))
            })
            .when(icon_only, |d| {
                d.child(self.nav_item(
                    "nav-search",
                    icons::SEARCH,
                    "Suchen",
                    Some("Strg+K"),
                    0,
                    false,
                    false,
                    true,
                    |app, window, cx| app.open_palette(window, cx),
                    cx,
                ))
            })
            .when(!icon_only, |d| {
                d.child(self.group_header(
                    "grp-status",
                    "Status",
                    status_open,
                    |s| s.status_group_open = !s.status_group_open,
                    cx,
                ))
                .when(status_open, |d| {
                    d.children(Status::ALL.map(|s| {
                        self.view_item(
                            format!("v-{}", s.label()),
                            s.label(),
                            Ok(p.status(s)),
                            count_status(s),
                            false,
                            Filter::Status(s),
                            cx,
                        )
                    }))
                })
                .when(self.workspace.projects.iter().any(|p| !p.archived), |d| {
                    let open = self.workspace.settings.projects_group_open;
                    d.child(self.group_header(
                        "grp-projects",
                        "Projekte",
                        open,
                        |s| s.projects_group_open = !s.projects_group_open,
                        cx,
                    ))
                    .when(open, |d| {
                        d.children(self.workspace.projects.iter().filter(|p| !p.archived).map(
                            |project| {
                                let id = project.id;
                                let active = self.project_filter == Some(id);
                                let entity = cx.entity();
                                kit::surface(SharedString::from(format!("project-{id}")), &p)
                                    .w_full()
                                    .h(px(palette::CTL_MD))
                                    .px(px(palette::CONTROL_PAD_SM))
                                    .rounded(px(palette::R_INPUT))
                                    .flex()
                                    .items_center()
                                    .justify_start()
                                    .gap(px(palette::SP_2))
                                    .text_size(palette::fs(palette::FS_BASE))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if active {
                                        p.text_primary
                                    } else {
                                        p.text_secondary
                                    })
                                    .when(active, |b| b.bg(p.bg_active))
                                    .child(dot(palette::project_color(project, &p)))
                                    .tooltip(tip(project.name.clone()))
                                    .child(div().truncate().child(project.name.clone()))
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.project_filter = if app.project_filter == Some(id) {
                                                None
                                            } else {
                                                Some(id)
                                            };
                                            app.save_updates_ui(cx);
                                        })
                                    })
                            },
                        ))
                    })
                })
                .when(!self.workspace.all_tags().is_empty(), |d| {
                    let open = self.workspace.settings.tags_group_open;
                    let active_tags = &self.workspace.settings.updates_ui.tags;
                    d.child(self.group_header(
                        "grp-tags",
                        "Tags",
                        open,
                        |s| s.tags_group_open = !s.tags_group_open,
                        cx,
                    ))
                    .when(open, |d| {
                        d.children(self.workspace.all_tags().into_iter().take(12).map(
                            |(tag, count)| {
                                let active = active_tags
                                    .iter()
                                    .any(|t| t.to_lowercase() == tag.to_lowercase());
                                let entity = cx.entity();
                                let clicked = tag.clone();
                                kit::surface(SharedString::from(format!("sidebar-tag-{tag}")), &p)
                                    .w_full()
                                    .h(px(palette::CTL_MD))
                                    .px(px(palette::CONTROL_PAD_SM))
                                    .rounded(px(palette::R_INPUT))
                                    .flex()
                                    .items_center()
                                    .justify_start()
                                    .gap(px(palette::SP_2))
                                    .text_size(palette::fs(palette::FS_BASE))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(if active {
                                        p.text_primary
                                    } else {
                                        p.text_secondary
                                    })
                                    .when(active, |b| b.bg(p.bg_active))
                                    .child(dot(palette::tag_color(&tag, &p)))
                                    .child(div().flex_1().truncate().child(format!("#{tag}")))
                                    .child(text_secondary(&p, count.to_string()))
                                    .on_click(move |_, _, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.toggle_tag_filter(&clicked, true, cx)
                                        })
                                    })
                            },
                        ))
                    })
                })
                .child(self.group_header(
                    "grp-filter",
                    "Filter",
                    filter_open,
                    |s| s.filter_group_open = !s.filter_group_open,
                    cx,
                ))
                .when(filter_open, |d| {
                    d.child(self.view_item(
                        "v-overdue".into(),
                        "Überfällig",
                        Ok(p.danger),
                        overdue,
                        true,
                        Filter::Overdue,
                        cx,
                    ))
                    .child(self.view_item(
                        "v-nodate".into(),
                        "Ohne Termin",
                        Ok(p.border_strong),
                        no_date,
                        false,
                        Filter::NoDate,
                        cx,
                    ))
                })
                .when(
                    !self.workspace.settings.updates_ui.presets.is_empty(),
                    |d| {
                        let open = self.workspace.settings.views_group_open;
                        d.child(self.group_header(
                            "grp-views",
                            "Ansichten",
                            open,
                            |s| s.views_group_open = !s.views_group_open,
                            cx,
                        ))
                        .when(open, |d| d.children(self.saved_view_items(cx)))
                    },
                )
            })
            .child(div().flex_1())
            .child(div().h(px(1.)).mx(px(4.)).my(px(4.)).bg(p.border_subtle))
            .when(icon_only, |d| {
                d.child(self.nav_item(
                    "nav-archive",
                    icons::ARCHIVE,
                    "Archiv",
                    None,
                    archived,
                    false,
                    self.view == View::Updates && self.filter == Filter::Archive,
                    true,
                    |app, _, cx| app.set_filter(Filter::Archive, cx),
                    cx,
                ))
                .child(self.nav_item(
                    "nav-trash",
                    icons::DELETE,
                    "Papierkorb",
                    None,
                    0,
                    false,
                    self.view == View::Updates && self.filter == Filter::Trash,
                    true,
                    |app, _, cx| app.set_filter(Filter::Trash, cx),
                    cx,
                ))
            })
            .when(!icon_only, |d| {
                d.child(self.view_item(
                    "v-archive".into(),
                    "Archiv",
                    Err(icons::ARCHIVE),
                    archived,
                    false,
                    Filter::Archive,
                    cx,
                ))
                .child(self.view_item(
                    "v-trash".into(),
                    "Papierkorb",
                    Err(icons::DELETE),
                    trashed,
                    false,
                    Filter::Trash,
                    cx,
                ))
            })
            .child(
                div()
                    .mt(px(4.))
                    .flex()
                    .items_center()
                    .when(icon_only, |d| d.flex_col().gap(px(4.)))
                    .when(!icon_only, |d| d.justify_between().px(px(4.)))
                    .child(
                        icon_btn(
                            "open-compact",
                            IconName::StickyNote,
                            "Notizzettel öffnen · Strg+Umschalt+M",
                        )
                        .on_click({
                            let entity = entity.clone();
                            move |_, window, cx| {
                                entity.update(cx, |app, cx| app.open_popup(window, cx))
                            }
                        }),
                    )
                    .child(
                        icon_btn("open-settings", icons::SETTINGS, "Einstellungen · Strg+,")
                            .on_click({
                                let entity = entity.clone();
                                move |_, window, cx| {
                                    entity.update(cx, |app, cx| app.open_settings(None, window, cx))
                                }
                            }),
                    )
                    .when(!icon_only, |d| {
                        d.child(
                            div()
                                .text_size(palette::fs(11.))
                                .text_color(p.text_tertiary)
                                .child(format!("v{VERSION}")),
                        )
                    })
                    .child(
                        icon_btn(
                            "toggle-sidebar",
                            if icon_only {
                                icons::SIDEBAR_OPEN
                            } else {
                                icons::SIDEBAR_CLOSE
                            },
                            "Seitenleiste ein-/ausklappen · Strg+B",
                        )
                        .on_click({
                            let entity = entity.clone();
                            move |_, _, cx| entity.update(cx, |app, cx| app.toggle_sidebar(cx))
                        }),
                    ),
            )
    }

    fn toasts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        div()
            .absolute()
            .right(px(16.))
            .bottom(px(16.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .children(self.toasts.iter().map(|toast| {
                let id = toast.id;
                let entity = cx.entity();
                let entity2 = cx.entity();
                div()
                    .id(("toast", id))
                    .role(Role::Status)
                    .on_hover({
                        let entity = cx.entity();
                        move |hovered, _, cx| {
                            entity.update(cx, |app, cx| app.pause_toast(id, *hovered, cx))
                        }
                    })
                    .relative()
                    .overflow_hidden()
                    .min_w(px(280.))
                    .max_w(px(420.))
                    .pl(px(palette::SP_4))
                    .pr(px(8.))
                    .py(px(palette::SP_3))
                    .gap(px(palette::SP_3))
                    .flex()
                    .items_center()
                    .rounded(px(palette::R_CARD))
                    .bg(p.bg_elevated)
                    .border_1()
                    .border_color(p.border_strong)
                    .shadow_lg()
                    .text_size(palette::fs(palette::FS_BASE))
                    .text_color(p.text_primary)
                    .child(ico(
                        if toast.undo.is_some() {
                            IconName::Info
                        } else {
                            IconName::CircleCheck
                        },
                        if toast.undo.is_some() {
                            p.accent
                        } else {
                            p.done
                        },
                    ))
                    .child(div().flex_1().min_w_0().child(toast.message.clone()))
                    .children(toast.actions.iter().enumerate().map(|(index, (label, _))| {
                        let entity = cx.entity();
                        kit::button(("toast-action", id as usize * 8 + index))
                            .ghost()
                            .with_size(gpui_kit::component::Size::Small)
                            .h(px(palette::CTL_SM))
                            .label(label.clone())
                            .text_color(p.accent)
                            .on_click(move |_, window, cx| {
                                entity.update(cx, |app, cx| {
                                    app.run_toast_action(id, index, window, cx)
                                })
                            })
                    }))
                    .when(toast.undo.is_some(), |d| {
                        d.child(
                            kit::button(("undo", id))
                                .ghost()
                                .with_size(gpui_kit::component::Size::Small)
                                .h(px(palette::CTL_SM))
                                .label("Rückgängig")
                                .icon(kit::icon(icons::UNDO, palette::ICON_SM))
                                .text_color(p.accent)
                                .on_click(move |_, window, cx| {
                                    entity.update(cx, |app, cx| app.run_undo(id, window, cx))
                                }),
                        )
                    })
                    .child(
                        icon_btn(("toast-close", id), icons::CLOSE, "Schließen").on_click(
                            move |_, _, cx| entity2.update(cx, |app, cx| app.dismiss_toast(id, cx)),
                        ),
                    )
                    .child(
                        // Remaining time until automatic hiding; stands still on hover.
                        div()
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .h(px(2.))
                            .w(relative(
                                1. - (toast.elapsed_ms as f32 / palette::TOAST_MS as f32).min(1.),
                            ))
                            .bg(p.accent.opacity(0.7)),
                    )
            }))
    }

    /// Splash on start: shield, name, version, progress line. Stays up on load errors.
    fn splash_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let reduce = self.workspace.settings.motion == model::Motion::Reduced;
        let opacity = match self.splash {
            Splash::Fading(t) => {
                1. - (t.elapsed().as_millis() as f32 / SPLASH_FADE_MS as f32).clamp(0., 1.)
            }
            _ => 1.,
        };
        let style = crate::logo::LogoStyle::new(5.);
        let logo: AnyElement = if reduce {
            crate::logo::shield(style).into_any_element()
        } else {
            // 0–220 ms fade in, 220–380 ms the check mark "pops" (1.0 → 1.12 → 1.0).
            crate::logo::shield(style)
                .into_any_element()
                .with_animation(
                    "splash-logo",
                    Animation::new(palette::motion(palette::MOTION_INTRO_MS)),
                    move |_, delta| {
                        let t = delta * 700.;
                        let scale = if (220. ..380.).contains(&t) {
                            let k = (t - 220.) / 160.;
                            1. + 0.12 * (1. - (2. * k - 1.).abs())
                        } else {
                            1.
                        };
                        div()
                            .opacity((t / 220.).min(1.))
                            .child(crate::logo::shield(style.check_scale(scale)))
                            .into_any_element()
                    },
                )
                .into_any_element()
        };
        let text_block = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(p.text_primary)
                    .child("Note Warden"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(p.text_tertiary)
                    .child(format!("Version {VERSION}")),
            );
        let text_block: AnyElement = if reduce {
            text_block.into_any_element()
        } else {
            text_block
                .with_animation(
                    "splash-text",
                    Animation::new(palette::motion(palette::MOTION_INTRO_MS)),
                    |el, delta| el.opacity(((delta * 700. - 380.) / 220.).clamp(0., 1.)),
                )
                .into_any_element()
        };
        let track = div()
            .w(px(120.))
            .h(px(2.))
            .rounded_full()
            .bg(p.border_subtle)
            .overflow_hidden();
        let progress: AnyElement = if reduce {
            track
                .child(div().w(px(40.)).h_full().ml(px(40.)).bg(p.accent))
                .into_any_element()
        } else {
            track
                .child(div().w(px(40.)).h_full().bg(p.accent).with_animation(
                    "splash-progress",
                    Animation::new(palette::motion(palette::MOTION_INTRO_MS * 2)).repeat(),
                    |el, delta| el.ml(px(-40. + delta * 160.)),
                ))
                .into_any_element()
        };
        let failed = !self.writable && self.error.is_some();
        let entity = cx.entity();
        div()
            .id("splash")
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(palette::SP_4))
            .bg(p.bg_app)
            .opacity(opacity)
            .child(logo)
            .child(text_block)
            .when(failed, |d| {
                d.child(
                    kit::card(&p)
                        .max_w(px(460.))
                        .p(px(palette::SP_4))
                        .gap(px(palette::SP_3))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.danger)
                                .child("Daten konnten nicht geladen werden"),
                        )
                        .child(text_secondary(&p, self.error.clone().unwrap_or_default()))
                        .child(
                            kit::button("splash-open")
                                .primary()
                                .label("Trotzdem öffnen")
                                .on_click(move |_, _, cx| {
                                    entity.update(cx, |app, cx| app.dismiss_splash(cx))
                                }),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .bottom(px(palette::SP_8))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(progress),
            )
    }

    /// Calm save status: briefly "Speichert …" (saving), then 2 s "Gespeichert" (saved), afterwards just a check icon.
    pub fn save_status(&self, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let row = || {
            div()
                .flex()
                .items_center()
                .gap(px(palette::SP_1))
                .text_size(palette::fs(palette::FS_SM))
        };
        if self.error.is_some() {
            let entity = cx.entity();
            return row()
                .text_color(p.danger)
                .child("Nicht gespeichert")
                .child(
                    kit::button("save-retry")
                        .ghost()
                        .with_size(gpui_kit::component::Size::Small)
                        .h(px(palette::CTL_SM))
                        .label("Erneut versuchen")
                        .text_color(p.danger)
                        .on_click(move |_, _, cx| entity.update(cx, |app, cx| app.retry_save(cx))),
                )
                .into_any_element();
        }
        if self.dirty {
            return row()
                .text_color(p.text_tertiary)
                .child("Speichert …")
                .into_any_element();
        }
        if self.recovered {
            return row()
                .text_color(p.text_tertiary)
                .child("Aus Sicherung wiederhergestellt")
                .into_any_element();
        }
        let fresh = self
            .saved_at
            .is_some_and(|t| t.elapsed() < StdDuration::from_secs(2));
        let check = kit::icon(icons::CONFIRM, palette::ICON_SM).text_color(p.text_tertiary);
        if fresh {
            row()
                .id("save-status")
                .text_color(p.text_tertiary)
                .child(check)
                .child("Gespeichert")
                .into_any_element()
        } else {
            div()
                .id("save-status")
                .size(px(palette::CTL_SM))
                .flex()
                .items_center()
                .justify_center()
                .tooltip(tip("Alle Änderungen gespeichert"))
                .child(check)
                .into_any_element()
        }
    }

    fn on_task_toggle(&mut self, a: &NoteTaskToggle, window: &mut Window, cx: &mut Context<Self>) {
        let Some(notes) = self.selected().map(|u| u.notes.clone()) else {
            return;
        };
        if let Some(new) = md::set_task_marker(&notes, a.start..a.end, a.checked) {
            self.notes_input
                .update(cx, |input, cx| input.set_value(new.clone(), window, cx));
            self.set_notes(new, cx);
        }
    }

    fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view != View::Updates {
            return;
        }
        if let Some(id) = self.selected_update {
            if self
                .workspace
                .update(id)
                .is_some_and(|u| u.deleted_on.is_some())
            {
                self.open_dialog(Dialog::ConfirmPurge(id), window, cx);
            } else {
                self.request_trash(id, window, cx);
            }
        }
    }
}

impl Render for NoteWarden {
    /// Error boundary: an error while building the interface shows the error screen instead of crashing.
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.crashed {
            let drawn = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.render_app(window, cx).into_any_element()
            }));
            if let Ok(element) = drawn {
                return element;
            }
            self.crashed = true;
            crate::log::error("Fehler beim Aufbau der Oberfläche");
            self.flush();
        }
        self.crash_screen(cx).into_any_element()
    }
}

impl NoteWarden {
    fn crash_screen(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let entity = cx.entity();
        let log_entity = cx.entity();
        div()
            .size_full()
            .bg(p.bg_app)
            .text_color(p.text_primary)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(palette::SP_4))
            .child(crate::logo::shield(
                crate::logo::LogoStyle::new(5.).mono(p.text_tertiary),
            ))
            .child(
                div()
                    .text_size(palette::fs(palette::FS_XL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Etwas ist schiefgelaufen"),
            )
            .child(text_secondary(
                &p,
                "Deine Daten sind gespeichert. Lade die Ansicht neu, um weiterzumachen.",
            ))
            .child(
                div()
                    .flex()
                    .gap(px(palette::SP_2))
                    .child(
                        kit::button("crash-reload")
                            .primary()
                            .label("Neu laden")
                            .on_click(move |_, _, cx| {
                                entity.update(cx, |app, cx| {
                                    app.crashed = false;
                                    cx.notify();
                                })
                            }),
                    )
                    .child(
                        kit::button("crash-log")
                            .outline()
                            .label("Logdatei öffnen")
                            .on_click(move |_, _, cx| {
                                log_entity.update(cx, |app, cx| app.open_log(cx))
                            }),
                    ),
            )
    }

    fn render_app(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = match self.workspace.settings.theme {
            ThemeChoice::Dark => true,
            ThemeChoice::Light => false,
            ThemeChoice::System => matches!(
                window.appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            ),
        };
        let st = &self.workspace.settings;
        let look = (dark, st.accent, st.density, st.font_scale);
        if self.applied_look != Some(look) {
            self.applied_look = Some(look);
            palette::apply(dark, st, cx);
        }
        let p = palette::get(cx);
        let width = window.bounds().size.width;
        if self.workspace.settings.remember_window {
            let size = window.bounds().size;
            let current = (
                f32::from(size.width).round(),
                f32::from(size.height).round(),
            );
            if !self.popup
                && self.workspace.settings.window_size != Some(current)
                && self.splash != Splash::Showing
            {
                self.workspace.settings.window_size = Some(current);
                self.persist(cx);
            }
        }
        palette::SETTINGS_STACKED.store(width < px(980.), std::sync::atomic::Ordering::Relaxed);
        let icon_only =
            self.workspace.settings.sidebar_collapsed || width < px(palette::SIDEBAR_COLLAPSE_W);
        let sidebar_w = if icon_only { 56. } else { 220. };
        self.content_width =
            (f32::from(width) - if self.focus_mode { 0. } else { sidebar_w }).max(0.);
        let narrow = self.content_width < 900.;

        let mut context = KeyContext::default();
        context.add("NoteWarden");
        if self.view == View::Calendar {
            context.add("Calendar");
        }
        if self.view == View::Updates {
            context.add("Updates");
        }
        if self.view == View::Today {
            context.add("Today");
        }

        let main = if self.popup {
            self.remember_compact_bounds(window, cx);
            self.compact_view(cx).into_any_element()
        } else {
            match self.view {
                View::Board => self.board_view(cx).into_any_element(),
                View::Releases => self.releases_view(cx).into_any_element(),
                View::Stickies => self.stickies_view(cx).into_any_element(),
                View::Inbox => self.inbox_view(cx).into_any_element(),
                View::Today => self
                    .today_view(f32::from(width) - sidebar_w, window, cx)
                    .into_any_element(),
                View::Calendar => self.calendar_view(narrow, cx).into_any_element(),
                View::Updates => {
                    let list_w = self.workspace.settings.list_width.clamp(300., 520.);
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .when(!self.focus_mode && (!narrow || !self.detail_open), |d| {
                            d.child(self.update_list(narrow, list_w, cx))
                        })
                        .when(!narrow && !self.focus_mode, |d| {
                            let entity = cx.entity();
                            d.child(
                                div()
                                    .id("list-resize")
                                    .w(px(5.))
                                    .h_full()
                                    .flex_none()
                                    .cursor_col_resize()
                                    .hover(|s| s.bg(p.accent.opacity(0.35)))
                                    .when(self.resizing_list, |d| d.bg(p.accent.opacity(0.5)))
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        entity.update(cx, |app, cx| {
                                            app.resizing_list = true;
                                            cx.notify();
                                        })
                                    }),
                            )
                        })
                        .when(!narrow || self.detail_open, |d| {
                            d.child(self.update_detail(narrow, cx))
                        })
                        .into_any_element()
                }
            }
        };

        #[cfg(debug_assertions)]
        let main = if self.styleguide_open {
            kit::styleguide(&self.search_input).into_any_element()
        } else {
            main
        };
        div()
            .id("app-root")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(p.bg_app)
            .text_color(p.text_primary)
            .text_size(palette::fs(palette::FS_BASE))
            .font_family(palette::FONT_UI)
            .track_focus(&self.focus_handle)
            .key_context(context)
            .on_action(cx.listener(|this, _: &HistoryUndo, w, cx| this.history_step(false, w, cx)))
            .on_action(cx.listener(|this, _: &HistoryRedo, w, cx| this.history_step(true, w, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousUpdate, w, cx| this.select_relative(-1, w, cx)),
            )
            .on_action(cx.listener(|this, _: &NextUpdate, w, cx| this.select_relative(1, w, cx)))
            .on_action(cx.listener(|this, _: &ToggleFocusMode, _, cx| {
                this.focus_mode = !this.focus_mode;
                cx.notify();
            }))
            .on_action(
                cx.listener(|this, _: &ToggleCompactMode, window, cx| {
                    this.toggle_popup(window, cx)
                }),
            )
            .on_action(cx.listener(|_this, _: &ToggleStyleguide, _, _cx| {
                #[cfg(debug_assertions)]
                {
                    _this.styleguide_open = !_this.styleguide_open;
                    _cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &NewUpdate, window, cx| {
                this.create_update(None, None, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &FocusSearch, window, cx| this.focus_search(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &CloseOverlay, window, cx| this.close_overlay(window, cx)),
            )
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(
                cx.listener(|this, _: &OpenPalette, window, cx| this.open_palette(window, cx)),
            )
            .on_action(cx.listener(|this, _: &ShowBoard, _, cx| this.set_view(View::Board, cx)))
            .on_action(cx.listener(|this, _: &ShowInbox, _, cx| this.set_view(View::Inbox, cx)))
            .on_action(cx.listener(|this, _: &ShowToday, _, cx| this.set_view(View::Today, cx)))
            .on_action(cx.listener(|this, _: &ShowUpdates, _, cx| this.set_view(View::Updates, cx)))
            .on_action(
                cx.listener(|this, _: &ShowCalendar, _, cx| this.set_view(View::Calendar, cx)),
            )
            .on_action(cx.listener(|this, _: &ShowShortcuts, window, cx| {
                this.open_dialog(Dialog::Shortcuts, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &DeleteSelected, window, cx| {
                    this.delete_selected(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &CalPrev, _, cx| this.shift_calendar(-1, cx)))
            .on_action(cx.listener(|this, _: &CalNext, _, cx| this.shift_calendar(1, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, window, cx| {
                this.open_settings(None, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &FocusQuickAdd, window, cx| this.focus_quick_add(window, cx)),
            )
            .on_action(cx.listener(|this, _: &CalMonth, _, cx| {
                this.cal_mode = CalMode::Month;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CalWeek, _, cx| {
                this.cal_mode = CalMode::Week;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CalFocusPrevDay, _, cx| this.move_cal_focus(-1, cx)))
            .on_action(cx.listener(|this, _: &CalFocusNextDay, _, cx| this.move_cal_focus(1, cx)))
            .on_action(cx.listener(|this, _: &CalFocusPrevWeek, _, cx| this.move_cal_focus(-7, cx)))
            .on_action(cx.listener(|this, _: &CalFocusNextWeek, _, cx| this.move_cal_focus(7, cx)))
            .on_action(cx.listener(|this, _: &CalOpenDay, _, cx| {
                if let Some(day) = this.cal_focus {
                    this.cal_more = Some(day);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &CalToday, _, cx| {
                this.cal_anchor = model::today();
                cx.notify();
            }))
            .on_action(cx.listener(Self::on_task_toggle))
            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, window, cx| {
                if this.resizing_panel {
                    let total = f32::from(window.bounds().size.width);
                    this.workspace.settings.cal_panel_width =
                        (total - f32::from(ev.position.x)).clamp(220., 420.);
                    cx.notify();
                }
                if this.resizing_list {
                    let w = (f32::from(ev.position.x) - sidebar_w).clamp(300., 520.);
                    this.workspace.settings.list_width = w;
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resizing_panel {
                        this.resizing_panel = false;
                        this.persist(cx);
                        cx.notify();
                    }
                    if this.resizing_list {
                        this.resizing_list = false;
                        this.persist(cx);
                        cx.notify();
                    }
                }),
            )
            .when(self.settings_open, |d| d.child(self.settings_overlay(cx)))
            .when(!self.settings_open, |d| {
                d.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .when(!self.focus_mode && !self.popup, |d| {
                            d.child(self.sidebar(icon_only, cx))
                        })
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .flex()
                                .flex_col()
                                .when_some(self.update_banner(cx), |col, banner| col.child(banner))
                                .when_some(self.restore_notice, |col, at| {
                                    col.child(self.restore_banner(at, cx))
                                })
                                .when_some(self.error.clone(), |col, error| {
                                    col.child(
                                        div()
                                            .px(px(palette::SP_4))
                                            .py(px(palette::SP_2))
                                            .bg(p.danger.opacity(0.16))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_size(palette::fs(palette::FS_SM))
                                                    .text_color(p.danger)
                                                    .child(error),
                                            )
                                            .child(if self.writable {
                                                let entity = cx.entity();
                                                kit::button("retry-save")
                                                    .outline()
                                                    .with_size(gpui_kit::component::Size::Small)
                                                    .h(px(palette::CTL_SM))
                                                    .label("Erneut speichern")
                                                    .on_click(move |_, _, cx| {
                                                        entity.update(cx, |app, cx| {
                                                            app.retry_save(cx)
                                                        })
                                                    })
                                                    .into_any_element()
                                            } else {
                                                let entity = cx.entity();
                                                kit::button("open-data-dir")
                                                    .outline()
                                                    .with_size(gpui_kit::component::Size::Small)
                                                    .h(px(palette::CTL_SM))
                                                    .label("Datenordner öffnen")
                                                    .on_click(move |_, _, cx| {
                                                        entity.update(cx, |app, cx| {
                                                            app.reveal_data_file(cx)
                                                        })
                                                    })
                                                    .into_any_element()
                                            }),
                                    )
                                })
                                .child(div().flex_1().min_h_0().child(main)),
                        ),
                )
            })
            .child(self.keyboard_popup(cx))
            .child(self.toasts(cx))
            .when(
                !self.settings_open
                    && self.view == View::Today
                    && self
                        .quick_input
                        .read(cx)
                        .focus_handle(cx)
                        .is_focused(window)
                    && !self.quick_options(cx).is_empty(),
                |d| d.child(self.quick_options_overlay(window, cx)),
            )
            .when(self.date_pop.is_some(), |d| d.child(self.date_popover(cx)))
            .when(self.pill_menu.is_some(), |d| {
                d.child(self.pill_menu_overlay(cx))
            })
            .when(self.palette_open, |d| d.child(self.command_palette(cx)))
            .when(self.history_panel.is_some(), |d| {
                d.child(self.history_panel_view(cx))
            })
            .when(self.dialog.is_some(), |d| d.child(self.dialog_overlay(cx)))
            .when(
                self.splash != Splash::Gone && self.view != View::Today,
                |d| d.child(self.splash_overlay(cx)),
            )
    }
}
