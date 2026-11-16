//! Reusable UI foundations (see docs/ui-guidelines.md). All sizes and colors
//! come from `palette`; components know no pixel values of their own outside the scale.

use super::*;
use crate::palette::{self, Palette};

pub type ClickFn = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

pub fn click_fn(f: impl Fn(&mut Window, &mut App) + 'static) -> ClickFn {
    std::rc::Rc::new(f)
}

/// Switch 36 × 20, knob 16; `role=switch` through label + keyboard (Space/Enter).
pub fn switch(
    id: impl Into<ElementId>,
    p: &Palette,
    on: bool,
    label: &'static str,
    handler: ClickFn,
) -> Button {
    button(id)
        .ghost()
        .accessibility_label(label)
        .role(Role::Switch)
        .toggled(on)
        .min_h(px(palette::MIN_HIT))
        .min_w(px(palette::MIN_HIT))
        .px(px(0.))
        .tooltip(label)
        .child(
            div()
                .w(px(36.))
                .h(px(20.))
                .rounded_full()
                .flex()
                .items_center()
                .px(px(2.))
                .bg(if on { p.accent } else { p.border_strong })
                .when(on, |d| d.justify_end())
                .child(div().size(px(16.)).rounded_full().bg(if on {
                    p.accent_contrast
                } else {
                    p.text_primary
                })),
        )
        .on_click(move |_, w, cx| handler(w, cx))
}

/// Segment for modes (Schreiben | Vorschau | Geteilt, i.e. Write | Preview | Split): compact container, 28 px, icons 14 px.
/// Without `show_text` only icons with a tooltip remain.
pub fn segment(
    id: &'static str,
    p: &Palette,
    items: &[(Option<IconName>, &'static str)],
    active: usize,
    show_text: bool,
    on: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let on = std::rc::Rc::new(on);
    let p = *p;
    div()
        .id(id)
        .role(Role::RadioGroup)
        .flex()
        .flex_none()
        .p(px(2.))
        .gap(px(2.))
        .h(px(palette::CTL_SM))
        .items_center()
        .rounded(px(palette::R_CONTROL_SM + 2.))
        .bg(p.bg_hover)
        .children(items.iter().enumerate().map(|(i, (name, label))| {
            let on = on.clone();
            let selected = i == active;
            button(SharedString::from(format!("{id}-{i}")))
                .ghost()
                .toggled(selected)
                .role(Role::RadioButton)
                .with_size(gpui_kit::component::Size::Small)
                .h_full()
                .px(px(palette::SP_2))
                .rounded(px(palette::R_CONTROL_SM))
                .text_size(palette::fs(palette::FS_CONTROL_SM))
                .text_color(if selected {
                    p.text_primary
                } else {
                    p.text_secondary
                })
                .when(selected, |d| d.bg(p.bg_panel))
                .when_some(*name, |d, name| d.icon(icon(name, palette::ICON_SM)))
                .when(show_text || name.is_none(), |d| d.label(*label))
                .tooltip(*label)
                .accessibility_label(*label)
                .on_click(move |_, window, cx| on(i, window, cx))
        }))
}

/// Underline tabs for page sections: 36 px, 13/500, active tab with a 2 px accent line.
/// Arrow keys, Home and End switch the tab (APG).
pub fn tabs(
    id: &'static str,
    p: &Palette,
    labels: &[&'static str],
    active: usize,
    animate: bool,
    on: impl Fn(usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let on = std::rc::Rc::new(on);
    let p = *p;
    let count = labels.len();
    div()
        .id(id)
        .role(Role::TabList)
        .w_full()
        .h(px(36.))
        .flex()
        .items_end()
        .gap(px(palette::SP_4))
        .border_b_1()
        .border_color(p.border_subtle)
        .children(labels.iter().enumerate().map(|(i, label)| {
            let (click, keys) = (on.clone(), on.clone());
            let selected = i == active;
            let underline = div()
                .absolute()
                .left_0()
                .right_0()
                .bottom(px(-1.))
                .h(px(2.))
                .rounded_full()
                .bg(p.accent);
            let underline = if selected && animate {
                underline
                    .with_animation(
                        SharedString::from(format!("{id}-underline-{active}")),
                        Animation::new(palette::motion(palette::MOTION_SLOW_MS))
                            .with_easing(palette::ease_out),
                        |d, t| d.opacity(t),
                    )
                    .into_any_element()
            } else if selected {
                underline.into_any_element()
            } else {
                div().into_any_element()
            };
            div()
                .relative()
                .h_full()
                .child(
                    surface(SharedString::from(format!("{id}-{i}")), &p)
                        .role(Role::Tab)
                        .selected(selected)
                        .tab_stop(selected)
                        .h_full()
                        .px(px(2.))
                        .flex()
                        .items_center()
                        .rounded(px(palette::R_CONTROL_SM))
                        .text_size(palette::fs(palette::FS_BASE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if selected {
                            p.text_primary
                        } else {
                            p.text_secondary
                        })
                        .child(*label)
                        .on_click(move |_, window, cx| click(i, window, cx))
                        .on_key_down(move |ev, window, cx| {
                            let next = match ev.keystroke.key.as_str() {
                                "left" => (active + count - 1) % count,
                                "right" => (active + 1) % count,
                                "home" => 0,
                                "end" => count - 1,
                                _ => return,
                            };
                            keys(next, window, cx);
                            cx.stop_propagation();
                        }),
                )
                .child(underline)
        }))
}

/// Property chip (28 px, pill): icon + value; empty = transparent with a border and a placeholder.
pub fn prop_chip(
    id: impl Into<ElementId>,
    p: &Palette,
    lead: impl IntoElement,
    text: impl Into<SharedString>,
    filled: bool,
) -> Button {
    let b = button(id)
        .rounded_full()
        .with_size(gpui_kit::component::Size::Small)
        .h(px(palette::CHIP_MD))
        .px(px(palette::CONTROL_PAD_SM))
        .text_size(palette::fs(palette::FS_CONTROL_SM))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(palette::ICON_GAP))
                .child(lead)
                .child(text.into()),
        );
    if filled {
        b.ghost().text_color(p.text_primary)
    } else {
        b.ghost()
            .text_color(p.text_tertiary)
            .border_1()
            .border_color(p.border_subtle)
    }
}

/// Color swatch (round, 24 px); the active one with a ring.
pub fn swatch(
    id: impl Into<ElementId>,
    p: &Palette,
    color: Hsla,
    selected: bool,
    label: &'static str,
    handler: ClickFn,
) -> Button {
    button(id)
        .ghost()
        .accessibility_label(label)
        .size(px(palette::MIN_HIT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .border_2()
        .border_color(if selected {
            p.text_primary
        } else {
            gpui_kit::transparent_black()
        })
        .tooltip(label)
        .child(
            div()
                .size(px(20.))
                .rounded_full()
                .bg(color)
                .when(selected, |d| {
                    d.flex()
                        .items_center()
                        .justify_center()
                        .child(ico(icons::CONFIRM, p.bg_app))
                }),
        )
        .on_click(move |_, window, cx| handler(window, cx))
}

/// Karte: `bg-panel`, 1 px `border-subtle`, Radius 10.
pub fn card(p: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(palette::R_CARD))
        .bg(p.bg_panel)
        .border_1()
        .border_color(p.border_subtle)
        .overflow_hidden()
}

/// Einstellungs-Zeile: Titel + Beschreibung links, Bedienelement rechts (min. 56 px).
pub fn settings_row(
    p: &Palette,
    first: bool,
    title: impl IntoElement,
    desc: Option<SharedString>,
    control: impl IntoElement,
) -> Div {
    div()
        .min_h(px(56.))
        .px(px(palette::SP_4))
        .py(px(palette::SP_3))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(palette::SP_4))
        .when(
            palette::SETTINGS_STACKED.load(std::sync::atomic::Ordering::Relaxed),
            |d| d.flex_col().items_start(),
        )
        .when(!first, |d| d.border_t_1().border_color(p.border_subtle))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(palette::fs(palette::FS_MD))
                        .text_color(p.text_primary)
                        .child(title),
                )
                .when_some(desc, |d, desc| {
                    d.child(
                        div()
                            .text_size(palette::fs(palette::FS_SM))
                            .text_color(p.text_secondary)
                            .child(desc),
                    )
                }),
        )
        .child(
            div()
                .min_w(px(200.))
                .flex_wrap()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(control),
        )
}

/// Tastenkappe: 1 px Rand, Bodenschatten, Mono 11 px.
pub fn keycap(p: &Palette, text: impl Into<SharedString>) -> Div {
    div()
        .min_w(px(20.))
        .h(px(20.))
        .px(px(6.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(palette::R_XS))
        .border_1()
        .border_color(p.border_strong)
        .border_b_2()
        .bg(p.bg_hover)
        .font_family(palette::FONT_MONO)
        .text_size(px(11.))
        .text_color(p.text_secondary)
        .child(text.into())
}

/// Key caps for a shortcut such as "Strg+Umschalt+K".
pub fn keycaps(p: &Palette, combo: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(4.))
        .children(combo.split('+').map(|k| keycap(p, k.to_owned())))
}

/// Empty state with a shield motif, title, sentence and an optional primary action.
pub fn empty_state(
    p: &Palette,
    title: &'static str,
    text: &'static str,
    action: Option<AnyElement>,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(px(palette::SP_2))
        .py(px(palette::SP_8))
        .child(crate::logo::shield(
            crate::logo::LogoStyle::new(3.).mono(p.text_tertiary),
        ))
        .child(
            div()
                .mt(px(palette::SP_2))
                .font_weight(FontWeight::MEDIUM)
                .text_color(p.text_primary)
                .child(title),
        )
        .child(text_secondary(p, text))
        .when_some(action, |d, a| d.mt(px(palette::SP_2)).child(a))
}

/// Small progress ring with an accessible text alternative.
pub fn progress_ring(p: &Palette, done: usize, total: usize) -> Stateful<Div> {
    let p = *p;
    let ratio = if total == 0 {
        0.
    } else {
        done as f32 / total as f32
    };
    div()
        .id(SharedString::from(format!("ring-{done}-{total}")))
        .size(px(if total == 0 { 0. } else { palette::ICON_XL }))
        .when(total == 0, |d| d.invisible())
        .flex_none()
        .tooltip(tip(format!("{done} von {total} erledigt")))
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let center = point(
                        bounds.origin.x + px(palette::ICON_XL / 2.),
                        bounds.origin.y + px(palette::ICON_XL / 2.),
                    );
                    for (fraction, color) in [(1., p.border_strong), (ratio, p.accent)] {
                        if fraction <= 0. {
                            continue;
                        }
                        let mut path = PathBuilder::stroke(px(2.));
                        let steps = (fraction * 48.).ceil() as usize;
                        for i in 0..=steps {
                            let angle = -std::f32::consts::FRAC_PI_2
                                + std::f32::consts::TAU * fraction * i as f32 / steps as f32;
                            let pt = point(
                                center.x + px(angle.cos() * 7.),
                                center.y + px(angle.sin() * 7.),
                            );
                            if i == 0 {
                                path.move_to(pt);
                            } else {
                                path.line_to(pt);
                            }
                        }
                        if let Ok(path) = path.build() {
                            window.paint_path(path, color);
                        }
                    }
                },
            )
            .size_full(),
        )
}

/// The only entry point for library icons.
pub fn icon(name: IconName, size: f32) -> Icon {
    debug_assert!(
        [
            palette::ICON_XS,
            palette::ICON_SM,
            palette::ICON_MD,
            palette::ICON_LG,
            palette::ICON_XL,
            palette::ICON_2XL
        ]
        .contains(&size)
    );
    Icon::new(name).size(px(size))
}

/// Native buttons provide keyboard, focus, disabled and loading from GPUI.
pub fn button(id: impl Into<ElementId>) -> Button {
    Button::new(id)
        .with_size(gpui_kit::component::Size::Medium)
        .h(px(palette::CTL_MD))
        .cursor_pointer()
        .rounded(px(palette::R_INPUT))
        .text_size(palette::fs(palette::FS_BASE))
        .font_weight(FontWeight::MEDIUM)
        .gap(px(palette::ICON_GAP))
        .px(px(palette::CONTROL_PAD_MD))
        .tooltip_show_delay(palette::motion(400))
}

pub fn input(
    state: &Entity<gpui_kit::component::input::InputState>,
) -> gpui_kit::component::input::Input {
    gpui_kit::component::input::Input::new(state)
        .h(px(palette::CTL_MD))
        .rounded(px(palette::R_INPUT))
        .text_size(palette::fs(palette::FS_BASE))
}

pub fn stat_chip(
    id: impl Into<ElementId>,
    p: &Palette,
    name: IconName,
    count: usize,
    label: &str,
    danger: bool,
) -> Button {
    button(id)
        .ghost()
        .rounded_full()
        .h(px(palette::CHIP_MD))
        .icon(icon(name, palette::ICON_SM))
        .label(format!("{count} {label}"))
        .text_color(if count == 0 {
            p.text_tertiary
        } else if danger {
            p.danger
        } else {
            p.text_secondary
        })
}

// The complete tone palette is exercised by the debug-only styleguide.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ButtonTone {
    Primary,
    Secondary,
    Ghost,
    Success,
    Warning,
    Danger,
    SolidDanger,
}

pub fn toned_button(
    id: impl Into<ElementId>,
    p: &Palette,
    tone: ButtonTone,
    height: f32,
) -> Button {
    let b = button(id)
        .with_size(px(height))
        .px(px(if height == palette::CTL_SM {
            palette::CONTROL_PAD_SM
        } else if height == palette::CTL_LG {
            palette::CONTROL_PAD_LG
        } else {
            palette::CONTROL_PAD_MD
        }))
        .rounded(px(if height == palette::CTL_SM {
            palette::R_CONTROL_SM
        } else {
            palette::R_INPUT
        }))
        .text_size(palette::fs(if height == palette::CTL_SM {
            palette::FS_CONTROL_SM
        } else if height == palette::CTL_LG {
            palette::FS_MD
        } else {
            palette::FS_BASE
        }));
    match tone {
        ButtonTone::Primary => b.primary().bg(p.accent).text_color(p.accent_contrast),
        ButtonTone::Secondary => b.outline().bg(p.bg_elevated).text_color(p.text_primary),
        ButtonTone::Ghost => b.ghost().text_color(p.text_secondary),
        ButtonTone::SolidDanger => b.danger().bg(p.danger),
        ButtonTone::Success | ButtonTone::Warning | ButtonTone::Danger => {
            let color = match tone {
                ButtonTone::Success => p.done,
                ButtonTone::Warning => p.planned,
                _ => p.danger,
            };
            b.ghost().bg(color.opacity(0.14)).text_color(color)
        }
    }
}

#[cfg(debug_assertions)]
pub fn styleguide(state: &Entity<gpui_kit::component::input::InputState>) -> impl IntoElement {
    let mut page = div()
        .id("ui-styleguide")
        .size_full()
        .overflow_y_scrollbar()
        .flex()
        .flex_col();
    for (theme, p) in [("Dark", Palette::dark()), ("Light", Palette::light())] {
        let mut section = div()
            .p(px(palette::SP_6))
            .bg(p.bg_app)
            .text_color(p.text_primary)
            .flex()
            .flex_col()
            .gap(px(palette::SP_3))
            .child(format!("UI-System · {theme} · Strg+Umschalt+D schließen"));
        for (n, tone) in [
            ButtonTone::Primary,
            ButtonTone::Secondary,
            ButtonTone::Ghost,
            ButtonTone::Success,
            ButtonTone::Warning,
            ButtonTone::Danger,
            ButtonTone::SolidDanger,
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(palette::SP_2));
            for (i, height) in [palette::CTL_SM, palette::CTL_MD, palette::CTL_LG]
                .into_iter()
                .enumerate()
            {
                for state in 0..3 {
                    row = row.child(
                        toned_button(
                            SharedString::from(format!("{theme}-{n}-{i}-{state}")),
                            &p,
                            tone,
                            height,
                        )
                        .label(["Speichern", "Deaktiviert", "Laden"][state])
                        .disabled(state == 1)
                        .loading(state == 2),
                    );
                }
            }
            section = section.child(row);
        }
        section = section.child(
            div().flex().items_center().gap(px(palette::SP_3)).children(
                [
                    palette::ICON_XS,
                    palette::ICON_SM,
                    palette::ICON_MD,
                    palette::ICON_LG,
                    palette::ICON_XL,
                    palette::ICON_2XL,
                ]
                .map(|size| icon(icons::SEARCH, size).text_color(p.text_primary)),
            ),
        );
        section = section.child(
            div()
                .flex()
                .gap(px(palette::SP_2))
                .child(keycaps(&p, "Strg+Umschalt+D"))
                .child(stat_chip(
                    SharedString::from(format!("{theme}-stat")),
                    &p,
                    icons::INBOX,
                    0,
                    "Inbox",
                    false,
                )),
        );
        section = section.child(input(state).aria_label("Styleguide Eingabe"));
        page = page.child(section);
    }
    page
}

/// Card link with a trailing arrow, calm typography and a fixed height.
pub fn link_button(id: impl Into<ElementId>, p: &Palette, label: &'static str) -> Button {
    button(id)
        .ghost()
        .with_size(gpui_kit::component::Size::Small)
        .h(px(palette::CTL_SM))
        .text_size(palette::fs(palette::FS_CONTROL_SM))
        .text_color(p.text_secondary)
        .child(label)
        .child(icon(icons::FORWARD, palette::ICON_SM))
}

/// Status shapes are distinguishable even without color perception.
pub fn status_icon(p: &Palette, status: Status, (done, total): (usize, usize)) -> Div {
    let color = p.status(status);
    div().size(px(palette::ICON_MD)).flex_none().child(
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let center = point(bounds.origin.x + px(8.), bounds.origin.y + px(8.));
                let ratio = if total == 0 {
                    0.5
                } else {
                    done as f32 / total as f32
                };
                let fraction = if status == Status::InArbeit {
                    ratio.max(0.04)
                } else {
                    1.
                };
                for layer in 0..if status == Status::InArbeit { 2 } else { 1 } {
                    let mut path = PathBuilder::stroke(px(1.75));
                    let amount = if layer == 0 { 1. } else { fraction };
                    for i in 0..=64 {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + std::f32::consts::TAU * amount * i as f32 / 64.;
                        let pt = point(
                            center.x + px(angle.cos() * 6.),
                            center.y + px(angle.sin() * 6.),
                        );
                        if i == 0 || (status == Status::Idee && i % 8 == 0) {
                            path.move_to(pt);
                        } else if status != Status::Idee || i % 8 < 5 {
                            path.line_to(pt);
                        }
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(
                            path,
                            if status == Status::InArbeit && layer == 0 {
                                color.opacity(0.25)
                            } else {
                                color
                            },
                        );
                    }
                }
                if status == Status::Erledigt {
                    let mut path = PathBuilder::stroke(px(1.75));
                    path.move_to(point(center.x - px(3.), center.y));
                    path.line_to(point(center.x - px(1.), center.y + px(2.)));
                    path.line_to(point(center.x + px(3.), center.y - px(2.)));
                    if let Ok(path) = path.build() {
                        window.paint_path(path, color);
                    }
                }
            },
        )
        .size_full(),
    )
}

pub fn card_header(p: &Palette, title: &'static str, action: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(palette::SP_2))
        .child(
            div()
                .text_size(palette::fs(palette::FS_MD))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(p.text_primary)
                .child(title),
        )
        .child(action)
}

/// Priority bars (16 px): count, color and tooltip word together convey the level.
pub fn priority_icon(id: impl Into<ElementId>, p: &Palette, priority: u8) -> Stateful<Div> {
    let level = priority.min(3) as usize;
    let on = match level {
        1 => p.prio_low,
        2 => p.prio_mid,
        _ => p.prio_high,
    };
    div()
        .id(id)
        .size(px(palette::ICON_MD))
        .flex_none()
        .flex()
        .items_end()
        .gap(px(2.))
        .tooltip(tip(format!(
            "Priorität: {}",
            ["Keine", "Niedrig", "Mittel", "Hoch"][level]
        )))
        .children((0..3).map(|i| {
            div()
                .w(px(3.))
                .h(px(5. + i as f32 * 4.))
                .rounded(px(1.))
                .bg(if i < level { on } else { p.prio_off })
        }))
}

pub fn task_checkbox(id: impl Into<ElementId>, p: &Palette, on: bool) -> Button {
    button(id)
        .ghost()
        .size(px(palette::ROW_LEADING_SLOT))
        .p(px(0.))
        .role(Role::CheckBox)
        .toggled(on)
        .accessibility_label(if on {
            "Aufgabe wieder öffnen"
        } else {
            "Aufgabe erledigen"
        })
        .child(
            div()
                .size(px(palette::ICON_LG))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .border_1()
                .border_color(if on { p.done } else { p.border_strong })
                .when(on, |d| {
                    d.bg(p.done)
                        .child(icon(icons::CONFIRM, palette::ICON_SM).text_color(p.bg_app))
                }),
        )
}

/// Meta-Zeile (12 px, `text-secondary`), Teile getrennt durch " · ".
pub fn meta_line(p: &Palette, parts: Vec<AnyElement>) -> Div {
    let mut line = div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(palette::SP_1))
        .text_size(palette::fs(palette::FS_SM))
        .font_weight(FontWeight::NORMAL)
        .text_color(p.text_secondary);
    for (n, part) in parts.into_iter().enumerate() {
        if n > 0 {
            line = line.child(div().flex_none().text_color(p.text_tertiary).child("·"));
        }
        line = line.child(part);
    }
    line
}

/// Shared leading slot of all rows: checkbox and status icon sit on the same title edge.
pub fn row_leading(child: impl IntoElement) -> Div {
    div()
        .size(px(palette::ROW_LEADING_SLOT))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(child)
}

/// Actions reserve their space even without hover: titles, dates and menus never overlap.
/// Visible on row hover, focus inside the container or explicit selection.
#[derive(IntoElement)]
pub struct RowActions {
    id: ElementId,
    group: SharedString,
    shown: bool,
    children: Vec<AnyElement>,
}

pub fn row_actions(id: impl Into<ElementId>, group: SharedString) -> RowActions {
    RowActions {
        id: id.into(),
        group,
        shown: false,
        children: Vec::new(),
    }
}

impl RowActions {
    pub fn shown(mut self, shown: bool) -> Self {
        self.shown = shown;
        self
    }
}

impl ParentElement for RowActions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for RowActions {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let visible = self.shown || handle.within_focused(window, cx);
        div()
            .id(self.id)
            .track_focus(&handle)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(palette::SP_1))
            .when(!visible, |d| {
                d.opacity(0.).group_hover(self.group, |s| s.opacity(1.))
            })
            .children(self.children)
    }
}

/// Native button for navigation, rows and multi-line calendar cells.
/// Unlike the labeled toolbar button, the child layout is preserved.
pub fn surface(id: impl Into<ElementId>, p: &Palette) -> gpui_kit::base::Button {
    gpui_kit::base::Button::new(id)
        .cursor_pointer()
        .line_height(relative(1.4))
        .hover(|s| s.bg(p.bg_hover))
        .active(|s| s.bg(p.bg_active))
        .focus_visible(|s| s.border_2().border_color(p.accent))
}
