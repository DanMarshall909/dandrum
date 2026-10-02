import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { ICON_NAMES, iconProps } from '../../web/shared/design-icons.mjs';

const require = createRequire(new URL('../../web/sampler/package.json', import.meta.url));
const { createElement } = require('react');
const { renderToStaticMarkup } = require('react-dom/server');
const iconRoot = new URL('../../docs/design-system/reference/assets/icons/', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('manifest.json', iconRoot), 'utf8'));

function attributes(text) {
  return Object.fromEntries([...text.matchAll(/([\w-]+)="([^"]*)"/g)]
    .map(([, name, value]) => [name, value]).sort(([a], [b]) => a.localeCompare(b)));
}
function shapes(svg) {
  return [...svg.matchAll(/<(path|circle|rect)\b([^>]*)>/g)]
    .map(([, tag, attrs]) => ({ tag, ...attributes(attrs) }));
}
const render = (name, options) => renderToStaticMarkup(createElement('svg', iconProps(name, options)));

test('every supplied design icon preserves its manifest identity and SVG geometry', () => {
  assert.deepEqual(ICON_NAMES, manifest.icons.map(icon => icon.name));
  for (const { name, file } of manifest.icons) {
    const expected = shapes(readFileSync(new URL(file, iconRoot), 'utf8'));
    assert.ok(expected.length > 0, `${name} has no supplied geometry`);
    assert.deepEqual(shapes(render(name)), expected, `${name} changes supplied geometry`);
  }
});

test('decorative icons inherit text colour and stay out of keyboard and accessible labels', () => {
  const attrs = attributes(render('keyboard').match(/<svg([^>]*)>/)[1]);
  assert.equal(attrs['data-dd-icon'], 'keyboard');
  assert.equal(attrs.width, '16');
  assert.equal(attrs.height, '16');
  assert.equal(attrs.viewBox, manifest.viewBox);
  assert.equal(attrs.fill, 'none');
  assert.equal(attrs.stroke, 'currentColor');
  assert.equal(attrs['stroke-width'], String(manifest.strokeWidth));
  assert.equal(attrs['stroke-linecap'], 'round');
  assert.equal(attrs['stroke-linejoin'], 'round');
  assert.equal(attrs.focusable, 'false');
  assert.equal(attrs['aria-hidden'], 'true');
  assert.equal(attrs.role, undefined);
  assert.equal(attrs['aria-label'], undefined);
});

test('a standalone icon exposes its escaped label and requested size and colour', () => {
  for (const size of manifest.sizes) {
    const html = render('warning', { size, color: 'var(--dd-warn)', title: 'Missing <sample>' });
    const attrs = attributes(html.match(/<svg([^>]*)>/)[1]);
    assert.equal(attrs.width, String(size));
    assert.equal(attrs.height, String(size));
    assert.equal(attrs.stroke, 'var(--dd-warn)');
    assert.equal(attrs.role, 'img');
    assert.equal(attrs['aria-label'], 'Missing &lt;sample&gt;');
    assert.equal(attrs['aria-hidden'], undefined);
    assert.ok(!html.includes('<sample>'));
  }
});
