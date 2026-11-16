use super::*;
use gpui_kit::base::FocusTrapElement as _;
use gpui_kit::component::text::TextView;

impl NoteWarden {
    /// Right side panel (360 px): list of versions, read-only preview, restore.
    pub(super) fn history_panel_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = palette::get(cx);
        let panel = self.history_panel.as_ref().expect("Verlauf");
        let current = self
            .workspace
            .update(panel.update_id)
            .map(|u| u.notes.as_str())
            .unwrap_or("");
        let selected = panel.snapshots.get(panel.selected);
        let can_restore = selected.is_some_and(|s| s.text != current);
        let now = model::now_millis();
        let rows = panel
            .snapshots
            .iter()
            .enumerate()
            .rev()
            .map(|(index, snapshot)| {
                let words = md::word_count(&snapshot.text) as i64;
                let before = if index == 0 {
                    None
                } else {
                    Some(md::word_count(&panel.snapshots[index - 1].text) as i64)
                };
                let delta = before
                    .map(|b| words - b)
                    .filter(|d| *d != 0)
                    .map(|d| format!("{d:+} Wörter"))
                    .unwrap_or_default();
                let when = dates::full_time(snapshot.at).unwrap_or_default();
                kit::button(("history-row", index))
                    .ghost()
                    .toggled(index == panel.selected)
                    .w_full()
                    .min_h(px(palette::MIN_HIT))
                    .px(px(palette::SP_2))
                    .tooltip(when)
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(palette::SP_2))
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_BASE))
                                    .child(dates::elapsed_at(snapshot.at, now)),
                            )
                            .child(
                                div()
                                    .text_size(palette::fs(palette::FS_SM))
                                    .text_color(p.text_tertiary)
                                    .child(
                                        format!(
                                            "{} · {delta}",
                                            model::plural(words as usize, "Wort", "Wörter")
                                        )
                                        .trim_end_matches(" · ")
                                        .to_owned(),
                                    ),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(panel) = &mut this.history_panel {
                            panel.selected = index;
                        }
                        cx.notify();
                    }))
            });
        div()
            .id("history-panel")
            .role(Role::Dialog)
            .track_focus(&self.overlay_focus)
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(360.))
            .max_w_full()
            .flex()
            .flex_col()
            .bg(p.bg_elevated)
            .border_l_1()
            .border_color(p.border_strong)
            .shadow_lg()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .px(px(palette::SP_4))
                    .pt(px(palette::SP_4))
                    .pb(px(palette::SP_2))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(palette::fs(palette::FS_LG))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Versionsverlauf"),
                    )
                    .child(
                        icon_btn("history-close", icons::CLOSE, "Schließen · Esc").on_click(
                            cx.listener(|this, _, window, cx| this.close_overlay(window, cx)),
                        ),
                    ),
            )
            .child(
                div()
                    .id("history-list")
                    .max_h(px(200.))
                    .overflow_y_scrollbar()
                    .pl(px(palette::SP_2))
                    .pr(px(palette::SCROLL_GUTTER))
                    .flex()
                    .flex_col()
                    .when(panel.snapshots.is_empty(), |d| {
                        d.child(div().px(px(palette::SP_2)).child(text_secondary(
                            &p,
                            "Noch keine Stände. Sie entstehen automatisch beim Schreiben.",
                        )))
                    })
                    .children(rows),
            )
            .child(
                div()
                    .id("history-preview")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .m(px(palette::SP_4))
                    .p(px(palette::SP_3))
                    .rounded(px(palette::R_INPUT))
                    .bg(p.bg_app)
                    .border_1()
                    .border_color(p.border_subtle)
                    .when_some(selected, |d, snapshot| {
                        d.child(if snapshot.text.trim().is_empty() {
                            text_secondary(&p, "Leerer Text.").into_any_element()
                        } else {
                            TextView::markdown("history-markdown", snapshot.text.clone())
                                .selectable(true)
                                .into_any_element()
                        })
                    }),
            )
            .child(
                div()
                    .px(px(palette::SP_4))
                    .pb(px(palette::SP_4))
                    .flex()
                    .justify_end()
                    .child(
                        kit::button("history-restore")
                            .primary()
                            .label("Diesen Stand wiederherstellen")
                            .disabled(!can_restore)
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.restore_snapshot(window, cx)
                                }),
                            ),
                    ),
            )
            .focus_trap("history-trap", &self.overlay_focus)
    }
}
