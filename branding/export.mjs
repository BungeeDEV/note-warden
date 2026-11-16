// Rendert PNGs und app.ico aus den SVGs. Aufruf aus dem Projektstamm (npm run brand).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { Resvg } from '@resvg/resvg-js';

const render = (file, size) =>
  new Resvg(readFileSync(file), { fitTo: { mode: 'width', value: size } }).render().asPng();

mkdirSync('branding/icons', { recursive: true });
writeFileSync('branding/logo-1024.png', render('branding/logo.svg', 1024));

// Kleine Größen aus der kleinen Variante, große aus dem vollen Logo.
const sizes = [
  [16, 'branding/logo-small.svg'], [24, 'branding/logo-small.svg'], [32, 'branding/logo-small.svg'],
  [48, 'branding/logo-small.svg'], [64, 'branding/logo.svg'], [128, 'branding/logo.svg'], [256, 'branding/logo.svg'],
];
const images = sizes.map(([size, file]) => ({ size, png: render(file, size) }));
for (const { size, png } of images) writeFileSync(`branding/icons/${size}.png`, png);

// ICO mit eingebetteten PNGs (Windows Vista und neuer).
const header = Buffer.alloc(6);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(images.length, 4);
let offset = 6 + images.length * 16;
const entries = images.map(({ size, png }) => {
  const e = Buffer.alloc(16);
  e[0] = size === 256 ? 0 : size;
  e[1] = size === 256 ? 0 : size;
  e.writeUInt16LE(1, 4);
  e.writeUInt16LE(32, 6);
  e.writeUInt32LE(png.length, 8);
  e.writeUInt32LE(offset, 12);
  offset += png.length;
  return e;
});
writeFileSync('branding/app.ico', Buffer.concat([header, ...entries, ...images.map((i) => i.png)]));
console.log('branding/logo-1024.png, icons/*.png und app.ico geschrieben');
