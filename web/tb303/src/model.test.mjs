import assert from 'node:assert/strict';
import test from 'node:test';

import { acceptState, controls, preparedCapabilities } from './model.mjs';

test('the panel binds exactly the seven prepared TB-303 public controls', () => {
  assert.deepEqual(controls.map(control => control.id).sort(), [
    'accent.brightness', 'amp.release_ms', 'filter.cutoff', 'filter.decay_ms',
    'filter.envelope_modulation', 'filter.resonance', 'slide.time_ms',
  ]);
  assert.equal(preparedCapabilities.waveformEditing, false);
  assert.equal(preparedCapabilities.patternEditing, false);
  assert.equal(preparedCapabilities.transport, false);
  assert.equal(preparedCapabilities.noteAudition, true);
});

test('older host messages cannot replace a newer value, but reload can', () => {
  const current = {generation: 3, sequence: 7, parameters: [{id: 'filter.cutoff', value: 0.7}]};
  const old = {generation: 3, sequence: 6, parameters: [{id: 'filter.cutoff', value: 0.1}]};
  assert.equal(acceptState(current, old), current);
  assert.equal(acceptState(current, {...old, sequence: 7}, 8), current);
  assert.equal(acceptState(current, {...old, generation: 2, sequence: 100}), current);
  const reload = {...old, generation: 4, sequence: 0};
  assert.equal(acceptState(current, reload, 8), reload);
  const hostAutomation = {...old, sequence: 7};
  assert.equal(acceptState(current, hostAutomation), hostAutomation);
});
