from pathlib import Path
p=Path('src/ui/workbench.rs');s=p.read_text(encoding='utf8').replace('-> Button {','-> impl IntoElement {');p.write_text(s,encoding='utf8')
p=Path('src/ui/list.rs');s=p.read_text(encoding='utf8').replace("Header(&'static str, usize)","Header(String, usize)").replace('Row::Header(status.label(),','Row::Header(status.label().into(),').replace('Row::Header(label, part.len())','Row::Header(label.into(), part.len())')
s=s.replace('        }\n        rows\n', '''            GroupBy::Project | GroupBy::Priority => {
                let mut groups: std::collections::BTreeMap<String, Vec<&model::Update>> = Default::default();
                for u in list { let label = if self.group == GroupBy::Project {
                    u.project_id.and_then(|id| self.workspace.projects.iter().find(|p|p.id==id)).map(|p|p.name.clone()).unwrap_or("Kein Projekt".into())
                } else { ["Keine Priorität", "Niedrig", "Mittel", "Hoch"][u.priority.min(3) as usize].into() };
                    groups.entry(label).or_default().push(u);
                }
                for (label, group) in groups { rows.push(Row::Header(label, group.len())); rows.extend(group.into_iter().map(Row::Card)); }
            }
        }
        rows
''',1)
s=s.replace('        list\n    }','        if self.workspace.settings.updates_ui.reverse { list.reverse(); }\n        list\n    }',1)
s=s.replace('.filter(|u| Self::matches_query(u, &query))','''.filter(|u| Self::matches_query(u, &query))
            .filter(|u| {
                let ui=&self.workspace.settings.updates_ui;
                (ui.statuses.is_empty() || ui.statuses.contains(&u.status)) &&
                (ui.priorities.is_empty() || ui.priorities.contains(&u.priority)) &&
                match ui.date_filter {
                    1 => u.overdue(today), 2 => u.planned_for==Some(today),
                    3 => u.planned_for.is_some_and(|d|d>=today && d<=week_end),
                    4 => u.planned_for.is_some_and(|d|d>=today && d<today+Duration::days(7)),
                    5 => u.planned_for.is_none(),
                    6 => u.planned_for.is_some_and(|d|ui.range_start.is_none_or(|a|d>=a) && ui.range_end.is_none_or(|b|d<=b)),
                    _ => true,
                }
            })''')
# Fuzzy subsequence search, including notes and tasks.
s=s.replace('query.is_empty()\n            || update.title.to_lowercase().contains(query)','query.is_empty()\n            || fuzzy_matches(&update.title, query)')
start=s.index('    fn card(');end=s.index('    pub(super) fn update_list',start)
s=s[:start]+'''    fn card(&self, update: &model::Update, cx: &mut Context<Self>) -> impl IntoElement {
        let p=palette::get(cx); let id=update.id; let e=cx.entity(); let compact=self.workspace.settings.updates_ui.compact;
        let selected=self.selected_update==Some(id) || self.multi.contains(&id);
        let group: SharedString=format!("row-{id}").into(); let (done,total)=update.task_progress();
        kit::button(("update-row",id)).ghost().w_full().h(px(if compact {palette::ROW_COMPACT} else {palette::ROW_COMFORTABLE}))
            .group(group.clone()).relative().rounded(px(palette::R_INPUT)).px(px(palette::SP_2))
            .bg(if selected {p.bg_active} else {p.bg_app}).border_l_2().border_color(if selected {p.accent} else {gpui_kit::transparent_black()})
            .accessibility_label(format!("{} · {}",update.title,update.status.label()))
            .on_click(move |ev,w,cx|e.update(cx,|a,cx|a.click_card(id,ev.modifiers().control,ev.modifiers().shift,w,cx)))
            .child(self.status_control(update,cx))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                .child(div().truncate().text_size(palette::fs(palette::FS_MD)).text_color(p.text_primary).child(highlighted(&update.title,&self.search_query,&p)))
                .when(!compact,|d|d.child(div().flex().items_center().gap(px(palette::SP_2))
                    .when_some(update.project_id.and_then(|pid|self.workspace.projects.iter().find(|p|p.id==pid)),|d,pj|d.child(kit::badge(&p,pj.name.clone())))
                    .child(kit::priority_icon(&p,update.priority))
                    .when(!update.version.is_empty(),|d|d.child(kit::badge(&p,update.version.clone())))
                    .when(total>0,|d|d.child(text_secondary(&p,format!("{done}/{total}"))).child(progress_bar(&p,done,total,48.)))
                    .when(!self.search_query.is_empty(),|d|d.child(text_secondary(&p,update.notes.chars().take(60).collect::<String>())))))
            .child(div().w(px(110.)).flex_none().flex().justify_end().child(self.date_chip(format!("row-date-{id}"),DateTarget::Update(id),update.planned_for,update.status==Status::Erledigt,cx)))
            .child(icon_btn(("row-more",id),icons::MORE,"Weitere Aktionen").dropdown_menu({let build=update_menu(cx.entity(),id,update.archived,update.deleted_on.is_some(),false);move |m,w,cx|build(m,w,cx)}))
            .context_menu({let build=update_menu(cx.entity(),id,update.archived,update.deleted_on.is_some(),false);move |m,w,cx|build(m,w,cx)})
    }

'''+s[end:]
s=s.replace('app.sort = key;\n                                                    cx.notify();','app.sort = key;\n                                                    app.save_updates_ui(cx);').replace('app.group = key;\n                                                    cx.notify();','app.group = key;\n                                                    app.save_updates_ui(cx);')
s=s.replace('.gap(px(8.))\n                    .when(rows.is_empty()', '.gap(px(2.))\n                    .when(rows.is_empty()')
# Replace static group headings with controls; collapsed state omits following rows.
s=s.replace('        rows\n    }','''        let mut collapsed=false;
        rows.retain(|row|match row { Row::Header(label,_) => {collapsed=self.workspace.settings.updates_ui.collapsed.contains(label);true}, Row::Card(_) => !collapsed });
        rows
    }''',1)
a=s.index('                            Row::Header(label, n) =>');b=s.index('                            Row::Card(update)',a)
s=s[:a]+'''                            Row::Header(label, n) => {
                                let closed=self.workspace.settings.updates_ui.collapsed.contains(&label); let e=cx.entity();
                                kit::button(SharedString::from(format!("group-{label}"))).ghost().w_full().justify_start()
                                    .icon(kit::icon(if closed {icons::COLLAPSE} else {icons::EXPAND},palette::ICON_MD))
                                    .label(format!("{label} · {n}")).on_click(move |_,_,cx|e.update(cx,|a,cx| {
                                        let groups=&mut a.workspace.settings.updates_ui.collapsed;
                                        if groups.contains(&label) {groups.retain(|s|s!=&label);} else {groups.push(label.clone());}
                                        a.persist(cx);cx.notify();
                                    })).into_any_element()
                            },
'''+s[b:]
s=s.replace('.id("update-list")','.id("update-list").key_context("UpdatesList").tab_stop(true)\n            .on_key_down(cx.listener(Self::update_keys))')
s=s.replace('.overflow_y_scrollbar()\n                    .px(px(12.))','.overflow_y_scroll().track_scroll(&self.list_scroll).vertical_scrollbar(&self.list_scroll)\n                    .px(px(12.))',1)
# Render actual filter controls before list.
s=s.replace('            .when(filter == Filter::Trash,', '            .child(self.updates_filters(cx))\n            .when(filter == Filter::Trash,',1)
s+='''
fn fuzzy_matches(text: &str, query: &str) -> bool {
    let text=text.to_lowercase(); let mut chars=text.chars(); query.chars().all(|wanted|chars.by_ref().any(|c|c==wanted))
}
'''
p.write_text(s,encoding='utf8')
# container-based widths and keyboard actions
p=Path('src/ui.rs');s=p.read_text(encoding='utf8');s=s.replace('        let narrow = width < px(780.);','        self.content_width = (f32::from(width) - if self.workspace.settings.sidebar_collapsed {64.} else {240.}).max(0.);\n        let narrow = self.content_width < 900.;');s=s.replace('clamp(280., 480.)','clamp(300., 520.)').replace('.when(!narrow || !self.detail_open,','.when(!self.focus_mode && (!narrow || !self.detail_open),').replace('.when(!narrow, |d| {\n                        let entity', '.when(!narrow && !self.focus_mode, |d| {\n                        let entity').replace('.child(self.sidebar(icon_only, cx))','.when(!self.focus_mode, |d| d.child(self.sidebar(icon_only, cx)))')
s=s.replace('.on_action(cx.listener(|_this, _: &ToggleStyleguide,', '''.on_action(cx.listener(|this, _: &HistoryUndo, w,cx|this.history_step(false,w,cx)))
            .on_action(cx.listener(|this, _: &HistoryRedo, w,cx|this.history_step(true,w,cx)))
            .on_action(cx.listener(|this, _: &PreviousUpdate, w,cx|this.select_relative(-1,w,cx)))
            .on_action(cx.listener(|this, _: &NextUpdate, w,cx|this.select_relative(1,w,cx)))
            .on_action(cx.listener(|this, _: &ToggleInspector, _,cx| {this.workspace.settings.updates_ui.inspector_hidden=!this.workspace.settings.updates_ui.inspector_hidden;this.persist(cx);cx.notify();}))
            .on_action(cx.listener(|this, _: &ToggleFocusMode, _,cx| {this.focus_mode=!this.focus_mode;cx.notify();}))
            .on_action(cx.listener(|_this, _: &ToggleStyleguide,''')
p.write_text(s,encoding='utf8')
p=Path('src/main.rs');s=p.read_text(encoding='utf8').replace('        ToggleStyleguide,','        HistoryUndo, HistoryRedo, PreviousUpdate, NextUpdate, ToggleInspector, ToggleFocusMode,\n        ToggleStyleguide,',1)
s=s.replace('            KeyBinding::new("ctrl-n",', '''            KeyBinding::new("ctrl-z", HistoryUndo, Some("NoteWarden && !Input && !Editor")),
            KeyBinding::new("ctrl-y", HistoryRedo, Some("NoteWarden && !Input && !Editor")),
            KeyBinding::new("ctrl-shift-z", HistoryRedo, Some("NoteWarden && !Input && !Editor")),
            KeyBinding::new("alt-up", PreviousUpdate, Some("NoteWarden && !TaskRow")),
            KeyBinding::new("alt-down", NextUpdate, Some("NoteWarden && !TaskRow")),
            KeyBinding::new("ctrl-shift-i", ToggleInspector, Some("NoteWarden")),
            KeyBinding::new("ctrl-shift-f", ToggleFocusMode, Some("NoteWarden")),
            KeyBinding::new("ctrl-n",''',1)
s=s.replace('clamp(280., 480.)','clamp(300., 520.)');p.write_text(s,encoding='utf8')
p=Path('src/ui/today.rs');s=p.read_text(encoding='utf8').replace('self.mini_month\n            - Duration::days(self.mini_month.weekday().num_days_from_monday() as i64)','dates::week_start(self.mini_month, self.workspace.settings.week_start)');p.write_text(s,encoding='utf8')
