import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const generator = path.join(root, 'scripts/generate-ui-tokens.mjs');
function fixture(run) {
  const dir = mkdtempSync(path.join(tmpdir(), 'dandrum-slint-tokens-'));
  const source = path.join(dir, 'tokens.json');
  const slint = path.join(dir, 'tokens.slint');
  const legacy = path.join(dir, 'legacy-tokens.slint');
  const data = { schemaVersion: 1, tokens: {
    'dd-ink-0': { kind: 'colour', value: '#130F0C' },
    'dd-scrim': { kind: 'colour', value: 'rgba(19,15,12,0.45)' },
    'surface-well': { kind: 'alias', value: 'dd-ink-0' },
    'space-5': { kind: 'number', value: 12, unit: 'px' },
    'pad-panel': { kind: 'alias', value: 'space-5' },
    'tracking-caps': { kind: 'number', value: 0.06, unit: 'em' },
    'font-ui': { kind: 'text', value: '"Barlow Semi Condensed", sans-serif' },
    'shadow-cap': { kind: 'text', value: '0 2px 3px rgba(0,0,0,0.55)' },
  } };
  const write = () => writeFileSync(source, JSON.stringify(data));
  const generate = (...extra) => spawnSync(process.execPath, [generator, '--source', source,
    '--css', path.join(dir, 'tokens.css'), '--cpp', path.join(dir, 'tokens.h'), '--slint', slint, '--legacy-slint', legacy, ...extra], { encoding: 'utf8' });
  try { write(); run({ dir, source, slint, legacy, data, write, generate }); }
  finally { rmSync(dir, { recursive: true, force: true }); }
}

test('Slint consumes the same semantic colours, lengths, ratios and native font families', () => {
  fixture(({ slint, data, write, generate }) => {
    let result = generate();
    assert.equal(result.status, 0, result.stderr);
    const output = readFileSync(slint, 'utf8');
    assert.match(output, /export global Tokens/);
    assert.match(output, /out property <color> surface-well: #130F0C;/);
    assert.match(output, /out property <color> dd-scrim: rgba\(19,15,12,0.45\);/);
    assert.match(output, /out property <length> pad-panel: 12px;/);
    assert.match(output, /out property <float> tracking-caps: 0.06;/);
    assert.match(output, /out property <string> font-ui: "Barlow Semi Condensed";/);
    assert.match(output, /out property <string> shadow-cap: "0 2px 3px rgba\(0,0,0,0.55\)";/);
    data.tokens['dd-ink-0'].value = '#abcdef';
    data.tokens['space-5'].value = 16;
    write(); result = generate();
    assert.equal(result.status, 0, result.stderr);
    const changed = readFileSync(slint, 'utf8');
    assert.match(changed, /surface-well: #ABCDEF;/);
    assert.match(changed, /pad-panel: 16px;/);
  });
});

test('Slint drift is reported without rewriting and invalid input preserves every output', () => {
  fixture(({ slint, data, write, generate }) => {
    assert.equal(generate().status, 0);
    writeFileSync(slint, '// altered\n');
    const drift = generate('--check');
    assert.equal(drift.status, 1);
    assert.match(drift.stderr, /tokens.slint.*out of date/);
    assert.equal(readFileSync(slint, 'utf8'), '// altered\n');
    data.tokens['surface-well'].value = 'missing-token'; write();
    assert.equal(generate().status, 1);
    assert.equal(readFileSync(slint, 'utf8'), '// altered\n');
  });
});

test('Slint output cannot overwrite a token source or another renderer output', () => {
  fixture(({ source, generate }) => {
    const before = readFileSync(source, 'utf8');
    const result = generate('--slint', source);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /paths must be distinct/);
    assert.equal(readFileSync(source, 'utf8'), before);
  });
});


test('both published Slint import contracts follow aliases and reject independent drift', () => {
  fixture(({ slint, legacy, data, write, generate }) => {
    assert.equal(generate().status, 0);
    assert.match(readFileSync(slint, 'utf8'), /global Tokens/);
    assert.match(readFileSync(legacy, 'utf8'), /global DesignTokens/);
    data.tokens['dd-ink-0'].value = '#abcdef'; write();
    assert.equal(generate().status, 0);
    assert.match(readFileSync(slint, 'utf8'), /surface-well: #ABCDEF;/);
    assert.match(readFileSync(legacy, 'utf8'), /surface-well: #ABCDEFFF;/);
    writeFileSync(legacy, '// drift\n');
    const result = generate('--check');
    assert.equal(result.status, 1);
    assert.match(result.stderr, /legacy-tokens.slint.*out of date/);
    assert.equal(readFileSync(legacy, 'utf8'), '// drift\n');
    assert.match(generate('--legacy-slint', slint).stderr, /paths must be distinct/);
  });
});
