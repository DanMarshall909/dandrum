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
const iconRoot = path.join(root, 'docs/design-system/reference/assets/icons');
const packagedIcons = {
  sampler: ['keyboard', 'host', 'level', 'alternate', 'choke', 'lock', 'error'],
  tb303: ['keyboard', 'midi', 'level', 'error'],
};

test('JUCE runtime observer scripts remain ASCII across the Linux IPC boundary', () => {
  const directory = path.join(root, 'tests/cpp');
  let scripts = 0;
  for (const name of readdirSync(directory).filter(name => /\.(?:cpp|h)$/.test(name))) {
    const source = readFileSync(path.join(directory, name), 'utf8');
    for (const [raw] of source.matchAll(/R"JS\([\s\S]*?\)JS"/g)) {
      scripts++;
      assert.ok(!/[^\x00-\x7f]/.test(raw),
        `${name}: runtime observer script must use JavaScript Unicode escapes`);
    }
  }
  assert.ok(scripts > 0, 'No runtime observer scripts were checked');
});

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
  test(`${app} packages the supplied stroke icons with its executable assets`, () => {
    const js = readFileSync(path.join(dist, 'app.js'), 'utf8');
    assert.ok(js.includes('data-dd-icon'), `${app} has no packaged design icons`);
    for (const name of packagedIcons[app]) {
      const svg = readFileSync(path.join(iconRoot, `dd-icon-${name}.svg`), 'utf8')
        .replace(/<metadata>[\s\S]*?<\/metadata>/g, '');
      for (const [, geometry] of svg.matchAll(/\bd="([^"]+)"/g))
        assert.ok(js.includes(geometry), `${app} packaged ${name} icon is missing supplied geometry ${geometry}`);
    }
  });
}
