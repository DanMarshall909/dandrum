import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const { tokens } = JSON.parse(readFileSync(new URL('../../ui/design-system/tokens.json', import.meta.url)));
const number = (name, value, unit = '') => assert.deepEqual(tokens[name], { kind: 'number', value, unit }, name);
const text = (name, value) => assert.deepEqual(tokens[name], { kind: 'text', value }, name);

test('parameter input calibration and timing preserve the native interaction contract', () => {
  for (const [name, value] of Object.entries({
    'gesture-range-pixels': 200, 'gesture-click-slop': 2, 'gesture-track-inset': 6,
  })) number(`editor-control-${name}`, value, 'px');
  for (const [name, value] of Object.entries({
    'editor-fine-scale': .1, 'guide-fine-scale': .25, 'keyboard-range-fraction': .05,
    'fine-keyboard-range-fraction': .01, 'wheel-range-fraction': .02,
    'fine-wheel-range-fraction': .01, 'default-step': .01, 'value-epsilon': .000001,
  })) number(`editor-control-${name}`, value);
  for (const [name, value] of Object.entries({
    'hover-open-delay': 300, 'hover-close-delay': 250, 'host-flash-duration': 400, 'nudge-flash-duration': 600,
  })) number(`editor-control-${name}`, value, 'ms');
});

test('parameter action identifiers and accessible labels remain host-compatible', () => {
  for (const [name, value] of Object.entries({ begin: 0, update: 1, commit: 2, cancel: 3 })) number(`editor-control-gesture-${name}`, value);
  text('editor-control-field-minimum', 'minimum');
  text('editor-control-field-maximum', 'maximum');
  for (const [name, value] of Object.entries({
    modulation: 'modulation', 'midi-learn': 'midi-learn', 'macro-map': 'macro-map',
    'add-route': 'modulation-add', 'edit-route': 'modulation-edit', 'invert-route': 'modulation-invert',
    'bypass-route': 'modulation-bypass', 'remove-route': 'modulation-remove', macro: 'macro',
  })) text(`editor-control-action-${name}`, value);
  text('editor-control-unit-pan', 'pan');
  text('editor-control-value-unit-separator', ' ');
  text('editor-control-menu-reset', 'Reset to default');
  text('editor-control-menu-type-value', 'Type value…');
  text('editor-control-readonly-instruction', 'Prepared setting. Read only.');
  assert.equal(tokens['editor-control-label-edit-prefix'].value + 'Cutoff' + tokens['editor-control-label-value-suffix'].value, 'Edit Cutoff value');
  assert.equal(tokens['editor-control-label-reset-prefix'].value + 'Cutoff' + tokens['editor-control-label-reset-suffix'].value, 'Reset Cutoff to default');
});

test('shared parameter and guide layout metrics retain their distinct geometry', () => {
  for (const [name, value] of Object.entries({
    'knob-label-height': 13, 'guide-label-height': 17, 'knob-size': 36, 'knob-min-width': 42,
    'field-height': 24, 'field-value-top': 18, 'field-labeled-height': 42,
    'menu-width': 190, 'menu-height': 134, 'popup-width': 174, 'popup-base-height': 40,
    'dense-label-size': 10, 'rotary-ring-inset-medium': 6.5, 'rotary-track-small': 1.25, 'rotary-highlight-inset': 1,
  })) number(`editor-control-${name}`, value, 'px');
  assert.equal(tokens['editor-control-field-height'].value + tokens['editor-control-field-value-top'].value, 42);
  number('editor-control-rotary-arc-start-degrees', 135);
  number('editor-control-rotary-arc-sweep-degrees', 270);
  number('editor-control-rotary-assign-ticks', 24);
  number('editor-control-guide-field-fine-key-multiplier', 10);
  number('editor-control-guide-field-fine-divisor', 3);
  number('editor-control-fallback-value-precision-scale', 1000);
  number('editor-control-slider-compact-vertical-length', 88, 'px');
  number('editor-control-slider-compact-horizontal-length', 112, 'px');
});

test('curve preview calibration and shape protocols are explicit and stable', () => {
  for (const [name, value] of Object.entries({
    'default-segments': 100, 'processor-segments': 64, 'grid-divisions': 4,
    'modulation-baseline': .95, 'modulation-unipolar-span': .9, 'modulation-bipolar-span': .45,
    'lfo-visible-cycles': 2, 'input-range-epsilon': .00001, 'processor-range-epsilon': .0001,
    'milliseconds-per-second': 1000, 'minimum-envelope-seconds': .01, 'sustain-preview-seconds': .5,
    'compressor-db-span': 60, 'filter-preview-order': 4, 'saturation-preview-drive': 8,
    'velocity-maximum': 127, 'amplitude-db-divisor': 20,
  })) number(`editor-curve-${name}`, value);
  for (const [name, value] of Object.entries({
    'triangle-legacy': '1', 'triangle-short': 'tri', 'square-legacy': '3',
    'sample-hold': 'sh', exponential: 'exp', logarithmic: 'log', 'random-smooth': 'smooth',
  })) text(`editor-curve-shape-${name}`, value);
  assert.deepEqual(Array.from({ length: 8 }, (_, i) => tokens[`editor-curve-hold-preview-level-${i + 1}`].value), [.6, -.3, .9, -.8, .2, -.5, .75, -.1]);
  assert.deepEqual(Array.from({ length: 11 }, (_, i) => tokens[`editor-curve-random-preview-level-${i + 1}`].value), [.3, -.7, .55, .1, -.4, .85, -.2, .6, -.9, .25, .3]);
});

test('native value notation has explicit unit scales, precision and symbols', () => {
  for (const [name, value] of Object.entries({ 'pan-display-extent': 50, 'frequency-unit-scale': 1000,
    'decibel-precision': 1, 'ratio-precision': 3, 'integer-precision': 0, 'generic-precision': 3,
  })) number(`editor-control-${name}`, value);
  for (const [name, value] of Object.entries({ 'unit-hertz': 'Hz', 'unit-kilohertz': 'kHz',
    'unit-decibels': 'dB', 'unit-milliseconds': 'ms', 'unit-ratio-ascii': 'x', 'unit-ratio': '×',
    'notation-minus': '−', 'notation-unavailable': '—', 'pan-center': 'C', 'pan-left': 'L', 'pan-right': 'R',
    'pan-center-lower': 'c', 'pan-left-lower': 'l', 'pan-right-lower': 'r',
  })) text(`editor-control-${name}`, value);
});
