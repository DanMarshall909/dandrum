import assert from 'node:assert/strict';
import { test } from 'node:test';
import { readFileSync } from 'node:fs';

const root = new URL('../../', import.meta.url);
const tokens = JSON.parse(readFileSync(new URL('ui/design-system/tokens.json', root), 'utf8')).tokens;

test('musical limits and keyboard fitting retain the independent MIDI and v3 contract', () => {
  const expected = {
    'editor-musical-notes-per-octave': 12,
    'editor-musical-white-per-octave': 7,
    'editor-musical-black-per-octave': 5,
    'editor-musical-midi-maximum': 127,
    'editor-musical-velocity-maximum': 127,
    'editor-musical-white-key-count': 75,
    'editor-musical-black-key-count': 53,
    'editor-musical-fit-key-width-minimum': 10,
    'editor-musical-key-width-maximum': 48,
    'editor-musical-key-zoom-step': 4,
    'editor-musical-pad-zoom-step': 8,
    'editor-musical-pad-visible-count': 64,
  };
  for (const [name, value] of Object.entries(expected))
    assert.deepEqual(tokens[name], { kind: 'number', value, unit: '' }, name);
});

test('plot and signed-wave geometry retains calibrated scales and rendering dimensions', () => {
  const expected = {
    'editor-envelope-attack-scale': [600, ''],
    'editor-envelope-decay-scale': [2111, ''],
    'editor-envelope-release-scale': [4500, ''],
    'editor-display-frequency-minimum': [20, ''],
    'editor-display-frequency-maximum': [20000, ''],
    'editor-display-response-steps': [120, ''],
    'editor-display-graph-horizontal-inset': [8, 'px'],
    'editor-display-wave-bucket-overlap': [2, 'px'],
    'editor-display-key-separator-width': [1.25, 'px'],
    'editor-display-overview-height': [149, 'px'],
  };
  for (const [name, [value, unit]] of Object.entries(expected))
    assert.deepEqual(tokens[name], { kind: 'number', value, unit }, name);
  assert.equal(tokens['editor-envelope-note-off-position'].value, 220 / 300);
});

test('display feedback and protocol defaults have one named text authority', () => {
  for (const [name, value] of Object.entries({
    'editor-display-state-missing': 'missing',
    'editor-display-mode-spectral': 'spectral',
    'editor-display-wave-missing-message': 'Sample unavailable · Relink or replace',
    'editor-display-wave-unsupported-message': 'Unsupported sample format',
    'editor-display-wave-viewport-label': 'Visible waveform viewport',
    'editor-display-overview-title': 'KEY MAP · READ-ONLY OVERVIEW',
    'editor-display-binding-invert-menu': 'Invert range',
  })) assert.deepEqual(tokens[name], { kind: 'text', value }, name);
});

test('reusable defaults and pitch tables are consumed through the shared constants surface', () => {
  const constants = readFileSync(new URL('ui/slint/editor/DisplayConstants.slint', root), 'utf8');
  for (const name of ['MusicalConstants', 'PitchNames', 'Ranges', 'GraphMetrics', 'DisplayStates', 'DisplayModes', 'WaveFields', 'EnvelopeStages'])
    assert.match(constants, new RegExp(`export global ${name}\\b`));
  assert.match(constants, /notes-per-octave: Tokens\.editor-musical-notes-per-octave/);
  const layout = readFileSync(new URL('ui/slint/editor/FitLayout.slint', root), 'utf8');
  assert.match(layout, /MusicalConstants\.notes-per-octave/);
  assert.match(layout, /PitchNames\.names/);
  const envelope = readFileSync(new URL('ui/slint/editor/EnvelopePlot.slint', root), 'utf8');
  assert.match(envelope, /GraphMetrics\.envelope-attack-scale/);
  const header = readFileSync(new URL('ui/slint/host/SpectralPreview.h', root), 'utf8');
  assert.match(header, /spectral_rgba_channels = 4/);
  assert.match(header, /spectral_palette_stops = 5/);
  assert.match(header, /spectral_rng_seed = 99/);
});
