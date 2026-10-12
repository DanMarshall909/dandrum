import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const tokens = JSON.parse(readFileSync(new URL('../../ui/design-system/tokens.json', import.meta.url))).tokens;
test('shared drag, dialog and tooltip defaults retain their independent interaction and geometry contract', () => {
  for (const [name, value, unit] of [
    ['editor-drag-threshold', 4, 'px'],
    ['editor-dialog-border', 1, 'px'],
    ['editor-dialog-radius', 6, 'px'],
    ['editor-text-size', 12, 'px'],
    ['tooltip-hover-delay', 500, 'ms'],
  ]) assert.deepEqual(tokens[name], {kind: 'number', value, unit}, name);
  for (const [name, value] of [['editor-drag-source', 'source'], ['editor-drag-modulator', 'modulator'], ['editor-drag-slice', 'slice']])
    assert.deepEqual(tokens[name], {kind: 'text', value}, name);
});

test('measured editor frame breakpoints and sizes preserve compact, default and expanded layouts', () => {
  for (const [name, value] of Object.entries({
    'editor-layout-compact-breakpoint': 1000,
    'editor-layout-expanded-breakpoint': 1500,
    'editor-layout-header-compact': 39,
    'editor-layout-header': 45,
    'editor-layout-rail-compact': 40,
    'editor-layout-rail': 68,
    'editor-layout-tree-compact': 36,
    'editor-layout-tree': 208,
    'editor-layout-tree-expanded': 248,
    'editor-layout-inspector-compact': 232,
    'editor-layout-inspector': 272,
    'editor-layout-inspector-expanded': 320,
    'editor-layout-workspace-gap': 20,
    'editor-layout-drawer-compact': 114,
    'editor-layout-drawer': 284,
    'editor-layout-drawer-expanded': 352,
    'editor-layout-body-inset': 68,
  })) assert.deepEqual(tokens[name], {kind: 'number', value, unit: 'px'}, name);
});
