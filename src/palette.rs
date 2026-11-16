//! Zentrale Design-Tokens. Komponenten lesen Farben ausschließlich hier
//! (`palette::get(cx)`); Abstände/Radien/Schriftgrößen sind unten als Konstanten.

use crate::model::{Accent, Density, FontScale, Settings, Status};
use gpui_kit::component::theme::{Theme, ThemeMode};
use gpui_kit::*;
use std::sync::atomic::{AtomicU32, Ordering};

// Spacing-Skala (4/8/12/16/24/32/48) und Control-Höhen (28/32/40).
pub const SP_1: f32 = 4.;
pub const SP_2: f32 = 8.;
pub const SP_3: f32 = 12.;
pub const SP_4: f32 = 16.;
pub const SP_6: f32 = 24.;
pub const SP_8: f32 = 32.;
/// Rechter Innenabstand scrollender Listen: Scrollbar-Daumen liegt als Overlay darüber und darf keinen Inhalt verdecken.
pub const SCROLL_GUTTER: f32 = SP_3;
pub const CTL_SM: f32 = 28.;
pub const CTL_MD: f32 = 32.;
#[allow(dead_code)]
pub const CTL_LG: f32 = 40.;

static SCALE_BITS: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0
pub static SETTINGS_STACKED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static COMPACT: AtomicU32 = AtomicU32::new(0);

/// Schriftgröße mit globaler Skalierung (Einstellung „Schriftgröße“).
pub fn fs(base: f32) -> Pixels {
    px((base * f32::from_bits(SCALE_BITS.load(Ordering::Relaxed))).round())
}

/// Zeilen-/Control-Höhen und Abstände; im kompakten Modus ca. 12 % kleiner.
pub fn dens(value: f32) -> Pixels {
    if COMPACT.load(Ordering::Relaxed) == 1 {
        px((value * 0.875).round())
    } else {
        px(value)
    }
}

pub fn set_metrics(font: FontScale, density: Density) {
    SCALE_BITS.store(font.factor().to_bits(), Ordering::Relaxed);
    COMPACT.store((density == Density::Compact) as u32, Ordering::Relaxed);
}

pub fn luminance(color: Hsla) -> f32 {
    let rgba = color.to_rgb();
    let lin = |v: f32| {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(rgba.r) + 0.7152 * lin(rgba.g) + 0.0722 * lin(rgba.b)
}

pub fn contrast(a: Hsla, b: Hsla) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Verschiebt die Helligkeit, bis `color` auf `bg` mindestens `min`:1 erreicht.
fn fit(mut color: Hsla, bg: Hsla, min: f32, lighten: bool) -> Hsla {
    for _ in 0..100 {
        if contrast(color, bg) >= min {
            break;
        }
        color.l = (color.l + if lighten { 0.01 } else { -0.01 }).clamp(0., 1.);
    }
    color
}

// Typografie-Skala
pub const FS_SM: f32 = 12.;
pub const FS_BASE: f32 = 13.;
pub const FS_MD: f32 = 14.;
pub const FS_LG: f32 = 16.;
pub const FS_XL: f32 = 20.;
pub const FS_TITLE: f32 = 24.;

// Radien
pub const R_INPUT: f32 = 8.;
pub const R_CARD: f32 = 12.;
pub const R_DIALOG: f32 = 14.;

pub const FONT_UI: &str = "Segoe UI Variable Text";
pub const FONT_MONO: &str = "Cascadia Mono";

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg_app: Hsla,
    pub bg_sidebar: Hsla,
    pub bg_panel: Hsla,
    pub bg_elevated: Hsla,
    pub bg_hover: Hsla,
    pub bg_active: Hsla,
    pub border_subtle: Hsla,
    pub border_strong: Hsla,
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_pressed: Hsla,
    pub accent_contrast: Hsla,
    pub idea: Hsla,
    pub planned: Hsla,
    pub progress: Hsla,
    pub done: Hsla,
    pub danger: Hsla,
    /// Prioritätsbalken: ungefüllt, Niedrig, Mittel, Hoch (je mindestens 3:1, siehe `build`).
    pub prio_off: Hsla,
    pub prio_low: Hsla,
    pub prio_mid: Hsla,
    pub prio_high: Hsla,
    /// Hover-/Auswahlfläche und Rand in Menüs und Popovern.
    pub bg_menu_hover: Hsla,
    pub border_popover: Hsla,
}

impl Global for Palette {}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

impl Palette {
    pub fn dark() -> Self {
        Self::build(true, Accent::Sky)
    }

    pub fn light() -> Self {
        Self::build(false, Accent::Sky)
    }

    pub fn build(dark: bool, choice: Accent) -> Self {
        let mut p = if dark {
            Self::dark_base()
        } else {
            Self::light_base()
        };
        let base: Hsla = c(choice.base());
        // Akzent muss als Text/Linie auf allen Flächen AA erreichen.
        let worst_bg = p.bg_hover;
        let accent = if dark {
            fit(base, worst_bg, 4.5, true)
        } else {
            let mut start = base;
            start.l = start.l.min(0.42);
            fit(start, worst_bg, 4.5, false)
        };
        let shift = |amount: f32| {
            let mut hover = accent;
            hover.l = (hover.l + amount).clamp(0., 1.);
            hover
        };
        p.accent = accent;
        p.accent_hover = if dark { shift(0.08) } else { shift(-0.06) };
        p.accent_pressed = if dark { shift(-0.08) } else { shift(-0.10) };
        let (light_text, dark_text) = (c(0xffffff), c(0x04141c));
        p.accent_contrast = if contrast(light_text, accent) >= contrast(dark_text, accent) {
            light_text
        } else {
            dark_text
        };
        p.bg_active = if dark { c(0x292a30) } else { c(0xe8e8ed) };
        // Prioritätsbalken: ungefüllt 3:1 gegen die Hover-Fläche, gefüllt 3:1 gegen ungefüllt.
        let toward_text = dark;
        p.prio_off = fit(p.border_strong, p.bg_hover, 3., toward_text);
        p.prio_low = fit(p.text_secondary, p.prio_off, 3., toward_text);
        p.prio_mid = fit(p.text_primary, p.prio_off, 3., toward_text);
        p.prio_high = fit(
            c(if dark { 0xfb923c } else { 0xea580c }),
            p.prio_off,
            3.,
            toward_text,
        );
        // Menüs: Hover klar von der Popover-Fläche getrennt, Rand und Trennlinien 3:1.
        p.bg_menu_hover = fit(p.bg_hover, p.bg_elevated, 1.25, dark);
        p.border_popover = fit(p.border_strong, p.bg_elevated, 3., dark);
        // Status bleibt unabhängig vom gewählten Interaktionsakzent.
        p
    }

    fn dark_base() -> Self {
        let accent = c(0x38bdf8);
        Self {
            dark: true,
            bg_app: c(0x0f1012),
            bg_sidebar: c(0x121316),
            bg_panel: c(0x16171a),
            bg_elevated: c(0x1c1d21),
            bg_hover: c(0x212227),
            bg_active: accent.opacity(0.14),
            border_subtle: c(0x26272c),
            border_strong: c(0x34363c),
            text_primary: c(0xececee),
            text_secondary: c(0xa1a1aa),
            // #71717a erreicht auf dem dunklen Grund nur ~3,9:1 (< AA); heller gewählt.
            text_tertiary: c(0x8b8b96),
            accent,
            accent_hover: c(0x7dd3fc),
            accent_pressed: c(0x0ea5e9),
            accent_contrast: c(0x04141c),
            idea: c(0xa78bfa),
            planned: c(0xfbbf24),
            progress: accent,
            done: c(0x4ade80),
            danger: c(0xf87171),
            prio_off: c(0x68686f),
            prio_low: c(0xc6c6ce),
            prio_mid: c(0xececee),
            prio_high: c(0xfb923c),
            bg_menu_hover: c(0x2d2f36),
            border_popover: c(0x6a6c74),
        }
    }

    fn light_base() -> Self {
        let accent = c(0x0369a1);
        Self {
            dark: false,
            bg_app: c(0xfafafa),
            bg_sidebar: c(0xf3f3f5),
            bg_panel: c(0xffffff),
            bg_elevated: c(0xffffff),
            bg_hover: c(0xeeeef1),
            bg_active: accent.opacity(0.12),
            border_subtle: c(0xe4e4e8),
            border_strong: c(0xcfcfd6),
            text_primary: c(0x18181b),
            text_secondary: c(0x52525b),
            text_tertiary: c(0x65656f),
            accent,
            accent_hover: c(0x075985),
            accent_pressed: c(0x0c4a6e),
            accent_contrast: c(0xffffff),
            idea: c(0x7c3aed),
            planned: c(0x92400e),
            progress: accent,
            done: c(0x15803d),
            danger: c(0xdc2626),
            prio_off: c(0x8c8c95),
            prio_low: c(0x3f3f46),
            prio_mid: c(0x18181b),
            prio_high: c(0x9a3412),
            bg_menu_hover: c(0xe0e0e6),
            border_popover: c(0x8c8c95),
        }
    }

    pub fn status(&self, status: Status) -> Hsla {
        match status {
            Status::Idee => self.idea,
            Status::Geplant => self.planned,
            Status::InArbeit => self.progress,
            Status::Erledigt => self.done,
        }
    }
}

pub fn get(cx: &App) -> Palette {
    *cx.global::<Palette>()
}

/// Setzt Palette und Theme der Komponentenbibliothek.
pub fn apply(dark: bool, settings: &Settings, cx: &mut App) {
    set_metrics(settings.font_scale, settings.density);
    let p = Palette::build(dark, settings.accent);
    cx.set_global(p);
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |theme| {
        theme.scrollbar_mode = gpui_kit::component::scroll::ScrollbarMode::Always;
        // Track unsichtbar, Daumen ruhig (3:1 gegen Popover-Fläche), Hover deutlicher.
        theme.colors.scrollbar = gpui_kit::transparent_black();
        theme.tokens.scrollbar_thumb = p.border_popover.into();
        theme.tokens.scrollbar_thumb_hover = p.text_tertiary.into();
        theme.font_family = FONT_UI.into();
        theme.mono_font_family = FONT_MONO.into();
        theme.font_size = fs(FS_BASE);
        theme.radius = px(R_INPUT);
        theme.radius_lg = px(R_CARD);
        let colors = &mut theme.colors;
        colors.background = p.bg_app;
        colors.foreground = p.text_primary;
        colors.border = p.border_popover;
        colors.input = p.border_strong;
        colors.muted = p.bg_hover;
        colors.muted_foreground = p.text_secondary;
        colors.popover = p.bg_elevated;
        colors.popover_foreground = p.text_primary;
        colors.primary = p.accent;
        colors.primary_hover = p.accent_hover;
        colors.primary_active = p.accent_pressed;
        colors.primary_foreground = p.accent_contrast;
        colors.button_primary = p.accent;
        colors.button_primary_hover = p.accent_hover;
        colors.button_primary_active = p.accent_pressed;
        colors.button_primary_foreground = p.accent_contrast;
        colors.accent = p.bg_menu_hover;
        colors.accent_foreground = p.text_primary;
        colors.secondary = p.bg_hover;
        colors.secondary_foreground = p.text_primary;
        colors.ring = p.accent;
        colors.caret = p.accent;
        colors.link = p.accent;
        colors.link_hover = p.accent_hover;
        colors.link_active = p.accent_pressed;
        colors.danger = p.danger;
        colors.selection = p.accent.opacity(0.3);
        colors.sidebar = p.bg_sidebar;
        colors.sidebar_border = p.border_subtle;
        colors.sidebar_foreground = p.text_primary;
        colors.progress_bar = p.accent;
    });
}

// UI-System: feste Größen unabhängig von der Inhaltsdichte.
pub const ICON_XS: f32 = 12.;
pub const ICON_SM: f32 = 14.;
pub const ICON_MD: f32 = 16.;
pub const ICON_LG: f32 = 18.;
pub const ICON_XL: f32 = 20.;
pub const ICON_2XL: f32 = 48.;
pub const ROW_COMPACT: f32 = 36.;
#[allow(dead_code)]
pub const ROW_DEFAULT: f32 = 44.;
pub const ROW_COMFORTABLE: f32 = 56.;
pub const CHIP_MD: f32 = 28.;
pub const R_CONTROL_SM: f32 = 6.;
pub const FS_CONTROL_SM: f32 = 12.5;
pub const ICON_GAP: f32 = 6.;

pub const DETAIL_TITLE: f32 = 28.;
pub const DETAIL_MAX_W: f32 = 720.;
pub const MENU_RADIUS: f32 = 10.;
pub const MIN_HIT: f32 = 32.;
/// Gemeinsamer Leading-Slot aller Zeilen (Checkbox, Status-Icon); danach `SP_2` bis zum Titel.
pub const ROW_LEADING_SLOT: f32 = 24.;
pub const CONTROL_PAD_SM: f32 = 10.;
pub const CONTROL_PAD_MD: f32 = 12.;
pub const CONTROL_PAD_LG: f32 = 16.;

// Radien (Ergänzung) und Layout-Schwellen.
pub const R_XS: f32 = 4.;
pub const SIDEBAR_COLLAPSE_W: f32 = 1100.;

// Elevation: Panel = Rand subtil · Karte = Rand strong · Popover/Toast = `shadow_lg` · Dialog = `shadow_lg` + Scrim.
// Bewegung: kurz 120 ms, normal 200 ms, lang 300 ms; Auslaufkurve kritisch gedämpft (kein Überschwingen).
pub const MOTION_FAST_MS: u64 = 120;
#[allow(dead_code)]
pub const MOTION_NORMAL_MS: u64 = 200;
pub const MOTION_SLOW_MS: u64 = 300;
pub const MOTION_INTRO_MS: u64 = 700;
pub const TOAST_MS: u64 = 6000;

pub fn motion(ms: u64) -> std::time::Duration {
    std::time::Duration::from_millis(ms)
}

/// Kritisch gedämpfte Auslaufkurve (ease-out, quint).
pub fn ease_out(t: f32) -> f32 {
    1. - (1. - t).powi(5)
}

/// Feste Palette für Projektfarben (klar unterscheidbare Töne).
pub const PROJECT_PALETTE: [u32; 8] = [
    0x38bdf8, 0xa78bfa, 0xf472b6, 0xfb923c, 0x4ade80, 0x2dd4bf, 0xfacc15, 0xf87171,
];

/// Startfarbe für ein neues Projekt: deterministisch aus der Projekt-ID.
pub fn default_project_color(id: u64) -> u32 {
    PROJECT_PALETTE[(id % PROJECT_PALETTE.len() as u64) as usize]
}

/// Einzige Quelle der Projektfarbe: gespeicherter Wert, sonst Palette per ID; mindestens 3:1 gegen Hover-Fläche.
pub fn project_color(project: &crate::model::Project, p: &Palette) -> Hsla {
    let base = if project.color == 0 {
        default_project_color(project.id)
    } else {
        project.color
    };
    fit(c(base), p.bg_hover, 3., p.dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn secondary_and_tertiary_text_meet_wcag_aa() {
        for p in [Palette::dark(), Palette::light()] {
            for bg in [
                p.bg_app,
                p.bg_sidebar,
                p.bg_panel,
                p.bg_elevated,
                p.bg_hover,
            ] {
                for (name, fg) in [
                    ("primary", p.text_primary),
                    ("secondary", p.text_secondary),
                    ("tertiary", p.text_tertiary),
                ] {
                    let ratio = contrast(fg, bg);
                    assert!(ratio >= 4.5, "{name} dark={} {ratio}", p.dark);
                }
            }
        }
    }

    #[::core::prelude::v1::test]
    fn every_accent_reaches_aa_in_both_themes() {
        for dark in [true, false] {
            for choice in Accent::ALL {
                let p = Palette::build(dark, choice);
                for bg in [
                    p.bg_app,
                    p.bg_sidebar,
                    p.bg_panel,
                    p.bg_elevated,
                    p.bg_hover,
                ] {
                    assert!(
                        contrast(p.accent, bg) >= 4.5,
                        "{} dark={dark} text on bg",
                        choice.label()
                    );
                }
                assert!(
                    contrast(p.accent_contrast, p.accent) >= 4.5,
                    "{} dark={dark} contrast text",
                    choice.label()
                );
            }
        }
    }

    #[::core::prelude::v1::test]
    fn status_and_accent_colors_are_readable() {
        for p in [Palette::dark(), Palette::light()] {
            for color in [p.accent, p.idea, p.planned, p.done, p.danger] {
                assert!(contrast(color, p.bg_panel) >= 4.5, "dark={}", p.dark);
            }
        }
    }
}
