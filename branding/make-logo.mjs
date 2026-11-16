// Erzeugt branding/logo.svg (Pixel-Schild, 32x32-Raster, Zelle = 32px bei 1024px Canvas)
// und branding/logo-small.svg (Variante für Größen <= 32 px). Aufruf aus dem Projektstamm.
import { writeFileSync, mkdirSync } from 'node:fs';
const C = 32;
const lerp = (a, b, t) => a.map((v, i) => Math.round(v + (b[i] - v) * t));
const hex = (c) => '#' + c.map((v) => v.toString(16).padStart(2, '0')).join('');
const top = [0x9b, 0xe3, 0xff], bot = [0x0a, 0x7f, 0xc4];
const ext = { 7: [8, 19], 8: [8, 20], 9: [8, 21], 10: [8, 22], 19: [9, 22], 20: [9, 22], 21: [10, 21], 22: [10, 21], 23: [11, 20], 24: [12, 19], 25: [13, 18], 26: [14, 17], 27: [15, 16] };
for (let r = 11; r <= 18; r++) ext[r] = [8, 23];
const cells = new Map();
const key = (x, y) => `${x},${y}`;
const set = (x, y, c) => cells.set(key(x, y), c);
for (const r of Object.keys(ext).map(Number).sort((a, b) => a - b)) {
  const [a, b] = ext[r];
  const t = Math.round(((r - 7) / 20) * 5) / 5;
  const base = lerp(top, bot, t);
  for (let x = a; x <= b; x++) {
    let col = base;
    if (x === a && r <= 22) col = lerp(base, [255, 255, 255], 0.22);
    if (x === b && r <= 22) col = lerp(base, [0, 20, 50], 0.16);
    set(x, r, col);
  }
}
for (const [x, y] of [[19,8],[19,9],[19,10],[20,11],[21,11],[22,11]]) set(x, y, lerp(cells.get(key(x, y)), [0, 40, 80], 0.28));
for (const [x, y] of [[20,9],[20,10],[21,10]]) set(x, y, [0xe9, 0xf8, 0xff]);   // Eselsohr hell
for (const [x, y] of [[20,8],[21,9],[22,10]]) set(x, y, [0xbf, 0xe7, 0xfa]);    // Eselsohr Kante
const dark = [0x0a, 0x16, 0x20], mid = [0x1d, 0x5a, 0x80];
for (let x = 11; x <= 19; x++) set(x, 11, dark);   // Notizzeile 1
for (let x = 11; x <= 16; x++) set(x, 13, mid);    // Notizzeile 2
for (const [x, y] of [[11,17],[12,18],[13,19],[14,20],[15,19],[16,18],[17,17],[18,16],[19,15],[20,14]]) { set(x, y, dark); set(x + 1, y, dark); } // Haken
const out = [
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">',
  '<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#1b2b36"/><stop offset="1" stop-color="#0b0e11"/></linearGradient><radialGradient id="glow" cx="0.5" cy="0.5" r="0.55"><stop offset="0" stop-color="#38bdf8" stop-opacity="0.26"/><stop offset="1" stop-color="#38bdf8" stop-opacity="0"/></radialGradient></defs>',
  '<rect width="1024" height="1024" rx="232" fill="url(#bg)"/><rect width="1024" height="1024" rx="232" fill="url(#glow)"/>',
  '<rect x="3" y="3" width="1018" height="1018" rx="229" fill="none" stroke="#fff" stroke-opacity="0.07" stroke-width="6"/>',
  '<g shape-rendering="crispEdges">',
];
for (const [x, y, o] of [[5,6,.5],[26,5,.35],[4,20,.3],[27,22,.45],[6,26,.25],[25,27,.3]]) out.push(`<rect x="${x*C}" y="${y*C}" width="${C}" height="${C}" fill="#7dd3fc" fill-opacity="${o}"/>`);
const shadow = new Set();
for (const k of cells.keys()) { const [x, y] = k.split(',').map(Number); shadow.add(key(x + 1, y + 1)); }
for (const k of shadow) if (!cells.has(k)) { const [x, y] = k.split(',').map(Number); out.push(`<rect x="${x*C}" y="${y*C}" width="${C}" height="${C}" fill="#02080c" fill-opacity="0.32"/>`); }
const sorted = [...cells.entries()].map(([k, c]) => [...k.split(',').map(Number), c]).sort((a, b) => a[1] - b[1] || a[0] - b[0]);
for (const [x, y, c] of sorted) out.push(`<rect x="${x*C}" y="${y*C}" width="${C + 0.5}" height="${C + 0.5}" fill="${hex(c)}"/>`);
out.push('</g></svg>');
mkdirSync('branding', { recursive: true });
writeFileSync('branding/logo.svg', out.join('\n'));
console.log('branding/logo.svg geschrieben');

// --- Kleine Variante: gleiche Form, ohne Funken, zweite Notizzeile und Schlagschatten ---
const row13 = lerp(top, bot, Math.round(((13 - 7) / 20) * 5) / 5);
for (let x = 11; x <= 16; x++) set(x, 13, row13); // Notizzeile 2 zurück auf Schildfarbe
const small = out.slice(0, 5);
const smallSorted = [...cells.entries()].map(([k, c]) => [...k.split(',').map(Number), c]).sort((a, b) => a[1] - b[1] || a[0] - b[0]);
for (const [x, y, c] of smallSorted) small.push(`<rect x="${x*C}" y="${y*C}" width="${C + 0.5}" height="${C + 0.5}" fill="${hex(c)}"/>`);
small.push('</g></svg>');
writeFileSync('branding/logo-small.svg', small.join('\n'));
console.log('branding/logo-small.svg geschrieben');
