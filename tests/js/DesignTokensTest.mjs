import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const generator = path.join(root, 'scripts/generate-ui-tokens.mjs');
const fixture = () => ({ schemaVersion: 1, tokens: {
  'dd-ink-0': { kind: 'colour', value: '#130F0C' },
  'dd-scrim': { kind: 'colour', value: 'rgba(19,15,12,0.45)' },
  'surface-window': { kind: 'alias', value: 'dd-ink-0' },
  'space-5': { kind: 'number', value: 12, unit: 'px' },
  'pad-panel': { kind: 'alias', value: 'space-5' },
  'tracking-caps': { kind: 'number', value: 0.06, unit: 'em' },
  'tracking-small': { kind: 'number', value: -1e-7, unit: 'em' },
  'font-ui': { kind: 'text', value: '"Barlow Semi Condensed", sans-serif' },
  'shadow-cap': { kind: 'text', value: '0 2px 3px rgba(0,0,0,0.55)' },
} });

function sandbox(run) {
  const dir = mkdtempSync(path.join(tmpdir(), 'dandrum-ui-tokens-'));
  const source = path.join(dir, 'tokens.json');
  const css = path.join(dir, 'tokens.css');
  const cpp = path.join(dir, 'tokens.h');
  const slint = path.join(dir, 'tokens.slint');
  const write = data => writeFileSync(source, JSON.stringify(data));
  const generate = (...args) => spawnSync(process.execPath,
    [generator, '--source', source, '--css', css, '--cpp', cpp, '--slint', slint, ...args], { encoding: 'utf8' });
  try { run({ dir, source, css, cpp, write, generate }); }
  finally { rmSync(dir, { recursive: true, force: true }); }
}

test('one source produces matching CSS and compilable C++ colour, spacing and typography values', () => {
  sandbox(({ dir, css, cpp, write, generate }) => {
    write(fixture());
    const result = generate();
    assert.equal(result.status, 0, result.stderr);
    const stylesheet = readFileSync(css, 'utf8');
    assert.match(stylesheet, /--dd-ink-0: #130F0C;/);
    assert.match(stylesheet, /--pad-panel: var\(--space-5\);/);
    assert.match(stylesheet, /--tracking-caps: 0.06em;/);
    const native = readFileSync(cpp, 'utf8');
    assert.match(native, /dd_scrim = 0x73130F0Cu;/);
    assert.match(native, /font_ui = "\\"Barlow Semi Condensed\\", sans-serif";/);
    writeFileSync(path.join(dir, 'check.cpp'), `#include "tokens.h"
      using namespace dandrum::ui::tokens;
      static_assert(dd_ink_0 == 0xFF130F0Cu && surface_window == dd_ink_0);
      static_assert(pad_panel == space_5 && pad_panel == 12.0f);
      static_assert(tracking_caps == 0.06f);
      static_assert(tracking_small == -1e-7f);
      static_assert(font_ui == "\\\"Barlow Semi Condensed\\\", sans-serif");
      static_assert(shadow_cap == "0 2px 3px rgba(0,0,0,0.55)");
      int main() { return 0; }
    `);
    const compiled = spawnSync('c++', ['-std=c++20', '-fsyntax-only', path.join(dir, 'check.cpp')], { encoding: 'utf8' });
    assert.equal(compiled.status, 0, compiled.stderr);
  });
});

test('changing a semantic colour and spacing updates both renderers and their alias chains', () => {
  sandbox(({ css, cpp, write, generate }) => {
    const source = fixture();
    source.tokens['control-padding'] = { kind: 'alias', value: 'pad-panel' };
    write(source);
    assert.equal(generate().status, 0);
    source.tokens['dd-ink-0'].value = '#abcdef';
    source.tokens['space-5'].value = 16;
    write(source);
    assert.equal(generate().status, 0);
    assert.match(readFileSync(css, 'utf8'), /--dd-ink-0: #ABCDEF;/);
    assert.match(readFileSync(css, 'utf8'), /--space-5: 16px;/);
    assert.match(readFileSync(cpp, 'utf8'), /surface_window = 0xFFABCDEFu;/);
    assert.match(readFileSync(cpp, 'utf8'), /control_padding = 16.0f;/);
    assert.equal(generate('--check').status, 0);
  });
});

test('check identifies each missing or independently edited generated output without rewriting it', () => {
  sandbox(({ css, cpp, write, generate }) => {
    write(fixture());
    let result = generate('--check');
    assert.equal(result.status, 1);
    assert.match(result.stderr, /tokens\.css.*out of date/);
    assert.match(result.stderr, /tokens\.h.*out of date/);
    assert.equal(generate().status, 0);
    writeFileSync(cpp, '// independently edited\n');
    result = generate('--check');
    assert.equal(result.status, 1);
    assert.match(result.stderr, /tokens\.h.*out of date/);
    assert.doesNotMatch(result.stderr, /tokens\.css.*out of date/);
    assert.equal(readFileSync(cpp, 'utf8'), '// independently edited\n');
    writeFileSync(css, 'wrong colour');
    result = generate('--check');
    assert.equal(result.status, 1);
    assert.match(result.stderr, /tokens\.css.*out of date/);
  });
});

test('invalid tokens fail with a named diagnostic and leave both outputs unchanged', () => {
  sandbox(({ css, cpp, write, generate }) => {
    write(fixture());
    assert.equal(generate().status, 0);
    const before = [readFileSync(css, 'utf8'), readFileSync(cpp, 'utf8')];
    const cases = [
      [data => data.schemaVersion = 2, /schemaVersion/],
      [data => data.tokens = [], /tokens.*object/],
      [data => data.tokens['bad_name'] = { kind: 'number', value: 1, unit: '' }, /bad_name.*name/],
      [data => data.tokens['dd-ink-0'].value = '#12', /dd-ink-0.*colour/],
      [data => data.tokens['dd-scrim'].value = 'rgba(300,15,12,0.45)', /dd-scrim.*colour/],
      [data => data.tokens['dd-scrim'].value = 'rgba(19,15,12,2)', /dd-scrim.*colour/],
      [data => data.tokens['space-5'].value = null, /space-5.*number/],
      [data => data.tokens['space-5'].value = 1e10, /space-5.*number/],
      [data => data.tokens['space-5'].unit = 'vh', /space-5.*unit/],
      [data => data.tokens['font-ui'].value = 'x; color: red', /font-ui.*text/],
      [data => data.tokens['font-ui'].value = 'bad\u0000font', /font-ui.*text/],
      [data => data.tokens['font-ui'] = null, /font-ui.*object/],
      [data => data.tokens['font-ui'].kind = 'unknown', /font-ui.*kind/],
      [data => data.tokens['pad-panel'].value = 'unknown', /pad-panel.*unknown/],
      [data => data.tokens['space-5'] = { kind: 'alias', value: 'pad-panel' }, /cycle.*pad-panel/],
    ];
    for (const [corrupt, diagnostic] of cases) {
      const data = fixture(); corrupt(data); write(data);
      const result = generate();
      assert.equal(result.status, 1, JSON.stringify(data));
      assert.match(result.stderr, diagnostic);
      assert.deepEqual([readFileSync(css, 'utf8'), readFileSync(cpp, 'utf8')], before);
    }
  });
});

test('output is deterministic regardless of token insertion order', () => {
  sandbox(({ css, cpp, write, generate }) => {
    const data = fixture(); write(data); assert.equal(generate().status, 0);
    const before = [readFileSync(css, 'utf8'), readFileSync(cpp, 'utf8')];
    data.tokens = Object.fromEntries(Object.entries(data.tokens).reverse());
    write(data); assert.equal(generate().status, 0);
    assert.deepEqual([readFileSync(css, 'utf8'), readFileSync(cpp, 'utf8')], before);
  });
});

test('repository outputs are current and preserve the supplied CSS semantics', () => {
  const result = spawnSync(process.execPath, [generator, '--check'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const data = JSON.parse(readFileSync(path.join(root, 'ui/design-system/tokens.json'), 'utf8'));
  const generated = readFileSync(path.join(root, 'web/shared/design-tokens.css'), 'utf8');
  for (const file of ['colors', 'spacing', 'shape', 'sizing', 'typography']) {
    const source = readFileSync(path.join(root, `docs/design-system/reference/tokens/${file}.css`), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    for (const [, name, value] of source.matchAll(/--([a-z][a-z0-9-]*)\s*:\s*([^;]+);/g)) {
      assert.ok(data.tokens[name], name);
      assert.ok(generated.includes(`--${name}: ${value.trim()};`), name);
    }
  }
  sandbox(({ dir }) => {
    writeFileSync(path.join(dir, 'production.cpp'), `#include ${JSON.stringify(path.join(root, 'src/juce-plugin/DesignTokens.h'))}
      using namespace dandrum::ui::tokens;
      static_assert(color_action == 0xFFE08A4Eu);
      static_assert(pad_panel == 12.0f && menu_width == 240.0f);
      static_assert(stroke_focus == 2.0f && stroke_focus_gap == 2.0f);
      int main() { return 0; }
    `);
    const compiled = spawnSync('c++', ['-std=c++20', '-fsyntax-only', path.join(dir, 'production.cpp')], { encoding: 'utf8' });
    assert.equal(compiled.status, 0, compiled.stderr);
  });
});

test('invalid CLI arguments and overlapping paths cannot overwrite the source', () => {
  const badArgument = spawnSync(process.execPath, [generator, '--unknown'], { encoding: 'utf8' });
  assert.equal(badArgument.status, 1);
  assert.match(badArgument.stderr, /invalid argument --unknown/);
  sandbox(({ source, css, write, generate }) => {
    write(fixture());
    const before = readFileSync(source, 'utf8');
    const collision = generate('--css', source);
    assert.equal(collision.status, 1);
    assert.match(collision.stderr, /paths must be distinct/);
    assert.equal(readFileSync(source, 'utf8'), before);
    writeFileSync(source, 'broken json');
    assert.equal(generate().status, 1);
    assert.throws(() => readFileSync(css), /ENOENT/);
  });
});

test('locally retained font binaries and license notices match pinned upstream provenance', () => {
  const directory = path.join(root, 'ui/design-system/fonts');
  const provenance = JSON.parse(readFileSync(path.join(directory, 'provenance.json'), 'utf8'));
  assert.equal(provenance.schemaVersion, 1);
  const retained = readdirSync(directory, { recursive: true, withFileTypes: true })
    .filter(entry => entry.isFile() && entry.name !== 'provenance.json')
    .map(entry => path.relative(directory, path.join(entry.parentPath, entry.name))).sort();
  assert.deepEqual(retained, Object.keys(provenance.files).sort());
  for (const [name, entry] of Object.entries(provenance.files)) {
    assert.match(entry.url, /^https:\/\/raw\.githubusercontent\.com\/(google\/fonts|JetBrains\/JetBrainsMono)\/[a-f0-9]{40}\//);
    const bytes = readFileSync(path.join(directory, name));
    assert.equal(bytes.length, entry.bytes, name);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256, name);
    if (name.endsWith('.ttf')) assert.equal(bytes.readUInt32BE(), 0x00010000, name);
    else assert.match(bytes.toString(), /SIL OPEN FONT LICENSE Version 1\.1/);
  }
  for (const family of ['Barlow', 'BarlowSemiCondensed', 'JetBrainsMono'])
    assert.ok(provenance.files[`${family}/OFL.txt`], family);
});
