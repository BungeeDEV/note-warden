from pathlib import Path
# Find fluent top-level call chains without rewriting nested child controls.
def end_call(s,start):
 depth=0;quote=None;i=start
 while i<len(s):
  c=s[i]
  if quote:
   if c=='\\':i+=2;continue
   if c==quote:quote=None
  elif c=='"':quote=c
  elif c=='(' :depth+=1
  elif c==')':
   depth-=1
   if depth==0:return i+1
  i+=1
 return start
import re
report=[]
for p in [Path('src/ui.rs'),*Path('src/ui').glob('*.rs')]:
 if p.name in ['kit.rs','icons.rs','workbench.rs']:continue
 s=p.read_text(encoding='utf8');replacements=[]
 for m in re.finditer(r'\bdiv\(\)',s):
  calls=[];pos=m.end()
  while True:
   match=re.match(r'\s*\.([a-zA-Z_][\w]*)\s*\(',s[pos:])
   if not match:break
   op=pos+match.end()-1;end=end_call(s,op)
   if end<=op:break
   calls.append((match[1],pos,op,end));pos=end
  names=[c[0] for c in calls]
  if 'on_click' not in names:continue
  ids=[c for c in calls if c[0]=='id']
  if not ids:
   report.append(str(p)+': click surface without id');continue
  ident=ids[0];arg=s[ident[2]+1:ident[3]-1]
  replacements.append((m.start(),m.end(),f'kit::button({arg}).ghost()'))
  replacements.append((ident[1],ident[3],''))
  for name,st,op,en in calls:
   if name=='tooltip' and s[op+1:en-1].strip().startswith('tip('):
    inner=s[op+1:en-1].strip()[4:-1];replacements.append((st,en,'.tooltip('+inner+')'))
 for a,b,r in sorted(replacements,reverse=True):s=s[:a]+r+s[b:]
 p.write_text(s,encoding='utf8')
 print(p,len(replacements))
print('\n'.join(report))
# signatures of central sidebar helpers now return actual native buttons.
p=Path('src/ui.rs');s=p.read_text(encoding='utf8').replace(') -> Stateful<Div> {',') -> Button {');p.write_text(s,encoding='utf8')
