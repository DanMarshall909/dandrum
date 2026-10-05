import assert from 'node:assert/strict';
import { after, afterEach, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { preparedPads } from './model.mjs';
import { createLayerStack } from './layer-stack.mjs';

const dom = new JSDOM('<!doctype html><html><body></body></html>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document,
  IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { createRoot } = await import('react-dom/client');
const LayerStack = createLayerStack(React);
const prepared = {
  generation: 7,
  sources: [
    { id: 'drums', sampleRateHz: 48000, channelCount: 1, frameCount: '51000',
      slices: [{ id: 'attack', startFrame: '20003', endFrame: '20301' }], regions: [
        { id: 'hard_a', startFrame: '20000', endFrame: '28000', rootNote: 38,
          gainDb: -3, pan: -0.25, reverse: false, fadeInMs: 1.5, fadeOutMs: 3.5 },
      ] },
    { id: 'other_drums', sampleRateHz: 44100, channelCount: 2,
      frameCount: '9007199254741993', slices: [], regions: [
        { id: 'hard_b_region', startFrame: '9007199254741001', endFrame: '9007199254741987',
          rootNote: 0, gainDb: 0, pan: 0, reverse: true, fadeInMs: 0, fadeOutMs: 7,
          loop: { mode: 'ping_pong', startFrame: '9007199254741013',
            endFrame: '9007199254741969', crossfadeMs: 4.25 } },
      ] },
  ],
  maps: [{ id: 'kit', selectionMode: 'round_robin', zones: [
    { id: 'hard_a', sourceIndex: 0, regionIndex: 0, startFrame: '20007', endFrame: '27999',
      keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127, controlGroup: 2,
      roundRobinGroup: 'hard_snare', chokeGroup: 'snare', weight: 3, gainDb: -6, pan: 0.5,
      pitchSemitones: 1.5 },
    { id: 'hard_b', sourceIndex: 1, regionIndex: 0,
      startFrame: '9007199254741021', endFrame: '9007199254741973',
      keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127, controlGroup: 4,
      roundRobinGroup: 'hard_snare', chokeGroup: '', weight: 5 },
  ] }],
  parameters: [
    { id: 'master.level', name: 'Master level', scope: 'instrument' },
    { id: 'snare.pitch', name: 'Snare pitch', scope: 'sampleGroup', controlGroup: 2 },
    { id: 'hat.level', name: 'Hat level', scope: 'sampleGroup', controlGroup: 4 },
    { id: 'hidden.gain', name: 'Hidden', scope: 'sampleGroup', controlGroup: 9 },
  ],
  capabilities: { synthLayer: false, nestedPatchLayer: false, moduleChain: false },
};
let mounted;
async function mount(data = prepared, overrides = {}) {
  document.body.innerHTML = '<div id="root"></div><section id="sampler-public-controls"></section>';
  const root = createRoot(document.querySelector('#root'));
  const selected = [], edits = [];
  let settings = { document: data, pad: preparedPads(data).find(pad => pad.zoneIds.length === 2),
    selectedZoneId: 'hard_a', onChange: (...args) => edits.push(args), ...overrides };
  function Harness({ settings }) {
    const [zone, setZone] = React.useState(settings.selectedZoneId);
    return React.createElement(LayerStack, { ...settings, selectedZoneId: zone,
      onSelect: (pad, id) => { selected.push([pad.id, id]); setZone(id); } });
  }
  const update = async changes => {
    settings = { ...settings, ...changes };
    await act(async () => root.render(React.createElement(Harness, { settings })));
  };
  const unmount = async () => { await act(async () => root.unmount()); mounted = null; };
  mounted = { update, unmount, selected, edits };
  await update({});
  return mounted;
}
afterEach(async () => { if (mounted) await mounted.unmount(); });
after(() => dom.window.close());
const source = id => document.querySelector(`[data-source-zone="${id}"]`);
const details = () => document.querySelector('[aria-label="Prepared source details"]');
const field = name => [...details().querySelectorAll('dt')]
  .find(item => item.textContent === name)?.nextElementSibling.textContent;
const click = target => act(async () => target.click());
const key = (target, key, repeat = false) => act(async () => target.dispatchEvent(
  new window.KeyboardEvent('keydown', { key, repeat, bubbles: true, cancelable: true })));

test('actual source alternatives expose exact region/mapping facts and declared public bindings', async () => {
  const fixture = await mount(prepared, { readOnlyIcon:
    React.createElement('span', { 'aria-hidden': true, 'data-read-only-icon': true }, 'Lock') });
  assert.equal(document.querySelector('[data-read-only-icon]').getAttribute('aria-hidden'), 'true');
  assert.match(document.body.textContent, /Alternatives.*round robin/s);
  assert.doesNotMatch(document.body.textContent, /all trigger|Layer 1|Filter|Saturate/);
  assert.equal(details(), null, 'inspection is initially closed');
  await click(source('hard_a'));
  assert.deepEqual(fixture.selected, [['kit:hard_a', 'hard_a']]);
  assert.equal(source('hard_a').getAttribute('aria-expanded'), 'true');
  assert.equal(details().dataset.generation, '7');
  assert.equal(field('Source'), 'drums');
  assert.equal(field('Asset'), '48000 Hz · 1 channel · 51000 frames');
  assert.equal(field('Region frames'), '20000..28000');
  assert.equal(field('Mapped frames'), '20007..27999');
  assert.equal(field('Root note'), '38');
  assert.equal(field('Region gain'), '-3 dB');
  assert.equal(field('Region pan'), '-0.25');
  assert.equal(field('Direction'), 'Forward');
  assert.equal(field('Fades'), '1.5 ms in · 3.5 ms out');
  assert.equal(field('Loop'), 'None');
  assert.equal(field('Keys / velocity'), '38..38 / 64..127');
  assert.equal(field('Mapping'), 'weight 3 · choke snare · gain -6 dB · pan 0.5 · pitch 1.5 st');
  assert.match(details().textContent, /attack: 20003\.\.20301/);
  assert.deepEqual([...details().querySelectorAll('[data-public-binding]')]
    .map(item => item.dataset.publicBinding), ['master.level', 'snare.pitch']);
  assert.equal(details().querySelector('a').getAttribute('href'), '#sampler-public-controls');
  assert.deepEqual(fixture.edits, [], 'inspection issues no structural change');
});

test('another source retains large frame identities, zero-valued metadata and its distinct scope', async () => {
  await mount();
  await click(source('hard_b'));
  assert.equal(details().dataset.sourceId, 'other_drums', 'alternative B shows its actual source identity');
  assert.equal(details().dataset.regionId, 'hard_b_region');
  assert.equal(field('Asset'), '44100 Hz · 2 channels · 9007199254741993 frames');
  assert.equal(field('Region frames'), '9007199254741001..9007199254741987', 'large region frames retain their exact decimal identities');
  assert.equal(field('Mapped frames'), '9007199254741021..9007199254741973');
  assert.equal(field('Root note'), '0');
  assert.equal(field('Region gain'), '0 dB');
  assert.equal(field('Region pan'), '0');
  assert.equal(field('Direction'), 'Reverse');
  assert.equal(field('Fades'), '0 ms in · 7 ms out');
  assert.equal(field('Loop'), 'ping pong · 9007199254741013..9007199254741969 · 4.25 ms crossfade');
  assert.equal(field('Slices'), 'None');
  assert.equal(field('Mapping'), 'weight 5 · choke None · gain Not declared · pan Not declared · pitch Not declared');
  assert.deepEqual([...details().querySelectorAll('[data-public-binding]')]
    .map(item => item.dataset.publicBinding), ['master.level', 'hat.level']);
});

test('keyboard opens once, Escape and close restore source focus, and unsupported editing stays absent', async () => {
  const data = structuredClone(prepared), before = structuredClone(data);
  const fixture = await mount(data);
  const button = source('hard_a');
  await act(async () => button.focus());
  await key(button, 'x');
  assert.equal(details(), null);
  await key(button, 'Enter');
  await key(button, 'Enter', true);
  assert.ok(details());
  assert.equal(fixture.selected.length, 1, 'repeat does not toggle or select again');
  const link = details().querySelector('a');
  await act(async () => link.focus());
  await key(link, 'x');
  assert.ok(details(), 'ordinary key in details keeps them open');
  await key(link, 'Escape');
  assert.equal(details(), null);
  assert.equal(document.activeElement, button, 'Escape returns focus from details to the source');
  await key(button, ' ');
  await key(button, 'Escape');
  assert.equal(details(), null, 'Escape on the source also closes details');
  await key(button, ' ');
  const close = document.querySelector('[aria-label="Close source details"]');
  await act(async () => close.focus());
  await click(close);
  assert.equal(document.activeElement, button);
  assert.equal(details(), null, 'close retires source details');
  await click(button);
  await click(button);
  assert.equal(details(), null, 'second source click closes details');
  assert.equal(document.querySelector('.dd-layer-stack input,.dd-layer-stack select'), null);
  assert.equal(document.querySelector('.dd-layer-stack [aria-label^="Mute"],.dd-layer-stack [aria-label^="Bypass"],.dd-layer-stack [aria-label^="Add"]'), null);
  assert.deepEqual(data, before, 'source and mapping snapshots remain unchanged');
  assert.deepEqual(fixture.edits, []);
});

test('new generation or pad retires open details instead of reopening old identities', async () => {
  const fixture = await mount();
  await click(source('hard_b'));
  await fixture.update({ document: { ...prepared, generation: 8 } });
  assert.equal(details(), null, 'new generation retires open source details');
  assert.equal(source('hard_b').getAttribute('aria-expanded'), 'false');
  await click(source('hard_b'));
  assert.equal(details().dataset.generation, '8');
  await fixture.update({ pad: { ...preparedPads(prepared)[0], id: 'other:hard_a' } });
  assert.equal(details(), null);
});

test('absent or invalid assignments are explicitly unavailable without borrowing another map', async () => {
  const data = structuredClone(prepared);
  data.maps.unshift({ id: 'other', zones: [{ ...data.maps[0].zones[0], sourceIndex: 1 }] });
  data.maps[1].zones[1].sourceIndex = 99;
  const fixture = await mount(data);
  assert.equal(source('hard_b').disabled, true);
  assert.match(source('hard_b').textContent, /Source assignment unavailable/);
  await click(source('hard_a'));
  assert.equal(field('Source'), 'drums', 'same zone ID in another map cannot replace the source');
  await fixture.update({ pad: { ...preparedPads(data).find(pad => pad.zoneIds.length === 2), mapId: 'missing' } });
  assert.equal(source('hard_a').disabled, true);
  assert.equal(details(), null);
  await fixture.update({ pad: null });
  assert.match(document.body.textContent, /No mapped sample selected/);
  assert.equal(document.querySelector('[data-source-zone]'), null);
});

test('undeclared playback values and source kinds remain unavailable', async () => {
  const data = structuredClone(prepared);
  delete data.sources[0].regions[0].rootNote;
  delete data.sources[0].regions[0].gainDb;
  delete data.sources[0].regions[0].pan;
  data.capabilities = { synthLayer: true, nestedPatchLayer: true, moduleChain: true };
  await mount(data);
  await click(source('hard_a'));
  assert.equal(field('Root note'), 'Not declared');
  assert.equal(field('Region gain'), 'Not declared');
  assert.equal(field('Region pan'), 'Not declared');
  assert.match(document.body.textContent, /Synth layers unavailable/);
  assert.match(document.body.textContent, /Nested patches unavailable/);
  assert.match(document.body.textContent, /Module chain unavailable/);
});

test('a missing region link stays unavailable and same-identity rerenders keep current inspection', async () => {
  const fixture = await mount();
  await click(source('hard_a'));
  await fixture.update({ document: { ...prepared } });
  assert.equal(field('Source'), 'drums', 'an ordinary parent rerender keeps current inspection');
  const invalid = structuredClone(prepared);
  invalid.maps[0].zones[0].regionIndex = 99;
  await fixture.update({ document: invalid });
  assert.equal(details(), null, 'invalid region retires details');
  assert.equal(source('hard_a').disabled, true);
  assert.match(source('hard_a').textContent, /Source assignment unavailable/);
});
