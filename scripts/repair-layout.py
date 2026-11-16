from pathlib import Path
import re
exec(Path('scripts/migrate-clicks.py').read_text(encoding='utf8').split('report=[]')[0])
# Styled library Size::Size changes icon sizing and does not establish text-button height.
# Use semantic sizes and explicit control heights instead.
for p in [Path('src/ui.rs'),*Path('src/ui').glob('*.rs')]:
 s=p.read_text(encoding='utf8')
 for token,kind in [('CTL_SM','Small'),('CTL_MD','Medium'),('CTL_LG','Medium')]:
  s=s.replace(f'.with_size(px(palette::{token}))',f'.with_size(gpui_kit::component::Size::{kind}).h(px(palette::{token}))')
 s=s.replace('.with_size(px(32.))','.with_size(gpui_kit::component::Size::Medium).h(px(palette::CTL_MD))')
 if p.name=='kit.rs':s=s.replace('Button::new(id).with_size(gpui_kit::component::Size::Medium).h(px(palette::CTL_MD))','Button::new(id).with_size(gpui_kit::component::Size::Medium).h(px(palette::CTL_MD)).cursor_pointer()')
 p.write_text(s,encoding='utf8')
# Complex clickable surfaces use the native base button: it preserves the actual
# child layout instead of injecting the styled button's single-line content box.
for p in [Path('src/ui.rs'),*Path('src/ui').glob('*.rs')]:
 if p.name in ['kit.rs','icons.rs']:continue
 s=p.read_text(encoding='utf8');changes=[]
 for m in re.finditer(r'kit::button\(',s):
  op=m.end()-1;end=end_call(s,op);arg=s[op+1:end-1];pos=end;calls=[]
  while True:
   c=re.match(r'\s*\.([A-Za-z_]\w*)\s*\(',s[pos:])
   if not c:break
   co=pos+c.end()-1;ce=end_call(s,co)
   if ce<=co:break
   calls.append((c[1],pos,co,ce));pos=ce
  names=[c[0] for c in calls]
  if 'on_click' not in names or 'child' not in names or 'label' in names or 'icon' in names:continue
  suppress=any(name=='on_click' and 'cx.stop_propagation()' in s[co:ce] and not ('update(' in s[co:ce]) for name,st,co,ce in calls)
  if suppress:
   changes.append((m.start(),end,f'div().id({arg}).role(Role::Group)'))
  else:changes.append((m.start(),end,f'kit::surface({arg}, &p)'))
  for name,st,co,ce in calls:
   if name in ['ghost','with_size','focus_visible']:changes.append((st,ce,''))
   elif name=='toggled':changes.append((st,ce,'.selected'+s[co:ce]))
   elif name=='tooltip':changes.append((st,ce,'.tooltip(tip('+s[co+1:ce-1]+'))'))
 for a,b,r in sorted(changes,reverse=True):s=s[:a]+r+s[b:]
 p.write_text(s,encoding='utf8')
p=Path('src/ui.rs');s=p.read_text(encoding='utf8');
for name in ['nav_item','view_item','group_header']:
 a=s.index('    fn '+name+'(');b=s.index(' {',a);head=s[a:b];s=s[:a]+head.replace('-> Button','-> gpui_kit::base::Button')+s[b:]
p.write_text(s,encoding='utf8')
p=Path('src/ui/kit.rs');s=p.read_text(encoding='utf8');s+='''
/// Native Button für Navigation, Zeilen und mehrzeilige Kalenderzellen.
/// Im Gegensatz zum beschrifteten Toolbar-Button bleibt das Child-Layout erhalten.
pub fn surface(id: impl Into<ElementId>, p: &Palette) -> gpui_kit::base::Button {
    gpui_kit::base::Button::new(id).cursor_pointer()
        .hover(|s|s.bg(p.bg_hover))
        .active(|s|s.bg(p.bg_active))
        .focus_visible(|s|s.border_2().border_color(p.accent))
}
''';p.write_text(s,encoding='utf8')
