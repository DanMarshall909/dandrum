import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const source = fs.readFileSync(new URL('../../src/juce-plugin/InstrumentHostWebBridge.cpp', import.meta.url), 'utf8');
const bootstrap = source.match(/R"JS\(([\s\S]*?)\)JS"/)?.[1];
assert.ok(bootstrap, 'native function bootstrap was not embedded in the host bridge');

const listeners = {};
const emitted = [];
const backend = {
  addEventListener: (name, callback) => { listeners[name] = callback; },
  emitEvent: (name, payload) => emitted.push([name, payload]),
};
vm.runInNewContext(bootstrap, {window: {__JUCE__: {backend}}});

const set = backend.getNativeFunction('setParameter');
const first = set('filter.cutoff', 0.5);
const second = backend.getNativeFunction('noteOn')(60, 0.9);
assert.equal(emitted.length, 2);
assert.equal(emitted[0][0], '__juce__invoke');
assert.equal(emitted[0][1].name, 'setParameter');
assert.equal(emitted[0][1].resultId, 0);
assert.equal(emitted[0][1].params[0], 'filter.cutoff');
assert.equal(emitted[1][1].resultId, 1);
assert.equal(emitted[1][1].name, 'noteOn');

listeners.__juce__complete({promiseId: 99, result: 'ignored'});
listeners.__juce__complete({promiseId: 1, result: 'queued'});
listeners.__juce__complete({promiseId: 0, result: undefined});
assert.equal(await first, undefined);
assert.equal(await second, 'queued');
