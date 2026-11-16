import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
function files(dir) { return readdirSync(dir, {withFileTypes:true}).flatMap(e => e.isDirectory() ? files(join(dir,e.name)) : [join(dir,e.name)]); }
const errors = [];
for (const path of files('src').filter(p => p.endsWith('.rs'))) {
  const text = readFileSync(path, 'utf8');
  const primitive = /ui[\\/]kit\.rs$/.test(path);
  const rules = [
    [/<button\b/, 'Raw HTML button'],
    [/<(?:div|span|li)\b[^>]*onClick\s*=/, 'Non-interactive HTML click target'],
    [/\.label\([^\n]*(?:→|⌄|‹|›)/, 'Text character used as an action icon'],
  ];
  const tokens = /palette\.rs$|logo\.rs$/.test(path);
  if (!tokens) rules.push(
    [/(?:rgb|rgba)\(0x[0-9a-fA-F]+/, 'Color literal outside palette.rs'],
    [/FontWeight::(?:BOLD|THIN|LIGHT|EXTRA_LIGHT|EXTRA_BOLD|BLACK)/, 'Font weight other than 400/500/600'],
    [/FontWeight\((?!400|500|600)\d+/, 'Font weight other than 400/500/600'],
    [/\.rounded\(px\((?:4|6|8)\.\)\)/, 'Radius literal instead of a token'],
    [/Animation::new\((?:std::time::)?(?:Std)?Duration::from_(?:millis|secs)\(/, 'Animation duration without a motion token'],
  );
  if (!primitive) rules.push(
    [/\bIcon::new\(/, 'Icon outside the wrapper'],
    [/\bButton::new\(/, 'Button outside the primitive'],
    [/\.icon\(IconName::/, 'Icon without a size wrapper'],
    [/kit::icon\([^,]+,\s*\d/, 'Icon size without a token'],
  );
  for (const [pattern, message] of rules) {
    const m = pattern.exec(text);
    if (m) errors.push(`${path}:${text.slice(0,m.index).split('\n').length}: ${message}`);
  }
}
if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
else console.log('UI check passed (primitives, icon wrappers, action arrows, color/weight/radius/motion tokens).');
