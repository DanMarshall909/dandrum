import assert from 'node:assert/strict';
import { after, afterEach, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { createKeyMap } from './key-map.mjs';
import { preparedPads } from './model.mjs';

// Real React/ReactDOM handle effects, state, focus and DOM event dispatch.
// jsdom has no layout or native pointer capture. Supply measured inputs and
// record capture requests; native capture/dimensions have separate browser tests.
const dom = new JSDOM('<!doctype html><html><body></body></html>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document,
  IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
let availableWidth = 910;
Object.defineProperty(window.HTMLElement.prototype, 'clientWidth', {
  get: () => availableWidth,
});
window.HTMLElement.prototype.setPointerCapture = function (id) {
  this.capturedPointer = id;
};
globalThis.ResizeObserver = class {
  observe() {}
  disconnect() {}
};
const { createRoot } = await import('react-dom/client');
const KeyMap = createKeyMap(React);
const pads = preparedPads({ maps: [{ id: 'kit', selectionMode: 'round_robin', zones: [
  { id: 'kick', keyLow: 36, keyHigh: 36, velocityLow: 1, velocityHigh: 127 },
  { id: 'snare_soft', keyLow: 38, keyHigh: 38, velocityLow: 1, velocityHigh: 63 },
  { id: 'hard_a', keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127,
    roundRobinGroup: 'hard_snare' },
  { id: 'hard_b', keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127,
    roundRobinGroup: 'hard_snare' },
  { id: 'closed_hat', keyLow: 42, keyHigh: 42, velocityLow: 1, velocityHigh: 127 },
  { id: 'open_hat', keyLow: 46, keyHigh: 46, velocityLow: 1, velocityHigh: 127 },
] }] });
let mounted;

async function mount(overrides = {}) {
  availableWidth = 910; window.innerWidth = 1200;
  document.body.innerHTML = '<div id="root"></div><button id="outside">Outside</button>';
  const notes = [], selections = [];
  let settings = { pads, selectedId: 'kit:kick', generation: 1,
    onNoteOn: (note, velocity) => notes.push(['on', note, velocity]),
    onNoteOff: note => notes.push(['off', note]), ...overrides };
  const root = createRoot(document.querySelector('#root'));
  function Harness({ settings }) {
    const [selected, setSelected] = React.useState(settings.selectedId);
    return React.createElement(KeyMap, { ...settings, key: settings.generation,
      selectedId: selected, onSelect: pad => {
        selections.push(pad.id); setSelected(pad.id);
      } });
  }
  const update = async changes => {
    settings = { ...settings, ...changes };
    await act(async () => root.render(React.createElement(Harness, { settings })));
  };
  const unmount = async () => { await act(async () => root.unmount()); mounted = null; };
  mounted = { notes, selections, update, unmount };
  await update({});
  return mounted;
}

afterEach(async () => { if (mounted) await mounted.unmount(); });
after(() => dom.window.close());

function key(note) { return document.querySelector(`.dd-piano-key[data-note="${note}"]`); }
function zone(id) { return document.querySelector(`[data-pad-id="kit:${id}"]`); }
async function emit(target, event) {
  await act(async () => target.dispatchEvent(event));
  return event;
}
async function pointer(target, type, options = {}) {
  target.getBoundingClientRect = () => ({ top: 10, left: 30, width: 80, height: 56 });
  const event = new window.MouseEvent(type, { bubbles: true, cancelable: true,
    button: 0, clientY: 38, ...options });
  Object.defineProperty(event, 'pointerId', { value: options.pointerId ?? 11 });
  return emit(target, event);
}
async function keyboard(target, type, key, options = {}) {
  return emit(target, new window.KeyboardEvent(type, {
    key, bubbles: true, cancelable: true, ...options,
  }));
}
async function focus(target) { await act(async () => target.focus()); }

test('lost pointer capture releases the mounted audition exactly once', async () => {
  const { notes } = await mount();
  const piano = key(38);
  await pointer(piano, 'pointerdown');
  assert.equal(piano.capturedPointer, 11, 'primary pointer requests actual capture');
  assert.deepEqual(notes, [['on', 38, 76 / 127]]);
  await pointer(piano, 'lostpointercapture');
  assert.deepEqual(notes, [['on', 38, 76 / 127], ['off', 38]],
    'lost pointer capture releases exactly once');
  assert.equal(piano.getAttribute('aria-pressed'), 'false');
  await pointer(piano, 'pointerup');
  await pointer(piano, 'lostpointercapture');
  assert.equal(notes.length, 2, 'later releases cannot duplicate note-off');
});

test('pointer height sets bounded velocity and cancellation preserves ownership', async () => {
  const { notes } = await mount();
  const piano = key(38);
  await pointer(piano, 'pointerdown', { button: 2 });
  assert.deepEqual(notes, [], 'secondary pointer does not audition');
  assert.equal(piano.capturedPointer, undefined);
  await pointer(piano, 'pointerdown');
  assert.deepEqual(notes, [['on', 38, 76 / 127]], 'vertical pointer velocity is 76/127');
  await pointer(piano, 'pointercancel', { pointerId: 12 });
  await keyboard(piano, 'keyup', 'Enter');
  assert.equal(notes.length, 1, 'unrelated pointer/key releases retain the held pointer');
  await pointer(piano, 'pointercancel');
  assert.deepEqual(notes, [['on', 38, 76 / 127], ['off', 38]],
    'cancellation releases before any later pointer-up');
  await pointer(piano, 'pointerup');
  assert.equal(notes.length, 2);
  await pointer(piano, 'pointerdown', { clientY: -10 });
  await pointer(piano, 'pointerup');
  await pointer(piano, 'pointerdown', { clientY: 100 });
  await pointer(piano, 'pointerup');
  assert.deepEqual(notes.slice(2), [['on', 38, 1 / 127], ['off', 38],
    ['on', 38, 1], ['off', 38]], 'pointer velocity clamps to MIDI 1..127');
});

test('zones select and navigate without editing bounds and audition mapped velocities', async () => {
  const { notes, selections } = await mount();
  const soft = zone('snare_soft'), hard = zone('hard_a');
  await act(async () => hard.click());
  assert.equal(hard.getAttribute('aria-pressed'), 'true', 'click selects the prepared zone');
  await focus(soft);
  assert.equal(soft.getAttribute('aria-pressed'), 'true');
  await keyboard(soft, 'keydown', 'x');
  await keyboard(soft, 'keydown', 'Enter');
  await keyboard(soft, 'keydown', 'Enter', { repeat: true });
  await keyboard(soft, 'keyup', 'x');
  assert.deepEqual(notes, [['on', 38, 32 / 127]], 'soft keyboard velocity stays within 1..63');
  await keyboard(soft, 'keyup', 'Enter');
  assert.deepEqual(notes, [['on', 38, 32 / 127], ['off', 38]]);
  await keyboard(soft, 'keydown', 'ArrowRight');
  assert.equal(document.activeElement, hard);
  assert.equal(hard.getAttribute('aria-pressed'), 'true');
  await keyboard(hard, 'keydown', ' ');
  await keyboard(hard, 'keydown', ' ', { repeat: true });
  await keyboard(key(36), 'keyup', 'Enter');
  assert.equal(notes.length, 3, 'another owner cannot release a held zone');
  await keyboard(hard, 'keyup', ' ');
  assert.deepEqual(notes.slice(2), [['on', 38, 96 / 127], ['off', 38]],
    'hard keyboard velocity stays within 64..127');
  await keyboard(hard, 'keydown', 'ArrowDown');
  assert.equal(document.activeElement, soft);
  await focus(zone('kick'));
  await keyboard(zone('kick'), 'keydown', 'ArrowLeft');
  assert.equal(document.activeElement, zone('kick'), 'navigation stops at first zone');
  await focus(zone('open_hat'));
  await keyboard(zone('open_hat'), 'keydown', 'ArrowUp');
  assert.equal(document.activeElement, zone('open_hat'), 'navigation stops at last zone');
  assert.ok(selections.includes('kit:snare_soft') && selections.includes('kit:hard_a'));
  assert.match(soft.getAttribute('aria-label'), /velocity 1 to 63/);
  assert.match(hard.getAttribute('aria-label'), /velocity 64 to 127/);
});

test('piano keyboard replacement releases first and unrelated key-up cannot release it', async () => {
  const { notes } = await mount();
  const piano = key(36);
  await focus(piano);
  await keyboard(piano, 'keydown', 'x');
  assert.deepEqual(notes, [], 'an ordinary key does not audition');
  await keyboard(piano, 'keydown', 'Enter');
  await keyboard(piano, 'keydown', 'Enter', { repeat: true });
  await keyboard(key(38), 'keyup', 'Enter');
  assert.deepEqual(notes, [['on', 36, 64 / 127]]);
  await keyboard(piano, 'keydown', ' ');
  assert.deepEqual(notes, [['on', 36, 64 / 127], ['off', 36], ['on', 36, 64 / 127]],
    'replacement releases the previous note before the new press');
  await keyboard(piano, 'keyup', ' ');
  assert.equal(piano.getAttribute('aria-pressed'), 'false');
  assert.deepEqual(notes.at(-1), ['off', 36]);
});

test('piano uses a neutral velocity when no selected zone contains the key', async () => {
  const { notes } = await mount({ selectedId: 'kit:hard_a' });
  await keyboard(key(36), 'keydown', 'Enter');
  await keyboard(key(36), 'keyup', 'Enter');
  assert.deepEqual(notes, [['on', 36, 96 / 127], ['off', 36]]);
});

test('a tiny velocity zone retains its accessible bounds when its visual label cannot fit', async () => {
  const tinyPads = preparedPads({ maps: [{ id: 'kit', zones: [
    { id: 'tiny', keyLow: 36, keyHigh: 36, velocityLow: 64, velocityHigh: 64 },
  ] }] });
  await mount({ pads: tinyPads, selectedId: 'kit:tiny' });
  assert.match(zone('tiny').getAttribute('aria-label'), /MIDI 36 to 36, velocity 64 to 64/);
  assert.equal(zone('tiny').querySelector('.dd-key-zone-label'), null);
  assert.equal(zone('tiny').getAttribute('aria-pressed'), 'true');
});

const exits = [
  ['focus loss', () => focus(document.querySelector('#outside'))],
  ['window blur', () => emit(window, new window.Event('blur'))],
  ['page hide', () => emit(window, new window.Event('pagehide'))],
  ['visibility transition', () => emit(document, new window.Event('visibilitychange'))],
  ['Escape', () => keyboard(window, 'keydown', 'Escape')],
  ['generation replacement', fixture => fixture.update({ generation: 2 })],
  ['unmount', fixture => fixture.unmount()],
];
for (const [name, leave] of exits) {
  test(`${name} releases mounted editor audition`, async () => {
    const fixture = await mount();
    await focus(key(36));
    await keyboard(key(36), 'keydown', 'Enter');
    assert.deepEqual(fixture.notes, [['on', 36, 64 / 127]]);
    await leave(fixture);
    assert.deepEqual(fixture.notes, [['on', 36, 64 / 127], ['off', 36]],
      `${name} releases the held note`);
    await emit(window, new window.Event('pagehide'));
    assert.equal(fixture.notes.length, 2, 'subsequent cleanup is idempotent');
  });
}

test('cleanup uses current callbacks after the component rerenders', async () => {
  const fixture = await mount();
  await keyboard(key(36), 'keydown', 'Enter');
  const currentReleases = [];
  await fixture.update({ onNoteOff: note => currentReleases.push(note) });
  await emit(window, new window.Event('blur'));
  assert.deepEqual(currentReleases, [36]);
  assert.deepEqual(fixture.notes, [['on', 36, 64 / 127]], 'old callback is retired');
});

test('mounted zoom is bounded, preserves the anchor and adapts to compact width', async () => {
  const fixture = await mount();
  const map = document.querySelector('.dd-key-map');
  const viewport = document.querySelector('.dd-key-map-scroll');
  const button = label => document.querySelector(`[aria-label="${label}"]`);
  viewport.getBoundingClientRect = () => ({ left: 40 });
  assert.equal(button('Zoom out').disabled, true);
  await act(async () => button('Zoom in').click());
  assert.equal(map.dataset.zoom, '1.5');
  assert.equal(viewport.scrollLeft, 212.5, 'header zoom retains the visible center');
  await act(async () => button('Zoom out').click());
  assert.equal(map.dataset.zoom, '1');
  assert.equal(viewport.scrollLeft, 0);
  await act(async () => button('Zoom in').click());
  await act(async () => button('Zoom to fit').click());
  assert.equal(map.dataset.zoom, '1');
  assert.equal(viewport.scrollLeft, 0);
  const wheel = options => emit(viewport, new window.WheelEvent('wheel', {
    bubbles: true, cancelable: true, clientX: 180, deltaY: -Math.log(2) / 0.004, ...options,
  }));
  assert.equal((await wheel()).defaultPrevented, false, 'ordinary wheel remains scrolling');
  assert.equal(map.dataset.zoom, '1');
  assert.equal((await wheel({ ctrlKey: true })).defaultPrevented, true);
  assert.equal(map.dataset.zoom, '2');
  assert.equal(viewport.scrollLeft, 110, 'Ctrl wheel retains the pointed note');
  await wheel({ metaKey: true, deltaY: Math.log(2) / 0.004 });
  assert.equal(map.dataset.zoom, '1');
  assert.equal(viewport.scrollLeft, 0);
  await wheel({ ctrlKey: true, deltaY: -10000 });
  assert.equal(map.dataset.zoom, '8');
  assert.equal(button('Zoom in').disabled, true);
  await act(async () => button('Zoom in').click());
  assert.equal(map.dataset.zoom, '8');
  await act(async () => button('Zoom to fit').click());
  availableWidth = 820; window.innerWidth = 820;
  await emit(window, new window.Event('resize'));
  assert.equal(map.dataset.compact, 'true');
  assert.equal(document.querySelector('.dd-key-map-grid').style.height, '100px');
  assert.equal(document.querySelector('.dd-key-map-piano').style.height, '40px');
  await fixture.update({ pads: [], generation: 2 });
  assert.match(document.querySelector('.dd-key-map').textContent, /No prepared zones/);
  assert.equal(button('Zoom to fit').disabled, true);
  await emit(document.querySelector('.dd-key-map-scroll'), new window.WheelEvent('wheel', {
    bubbles: true, cancelable: true, ctrlKey: true, deltaY: -10000,
  }));
  assert.equal(document.querySelector('.dd-key-map').dataset.zoom, '1',
    'an empty map ignores zoom requests');
});
