from pathlib import Path
p=Path('src/main.rs');s=p.read_text(encoding='utf8').replace('    pub search_gen: u64,','    pub keyboard_menu: Option<(u64,usize)>,\n    pub keyboard_index: usize,\n    pub search_gen: u64,').replace('            search_gen: 0,','            keyboard_menu: None, keyboard_index: 0,\n            search_gen: 0,').replace('        if self.focus_mode {','        if self.keyboard_menu.take().is_some() {self.focus_handle.focus(window,cx);cx.notify();return;}\n        if self.focus_mode {',1);p.write_text(s,encoding='utf8')
p=Path('src/ui/workbench.rs');s=p.read_text(encoding='utf8').replace('self.keyboard_menu=Some((id,0));cx.notify();','self.keyboard_menu=Some((id,0));self.keyboard_index=0;self.overlay_focus.focus(window,cx);cx.notify();').replace('self.keyboard_menu=Some((id,1));cx.notify();','self.keyboard_menu=Some((id,1));self.keyboard_index=0;self.overlay_focus.focus(window,cx);cx.notify();').replace('self.keyboard_menu=Some((id,2));cx.notify();','self.keyboard_menu=Some((id,2));self.keyboard_index=0;self.overlay_focus.focus(window,cx);cx.notify();')
s+='''
impl NoteWarden {
    fn keyboard_options(&self)->Vec<String> {
        match self.keyboard_menu.map(|(_,n)|n) {Some(0)=>Status::ALL.iter().map(|s|s.label().to_owned()).collect(),Some(1)=>["Keine Priorität","Niedrig","Mittel","Hoch"].map(str::to_owned).to_vec(),Some(2)=>std::iter::once("Kein Projekt".to_owned()).chain(self.workspace.projects.iter().filter(|p|!p.archived).map(|p|p.name.clone())).collect(),_=>vec![]}
    }
    fn keyboard_choose(&mut self,n:usize,w:&mut Window,cx:&mut Context<Self>) {
        let Some((id,kind))=self.keyboard_menu.take() else{return;};
        if self.writable {match kind {0=>self.set_status(id,Status::ALL[n.min(3)],cx),1=>{if let Some(u)=self.workspace.update_mut(id){u.priority=n.min(3) as u8;u.touch();}self.persist(cx);},_=>{let project=if n==0{None}else{self.workspace.projects.iter().filter(|p|!p.archived).nth(n-1).map(|p|p.id)};if let Some(u)=self.workspace.update_mut(id){u.project_id=project;u.touch();}self.persist(cx);}}}
        self.focus_handle.focus(w,cx);cx.notify();
    }
    pub fn keyboard_popup(&self,cx:&mut Context<Self>)->AnyElement {
        if self.keyboard_menu.is_none(){return div().into_any_element();}
        let p=palette::get(cx);let options=self.keyboard_options();
        div().id("update-keyboard-menu").role(Role::Menu).track_focus(&self.overlay_focus).absolute().top(px(96.)).left(px(240.)).w(px(240.)).p(px(palette::SP_2)).rounded(px(palette::MENU_RADIUS)).bg(p.bg_elevated).border_1().border_color(p.border_subtle).shadow_lg().flex().flex_col()
            .on_key_down(cx.listener(|a,ev:&KeyDownEvent,w,cx|{let n=a.keyboard_options().len();match ev.keystroke.key.as_str(){"down"=>a.keyboard_index=(a.keyboard_index+1)%n,"up"=>a.keyboard_index=(a.keyboard_index+n-1)%n,"enter"=>a.keyboard_choose(a.keyboard_index,w,cx),"1"|"2"|"3"|"4"=>a.keyboard_choose(ev.keystroke.key.parse::<usize>().unwrap()-1,w,cx),_=>return}cx.stop_propagation();cx.notify();}))
            .children(options.into_iter().enumerate().map(|(n,label)|{let e=cx.entity();kit::button(("keyboard-option",n)).ghost().role(Role::MenuItem).w_full().justify_start().label(label).when(n==self.keyboard_index,|b|b.bg(p.bg_active)).on_click(move |_,w,cx|e.update(cx,|a,cx|a.keyboard_choose(n,w,cx)))})).into_any_element()
    }
}
''';p.write_text(s,encoding='utf8')
p=Path('src/ui.rs');s=p.read_text(encoding='utf8').replace('            .child(self.toasts(cx))','            .child(self.keyboard_popup(cx))\n            .child(self.toasts(cx))');p.write_text(s,encoding='utf8')
