import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { fontLicenseBanner } from '../../web/shared/font-licenses.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const fontRoot = path.join(root, 'ui/design-system/fonts');
const provenance = JSON.parse(readFileSync(path.join(fontRoot, 'provenance.json'), 'utf8'));
const expectedFonts = Object.fromEntries([
  ['Barlow', 'Barlow'], ['Barlow Semi Condensed', 'BarlowSemiCondensed'],
  ['JetBrains Mono', 'JetBrainsMono'],
].flatMap(([family, directory]) => (family === 'JetBrains Mono'
  ? [['Medium', 500], ['SemiBold', 600]] : [['Medium', 500], ['SemiBold', 600], ['Bold', 700]])
  .map(([style, weight]) => [`${family}:${weight}`,
    provenance.files[`${directory}/${directory}-${style}.ttf`].sha256])));

for (const app of ['sampler', 'tb303']) {
  const dist = path.join(root, 'web', app, 'dist');
  test(`${app} packages all declared fonts into locally served CSS`, () => {
    const css = readFileSync(path.join(dist, 'app.css'), 'utf8');
    const faces = [...css.matchAll(/@font-face\s*\{([^}]+)\}/g)].map(([, face]) => {
      const family = face.match(/font-family:\s*["']?([^;"']+)/)?.[1].trim();
      const weight = face.match(/font-weight:\s*(\d+)/)?.[1];
      const encoded = face.match(/url\(["']?data:font\/ttf;base64,([A-Za-z0-9+/=]+)["']?\)/)?.[1];
      assert.ok(family && weight && encoded, `invalid local font face: ${family}:${weight}`);
      return [`${family}:${weight}`, createHash('sha256').update(Buffer.from(encoded, 'base64')).digest('hex')];
    });
    assert.equal(faces.length, 8);
    const inlineFonts = Object.fromEntries(faces);
    assert.deepEqual(inlineFonts, expectedFonts);
    for (const family of ['Barlow', 'Barlow Semi Condensed', 'JetBrains Mono'])
      assert.ok(css.includes(family), family);
    const declaredTokens = new Set([...css.matchAll(/(--font-[A-Za-z0-9_-]+)\s*:/g)].map(([, name]) => name));
    for (const [, token] of css.matchAll(/var\(\s*(--font-[A-Za-z0-9_-]+)/g))
      assert.ok(declaredTokens.has(token), `undeclared font token ${token}`);
    assert.doesNotMatch(css, /@import|fonts\.googleapis\.com|https?:\/\//);
  });
  test(`${app} executable assets retain original font licenses and need only embedded routes`, () => {
    const js = readFileSync(path.join(dist, 'app.js'), 'utf8');
    for (const family of ['Barlow', 'BarlowSemiCondensed', 'JetBrainsMono']) {
      const notice = readFileSync(path.join(fontRoot, family, 'OFL.txt'), 'utf8');
      assert.ok(fontLicenseBanner.includes(notice), `${family} notice must be prepared unchanged`);
      assert.ok(js.includes(notice), `${family} notice must be distributed unchanged`);
    }
    assert.deepEqual(readdirSync(dist).sort(), ['app.css', 'app.js', 'index.html']);
    const html = readFileSync(path.join(dist, 'index.html'), 'utf8');
    const links = [...html.matchAll(/(?:src|href)=["']([^"']+)["']/g)].map(([, value]) => value).sort();
    assert.deepEqual(links, ['/app.css', '/app.js']);
    assert.doesNotMatch(html + js, /unpkg\.com|cdn\.jsdelivr\.net|cdnjs\.cloudflare\.com|babel(?:\.min)?\.js/);
  });
}
