from pathlib import Path
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8');lines=s.splitlines();
for i,l in enumerate(lines):
 if '.child(div().capture_action' in l and 'self.title_input' in l:lines[i]='''                        .child(div().capture_action(cx.listener(|a,_:&Enter,w,cx|{a.notes_input.update(cx,|i,cx|i.focus(w,cx));cx.stop_propagation();})).child(Textarea::new(&self.title_input).appearance(false).bordered(false).aria_label("Titel des Updates").text_size(palette::fs(palette::DETAIL_TITLE)).font_weight(FontWeight::SEMIBOLD).px(px(0.)).w_full().disabled(!self.writable)))'''
p.write_text('\n'.join(lines)+'\n',encoding='utf8')
