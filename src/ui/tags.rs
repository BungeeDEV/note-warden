//! Tags on updates and their tasks: chips, the editing row and the tag filter.
use super::*;

/// Small colored `#tag` label; the color follows the tag's name.
pub fn tag_pill(p: &Palette, tag: &str) -> Div {
    let color = palette::tag_color(tag, p);
    div()
        .flex_none()
        .h(px(20.))
        .px(px(palette::SP_2))
        .rounded_full()
        .flex()
        .items_center()
        .bg(color.opacity(0.16))
        .text_color(color)
        .text_size(palette::fs(palette::FS_SM))
        .font_weight(FontWeight::MEDIUM)
        .child(format!("#{tag}"))
}

impl NoteWarden {
    pub fn add_tag(&mut self, id: u64, raw: &str, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self.workspace.update_mut(id)
            && model::push_tag(&mut update.tags, raw)
        {
            update.touch();
            self.persist(cx);
            cx.notify();
        }
    }

    pub fn remove_tag(&mut self, id: u64, tag: &str, cx: &mut Context<Self>) {
        if !self.writable {
            return;
        }
        if let Some(update) = self.workspace.update_mut(id) {
            let before = update.tags.len();
            update.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
            if update.tags.len() != before {
                update.touch();
                self.persist(cx);
                cx.notify();
            }
        }
    }

    pub fn commit_tag_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let raw = self.tag_input.read(cx).value().to_string();
        if let Some(id) = self.selected_update {
            for part in raw.split([',', ';']) {
                self.add_tag(id, part, cx);
            }
        }
        self.tag_input
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// Filters the update list by `tag`, or clears the filter if it is the only one active.
    pub fn toggle_tag_filter(&mut self, tag: &str, show_list: bool, cx: &mut Context<Self>) {
        let only = {
            let tags = &self.workspace.settings.updates_ui.tags;
            tags.len() == 1 && tags[0].to_lowercase() == tag.to_lowercase()
        };
        if show_list {
            self.set_view(View::Updates, cx);
            self.filter = Filter::All;
        }
        let ui = &mut self.workspace.settings.updates_ui;
        ui.tags = if only {
            Vec::new()
        } else {
            vec![tag.to_owned()]
        };
        self.save_updates_ui(cx);
    }

    pub fn passes_tag_filter(&self, update: &model::Update) -> bool {
        self.workspace
            .settings
            .updates_ui
            .tags
            .iter()
            .all(|tag| update.has_tag(tag))
    }

    /// Chips of the update's tags (click filters, x removes) plus the input and a few suggestions.
    pub fn tags_row(&self, update: &model::Update, cx: &mut Context<Self>) -> Div {
        let p = palette::get(cx);
        let id = update.id;
        let mut row = div()
            .mt(px(palette::SP_2))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(palette::SP_2));
        for tag in &update.tags {
            let color = palette::tag_color(tag, &p);
            let filter = cx.entity();
            let remove = cx.entity();
            let (filter_tag, remove_tag) = (tag.clone(), tag.clone());
            row = row.child(
                div()
                    .h(px(24.))
                    .pl(px(palette::SP_1))
                    .pr(px(2.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .bg(color.opacity(0.16))
                    .child(
                        kit::surface(SharedString::from(format!("tag-{id}-{tag}")), &p)
                            .h(px(20.))
                            .px(px(palette::SP_1))
                            .rounded_full()
                            .text_color(color)
                            .text_size(palette::fs(palette::FS_SM))
                            .font_weight(FontWeight::MEDIUM)
                            .tooltip(tip("Nach diesem Tag filtern"))
                            .child(format!("#{tag}"))
                            .on_click(move |_, _, cx| {
                                filter.update(cx, |app, cx| {
                                    app.toggle_tag_filter(&filter_tag, false, cx)
                                })
                            }),
                    )
                    .child(
                        kit::surface(SharedString::from(format!("tag-rm-{id}-{tag}")), &p)
                            .size(px(20.))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .tooltip(tip("Tag entfernen"))
                            .accessibility_label(format!("Tag {tag} entfernen"))
                            .child(kit::icon(icons::CLOSE, palette::ICON_XS).text_color(color))
                            .on_click(move |_, _, cx| {
                                remove.update(cx, |app, cx| app.remove_tag(id, &remove_tag, cx))
                            }),
                    ),
            );
        }
        row = row.child(
            kit::input(&self.tag_input)
                .aria_label("Tag hinzufügen")
                .w(px(132.)),
        );
        let suggestions = self
            .workspace
            .all_tags()
            .into_iter()
            .filter(|(tag, _)| {
                !update
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == tag.to_lowercase())
            })
            .take(4);
        for (tag, _) in suggestions {
            let add = cx.entity();
            let added = tag.clone();
            row = row.child(
                kit::surface(SharedString::from(format!("tag-add-{id}-{tag}")), &p)
                    .h(px(24.))
                    .px(px(palette::SP_2))
                    .rounded_full()
                    .border_1()
                    .border_dashed()
                    .border_color(p.border_strong)
                    .text_color(p.text_secondary)
                    .text_size(palette::fs(palette::FS_SM))
                    .tooltip(tip("Tag hinzufügen"))
                    .child(format!("+ #{tag}"))
                    .on_click(move |_, _, cx| {
                        add.update(cx, |app, cx| app.add_tag(id, &added, cx))
                    }),
            );
        }
        row
    }
}
