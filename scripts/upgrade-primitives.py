from pathlib import Path
p=Path('src/ui/kit.rs');s=p.read_text(encoding='utf8')
a=s.index('    let handler2 = handler.clone();');b=s.index('\n/// Segment-Control',a)
s=s[:a]+'''    button(id).ghost().accessibility_label(label).role(Role::Switch).toggled(on)
        .min_h(px(palette::MIN_HIT)).min_w(px(palette::MIN_HIT)).px(px(0.)).tooltip(label)
        .child(div().w(px(36.)).h(px(20.)).rounded_full().flex().items_center().px(px(2.))
            .bg(if on {p.accent} else {p.border_strong}).when(on,|d|d.justify_end())
            .child(div().size(px(16.)).rounded_full().bg(if on {p.accent_contrast} else {p.text_primary})))
        .on_click(move |_,w,cx|handler(w,cx))
}
'''+s[b:];s=s.replace(') -> Stateful<Div> {',') -> Button {',1)
s=s.replace('            div()\n                .id(SharedString::from(format!("{id}-{i}")))','            button(SharedString::from(format!("{id}-{i}"))).ghost().toggled(selected).role(Role::Tab)')
# swatch actual button preserves keyboard accessibility
s=s.replace(') -> Stateful<Div> {\n    div()\n        .id(id)\n        .size(px(28.))',') -> Button {\n    button(id).ghost().accessibility_label(label)\n        .size(px(palette::MIN_HIT))').replace('.tooltip(tip(label))','.tooltip(label)')
s=s.replace('ButtonTone::Primary => b.primary(),','ButtonTone::Primary => b.primary().bg(p.accent).text_color(p.accent_contrast),').replace('ButtonTone::Secondary => b.outline(),','ButtonTone::Secondary => b.outline().bg(p.bg_elevated).text_color(p.text_primary),').replace('ButtonTone::Ghost => b.ghost(),','ButtonTone::Ghost => b.ghost().text_color(p.text_secondary),').replace('ButtonTone::SolidDanger => b.danger(),','ButtonTone::SolidDanger => b.danger().bg(p.danger),')
s=s.replace('        .rounded(px(if height == palette::CTL_SM', '        .px(px(if height == palette::CTL_SM {palette::CONTROL_PAD_SM} else if height == palette::CTL_LG {palette::CONTROL_PAD_LG} else {palette::CONTROL_PAD_MD}))\n        .rounded(px(if height == palette::CTL_SM')
s+='''
pub fn task_checkbox(id: impl Into<ElementId>,p:&Palette,on:bool) -> Button {
    button(id).ghost().size(px(palette::MIN_HIT)).p(px(0.)).role(Role::CheckBox).toggled(on)
        .accessibility_label(if on {"Aufgabe wieder öffnen"} else {"Aufgabe erledigen"})
        .child(div().size(px(palette::ICON_LG)).rounded_full().border_1().border_color(if on {p.done}else{p.border_strong})
            .when(on,|d|d.bg(p.done).child(icon(icons::CONFIRM,palette::ICON_SM).text_color(p.bg_app))))
}
'''
p.write_text(s,encoding='utf8')
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8');a=s.index('                        kit::button(("check", task_id))');b=s.index('                            .on_click(',a);s=s[:a]+'                        kit::task_checkbox(("check",task_id),&p,done)\n'+s[b:];
# Remove obsolete helpers rather than leaving a second status control.
a=s.index('    fn property_row(');b=s.index('    fn project_control(',a);s=s[:a]+s[b:]
s=s.replace('        let p=palette::get(cx);let text=md::changelog_filtered','        let text=md::changelog_filtered')
# tasks by type and complete all live together in the task menu
s=s.replace('        let tasks: Vec<&model::Task>', '        let mut tasks: Vec<&model::Task>')
s=s.replace('            .collect();\n        div()\n            .flex()\n            .flex_col()\n            .gap(px(8.))\n            .child(', '            .collect();\n        if self.task_grouped { tasks.sort_by_key(|t|t.kind as usize); }\n        div()\n            .flex()\n            .flex_col()\n            .gap(px(8.))\n            .child(',1)
s=s.replace('        let paste_entity = cx.entity();','        let paste_entity = cx.entity();\n        let task_menu = cx.entity();')
s=s.replace('                    .when(done > 0, |d| {','''                    .child(icon_btn("tasks-menu",icons::MORE,"Aufgaben-Aktionen").dropdown_menu(move |m,_,_| {
                        let grouped=task_menu.clone();let complete=task_menu.clone();
                        m.item(PopupMenuItem::new("Nach Typ gruppieren").on_click(move |_,_,cx|grouped.update(cx,|a,cx|{a.task_grouped=!a.task_grouped;cx.notify();})))
                            .item(PopupMenuItem::new("Alle erledigen").on_click(move |_,_,cx|complete.update(cx,|a,cx|{if !a.writable{return;}if let Some(u)=a.workspace.update_mut(id){for t in &mut u.tasks {t.done=true;}u.touch();}a.persist(cx);cx.notify();})))
                    }))
                    .when(done > 0, |d| {''',1)
p.write_text(s,encoding='utf8')
p=Path('src/ui/today.rs');s=p.read_text(encoding='utf8');a=s.index('                    icon_btn(SharedString::from(format!("{row_id}-status"))');b=s.index('                    }).into_any_element()',a)+len('                    }).into_any_element()');s=s[:a]+'''                    self.status_control(self.workspace.update(uid).unwrap(),cx).into_any_element()'''+s[b:]
a=s.index('                } else { kit::button(SharedString::from(format!("{row_id}-check")))');b=s.index('                    .on_click(',a);s=s[:a]+'''                } else { kit::task_checkbox(SharedString::from(format!("{row_id}-check")),&p,checked)
'''+s[b:]
s=s.replace('.h(px(28.))\n                        .px', '.h(px(palette::CTL_MD))\n                        .px').replace('.text_size(palette::fs(palette::FS_MD))\n                                .font_weight(FontWeight::SEMIBOLD)\n                                .child(label)', '.text_size(palette::fs(palette::FS_BASE))\n                                .text_color(p.text_secondary).font_weight(FontWeight::SEMIBOLD)\n                                .child(label)')
p.write_text(s,encoding='utf8')
p=Path('src/ui.rs');s=p.read_text(encoding='utf8').replace('.h(px(24.))\n            .px(px(10.))','.h(px(palette::CTL_MD))\n            .px(px(palette::CONTROL_PAD_SM))');s=s.replace('.with_size(px(32.))\n        .icon', '.with_size(px(palette::CTL_MD)).w(px(palette::CTL_MD)).p(px(0.))\n        .icon')
# meaning-aware native keycap tooltips for common actions
s=s.replace('    kit::button(id)\n        .ghost()', '    let b=kit::button(id)\n        .ghost()',1).replace('        .accessibility_label(tip)\n}', '''        .accessibility_label(tip);
    match name {
        icons::SEARCH => b.tooltip_with_action(tip,&OpenPalette,None),
        icons::SETTINGS => b.tooltip_with_action(tip,&OpenSettings,None),
        icons::UNDO => b.tooltip_with_action(tip,&HistoryUndo,None),
        icons::SIDEBAR_CLOSE | icons::SIDEBAR_OPEN => b.tooltip_with_action(tip,&ToggleSidebar,Some("NoteWarden")),
        _ => b,
    }
}''',1)
p.write_text(s,encoding='utf8')
p=Path('src/ui/workbench.rs');s=p.read_text(encoding='utf8').replace('let p=palette::get(cx); let ui=', 'let ui=');p.write_text(s,encoding='utf8')
