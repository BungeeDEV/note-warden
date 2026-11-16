from pathlib import Path
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8');a=s.index('    pub(super) fn update_detail(');b=s.index('\n}\n\n/// Stabiler',a)
s=s[:a]+'''    fn properties(&self,u:&model::Update,inspector:bool,cx:&mut Context<Self>) -> Div {
        let p=palette::get(cx);let (done,total)=u.task_progress();
        div().flex().when(inspector,|d|d.flex_col()).when(!inspector,|d|d.flex_wrap()).gap(px(palette::SP_2))
            .child(self.project_control(u.id,u.project_id,cx))
            .child(self.priority_control(u.id,u.priority,cx))
            .child(self.date_chip("detail-date".into(),DateTarget::Update(u.id),u.planned_for,u.status==Status::Erledigt,cx))
            .child(kit::input(&self.version_input).aria_label("Version des Updates").w(px(120.)).disabled(!self.writable))
            .when(total>0,|d|d.child(div().flex().items_center().gap(px(palette::SP_2)).child(kit::progress_ring(&p,done,total)).child(text_secondary(&p,format!("{done} von {total} erledigt")))))
            .when(inspector,|d|d.child(text_secondary(&p,format!("Erstellt: {}",dates::elapsed(u.created_at))))
                .child(text_secondary(&p,format!("Bearbeitet: {}",dates::elapsed(u.edited_at)))))
    }
    fn status_bar(&self,u:&model::Update,cx:&mut Context<Self>) -> Div {
        let p=palette::get(cx);let id=u.id;
        div().flex().flex_wrap().gap(px(palette::SP_1)).children(Status::ALL.into_iter().map(|status| {
            let e=cx.entity();kit::button(("detail-status",status as usize)).ghost().label(status.label())
                .child(kit::status_icon(&p,status,u.task_progress()))
                .when(status==u.status,|b|b.bg(p.status(status).opacity(0.14)).text_color(p.text_primary))
                .on_click(move |_,_,cx|e.update(cx,|a,cx|a.set_status(id,status,cx)))
                .on_key_down({let e=cx.entity();move |ev,w,cx| {
                    let current=Status::ALL.iter().position(|s|*s==status).unwrap_or(0);
                    let next=match ev.keystroke.key.as_str(){"left"=>Some((current+3)%4),"right"=>Some((current+1)%4),"1"=>Some(0),"2"=>Some(1),"3"=>Some(2),"4"=>Some(3),_=>None};
                    if let Some(n)=next {e.update(cx,|a,cx|a.set_status(id,Status::ALL[n],cx));cx.stop_propagation();let _=w;}
                }})
        }))
    }
    fn changelog_panel(&self,u:&model::Update,cx:&mut Context<Self>) -> Div {
        let p=palette::get(cx);let text=md::changelog(u,self.changelog_done_only);let copy=text.clone();let e=cx.entity();let e2=cx.entity();let id=u.id;
        div().flex().flex_col().gap(px(palette::SP_3))
            .child(div().flex().gap(px(palette::SP_2))
                .child(kit::button("changelog-done").ghost().label(if self.changelog_done_only{"Alle Aufgaben zeigen"}else{"Nur erledigte Aufgaben"}).on_click(move |_,_,cx|e.update(cx,|a,cx|{a.changelog_done_only=!a.changelog_done_only;cx.notify();})))
                .child(kit::button("changelog-copy").outline().label("Kopieren").on_click(move |_,_,cx|{cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()));e2.update(cx,|a,cx|a.toast("Changelog kopiert",None,cx));}))
                .child(kit::button("changelog-save").outline().label("Als Datei speichern").on_click({let e=cx.entity();move |_,_,cx|e.update(cx,|a,cx|a.export_update_md(id,cx))})))
            .child(div().font_family(palette::FONT_MONO).text_size(palette::fs(palette::FS_BASE)).child(text))
    }
    pub(super) fn update_detail(&self,narrow:bool,cx:&mut Context<Self>) -> impl IntoElement {
        let p=palette::get(cx);let Some(u)=self.selected() else {return kit::empty_state(&p,"Wähle ein Update aus","↑/↓ auswählen · Enter öffnen · Strg+N anlegen",None).flex_1().into_any_element();};
        let id=u.id;let inspector=self.content_width>=1280. && !self.workspace.settings.updates_ui.inspector_hidden && !self.focus_mode;
        let e=cx.entity();let prev=cx.entity();let next=cx.entity();let toggle=cx.entity();let tabs=cx.entity();let complete=cx.entity();let dismiss=cx.entity();
        let (done,total)=u.task_progress();let mode=self.effective_mode(u);
        div().flex_1().min_w_0().h_full().flex().bg(p.bg_panel)
            .child(div().flex_1().min_w_0().h_full().flex().flex_col()
                .child(div().h(px(palette::SP_8+palette::SP_4)).px(px(palette::SP_4)).flex_none().flex().items_center().justify_between()
                    .when(self.detail_scroll.offset().y<px(-4.),|d|d.shadow_sm())
                    .child(div().flex().items_center().gap(px(palette::SP_2))
                        .when(narrow,|d|d.child(icon_btn("detail-back",icons::BACK,"Zurück").on_click(move |_,_,cx|e.update(cx,|a,cx|{a.discard_untouched_drafts(None,cx);a.detail_open=false;cx.notify();}))))
                        .when(self.detail_scroll.offset().y<px(-80.),|d|d.child(text_secondary(&p,format!("Updates / {}",u.title)))))
                    .child(div().flex().items_center().gap(px(palette::SP_1))
                        .child(self.save_status(&p))
                        .child(icon_btn("detail-prev",IconName::ChevronLeft,"Vorheriges Update · Alt+↑").on_click(move |_,w,cx|prev.update(cx,|a,cx|a.select_relative(-1,w,cx))))
                        .child(icon_btn("detail-next",IconName::ChevronRight,"Nächstes Update · Alt+↓").on_click(move |_,w,cx|next.update(cx,|a,cx|a.select_relative(1,w,cx))))
                        .child(icon_btn("detail-inspector",IconName::PanelRightOpen,"Inspector · Strg+Umschalt+I").on_click(move |_,_,cx|toggle.update(cx,|a,cx|{a.workspace.settings.updates_ui.inspector_hidden=!a.workspace.settings.updates_ui.inspector_hidden;a.persist(cx);cx.notify();})))
                        .child(icon_btn("detail-menu",icons::MORE,"Update-Aktionen").dropdown_menu({let build=update_menu(cx.entity(),id,u.archived,u.deleted_on.is_some(),true);move |m,w,cx|build(m,w,cx)}))))
                .child(div().id("detail-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.detail_scroll).vertical_scrollbar(&self.detail_scroll)
                    .on_scroll(cx.listener(|_,_,_,cx|cx.notify())).px(px(palette::SP_6)).flex().justify_center().items_start()
                    .child(div().w_full().max_w(px(palette::DETAIL_MAX_W)).pt(px(palette::SP_8)).pb(px(96.)).flex().flex_col().gap(px(palette::SP_6))
                        .child(Textarea::new(&self.title_input).appearance(false).bordered(false).aria_label("Titel des Updates").text_size(palette::fs(palette::DETAIL_TITLE)).font_weight(FontWeight::SEMIBOLD).px(px(0.)).w_full().disabled(!self.writable))
                        .child(self.status_bar(u,cx))
                        .when(!inspector,|d|d.child(self.properties(u,false,cx)))
                        .child(kit::segment("detail-tabs",&p,&["Übersicht","Changelog","Aktivität"],self.detail_tab,move |tab,_,cx|tabs.update(cx,|a,cx|{a.detail_tab=tab;cx.notify();})))
                        .when(self.detail_tab==0,|d|d
                            .child(kit::card_header(&p,"Notizen",self.mode_switch(mode,id,cx)))
                            .child(self.editor(u,cx)).child(self.task_section(u,cx))
                            .when(total>0 && done==total && u.status!=Status::Erledigt && self.completion_dismissed!=Some(id),|d|d.child(div().flex().flex_wrap().items_center().gap(px(palette::SP_2))
                                .child(text_secondary(&p,"Alle Aufgaben erledigt. Update abschließen?"))
                                .child(kit::toned_button("finish-update",&p,kit::ButtonTone::Success,palette::CTL_MD).label("Auf Erledigt setzen").on_click(move |_,_,cx|complete.update(cx,|a,cx|a.set_status(id,Status::Erledigt,cx))))
                                .child(kit::button("finish-later").ghost().label("Später").on_click(move |_,_,cx|dismiss.update(cx,|a,cx|{a.completion_dismissed=Some(id);cx.notify();}))))))
                        .when(self.detail_tab==1,|d|d.child(self.changelog_panel(u,cx)))
                        .when(self.detail_tab==2,|d|d.child(div().flex().flex_col().gap(px(palette::SP_3)).children(u.activity.iter().rev().map(|event|div().flex().justify_between().gap(px(palette::SP_2)).child(event.data.clone()).child(text_secondary(&p,dates::elapsed(event.at)))))))
                    )))
            .when(inspector,|d|d.child(div().w(px(palette::INSPECTOR_W)).h_full().flex_none().p(px(palette::SP_4)).bg(p.bg_sidebar).border_l_1().border_color(p.border_subtle).flex().flex_col().gap(px(palette::SP_4))
                .child("Eigenschaften").child(self.properties(u,true,cx))))
            .into_any_element()
    }
'''+s[b:]
s=s.replace('.when(self.workspace.settings.toolbar,', '.when(self.workspace.settings.toolbar && self.notes_input.read(cx).focus_handle(cx).is_focused(cx.window()),') if False else s
# Toolbar focus visibility can use focus handle via window passed at render in later change.
p.write_text(s,encoding='utf8')
