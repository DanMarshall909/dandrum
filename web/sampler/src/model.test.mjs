import assert from 'node:assert/strict';
import test from 'node:test';
import { preparedPads, visibleParameters, selectedRegion, normalizedDraft,
  needsDocumentRefresh, padReleaseHandlers, auditionFocusRelease } from './model.mjs';
import { createNoteAudition } from '../../tb303/src/note-audition.mjs';

const document = {
  generation: 3,
  sources: [{ id: 'drums', regions: [
    { id: 'kick' }, { id: 'snare_soft' }, { id: 'snare_hard_a' },
    { id: 'snare_hard_b' }, { id: 'hat_open' },
  ] }],
  maps: [{ id: 'kit', selectionMode: 'round_robin', zones: [
    { id: 'kick', sourceIndex: 0, regionIndex: 0, keyLow: 36, keyHigh: 36,
      velocityLow: 1, velocityHigh: 127, controlGroup: 1, roundRobinGroup: '', chokeGroup: '' },
    { id: 'snare_soft', sourceIndex: 0, regionIndex: 1, keyLow: 38, keyHigh: 38,
      velocityLow: 1, velocityHigh: 63, controlGroup: 2, roundRobinGroup: '', chokeGroup: '' },
    { id: 'snare_hard_a', sourceIndex: 0, regionIndex: 2, keyLow: 38, keyHigh: 38,
      velocityLow: 64, velocityHigh: 127, controlGroup: 2,
      roundRobinGroup: 'hard_snare', chokeGroup: '' },
    { id: 'snare_hard_b', sourceIndex: 0, regionIndex: 3, keyLow: 38, keyHigh: 38,
      velocityLow: 64, velocityHigh: 127, controlGroup: 2,
      roundRobinGroup: 'hard_snare', chokeGroup: '' },
    { id: 'hat_open', sourceIndex: 0, regionIndex: 4, keyLow: 46, keyHigh: 46,
      velocityLow: 1, velocityHigh: 127, controlGroup: 4,
      roundRobinGroup: '', chokeGroup: 'hats' },
  ] }],
  parameters: [
    { id: 'master.level', scope: 'instrument' },
    { id: 'snare.pitch', scope: 'sampleGroup', controlGroup: 2 },
    { id: 'hat.level', scope: 'sampleGroup', controlGroup: 4 },
  ],
};

test('prepared pads retain real snare velocity boundaries and alternate semantics', () => {
  const pads = preparedPads(document);
  assert.equal(pads.length, 4);
  assert.deepEqual(pads.map(pad => [pad.keyLow, pad.velocityLow, pad.velocityHigh]),
    [[36, 1, 127], [38, 1, 63], [38, 64, 127], [46, 1, 127]]);
  assert.deepEqual(pads[2].zoneIds, ['snare_hard_a', 'snare_hard_b']);
  assert.equal(pads[2].label, 'hard snare');
  assert.equal(pads[2].selectionMode, 'round_robin');
  assert.equal(pads[2].simultaneousLayers, false);
  assert.equal(pads[2].velocity, 96 / 127);
  assert.equal(pads[3].chokeGroup, 'hats');
  assert.equal(pads.every(pad => pad.editable === false), true);
});

test('selected region and controls come from prepared source and declared scope', () => {
  const pads = preparedPads(document);
  assert.deepEqual(selectedRegion(document, pads[2]),
    { sourceId: 'drums', regionId: 'snare_hard_a' });
  assert.deepEqual(visibleParameters(document, pads[2]).map(parameter => parameter.id),
    ['master.level', 'snare.pitch']);
  assert.deepEqual(visibleParameters(document, pads[3]).map(parameter => parameter.id),
    ['master.level', 'hat.level']);
  assert.deepEqual(visibleParameters(document, null).map(parameter => parameter.id),
    ['master.level']);
});

test('absent maps and invalid source links remain unavailable', () => {
  assert.deepEqual(preparedPads({ maps: [] }), []);
  assert.equal(selectedRegion(document, null), null);
  assert.equal(selectedRegion(document, {
    ...preparedPads(document)[0], sourceIndex: 99,
  }), null);
});

test('typed normalized entry commits finite values and Escape cancels it', () => {
  assert.equal(normalizedDraft('0.901', false), 0.901);
  assert.equal(normalizedDraft('0.901', true), null);
  assert.equal(normalizedDraft('', false), null);
  assert.equal(normalizedDraft('NaN', false), null);
  assert.equal(normalizedDraft('1.1', false), null);
});

test('parameter changes reuse the prepared document until the instrument generation changes', () => {
  assert.equal(needsDocumentRefresh(3, 3), false);
  assert.equal(needsDocumentRefresh(2, 3), false);
  assert.equal(needsDocumentRefresh(4, 3), true);
});

test('pad blur and window focus loss release editor notes and clear pressed feedback', async () => {
  const calls = [];
  const audition = createNoteAudition(async (...args) => { calls.push(args); }, () => {});
  let pressed = true;
  await audition.press(36, 0.5, 3);
  const pad = padReleaseHandlers(() => audition.release(36));
  await pad.onBlur();
  assert.deepEqual(calls.map(call => call[0]), ['noteOn', 'noteOff']);

  await audition.press(36, 0.5, 3);
  let prevented = false;
  pad.onKeyUp({ key: 'Tab', preventDefault() { prevented = true; } });
  assert.equal(prevented, false);
  await pad.onKeyUp({ key: ' ', preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
  assert.deepEqual(calls.map(call => call[0]), ['noteOn', 'noteOff', 'noteOn', 'noteOff']);

  await audition.press(38, 0.5, 3);
  const releaseFocus = auditionFocusRelease(audition, () => { pressed = false; });
  await releaseFocus();
  assert.deepEqual(calls.map(call => call[0]),
    ['noteOn', 'noteOff', 'noteOn', 'noteOff', 'noteOn', 'noteOff']);
  assert.equal(pressed, false);
});
