import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const tokens = JSON.parse(readFileSync(new URL('../../ui/design-system/tokens.json', import.meta.url))).tokens;
const value = name => {
  assert.ok(tokens[name], `Missing public token ${name}`);
  return tokens[name].value;
};

test('editor chrome retains its established typography and geometry', () => {
  for (const [name, expected] of Object.entries({
    'editor-type-small': 11, 'editor-type-caption': 11, 'editor-type-body': 12,
    'editor-type-control': 13, 'editor-type-heading': 14, 'editor-type-legacy-small': 10,
    'editor-button-height': 26, 'editor-button-compact-size': 24,
    'editor-button-glow-stroke': 1.25, 'editor-button-caps-tracking': 0.66,
    'editor-option-strip-height': 28, 'editor-panel-header-height': 34,
    'editor-panel-title-inset': 11, 'editor-rollout-header-height': 24,
    'editor-browser-row-height': 28, 'editor-browser-compact-row-height': 30,
    'editor-navigation-item-height': 40, 'editor-navigation-compact-item-height': 30,
    'editor-item-height': 26, 'editor-item-progress-height': 2,
    'editor-empty-content-width': 330, 'editor-empty-content-height': 216,
  })) {
    assert.equal(value(name), expected, name);
    assert.equal(tokens[name].unit, 'px', `${name} must retain a length unit`);
  }
  assert.equal(value('editor-rollout-summary-start-fraction'), 0.52);
  assert.equal(value('editor-rollout-summary-width-fraction'), 0.48);
  assert.equal(value('editor-browser-name-column-fraction'), 0.34);
  assert.equal(value('editor-browser-detail-column-fraction'), 0.66);
  assert.equal(value('editor-choice-column-count'), 4);
});

test('browser vocabulary preserves host actions, categories, and accessible labels', () => {
  for (const [name, expected] of Object.entries({
    'browser-category-all': 'All', 'browser-category-favourites': 'Favourites',
    'browser-category-user': 'User', 'browser-category-drums': 'Drums',
    'browser-category-keys': 'Keys', 'browser-category-hybrid': 'Hybrid',
    'browser-category-synth': 'Synth', 'browser-action-import': 'import',
    'browser-action-export': 'export', 'browser-action-copy': 'copy',
    'browser-search-label': 'Search patches', 'browser-close-label': 'Close patch browser (Esc)',
    'browser-favourite-prefix': 'Favourite ', 'browser-unfavourite-prefix': 'Remove favourite ',
    'strings-detail-separator': ' · ', 'strings-custom-color-suffix': ' custom color',
    'icon-star': 'star', 'icon-one-shot': 'one-shot', 'icon-settings': 'settings',
  })) assert.equal(value(`editor-${name}`), expected, name);
});

test('editor material defaults retain the existing palette and lighting', () => {
  assert.equal(value('editor-theme-default-accent'), '#E8C47A');
  assert.equal(value('editor-theme-default-surface-window'), '#161718');
  assert.equal(value('editor-theme-default-paper-1'), '#EDE9E1');
  assert.equal(value('editor-theme-brushed-light-angle'), 100);
  assert.equal(value('editor-theme-raised-overlay-position-2'), 0.5);
  assert.equal(value('editor-material-light-grain-period'), 11);
  assert.equal(value('editor-material-fine-grain-period'), 3);
  assert.equal(value('editor-material-dark-grain-period'), 5);
  assert.equal(value('editor-material-wash-opacity'), 0.12);
});

test('sampler chrome retains its established layout through the shared geometry roles', () => {
  for (const [name, expected] of Object.entries({
    'editor-tree-menu-width': 190, 'editor-tree-menu-height': 212,
    'editor-source-search-top': 38, 'editor-source-category-top': 70,
    'editor-inspector-heading-height': 57, 'editor-inspector-overrides-heading-height': 85,
    'editor-inspector-history-row-stride': 25, 'editor-header-patch-width': 200,
    'editor-header-compact-patch-width': 152, 'editor-diagnostics-trace-height': 177,
    'editor-diagnostics-hits-width': 241, 'editor-diagnostics-reasons-width': 255,
    'editor-routing-table-body-top': 62, 'editor-macro-menu-width': 260,
    'editor-macro-menu-height': 270, 'editor-macro-rename-menu-height': 304,
  })) assert.equal(value(name), expected, name);
  assert.equal(value('editor-tree-instrument-fraction'), 0.5);
  assert.equal(value('editor-tree-source-fraction'), 0.25);
  assert.equal(value('editor-tree-modulator-start-fraction'), 0.75);
  assert.equal(value('editor-inspector-knob-columns'), 5);
  assert.equal(value('editor-header-meter-warning-threshold'), 0.95);
});

test('shared editor chrome keeps visual constants at its public token boundary', () => {
  const modules = ['Theme', 'Button', 'OptionStrip', 'PagePanel', 'Rollout', 'ResizeHandle',
    'ChoiceGrid', 'ColorChoice', 'PresetBrowser', 'PresetRow', 'NavigationRail', 'ItemRow',
    'StatusRow', 'EmptyState', 'SearchField', 'TextField', 'Icon', 'Material'];
  for (const name of modules) {
    const source = readFileSync(new URL(`../../ui/slint/editor/${name}.slint`, import.meta.url), 'utf8')
      .replace(/\/\/[^\n]*/g, '');
    assert.doesNotMatch(source, /(?<![\w.])(?:[1-9]\d*(?:\.\d+)?|0\.\d*[1-9]\d*)px\b/, `${name}: use a named visual length`);
    assert.doesNotMatch(source, /#[\da-fA-F]{6,8}\b/, `${name}: use a theme or material color role`);
    assert.doesNotMatch(source, /(?:font-weight|text-weight):\s*\d/, `${name}: use a named type weight`);
  }
});
