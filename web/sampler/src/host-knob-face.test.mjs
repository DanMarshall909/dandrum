import assert from 'node:assert/strict';
import { after, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { createHostKnob } from '../../shared/host-knob.mjs';
const dom = new JSDOM('<div id="root"></div>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document, IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { createRoot } = await import('react-dom/client');
const root = createRoot(document.querySelector('#root')), Knob = createHostKnob(React);
after(async () => { await act(async () => root.unmount()); dom.window.close(); });
const part = name => document.querySelector(`[data-dd-knob-part="${name}"]`);
const parameter = value => ({ id: 'level', name: 'Level', generation: 1, sequence: 0, value });
async function render(size, value) {
  await act(async () => root.render(React.createElement(Knob, {
    key: `${size}-${value}`, size, parameter: value === null ? null : parameter(value),
    command: async () => ({ status: 'accepted', generation: 1, sequence: 1 }), onError: assert.fail })));
}
test('shared knob keeps its SVG geometry, disabled colours and active feedback at every size', async () => {
  await render(32, null);
  assert.equal(document.querySelector('svg').getAttribute('viewBox'), '0 0 32 32');
  assert.equal(part('cap').getAttribute('r'), '6.5');
  assert.equal(part('cap').getAttribute('fill'), 'var(--dd-ink-4)');
  assert.equal(part('cap').getAttribute('stroke-opacity'), '0.3');
  assert.equal(part('value-arc'), null);
  assert.equal(document.querySelector('svg path').getAttribute('stroke'), 'var(--dd-ink-4)');
  await render(36, 0);
  assert.equal(part('cap').getAttribute('r'), '8.5');
  assert.equal(part('value-arc').getAttribute('d'), '');
  assert.equal(part('value-arc').getAttribute('stroke-width'), '1.25');
  await render(48, 0.5);
  assert.equal(part('cap').getAttribute('r'), '12');
  assert.equal(part('cap').getAttribute('fill'), 'var(--dd-ink-5)');
  assert.equal(part('value-arc').getAttribute('d'), 'M12.69 35.31A16 16 0 0 1 24.00 8.00');
  await act(async () => document.querySelector('[role=slider]').dispatchEvent(new window.MouseEvent('mouseover', { bubbles: true })));
  assert.equal(part('cap').getAttribute('fill'), 'var(--dd-cap-hover)');
  const slider = document.querySelector('[role=slider]'); slider.setPointerCapture = () => {};
  const down = new window.MouseEvent('pointerdown', { button: 0, bubbles: true });
  Object.defineProperty(down, 'pointerId', { value: 1 });
  await act(async () => slider.dispatchEvent(down));
  assert.equal(part('cap').getAttribute('fill'), 'var(--dd-ink-6)');
  assert.equal(part('value-arc').getAttribute('stroke-width'), '3');
  await render(64, 1);
  assert.equal(part('cap').getAttribute('r'), '19');
  assert.equal(part('value-arc').getAttribute('d'), 'M15.38 48.62A23.5 23.5 0 1 1 48.62 48.62');
  assert.equal(part('cap').getAttribute('stroke-opacity'), '0.6');
  assert.equal(document.querySelector('svg line').getAttribute('opacity'), '1');
});
