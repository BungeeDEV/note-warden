//! Persistent presentation preferences. Notes remain ordinary workspace updates.
use serde::{Deserialize, Serialize};

pub const DEFAULT_SIZE: (f32, f32) = (420., 620.);
pub const MIN_SIZE: (f32, f32) = (320., 360.);

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CompactSettings {
    pub enabled: bool,
    pub pinned: bool,
    pub bounds: Option<(f32, f32, f32, f32)>,
}

impl CompactSettings {
    pub fn size(&self) -> (f32, f32) {
        self.bounds
            .filter(|(_, _, w, h)| w.is_finite() && h.is_finite())
            .map(|(_, _, w, h)| (w.clamp(MIN_SIZE.0, 760.), h.clamp(MIN_SIZE.1, 1200.)))
            .unwrap_or(DEFAULT_SIZE)
    }

    /// Recover a saved origin only if its title bar is still reachable on a monitor.
    pub fn origin(&self, displays: &[(f32, f32, f32, f32)]) -> Option<(f32, f32)> {
        let (x, y, _, _) = self.bounds?;
        let (w, _) = self.size();
        (x.is_finite()
            && y.is_finite()
            && displays.iter().any(|&(dx, dy, dw, dh)| {
                x + w.min(96.) > dx && x < dx + dw - 64. && y >= dy && y + 32. < dy + dh
            }))
        .then_some((x, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_preferences_default_to_normal_unpinned_window() {
        let prefs: CompactSettings = serde_json::from_str("{}").unwrap();
        assert!(!prefs.enabled && !prefs.pinned);
        assert_eq!(prefs.size(), DEFAULT_SIZE);
    }

    #[test]
    fn corrupt_sizes_are_bounded_and_disconnected_monitors_are_recovered() {
        let mut prefs = CompactSettings {
            bounds: Some((4000., 0., 10., 9000.)),
            ..Default::default()
        };
        assert_eq!(prefs.size(), (320., 1200.));
        assert_eq!(prefs.origin(&[(0., 0., 1920., 1080.)]), None);
        prefs.bounds = Some((-1000., 100., 420., 620.));
        assert_eq!(
            prefs.origin(&[(-1920., 0., 1920., 1080.)]),
            Some((-1000., 100.))
        );
        prefs.bounds = Some((0., -500., f32::NAN, f32::INFINITY));
        assert_eq!(prefs.size(), DEFAULT_SIZE);
        assert_eq!(prefs.origin(&[(0., 0., 1920., 1080.)]), None);
    }
}
