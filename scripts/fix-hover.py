from pathlib import Path
exec(Path('scripts/migrate-clicks.py').read_text(encoding='utf8').split('report=[]')[0])
for p in [Path('src/ui.rs'),*Path('src/ui').glob('*.rs')]:
 if p.name=='icons.rs':continue
 s=p.read_text(encoding='utf8');reps=[]
 # Hover on native buttons is supplied by GPUI. Remove feature hover refinements
 # from button chains (including conditional refinements), leaving layout divs intact.
 for match in re.finditer(r'(?:kit::)?(?:button|task_checkbox)\(',s):
  start=match.end()-1;pos=end_call(s,start)
  while True:
   m=re.match(r'\s*\.([A-Za-z_]\w*)\s*\(',s[pos:])
   if not m:break
   op=pos+m.end()-1;en=end_call(s,op)
   if en<=op:break
   pos=en
  region=s[match.start():pos]
  for m in re.finditer(r'\.hover\s*\(',region):
   op=match.start()+m.end()-1;en=end_call(s,op);reps.append((match.start()+m.start(),en,''))
 for a,b,r in sorted(set(reps),reverse=True):s=s[:a]+r+s[b:]
 p.write_text(s,encoding='utf8')
