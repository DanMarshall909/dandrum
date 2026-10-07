import assert from 'node:assert/strict';
import { after, afterEach, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { createHostParameters } from '../../shared/host-parameters.mjs';
import { createHostKnob } from '../../shared/host-knob.mjs';
import { admittedParameter } from '../../shared/parameter-value.mjs';
const dom = new JSDOM('<div id="root"></div>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document, IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { createRoot } = await import('react-dom/client');
const useHost = createHostParameters(React), Knob = createHostKnob(React);
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
let root;
afterEach(async () => { if (root) await act(async () => root.unmount()); root = null; });
after(() => dom.window.close());
const knob = () => document.querySelector('[role=slider]');
const value = () => Number(knob().getAttribute('aria-valuenow'));
async function pointer(type, y) {
  const event = new window.MouseEvent(type, { button: 0, clientY: y, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerId', { value: 1 });
  await act(async () => knob().dispatchEvent(event));
}
async function mount({ rejectWrite = false, stallEnd = false, stallWrite = false, defaultValue = 0.5 } = {}) {
  document.body.innerHTML = '<div id="root"></div>';
  let sequence = 0, hostValue = 0.5, reads = 0, listener;
  const read = deferred(), end = deferred(), write = deferred(), calls = [];
  const snapshot = () => ({ generation: 1, sequence, parameters: [{ id: 'level', name: 'Level', value: hostValue }] });
  const backend = { addEventListener(name, callback) { listener = callback; }, removeEventListener() {},
    getNativeFunction(name) { return async (...args) => {
      calls.push(name);
      if (name === 'getParameterState') return ++reads === 1 ? snapshot() : read.promise;
      if (name === 'getPreparedDocument') return { generation: 1, parameters: [{ id: 'level', name: 'Level',
        minValue: 0, maxValue: 1, normalisedDefaultValue: defaultValue }] };
      if (name === 'endGesture' && stallEnd) { const reply = await end.promise; if (reply) return reply; }
      if (name === 'setParameter' && rejectWrite) return { status: 'queueFull', generation: 1 };
      if (name === 'setParameter' && stallWrite) { stallWrite = false; await write.promise; }
      if (name === 'setParameter') hostValue = args[1];
      return { status: 'accepted', generation: 1, sequence: ++sequence };
    }; } };
  function Panel() {
    const host = useHost(backend);
    return React.createElement('div', null, React.createElement(Knob, {
      parameter: admittedParameter(host.state, host.document, 'level'),
      command: host.command, onError: host.reportError }), React.createElement('span', { role: 'alert' }, host.error));
  }
  root = createRoot(document.querySelector('#root'));
  await act(async () => root.render(React.createElement(Panel)));
  knob().setPointerCapture = () => {};
  return { read, end, write, calls, snapshot, async automate(next) { hostValue = next; await act(async () => listener(snapshot())); } };
}

test('accepted drag remains visible on release while reads stall, then automation stays authoritative', async () => {
  const host = await mount(); assert.equal(value(), 0.5);
  await pointer('pointerdown', 100); await pointer('pointermove', 50); assert.equal(value(), 0.75);
  await pointer('pointerup', 50); assert.equal(value(), 0.75, 'release rolled back the accepted write');
  assert.equal(host.snapshot().parameters[0].value, 0.75);
  await host.automate(0.5); assert.equal(value(), 0.5, 'newer equal-baseline automation did not reconcile');
  await act(async () => host.read.resolve({ ...host.snapshot(), sequence: 0, parameters: [{ id: 'level', value: 0.75 }] }));
  assert.equal(value(), 0.5, 'late read restored a superseded write');
});

test('rejected drag restores host value and reports the rejection without waiting for a read', async () => {
  await mount({ rejectWrite: true });
  await pointer('pointerdown', 100); await pointer('pointermove', 50); await pointer('pointerup', 50);
  assert.equal(value(), 0.5); assert.match(document.querySelector('[role=alert]').textContent, /queueFull/);
});

test('automation observed while dragging reconciles on release', async () => {
  const host = await mount(); await pointer('pointerdown', 100); await pointer('pointermove', 50);
  await host.automate(0.4); assert.equal(value(), 0.75);
  await pointer('pointerup', 50); assert.equal(value(), 0.4);
});

test('later keyboard interaction retains its display during previous gesture closure', async () => {
  const host = await mount({ stallEnd: true });
  await pointer('pointerdown', 100); await pointer('pointermove', 50); await pointer('pointerup', 50);
  await act(async () => knob().dispatchEvent(new window.KeyboardEvent('keydown', { key: 'ArrowUp', bubbles: true, cancelable: true })));
  assert.equal(value(), 0.8);
  await host.automate(0.75); assert.equal(value(), 0.8, 'an older snapshot stole the queued keyboard display');
  await act(async () => host.end.resolve()); assert.equal(value(), 0.8);
  assert.equal(host.snapshot().parameters[0].value, 0.8);
});

test('an interim snapshot cannot roll back a later accepted move on release', async () => {
  const host = await mount(); await pointer('pointerdown', 100);
  await pointer('pointermove', 80); assert.equal(value(), 0.6);
  await host.automate(0.6);
  await pointer('pointermove', 50); assert.equal(value(), 0.75);
  await pointer('pointerup', 50); assert.equal(value(), 0.75, 'interim snapshot replaced the final accepted move');
  await host.automate(0.4); assert.equal(value(), 0.4);
});

async function key(element, name) {
  await act(async () => element.dispatchEvent(new window.KeyboardEvent('keydown', {
    key: name, bubbles: true, cancelable: true })));
}

test('cancelled, unchanged and invalid typed edits preserve an accepted drag while reads stall', async () => {
  await mount(); await pointer('pointerdown', 100); await pointer('pointermove', 50);
  await pointer('pointerup', 50);
  for (const ending of ['Escape', 'Enter', 'invalid', 'blur']) {
    await key(knob(), 'Enter');
    const input = document.querySelector('input'); assert.ok(input);
    if (ending === 'invalid') {
      await act(async () => {
        Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set.call(input, 'bad');
        input.dispatchEvent(new window.Event('input', { bubbles: true }));
      });
      await key(input, 'Enter');
    } else if (ending === 'blur') await act(async () => input.blur());
    else await key(input, ending);
    assert.equal(value(), 0.75, `${ending} rolled back the accepted drag`);
  }
});

test('an accepted default reset survives opening and closing an unchanged value editor', async () => {
  const host = await mount({ defaultValue: 0.25 });
  await pointer('pointerdown', 100); await pointer('pointermove', 50); await pointer('pointerup', 50);
  await key(knob(), 'Delete'); assert.equal(value(), 0.25);
  await key(knob(), 'Enter'); await key(document.querySelector('input'), 'Enter');
  assert.equal(value(), 0.25);
  assert.equal(host.snapshot().parameters[0].value, 0.25);
});

test('a rejected earlier closure cannot restore the display owned by a later drag', async () => {
  const host = await mount({ stallEnd: true });
  await pointer('pointerdown', 100); await pointer('pointermove', 50); await pointer('pointerup', 50);
  await pointer('pointerdown', 100); await pointer('pointermove', 80); assert.equal(value(), 0.85);
  await act(async () => host.end.resolve({ status: 'noGesture', generation: 1 }));
  assert.equal(host.snapshot().parameters[0].value, 0.85, 'later write was not admitted');
  assert.equal(value(), 0.85, 'retired rejection stole the later drag display');
});


test('queued older closure rejection cannot reclaim a later active drag', async () => {
  const host = await mount({ stallEnd: true, stallWrite: true });
  await pointer('pointerdown', 100); await pointer('pointermove', 50); await pointer('pointerup', 50);
  assert.ok(!host.calls.includes('endGesture'), 'old closure must still wait for its write');
  await pointer('pointerdown', 100); await pointer('pointermove', 80); assert.equal(value(), 0.85);
  await act(async () => host.write.resolve());
  assert.ok(host.calls.includes('endGesture'));
  await act(async () => host.end.resolve({ status: 'noGesture', generation: 1 }));
  assert.equal(host.snapshot().parameters[0].value, 0.85);
  assert.equal(value(), 0.85, 'queued retired rejection stole the later drag display');
});
