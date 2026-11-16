from pathlib import Path
p=Path('src/ui.rs');s=p.read_text(encoding='utf8');s=s.replace('        self.content_width = (f32::from(width) - if self.workspace.settings.sidebar_collapsed {64.} else {240.}).max(0.);\n        let narrow = self.content_width < 900.;\n','');s=s.replace('        let sidebar_w = if icon_only { 56. } else { 220. };','        let sidebar_w = if icon_only { 56. } else { 220. };\n        self.content_width=(f32::from(width)-if self.focus_mode {0.} else {sidebar_w}).max(0.);\n        let narrow=self.content_width<900.;')
s=s.replace('.id(("toast", id))','.id(("toast", id)).role(Role::Status)')
p.write_text(s,encoding='utf8')
p=Path('src/main.rs');s=p.read_text(encoding='utf8').replace('    pub history:','    pub search_gen: u64,\n    pub notes_focused: bool,\n    pub slash_index: usize,\n    pub history:',1).replace('            history, drafts:','            search_gen: 0, notes_focused: false, slash_index: 0,\n            history, drafts:',1)
s=s.replace('''                this.search_query = input.read(cx).value().to_string();
                cx.notify();''','''                let value=input.read(cx).value().to_string();
                this.search_gen+=1;let generation=this.search_gen;
                cx.spawn(async move |entity,cx|{
                    cx.background_executor().timer(StdDuration::from_millis(120)).await;
                    entity.update(cx,|a,cx|{if a.search_gen==generation{a.search_query=value;cx.notify();}}).ok();
                }).detach();''',1)
s=s.replace('''        let notes_sub = cx.subscribe(&notes_input, |this, input, event: &InputEvent, cx| {
            if''','''        let notes_sub = cx.subscribe(&notes_input, |this, input, event: &InputEvent, cx| {
            match event { InputEvent::Focus => {this.notes_focused=true;cx.notify();},InputEvent::Blur => {this.notes_focused=false;cx.notify();},_=>{} }
            if''',1)
s=s.replace('''        let quit_sub = cx.on_app_quit(|this, _| {
            this.flush();''','''        let quit_sub = cx.on_app_quit(|this, _| {
            let discard:Vec<_>=this.drafts.iter().filter_map(|(id,sig)|this.workspace.update(*id).filter(|u|u.title.trim().is_empty() && Self::draft_signature(u)==*sig).map(|_|*id)).collect();
            if !discard.is_empty(){this.workspace.updates.retain(|u|!discard.contains(&u.id));this.dirty=true;}
            this.flush();''')
p.write_text(s,encoding='utf8')
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8').replace('.when(self.workspace.settings.toolbar,','.when(self.workspace.settings.toolbar && self.notes_focused,')
s=s.replace('''                let k = &ev.keystroke;
                if k.modifiers''','''                if this.slash_key(ev,window,cx) {return;}
                let k = &ev.keystroke;
                if k.modifiers''',1)
s=s.replace('            .when(self.workspace.settings.word_count,', '            .child(self.slash_menu(cx))\n            .when(self.workspace.settings.word_count,',1)
s=s.replace('.role(Role::RadioButton)', '.role(Role::RadioButton)')
s=s.replace('div().flex().flex_wrap().gap(px(palette::SP_1)).children(Status::ALL', 'div().id("status-bar").role(Role::RadioGroup).aria_label("Update-Status").flex().flex_wrap().gap(px(palette::SP_1)).children(Status::ALL')
s=s.replace('kit::button(("detail-status",status as usize)).ghost().label(status.label())','kit::button(("detail-status",status as usize)).ghost().role(Role::RadioButton).toggled(status==u.status)')
s=s.replace('.child(kit::status_icon(&p,status,u.task_progress()))','.child(kit::status_icon(&p,status,u.task_progress())).child(status.label())')
s=s.replace('.disabled(!self.writable))\n                        .child(self.status_bar', '.disabled(!self.writable).capture_action(cx.listener(|a,_:&Enter,w,cx|{a.notes_input.update(cx,|i,cx|i.focus(w,cx));cx.stop_propagation();})))\n                        .child(self.status_bar')
p.write_text(s,encoding='utf8')
# Public keyboard command registration list.
p=Path('src/ui/settings.rs');s=p.read_text(encoding='utf8').replace('        "Navigation",','''        "Updates",
        &[
            ("↑ / ↓ / J / K", "Update auswählen"),
            ("Enter", "Titel bearbeiten"),
            ("1 / 2 / 3 / 4", "Status setzen"),
            ("D", "Datum setzen"),
            ("Strg+D", "Update duplizieren"),
            ("Strg+Z", "Änderung rückgängig machen"),
            ("Strg+Y / Strg+Umschalt+Z", "Änderung wiederholen"),
            ("Alt+↑ / Alt+↓", "Vorheriges/nächstes Update"),
            ("Strg+Umschalt+I", "Inspector umschalten"),
            ("Strg+Umschalt+F", "Fokus-Modus umschalten"),
            ("Strg+A", "Alle sichtbaren Updates auswählen"),
            ("X", "Auswahl umschalten"),
        ],
    ),
    (
        "Navigation",''',1)
p.write_text(s,encoding='utf8')
