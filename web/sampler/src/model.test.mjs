import assert from 'node:assert/strict';
import test from 'node:test';
import { preparedPads, visibleParameters, selectedRegion,
  padReleaseHandlers, auditionFocusRelease } from './model.mjs';
import { createNoteAudition } from '../../tb303/src/note-audition.mjs';
import * as model from './model.mjs';

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

test('selecting a round-robin alternative inspects its actual region and control scope', () => {
  const pads = preparedPads(document);
  assert.deepEqual(selectedRegion(document, pads[2], 'snare_hard_b'),
    { sourceId: 'drums', regionId: 'snare_hard_b' });
  assert.deepEqual(visibleParameters(document, pads[2], 'snare_hard_b')
    .map(parameter => parameter.id), ['master.level', 'snare.pitch']);

  // The same zone ID can occur in different maps. A group member may also
  // refer to a different source and declared control group.
  const other = structuredClone(document);
  other.sources.push({ id: 'other_drums', regions: [{ id: 'different_hard_hit' }] });
  Object.assign(other.maps[0].zones[3], { sourceIndex: 1, regionIndex: 0, controlGroup: 4 });
  other.maps.unshift({ id: 'other_kit', selectionMode: 'round_robin',
    zones: [{ ...document.maps[0].zones[3],
      sourceIndex: 0, regionIndex: 0 }] });
  const selected = preparedPads(other).find(pad => pad.label === 'hard snare'
    && pad.zoneIds.length === 2);
  assert.deepEqual(selectedRegion(other, selected, 'snare_hard_b'),
    { sourceId: 'other_drums', regionId: 'different_hard_hit' });
  assert.deepEqual(visibleParameters(other, selected, 'snare_hard_b')
    .map(parameter => parameter.id), ['master.level', 'hat.level']);
  assert.equal(selectedRegion(other, selected, 'kick'), null);
  assert.deepEqual(visibleParameters(other, selected, 'missing')
    .map(parameter => parameter.id), ['master.level']);
  assert.equal(selectedRegion(other, { ...selected, mapId: 'missing' }, 'snare_hard_b'), null);
});

test('absent maps and invalid source links remain unavailable', () => {
  assert.deepEqual(preparedPads({ maps: [] }), []);
  assert.equal(selectedRegion(document, null), null);
  assert.equal(selectedRegion(document, {
    ...preparedPads(document)[0], sourceIndex: 99,
  }), null);
});

test('key map fits actual MIDI keys and draws the inclusive snare velocity split', () => {
  const layout = model.keyMapLayout?.(preparedPads(document), 910);
  assert.deepEqual(layout && [layout.lowNote, layout.highNote, layout.keyWidth,
    layout.gridHeight, layout.keyboardHeight], [36, 46, 80, 140, 56]);
  assert.deepEqual(layout?.zones.map(zone => [zone.id, zone.left, zone.top,
    zone.width, zone.height]), [
    ['kit:kick', 0, 0, 80, 140],
    ['kit:snare_soft', 160, 64 / 127 * 140, 80, 63 / 127 * 140],
    ['kit:snare_hard_a', 160, 0, 80, 64 / 127 * 140],
    ['kit:hat_open', 800, 0, 80, 140],
  ]);
  assert.equal(layout?.notes.find(note => note.note === 36)?.name, 'C2');
  assert.equal(layout?.notes.find(note => note.note === 37)?.black, true);
  assert.deepEqual(layout?.whiteKeys.find(note => note.note === 36),
    { note: 36, name: 'C2', left: 0, width: 12 / 7 * 80 });
  assert.ok(Math.abs(layout.whiteKeys.find(note => note.note === 38).left
    - 137.14285714285714) < 1e-10, 'D2 begins one white-key width after C2');
});

test('key map compact sizing and bounded zoom preserve MIDI and velocity coordinates', () => {
  const pads = preparedPads(document);
  const compact = model.keyMapLayout(pads, 470, 2, true);
  assert.deepEqual([compact.width, compact.keyWidth, compact.gridHeight, compact.keyboardHeight],
    [880, 80, 100, 40]);
  assert.deepEqual([compact.zones[1].top, compact.zones[1].height],
    [64 / 127 * 100, 63 / 127 * 100]);
  assert.equal(model.keyMapLayout(pads, 470, 0).width, 440);
  assert.equal(model.keyMapLayout(pads, 470, 10).width, 3520);
  assert.equal(model.keyMapLayout([], 470), null);
  const edges = model.keyMapLayout([{ id: 'all', keyLow: 0, keyHigh: 127,
    velocityLow: 1, velocityHigh: 127 }], 158);
  assert.equal(edges.notes.length, 128);
  assert.deepEqual([edges.notes[0].name, edges.notes[127].name], ['C-1', 'G9']);
  assert.equal(edges.whiteKeys.length, 75);
  assert.equal(edges.whiteKeys.every(key => key.note >= 0 && key.note <= 127), true);
  assert.equal(edges.zones[0].width, 128);
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
