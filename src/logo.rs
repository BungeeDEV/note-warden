//! Pixel-Schild (gleiche Geometrie wie `branding/make-logo.mjs`), direkt als scharfe
//! Quads gezeichnet: kein Anti-Aliasing, Zellgröße immer ganzzahlig.

use gpui_kit::*;

const EXT: &[(i32, i32, i32)] = &[
    (7, 8, 19),
    (8, 8, 20),
    (9, 8, 21),
    (10, 8, 22),
    (19, 9, 22),
    (20, 9, 22),
    (21, 10, 21),
    (22, 10, 21),
    (23, 11, 20),
    (24, 12, 19),
    (25, 13, 18),
    (26, 14, 17),
    (27, 15, 16),
];

/// Schild belegt Zellen x 8..24, y 7..28 (16 × 21); der Schatten ragt eine Zelle darüber hinaus.
pub const COLS: i32 = 16;
pub const ROWS: i32 = 21;
const X0: i32 = 8;
const Y0: i32 = 7;

type Rgb = [u8; 3];

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let f = |i: usize| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t).round() as u8;
    [f(0), f(1), f(2)]
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Body,
    Detail, // Notizzeilen und Haken
}

struct Cell {
    x: i32,
    y: i32,
    color: Rgb,
    role: Role,
    check: bool,
}

fn cells(small: bool) -> Vec<Cell> {
    let top: Rgb = [0x9b, 0xe3, 0xff];
    let bot: Rgb = [0x0a, 0x7f, 0xc4];
    let mut map: std::collections::BTreeMap<(i32, i32), (Rgb, Role, bool)> = Default::default();
    let mut rows: Vec<(i32, i32, i32)> = EXT.to_vec();
    for r in 11..=18 {
        rows.push((r, 8, 23));
    }
    for (r, a, b) in rows {
        let t = (((r - 7) as f32 / 20.) * 5.).round() / 5.;
        let base = lerp(top, bot, t);
        for x in a..=b {
            let mut col = base;
            if x == a && r <= 22 {
                col = lerp(base, [255, 255, 255], 0.22);
            }
            if x == b && r <= 22 {
                col = lerp(base, [0, 20, 50], 0.16);
            }
            map.insert((x, r), (col, Role::Body, false));
        }
    }
    for (x, y) in [(19, 8), (19, 9), (19, 10), (20, 11), (21, 11), (22, 11)] {
        if let Some(entry) = map.get_mut(&(x, y)) {
            entry.0 = lerp(entry.0, [0, 40, 80], 0.28);
        }
    }
    for (x, y) in [(20, 9), (20, 10), (21, 10)] {
        map.insert((x, y), ([0xe9, 0xf8, 0xff], Role::Body, false));
    }
    for (x, y) in [(20, 8), (21, 9), (22, 10)] {
        map.insert((x, y), ([0xbf, 0xe7, 0xfa], Role::Body, false));
    }
    let dark: Rgb = [0x0a, 0x16, 0x20];
    let mid: Rgb = [0x1d, 0x5a, 0x80];
    for x in 11..=19 {
        map.insert((x, 11), (dark, Role::Detail, false));
    }
    if !small {
        for x in 11..=16 {
            map.insert((x, 13), (mid, Role::Detail, false));
        }
    }
    for (x, y) in [
        (11, 17),
        (12, 18),
        (13, 19),
        (14, 20),
        (15, 19),
        (16, 18),
        (17, 17),
        (18, 16),
        (19, 15),
        (20, 14),
    ] {
        map.insert((x, y), (dark, Role::Detail, true));
        map.insert((x + 1, y), (dark, Role::Detail, true));
    }
    map.into_iter()
        .map(|((x, y), (color, role, check))| Cell {
            x,
            y,
            color,
            role,
            check,
        })
        .collect()
}

/// Darstellung des Schilds.
#[derive(Clone, Copy)]
pub struct LogoStyle {
    /// Ganzzahlige Zellgröße in logischen Pixeln.
    pub cell: f32,
    /// Kleine Variante (ohne zweite Notizzeile, ohne Schatten).
    pub small: bool,
    /// Harter Schatten eine Zelle nach unten rechts.
    pub shadow: bool,
    /// Skalierung des Hakens (1.0 = normal) für den Splash.
    pub check_scale: f32,
    /// Einfarbig (Leerzustände): Fläche gedämpft, Details voll.
    pub mono: Option<Hsla>,
}

impl LogoStyle {
    pub fn new(cell: f32) -> Self {
        Self {
            cell: cell.max(1.).floor(),
            small: cell < 3.,
            shadow: cell >= 3.,
            check_scale: 1.,
            mono: None,
        }
    }

    pub fn mono(mut self, color: Hsla) -> Self {
        self.mono = Some(color);
        self.shadow = false;
        self
    }

    pub fn check_scale(mut self, scale: f32) -> Self {
        self.check_scale = scale;
        self
    }

    pub fn width(&self) -> f32 {
        (COLS + self.shadow as i32) as f32 * self.cell
    }

    pub fn height(&self) -> f32 {
        (ROWS + self.shadow as i32) as f32 * self.cell
    }
}

fn to_hsla(c: Rgb) -> Hsla {
    rgb(((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32).into()
}

/// Das Schild als Element mit fester, ganzzahliger Größe.
pub fn shield(style: LogoStyle) -> impl IntoElement {
    let data = cells(style.small);
    div()
        .w(px(style.width()))
        .h(px(style.height()))
        .flex_none()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    let origin = bounds.origin;
                    let s = style.cell;
                    let rect = |x: f32, y: f32, w: f32, h: f32| {
                        Bounds::new(
                            point(origin.x + px(x), origin.y + px(y)),
                            size(px(w), px(h)),
                        )
                    };
                    if style.shadow {
                        let occupied: std::collections::HashSet<(i32, i32)> =
                            data.iter().map(|c| (c.x, c.y)).collect();
                        for c in &data {
                            let (sx, sy) = (c.x + 1, c.y + 1);
                            if !occupied.contains(&(sx, sy)) {
                                window.paint_quad(fill(
                                    rect((sx - X0) as f32 * s, (sy - Y0) as f32 * s, s, s),
                                    to_hsla([0x02, 0x08, 0x0c]).opacity(0.32),
                                ));
                            }
                        }
                    }
                    for c in &data {
                        let color = match style.mono {
                            Some(m) => {
                                if c.role == Role::Body {
                                    m.opacity(0.28)
                                } else {
                                    m.opacity(0.9)
                                }
                            }
                            None => to_hsla(c.color),
                        };
                        let (mut x, mut y, mut w) =
                            ((c.x - X0) as f32 * s, (c.y - Y0) as f32 * s, s);
                        if c.check && (style.check_scale - 1.).abs() > f32::EPSILON {
                            // Haken wächst um seine Mitte (Zelle 15,17 relativ zum Schild).
                            let (cx, cy) = ((15 - X0) as f32 * s, (17 - Y0) as f32 * s);
                            x = cx + (x - cx) * style.check_scale;
                            y = cy + (y - cy) * style.check_scale;
                            w = s * style.check_scale;
                        }
                        window.paint_quad(fill(
                            rect(x.round(), y.round(), w.ceil(), w.ceil()),
                            color,
                        ));
                    }
                },
            )
            .size_full(),
        )
}
