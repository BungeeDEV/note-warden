//! Post-its: tiny notes that live in the compact window and in their own main view.
use super::*;
use gpui_kit::component::input::Textarea;

#[derive(Clone)]
pub struct StickyDrag {
    pub id: u64,
    pub label: String,
}

impl NoteWarden {
    pub fn new_sticky(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        self.finish_sticky_edit(cx);
        let id = self.workspace.new_sticky();
        self.persist(cx);
        self.start_sticky_edit(id, window, cx);
    }

    pub fn start_sticky_edit(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if !self.writable || self.editing_sticky == Some(id) {
            return;
        }
        let Some(text) = self
            .workspace
            .stickies
            .iter()
            .find(|s| s.id == id)
            .map(|s| {
                // Tags are edited as `#tag` words at the end of the text.
                let tags: Vec<String> = s.tags.iter().map(|t| format!("#{t}")).collect();
                if tags.is_empty() {
                    s.text.clone()
                } else {
                    format!("{}\n{}", s.text.trim_end(), tags.join(" "))
                }
            })
        else {
            return;
        };
        self.finish_sticky_edit(cx);
        self.editing_sticky = Some(id);
        self.sticky_input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub fn set_sticky_text(&mut self, value: &str, cx: &mut Context<Self>) {
        let Some(id) = self.editing_sticky.filter(|_| self.writable) else {
            return;
        };
        if let Some(sticky) = self.workspace.stickies.iter_mut().find(|s| s.id == id)
            && sticky.text != value
        {
            sticky.text = value.to_owned();
            self.persist(cx);
            cx.notify();
        }
    }

    /// Leaves edit mode: `#tag` words become the post-it's tags; a post-it left empty disappears.
    pub fn finish_sticky_edit(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.editing_sticky.take() else {
            return;
        };
        let before = self.workspace.stickies.len();
        if let Some(sticky) = self.workspace.stickies.iter_mut().find(|s| s.id == id) {
            let (text, tags) = quick::split_text_tags(&sticky.text);
            sticky.text = text;
            sticky.tags = tags;
        }
        self.workspace
            .stickies
            .retain(|s| s.id != id || !s.text.trim().is_empty() || !s.tags.is_empty());
        self.persist(cx);
        if self.workspace.stickies.len() != before {
            cx.notify();
        }
        cx.notify();
    }

    pub fn toggle_sticky_tag(&mut self, tag: &str, cx: &mut Context<Self>) {
        let same = self
            .sticky_tag
            .as_ref()
            .is_some_and(|t| t.to_lowercase() == tag.to_lowercase());
        self.sticky_tag = if same { None } else { Some(tag.to_owned()) };
        cx.notify();
    }

    pub fn cycle_sticky_color(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(sticky) = self.workspace.stickies.iter_mut().find(|s| s.id == id) {
            sticky.color = (sticky.color + 1) % palette::STICKY_COLORS.len() as u8;
            self.persist(cx);
            cx.notify();
        }
    }

    pub fn delete_sticky(&mut self, id: u64, cx: &mut Context<Self>) {
        if self.editing_sticky == Some(id) {
            self.editing_sticky = None;
        }
        if let Some(index) = self.workspace.stickies.iter().position(|s| s.id == id) {
            let sticky = self.workspace.stickies.remove(index);
            self.persist(cx);
            self.toast("Post-it gelöscht", Some(Undo::Sticky(index, sticky)), cx);
        }
    }

    pub fn move_sticky(&mut self, id: u64, before: Option<u64>, cx: &mut Context<Self>) {
        if self.writable && Some(id) != before {
            self.workspace.move_sticky(id, before);
            self.persist(cx);
            cx.notify();
        }
    }

    /// Post-its matching the search text or tag and the active tag filter (all when both are empty).
    pub fn visible_stickies(&self, query: &str) -> Vec<model::Sticky> {
        let query = query.trim().to_lowercase();
        let tag_query = query.trim_start_matches('#');
        self.workspace
            .stickies
            .iter()
            .filter(|s| {
                self.sticky_tag.as_ref().is_none_or(|tag| {
                    s.tags
                        .iter()
                        .any(|t| t.to_lowercase() == tag.to_lowercase())
                })
            })
            .filter(|s| {
                query.is_empty()
                    || s.text.to_lowercase().contains(&query)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(tag_query))
                    || self.editing_sticky == Some(s.id)
            })
            .cloned()
            .collect()
    }

    /// Draggable grid of post-its with a trailing "add" tile.
    pub fn sticky_grid(
        &self,
        stickies: &[model::Sticky],
        cols: u16,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let p = palette::get(cx);
        let min_h = if compact { 64. } else { 132. };
        let lines = if compact { 3 } else { 8 };
        let mut grid = div()
            .w_full()
            .min_w_0()
            .grid()
            .grid_cols(cols)
            .gap(px(palette::SP_2));
        for sticky in stickies {
            let id = sticky.id;
            let editing = self.editing_sticky == Some(id);
            let group = SharedString::from(format!("sticky-{id}"));
            let label: String = sticky
                .text
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(40)
                .collect();
            let open = cx.entity();
            let drop = cx.entity();
            let color = cx.entity();
            let delete = cx.entity();
            let actions = div()
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .opacity(0.)
                .group_hover(group.clone(), |s| s.opacity(1.))
                .child(
                    kit::surface(("sticky-color", id), &p)
                        .size(px(palette::CTL_SM - 8.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            kit::icon(IconName::Palette, palette::ICON_XS)
                                .text_color(p.text_secondary),
                        )
                        .tooltip(tip("Farbe wechseln"))
                        .accessibility_label("Farbe wechseln")
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            color.update(cx, |app, cx| app.cycle_sticky_color(id, cx));
                        }),
                )
                .child(
                    kit::surface(("sticky-delete", id), &p)
                        .size(px(palette::CTL_SM - 8.))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            kit::icon(icons::DELETE, palette::ICON_XS).text_color(p.text_secondary),
                        )
                        .tooltip(tip("Post-it löschen"))
                        .accessibility_label("Post-it löschen")
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            delete.update(cx, |app, cx| app.delete_sticky(id, cx));
                        }),
                );
            let body = if editing {
                // The textarea brings its own padding; pull it back so the text does not jump.
                div()
                    .w_full()
                    .ml(px(-10.))
                    .mt(px(-7.))
                    .child(
                        Textarea::new(&self.sticky_input)
                            .appearance(false)
                            .bordered(false)
                            .aria_label("Post-it-Text")
                            .w_full(),
                    )
                    .into_any_element()
            } else {
                div()
                    .w_full()
                    .min_w_0()
                    .text_size(palette::fs(palette::FS_BASE))
                    .text_color(p.text_primary)
                    .line_height(relative(1.35))
                    .line_clamp(lines)
                    .child(sticky.text.clone())
                    .into_any_element()
            };
            let pills = div().flex().flex_wrap().gap(px(palette::SP_1)).children(
                sticky.tags.iter().take(4).map(|tag| {
                    let filter = cx.entity();
                    let clicked = tag.clone();
                    kit::surface(SharedString::from(format!("sticky-tag-{id}-{tag}")), &p)
                        .h(px(18.))
                        .px(px(palette::SP_2))
                        .rounded_full()
                        .bg(p.bg_app.opacity(0.35))
                        .flex()
                        .items_center()
                        .gap(px(palette::SP_1))
                        .text_size(palette::fs(palette::FS_SM))
                        .text_color(p.text_primary)
                        .child(dot(palette::tag_color(tag, &p)))
                        .child(tag.clone())
                        .tooltip(tip("Nach diesem Tag filtern"))
                        .on_click(move |_, _, cx| {
                            cx.stop_propagation();
                            filter.update(cx, |app, cx| app.toggle_sticky_tag(&clicked, cx))
                        })
                }),
            );
            let tile = div()
                .id(("sticky", id))
                .group(group)
                .min_w_0()
                .min_h(px(min_h))
                .p(px(palette::SP_2))
                .rounded(px(palette::R_CARD))
                .bg(palette::sticky_fill(p.dark, sticky.color))
                .border_1()
                .border_color(if editing { p.accent } else { p.border_subtle })
                .flex()
                .flex_col()
                .justify_between()
                .gap(px(palette::SP_1))
                .when(!editing, |d| {
                    d.cursor_pointer()
                        .on_drag(
                            StickyDrag {
                                id,
                                label: label.clone(),
                            },
                            |d, _, _, cx| {
                                let label = d.label.clone();
                                cx.new(|_| DragPreview { label })
                            },
                        )
                        .on_click(move |_, window, cx| {
                            open.update(cx, |app, cx| app.start_sticky_edit(id, window, cx))
                        })
                })
                .drag_over::<StickyDrag>(move |s, _, _, _| s.border_2().border_color(p.accent))
                .on_drop(move |d: &StickyDrag, _, cx| {
                    let dragged = d.id;
                    drop.update(cx, |app, cx| app.move_sticky(dragged, Some(id), cx));
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(palette::SP_1))
                        .child(body)
                        .when(!editing && !sticky.tags.is_empty(), |d| d.child(pills)),
                )
                .child(actions);
            grid = grid.child(tile);
        }
        let add = cx.entity();
        let end = cx.entity();
        grid.child(
            div()
                .id("sticky-add")
                .min_w_0()
                .min_h(px(min_h.min(64.)))
                .rounded(px(palette::R_CARD))
                .border_1()
                .border_dashed()
                .border_color(p.border_strong)
                .flex()
                .items_center()
                .justify_center()
                .gap(px(palette::ICON_GAP))
                .cursor_pointer()
                .text_size(palette::fs(palette::FS_BASE))
                .text_color(p.text_secondary)
                .hover(|s| s.bg(p.bg_hover))
                .drag_over::<StickyDrag>(move |s, _, _, _| s.border_2().border_color(p.accent))
                .on_drop(move |d: &StickyDrag, _, cx| {
                    let dragged = d.id;
                    end.update(cx, |app, cx| app.move_sticky(dragged, None, cx));
                })
                .on_click(move |_, window, cx| add.update(cx, |app, cx| app.new_sticky(window, cx)))
                .child(kit::icon(icons::ADD, palette::ICON_SM).text_color(p.text_secondary))
                .child("Post-it"),
        )
    }

    /// Tags used by post-its as toggle chips; none if no post-it has a tag.
    fn sticky_tag_bar(&self, cx: &mut Context<Self>) -> Option<Div> {
        let tags = self.workspace.sticky_tags();
        if tags.is_empty() {
            return None;
        }
        let p = palette::get(cx);
        Some(
            div()
                .flex_none()
                .flex()
                .flex_wrap()
                .gap(px(palette::SP_2))
                .children(tags.into_iter().map(|(tag, count)| {
                    let active = self
                        .sticky_tag
                        .as_ref()
                        .is_some_and(|t| t.to_lowercase() == tag.to_lowercase());
                    let color = palette::tag_color(&tag, &p);
                    let filter = cx.entity();
                    let clicked = tag.clone();
                    kit::surface(SharedString::from(format!("sticky-filter-{tag}")), &p)
                        .h(px(24.))
                        .px(px(palette::SP_3))
                        .rounded_full()
                        .border_1()
                        .border_color(if active { color } else { p.border_subtle })
                        .bg(if active {
                            color.opacity(0.16)
                        } else {
                            gpui_kit::transparent_black()
                        })
                        .text_size(palette::fs(palette::FS_SM))
                        .text_color(if active { color } else { p.text_secondary })
                        .child(format!("#{tag} {count}"))
                        .on_click(move |_, _, cx| {
                            filter.update(cx, |app, cx| app.toggle_sticky_tag(&clicked, cx))
                        })
                })),
        )
    }

    pub fn stickies_view(&self, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let cols = ((self.content_width - 2. * palette::SP_6 - palette::SCROLL_GUTTER) / 200.)
            .floor()
            .clamp(2., 6.) as u16;
        let stickies = self.visible_stickies("");
        let grid = self.sticky_grid(&stickies, cols, false, cx);
        let tag_bar = self.sticky_tag_bar(cx);
        div()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .p(px(palette::SP_6))
            .gap(px(palette::SP_4))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(px(palette::SP_1))
                    .child(
                        div()
                            .text_size(palette::fs(palette::FS_XL))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Post-its"),
                    )
                    .child(text_secondary(
                        &p,
                        "Schnelle Notizen. Ziehen zum Sortieren, auch im Notizzettel-Modus verfügbar.",
                    )),
            )
            .children(tag_bar)
            .child(
                div()
                    .id("stickies-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .pr(px(palette::SCROLL_GUTTER))
                    .child(grid),
            )
    }
}
