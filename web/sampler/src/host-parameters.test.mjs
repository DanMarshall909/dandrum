import assert from 'node:assert/strict';
import { after, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { createHostParameters } from '../../shared/host-parameters.mjs';
const dom = new JSDOM('<div id="root"></div>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document, IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { createRoot } = await import('react-dom/client');
const useHost = createHostParameters(React);
after(() => dom.window.close());

test('shared React hook renders only changed state, loads matching metadata and removes its subscription', async () => {
  let listener, removed, result; const calls = [];
  let next = { generation: 1, sequence: 0, parameters: [{ id: 'level', name: 'Level', value: 0.5 }] };
  const backend = { addEventListener(name, callback) { listener = callback; },
    removeEventListener(name, callback) { removed = callback; },
    getNativeFunction(name) { return async () => { calls.push(name);
      if (name === 'getParameterState') return next;
      if (name === 'getPreparedDocument') return { generation: next.generation, parameters: [] };
      return { status: 'accepted', generation: 1, sequence: 1 };
    }; } };
  let renders = 0;
  function Panel() { result = useHost(backend); renders++; return React.createElement('span', null, result.state?.parameters[0]?.value); }
  const root = createRoot(document.querySelector('#root'));
  await act(async () => root.render(React.createElement(Panel)));
  assert.equal(document.body.textContent, '0.5'); assert.equal(result.document.generation, 1);
  const before = renders;
  await act(async () => listener(structuredClone(next))); assert.equal(renders, before);
  next = { ...next, parameters: [{ ...next.parameters[0], value: 0.8 }] };
  await act(async () => listener(next)); assert.equal(document.body.textContent, '0.8');
  await act(async () => result.command('beginGesture'));
  assert.equal(calls.filter(name => name === 'getParameterState').length, 1);
  await act(async () => result.command('setParameter'));
  assert.equal(calls.filter(name => name === 'getParameterState').length, 2);
  await act(async () => root.unmount()); assert.equal(removed, listener);
});

test('shared React hook reports unavailable hosts, native errors and missing commands', async () => {
  const container = document.createElement('div'); document.body.append(container);
  let result;
  function Panel({ backend }) { result = useHost(backend); return React.createElement('span', null, result.error); }
  const root = createRoot(container);
  await act(async () => root.render(React.createElement(Panel)));
  assert.match(container.textContent, /plugin host/);
  const backend = { addEventListener() {}, getNativeFunction(name) {
    if (name === 'getParameterState') return async () => 'native read failed';
    return null;
  } };
  await act(async () => root.render(React.createElement(Panel, { backend })));
  assert.match(container.textContent, /native read failed/);
  await act(async () => { await assert.rejects(result.command('beginGesture'), /unavailable/); });
  await act(async () => root.unmount()); container.remove();
});
