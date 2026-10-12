import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const read = name => readFileSync(new URL('../../' + name, import.meta.url), 'utf8');
const tokens = JSON.parse(read('ui/design-system/tokens.json')).tokens;

// Numeric oracles retain the original frame/drawer calibration independently
// of the generated files and their consuming property expressions.
test('shared editor frame and drawer geometry retains the existing default calibration', () => {
  for (const [name, value, unit] of [
    ["editor-layout-alert-banner-height", 44, "px"],
    ["editor-layout-draft-banner-height", 34, "px"],
    ["editor-layout-diagnostics-reserve", 180, "px"],
    ["editor-layout-event-log-reserve", 184, "px"],
    ["editor-layout-diagnostics-top-gap", 7, "px"],
    ["editor-layout-diagnostics-height", 177, "px"],
    ["editor-layout-event-log-height", 180, "px"],
    ["editor-layout-banner-line-height", 15, "px"],
    ["editor-layout-banner-detail-top", 20, "px"],
    ["editor-layout-banner-action-reserve", 180, "px"],
    ["editor-layout-discard-right", 164, "px"],
    ["editor-layout-reload-right", 89, "px"],
    ["editor-layout-scope-height", 29, "px"],
    ["editor-layout-footer-height", 27, "px"],
    ["editor-layout-scope-caption-width", 36, "px"],
    ["editor-layout-scope-selector-left", 52, "px"],
    ["editor-layout-scope-selector-width", 216, "px"],
    ["editor-layout-scope-selector-height", 28, "px"],
    ["editor-layout-scope-option-height", 22, "px"],
    ["editor-layout-scope-all-width", 35, "px"],
    ["editor-layout-scope-group-width", 49, "px"],
    ["editor-layout-scope-sound-width", 49, "px"],
    ["editor-layout-scope-selection-width", 71, "px"],
    ["editor-layout-footer-text-reserve", 110, "px"],
    ["editor-layout-about-right", 92, "px"],
    ["editor-layout-about-width", 84, "px"],
    ["editor-layout-about-height", 23, "px"],
    ["editor-drawer-minimum-compact", 60, "px"],
    ["editor-drawer-minimum", 120, "px"],
    ["editor-drawer-macro-width-compact", 214, "px"],
    ["editor-drawer-macro-width", 130, "px"],
    ["editor-drawer-macro-width-expanded", 158, "px"],
    ["editor-drawer-pad-area-minimum", 120, "px"],
    ["editor-drawer-pad-only-reserve", 36, "px"],
    ["editor-drawer-pad-width-compact", 234, "px"],
    ["editor-drawer-pad-width", 386, "px"],
    ["editor-drawer-pad-width-expanded", 466, "px"],
    ["editor-drawer-bar-height", 28, "px"],
    ["editor-drawer-resize-top", 3, "px"],
    ["editor-drawer-resize-thickness", 7, "px"],
    ["editor-drawer-window-reserve-compact", 250, "px"],
    ["editor-drawer-window-reserve", 300, "px"],
    ["editor-drawer-macro-face-compact", 28, "px"],
    ["editor-drawer-macro-face", 36, "px"],
    ["editor-drawer-macro-face-expanded", 48, "px"],
    ["editor-drawer-macro-budget-compact", 46, "px"],
    ["editor-drawer-macro-budget", 56, "px"],
    ["editor-drawer-macro-budget-expanded", 70, "px"],
    ["editor-drawer-macro-width-minimum", 50, "px"],
    ["editor-drawer-pad-with-macros-offset", 44, "px"],
    ["editor-drawer-pad-width-minimum", 80, "px"],
    ["editor-drawer-key-width-reserve", 240, "px"],
    ["editor-drawer-resize-z", 5, ""],
    ["editor-drawer-pad-minimum-compact", 36, ""],
    ["editor-drawer-pad-maximum-compact", 64, ""],
    ["editor-drawer-pad-minimum-expanded", 72, ""],
    ["editor-drawer-pad-maximum-expanded", 110, ""],
  ]) assert.deepEqual(tokens[name], {kind: 'number', value, unit}, name);
});

test('sampler frame and drawer consume named shared geometry and typography', () => {
  for (const file of ['ui/advanced-sampler/chrome/EditorFrame.slint', 'ui/advanced-sampler/performance/PlayDrawer.slint']) {
    const source = read(file);
    assert.match(source, /EditorLayoutMetrics/);
    assert.doesNotMatch(source, /(?<![\w.-])(?:[2-9]\d*|1\d+|0\.\d+)(?:px|ms|s)\b/, file);
    assert.doesNotMatch(source, /font-(?:size|weight):\s*\d/, file);
  }
});
