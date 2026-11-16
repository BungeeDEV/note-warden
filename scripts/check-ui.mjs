import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
function files(dir) { return readdirSync(dir, {withFileTypes:true}).flatMap(e => e.isDirectory() ? files(join(dir,e.name)) : [join(dir,e.name)]); }
const errors = [];
for (const path of files('src').filter(p => p.endsWith('.rs'))) {
  const text = readFileSync(path, 'utf8');
  const primitive = /ui[\\/]kit\.rs$/.test(path);
  const rules = [
    [/<button\b/, 'Roher HTML-Button'],
    [/<(?:div|span|li)\b[^>]*onClick\s*=/, 'Nicht interaktives HTML-Klickziel'],
    [/\.label\([^\n]*(?:→|⌄|‹|›)/, 'Textzeichen als Aktionsicon'],
  ];
  const tokens = /palette\.rs$|logo\.rs$/.test(path);
  if (!tokens) rules.push(
    [/(?:rgb|rgba)\(0x[0-9a-fA-F]+/, 'Farbliteral außerhalb von palette.rs'],
    [/FontWeight::(?:BOLD|THIN|LIGHT|EXTRA_LIGHT|EXTRA_BOLD|BLACK)/, 'Schriftgewicht außer 400/500/600'],
    [/FontWeight\((?!400|500|600)\d+/, 'Schriftgewicht außer 400/500/600'],
    [/\.rounded\(px\((?:4|6|8)\.\)\)/, 'Radius-Literal statt Token'],
    [/Animation::new\((?:std::time::)?(?:Std)?Duration::from_(?:millis|secs)\(/, 'Animationsdauer ohne Motion-Token'],
  );
  if (!primitive) rules.push(
    [/\bIcon::new\(/, 'Icon außerhalb des Wrappers'],
    [/\bButton::new\(/, 'Button außerhalb des Primitives'],
    [/\.icon\(IconName::/, 'Icon ohne Größen-Wrapper'],
    [/kit::icon\([^,]+,\s*\d/, 'Icongröße ohne Token'],
  );
  for (const [pattern, message] of rules) {
    const m = pattern.exec(text);
    if (m) errors.push(`${path}:${text.slice(0,m.index).split('\n').length}: ${message}`);
  }
}
if (errors.length) { console.error(errors.join('\n')); process.exitCode = 1; }
else console.log('UI-Check erfolgreich (Primitive, Icon-Wrapper, Aktionspfeile, Farb-/Gewichts-/Radius-/Motion-Tokens).');
