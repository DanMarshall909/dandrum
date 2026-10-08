import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';

const root = fileURLToPath(new URL('../../', import.meta.url));
const read = name => readFileSync(path.join(root, name), 'utf8');
const files = directory => readdirSync(path.join(root, directory), { recursive: true })
  .filter(name => typeof name === 'string').map(name => `${directory}/${name}`);

test('the public Slint vocabulary covers every guide component and the scrollbar', () => {
  const guide = new Map();
  for (const file of files('docs/design-system/reference/components').filter(name => name.endsWith('.d.ts'))) {
    for (const match of read(file).matchAll(/export declare function ([A-Z]\w*)\([^\n]*\): JSX\.Element/g)) {
      guide.set(match[1], file);
    }
  }
  assert.equal(guide.size, 29, 'Review guide additions explicitly');
  const inventory = JSON.parse(read('ui/slint/components.json'));
  assert.deepEqual(inventory.components.map(row => row.name).sort(), [...guide.keys(), 'Scrollbar'].sort());
  const exports = new Set([...read('ui/slint/dandrum.slint').matchAll(/export\s*\{([^}]+)\}/g)]
    .flatMap(match => match[1].split(',').map(name => name.trim())));
  for (const row of inventory.components) {
    assert.ok(exports.has(row.name), `Missing public export ${row.name}`);
    assert.ok(existsSync(path.join(root, row.source)), `Missing source ${row.source}`);
    assert.ok(read(row.source).includes(`export component ${row.name} `));
    if (row.name !== 'Scrollbar') assert.equal(row.guide, guide.get(row.name));
    assert.ok(existsSync(path.join(root, row.catalog)), `Missing catalog section for ${row.name}`);
    assert.ok(row.parts.length > 0, `Document the composition of ${row.name}`);
    for (const part of row.parts) assert.ok(existsSync(path.join(root, part)), `Missing subcomponent ${part}`);
  }
});

test('authored Slint files stay small and text respects the guide minimum', () => {
  for (const file of files('ui/slint').filter(name => name.endsWith('.slint'))) {
    const source = read(file);
    if (!source.startsWith('// Generated')) assert.ok(source.trimEnd().split('\n').length <= 200, `${file} needs decomposition`);
    for (const match of source.matchAll(/font-size:\s*(\d+(?:\.\d+)?)px/g)) {
      assert.ok(Number(match[1]) >= 11, `${file}: ${match[0]} is below the guide minimum`);
    }
  }
});

test('packaged icons preserve the original guide SVG bytes', () => {
  const names = files('ui/slint/assets/icons').filter(name => name.endsWith('.svg'));
  assert.equal(names.length, 33);
  const hash = file => createHash('sha256').update(readFileSync(path.join(root, file))).digest('hex');
  for (const file of names) {
    assert.equal(hash(file), hash(`docs/design-system/reference/assets/icons/${path.basename(file)}`), file);
  }
});
