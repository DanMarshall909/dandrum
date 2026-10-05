import assert from 'node:assert/strict';
import test from 'node:test';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createKeyMap } from './key-map.mjs';
import { preparedPads } from './model.mjs';

const KeyMap = createKeyMap(React);
const pads = preparedPads({ maps: [{ id: 'kit', selectionMode: 'round_robin', zones: [
  { id: 'kick', keyLow: 36, keyHigh: 36, velocityLow: 1, velocityHigh: 127 },
  { id: 'snare_soft', keyLow: 38, keyHigh: 38, velocityLow: 1, velocityHigh: 63 },
  { id: 'hard_a', keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127,
    roundRobinGroup: 'hard_snare' },
  { id: 'hard_b', keyLow: 38, keyHigh: 38, velocityLow: 64, velocityHigh: 127,
    roundRobinGroup: 'hard_snare' },
] }] });

function render(pads, selectedId, calls) {
  return renderToStaticMarkup(React.createElement(KeyMap, { pads, selectedId,
    onSelect: pad => calls.push(['select', pad.id]),
    onNoteOn: (...args) => calls.push(['on', ...args]),
    onNoteOff: note => calls.push(['off', note]),
  }));
}

test('key map renders real prepared ranges and grouped alternatives without issuing commands', () => {
  const calls = [];
  const html = render(pads, 'kit:hard_a', calls);
  assert.match(html, /data-low-note="36"/);
  assert.match(html, /data-high-note="38"/);
  assert.equal([...html.matchAll(/class="dd-key-zone"/g)].length, 3);
  assert.match(html, /aria-label="snare soft, MIDI 38 to 38, velocity 1 to 63"/);
  assert.match(html, /aria-label="hard snare, MIDI 38 to 38, velocity 64 to 127"/);
  assert.match(html, /KEY 38–38 · VEL 64–127/);
  assert.match(html, /aria-label="Editor audition keyboard"/);
  assert.match(html, /aria-label="Key 36 C2"/);
  assert.match(html, /Playback feedback unavailable/);
  assert.doesNotMatch(html, /Layered|EDITOR PRESS/);
  assert.deepEqual(calls, []);
});

test('an empty prepared key map has an explicit empty state and disabled zoom', () => {
  const html = render([], null, []);
  assert.match(html, /No prepared zones/);
  assert.doesNotMatch(html, /class="dd-key-zone"|aria-label="Editor audition keyboard"/);
  const zoomButtons = [...html.matchAll(/<button\b[^>]*>/g)].map(match => match[0]);
  assert.equal(zoomButtons.length, 3);
  assert.equal(zoomButtons.every(button => button.includes('disabled=""')), true);
});
