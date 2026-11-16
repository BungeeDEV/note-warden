from pathlib import Path
p=Path('src/ui.rs');s=p.read_text(encoding='utf8').replace('div()\n            .id(SharedString::from(id))\n            .h(palette::dens(30.))','kit::button(SharedString::from(id)).ghost()\n            .h(px(palette::CTL_MD))');p.write_text(s,encoding='utf8')
p=Path('src/ui/overlays.rs');s=p.read_text(encoding='utf8').replace('kit::button("dialog").ghost()','div().id("dialog").role(Role::Dialog)');p.write_text(s,encoding='utf8')
p=Path('src/main.rs');s=p.read_text(encoding='utf8');s=s.replace('    pub fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {','''    pub fn close_overlay(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_mode {self.focus_mode=false;cx.notify();return;}
        if !self.search_query.is_empty() && self.search_input.read(cx).focus_handle(cx).is_focused(window) {
            self.search_query.clear(); self.search_input.update(cx,|i,cx|i.set_value("",window,cx));cx.notify();return;
        }
        if self.view==View::Updates && self.detail_open && self.dialog.is_none() && self.date_pop.is_none() && !self.palette_open {
            self.discard_untouched_drafts(None,cx);self.detail_open=false;self.focus_handle.focus(window,cx);cx.notify();return;
        }''')
# Persisted filter state also changes through sidebar.
s=s.replace('        self.filter = filter;\n        self.detail_open = false;','        self.filter = filter;\n        self.save_updates_ui(cx);\n        self.detail_open = false;')
# quick syntax for tasks, preserving existing task creation logic.
a=s.index('        if let Some(id) = self.selected_update {\n            let title = self.task_input');b=s.index('\n    pub fn toggle_task',a)
s=s[:a]+'''        let raw=self.task_input.read(cx).value().to_string();
        self.add_tasks(&raw,cx);
        self.task_input.update(cx,|i,cx|i.set_value("",window,cx));
    }

    pub fn add_tasks(&mut self,text:&str,cx:&mut Context<Self>) {
        if !self.writable {return;}
        if let Some(id)=self.selected_update {
            for line in text.lines() {
                let line=line.trim().trim_start_matches(['-','*','•']).trim_start_matches("[ ]").trim();
                let (kind,clean)=quick::task_kind(line);
                let parsed=quick::parse_quick_input(&clean,model::today(),&self.workspace.projects);
                if let Some(tid)=self.workspace.new_task(id,parsed.title,parsed.date) && let Some(u)=self.workspace.update_mut(id) && let Some(t)=u.tasks.iter_mut().find(|t|t.id==tid) {
                    t.kind=kind;t.priority=parsed.priority;
                }
            }
            self.persist(cx);cx.notify();
        }
    }
'''+s[b:]
p.write_text(s,encoding='utf8')
p=Path('src/quick.rs');s=p.read_text(encoding='utf8');s+='''
/// Typ-Tokens ergänzen die bestehende Quick-Add-Syntax für Aufgaben.
pub fn task_kind(text:&str)->(crate::model::ChangeKind,String) {
    use crate::model::ChangeKind;
    let mut kind=ChangeKind::Added;let mut words=Vec::new();
    for word in text.split_whitespace() {match word.to_lowercase().as_str(){":added"=>kind=ChangeKind::Added,":changed"=>kind=ChangeKind::Changed,":removed"=>kind=ChangeKind::Removed,_=>words.push(word)}}
    (kind,words.join(" "))
}
''';p.write_text(s,encoding='utf8')
# Empty historical creation time stays unknown; never invent timestamps during migration.
p=Path('src/model.rs');s=p.read_text(encoding='utf8').replace('        self.schema_version = SCHEMA_VERSION;','''        self.settings.list_width=self.settings.list_width.clamp(300.,520.);
        for u in &mut self.updates {if u.activity.len()>200 {u.activity.drain(..u.activity.len()-200);}}
        self.schema_version = SCHEMA_VERSION;''');p.write_text(s,encoding='utf8')
# Selected UI state gets correct scope; root shortcut handling outside editor/inputs.
p=Path('src/ui.rs');s=p.read_text(encoding='utf8').replace('        if self.view == View::Today {','        if self.view == View::Updates { context.add("Updates"); }\n        if self.view == View::Today {',1);p.write_text(s,encoding='utf8')
# New standard export uses exact preview text.
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8').replace('a.export_update_md(id,cx)','a.export_changelog(id,cx)');p.write_text(s,encoding='utf8')
p=Path('src/ui/workbench.rs');s=p.read_text(encoding='utf8');s+='''
impl NoteWarden {
    pub fn export_changelog(&mut self,id:u64,cx:&mut Context<Self>) {
        if let Some(u)=self.workspace.update(id) {let text=md::changelog_filtered(u,self.changelog_done_only);self.save_with_dialog("CHANGELOG.md".into(),text.into_bytes(),"Changelog gespeichert",cx);}
    }
}
''';p.write_text(s,encoding='utf8')
