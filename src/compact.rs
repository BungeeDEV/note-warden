//! Persistent presentation preferences. Notes remain ordinary workspace updates.
use serde::{Deserialize, Serialize};

pub const DEFAULT_SIZE: (f32, f32) = (420., 620.);
pub const MIN_SIZE: (f32, f32) = (320., 360.);

#[derive(Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StickyStartView {
    #[default]
    Overview,
    LastUpdate,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CompactSettings {
    pub enabled: bool,
    pub pinned: bool,
    pub bounds: Option<(f32, f32, f32, f32)>,
    pub hide_completed: bool,
    pub compact_cards: bool,
    pub start_view: StickyStartView,
    /// Independent of the update selected in the main window.
    pub last_update: Option<u64>,
}

impl CompactSettings {
    /// Missing, archived, trashed or currently hidden entries fall back to the overview.
    pub fn restored_update(&self, updates: &[crate::model::Update]) -> Option<u64> {
        if self.start_view != StickyStartView::LastUpdate {
            return None;
        }
        self.last_update.filter(|id| {
            updates.iter().any(|u| {
                u.id == *id
                    && u.is_live()
                    && (!self.hide_completed || u.status != crate::model::Status::Erledigt)
            })
        })
    }

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
        assert!(!prefs.hide_completed && !prefs.compact_cards);
        assert!(prefs.start_view == StickyStartView::Overview && prefs.last_update.is_none());
        assert_eq!(prefs.size(), DEFAULT_SIZE);
    }

    #[test]
    fn sticky_start_is_independent_of_main_selection_and_rejects_hidden_entries() {
        use crate::model::{Status, Workspace};
        let mut ws = Workspace::default();
        let sticky = ws.new_update(None);
        let main = ws.new_update(None);
        ws.settings.last_update = Some(main);
        ws.settings.compact.last_update = Some(sticky);
        assert_eq!(ws.settings.compact.restored_update(&ws.updates), None);
        ws.settings.compact.start_view = StickyStartView::LastUpdate;
        ws.settings.compact.compact_cards = true;
        ws.settings.compact.hide_completed = true;
        ws.settings.compact.bounds = Some((100., 200., 380., 620.));
        let prefs: CompactSettings =
            serde_json::from_str(&serde_json::to_string(&ws.settings.compact).unwrap()).unwrap();
        assert!(prefs.compact_cards && prefs.hide_completed);
        assert_eq!(prefs.size(), (380., 620.));
        assert_eq!(prefs.restored_update(&ws.updates), Some(sticky));
        ws.update_mut(sticky).unwrap().archived = true;
        assert_eq!(prefs.restored_update(&ws.updates), None);
        ws.update_mut(sticky).unwrap().archived = false;
        ws.update_mut(sticky).unwrap().deleted_on = Some(crate::model::today());
        assert_eq!(prefs.restored_update(&ws.updates), None);
        ws.update_mut(sticky).unwrap().deleted_on = None;
        ws.update_mut(sticky).unwrap().status = Status::Erledigt;
        assert_eq!(prefs.restored_update(&ws.updates), None);
        let visible_completed = CompactSettings {
            hide_completed: false,
            ..prefs.clone()
        };
        assert_eq!(visible_completed.restored_update(&ws.updates), Some(sticky));
        ws.updates.retain(|u| u.id != sticky);
        assert_eq!(prefs.restored_update(&ws.updates), None);
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
