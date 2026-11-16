from pathlib import Path
p=Path('src/ui/list.rs');s=p.read_text(encoding='utf8').replace("enum Row<'a>","#[derive(Clone)]\nenum Row<'a>")
a=s.index('                    .children(rows.into_iter().map(|row| {');b=s.index('                    })),',a)+len('                    })),');chunk=s[a:b];body=chunk[chunk.index('                        match row {'):chunk.rfind('                    })),')];
s=s[:a]+'''                    .when(count<100,|d|d.children(rows.into_iter().map(|row|self.render_row(row,cx))))
                    .when(count>=100,|d| {
                        let e=cx.entity();let row_count=self.rows(self.visible_updates()).len();
                        d.overflow_hidden().child(uniform_list("virtual-updates",row_count,move |range,_,cx|e.update(cx,|a,cx|{
                            let rows=a.rows(a.visible_updates());range.filter_map(|i|rows.get(i).cloned()).map(|row|a.render_row(row,cx)).collect::<Vec<_>>()
                        })).track_scroll(&self.virtual_scroll).size_full())
                    }),'''+s[b:]
pos=s.index('    pub(super) fn update_list(');s=s[:pos]+'    fn render_row(&self,row:Row<\'_>,cx:&mut Context<Self>)->AnyElement {\n'+body+'\n    }\n\n'+s[pos:]
s=s.replace('            .when(self.multi.len() > 1,','            .when(!self.multi.is_empty(),')
s=s.replace('.id("update-list").key_context', '.id("update-list").role(Role::ListBox).aria_label("Updates").key_context')
s=s.replace('.group(group.clone()).relative()', '.role(Role::ListBoxOption).tab_stop(selected).toggled(selected).group(group.clone()).relative()')
# Sort direction control, common empty actions.
s=s.replace('            .child(self.updates_filters(cx))','''            .child(self.updates_filters(cx))
            .child(kit::button("sort-direction").ghost().label(if self.workspace.settings.updates_ui.reverse {"Aufsteigend sortieren"} else {"Sortierung umkehren"}).on_click({let e=cx.entity();move |_,_,cx|e.update(cx,|a,cx|{a.workspace.settings.updates_ui.reverse=!a.workspace.settings.updates_ui.reverse;a.persist(cx);cx.notify();})}))''')
s=s.replace('"Keine Treffer"','"Nichts gefunden"').replace('"Nichts zu sehen"','"Keine Updates passen zu den Filtern"')
s=s.replace('                                .when(!any_updates && filter == Filter::All && !searching,', '''                                .when(any_updates || searching,|d|d.child(kit::button("clear-list-filters").ghost().label(if searching {"Suche leeren"} else {"Filter zurücksetzen"}).on_click({let e=cx.entity();move |_,w,cx|e.update(cx,|a,cx|{a.filter=Filter::All;a.project_filter=None;a.workspace.settings.updates_ui.statuses.clear();a.workspace.settings.updates_ui.priorities.clear();a.workspace.settings.updates_ui.date_filter=0;a.search_query.clear();a.search_input.update(cx,|i,cx|i.set_value("",w,cx));a.save_updates_ui(cx);})})))
                                .when(!any_updates && filter == Filter::All && !searching,''',1)
p.write_text(s,encoding='utf8')
p=Path('src/main.rs');s=p.read_text(encoding='utf8').replace('    pub list_scroll: ScrollHandle,','    pub list_scroll: ScrollHandle,\n    pub virtual_scroll: UniformListScrollHandle,').replace('            list_scroll: ScrollHandle::new(),','            list_scroll: ScrollHandle::new(), virtual_scroll: UniformListScrollHandle::new(),');p.write_text(s,encoding='utf8')
p=Path('src/ui/workbench.rs');s=p.read_text(encoding='utf8');s=s.replace('        if mods.control || mods.alt || mods.platform { return; }','''        if mods.control && key=="a" {self.multi=self.visible_updates().iter().map(|u|u.id).collect();cx.notify();cx.stop_propagation();return;}
        if mods.control && key=="d" {if let Some(id)=self.selected_update{self.duplicate_update(id,window,cx);}cx.stop_propagation();return;}
        if mods.control || mods.alt || mods.platform { return; }''')
s=s.replace('            "/" => self.focus_search', '''            "x" => {if let Some(id)=self.selected_update {if !self.multi.remove(&id){self.multi.insert(id);}cx.notify();}},
            "s" => {if let Some(id)=self.selected_update {self.keyboard_menu=Some((id,0));cx.notify();}},
            "p" => {if let Some(id)=self.selected_update {self.keyboard_menu=Some((id,1));cx.notify();}},
            "l" => {if let Some(id)=self.selected_update {self.keyboard_menu=Some((id,2));cx.notify();}},
            "/" => self.focus_search''')
p.write_text(s,encoding='utf8')
