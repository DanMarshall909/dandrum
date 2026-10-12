import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../../', import.meta.url));
const read = name => readFileSync(path.join(root, name), 'utf8');
const publicNames = source => [...source.matchAll(/export\s*\{([^}]+)\}/g)]
  .flatMap(match => match[1].split(',').map(value => value.trim().split(/\s+as\s+/).at(-1)));
const consumer = read('tests/cmake/slint-editor-consumer/Consumer.slint.in');
const required = [...consumer.matchAll(/import\s*\{([^}]+)\}/g)]
  .flatMap(match => match[1].split(',').map(name => name.trim()));

test('an independent editor can import every shared editor API from the umbrella', () => {
  assert.ok(required.length > 80, 'Keep the external consumer broad enough to exercise the shared API');
  const actual = new Set(publicNames(read('ui/slint/dandrum.slint')));
  for (const name of required) assert.ok(actual.has(name), `Missing public editor API ${name}`);
  for (const name of ['PropertyRow', 'EmptyState', 'Knob', 'NumericField']) {
    assert.ok(actual.has(name), `Preserve the original guide API ${name}`);
  }
});

test('the editor inventory describes the public components, data and constants', () => {
  const inventory = JSON.parse(read('ui/slint/components.json'));
  assert.ok(Array.isArray(inventory.editor), 'Describe editor APIs separately from the unchanged guide inventory');
  const rows = new Map(inventory.editor.map(row => [row.name, row]));
  assert.equal(rows.size, inventory.editor.length, 'Public names must be unique');
  for (const name of required) {
    const row = rows.get(name);
    assert.ok(row, `Missing editor inventory entry ${name}`);
    assert.ok(['component', 'global', 'struct'].includes(row.kind), `Describe the kind of ${name}`);
    assert.ok(existsSync(path.join(root, row.source)), `Missing source for ${name}`);
    assert.match(read(row.source), new RegExp(`export ${row.kind} ${row.declaration ?? name}\\b`));
  }
  const aliases = new Map([
    ['PropertyRow', 'EditorPropertyRow'], ['EmptyState', 'EditorEmptyState'], ['RotaryFace', 'EditorRotaryFace'],
  ]);
  const exported = new Set(publicNames(read('ui/slint/dandrum.slint')));
  for (const file of readdirSync(path.join(root, 'ui/slint/editor')).filter(name => name.endsWith('.slint'))) {
    for (const declaration of read(`ui/slint/editor/${file}`).matchAll(/^export (?:component|global|struct) (\w+)/gm)) {
      const name = aliases.get(declaration[1]) ?? declaration[1];
      assert.ok(rows.has(name) && exported.has(name), `Orphaned editor API ${name}`);
      assert.ok(required.includes(name), `The external consumer must import ${name}`);
    }
  }
});

function boundaryViolations(source, name) {
  const issues = [];
  if (/\bSession\b/.test(source)) issues.push(`${name}: application session state`);
  for (const match of source.matchAll(/(?:from\s*|import\s*|#\s*include\s*)["<]([^">]+)[">]/g)) {
    if (/advanced-sampler|(?:^|\/)engine\/|Session\.h/.test(match[1])) {
      issues.push(`${name}: application dependency ${match[1]}`);
    }
  }
  return issues;
}

test('shared Slint and C++ sources have no sampler or Session dependency', () => {
  const files = readdirSync(path.join(root, 'ui/slint'), { recursive: true })
    .filter(name => /\.(?:slint|h|hpp|cpp|cc)$/.test(name));
  assert.ok(files.length > 100, 'Inspect the complete package, including host and platform code');
  assert.deepEqual(files.flatMap(name => boundaryViolations(read(`ui/slint/${name}`), name)), []);
});

test('the dependency guard rejects an application include and session access', () => {
  assert.deepEqual(boundaryViolations('import { Session } from "../../advanced-sampler/engine/Session.slint";', 'bad.slint'), [
    'bad.slint: application session state',
    'bad.slint: application dependency ../../advanced-sampler/engine/Session.slint',
  ]);
  assert.deepEqual(boundaryViolations('#include "../../../advanced-sampler/engine/Model.h"', 'bad.h'), [
    'bad.h: application dependency ../../../advanced-sampler/engine/Model.h',
  ]);
  assert.deepEqual(boundaryViolations('import { EditorTheme } from "Theme.slint"; #include <slint.h>', 'good.slint'), []);
});
