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
vm.runInNewContext(bootstrap, {window: {__JUCE__: {backend}}}, {filename:'dandrum-native-function-bootstrap.js'});

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

// A frozen event loop must not permit native timer snapshots to accumulate.
// The shared bootstrap acknowledges a delivered state without changing it.
assert.equal(typeof listeners.parameterStateChanged, 'function',
  'host-state publication has no receive acknowledgement');
const beforeState = emitted.length;
listeners.parameterStateChanged({generation:7, publication:'9007199254740993', parameters:[]});
assert.equal(emitted.length, beforeState + 1);
assert.equal(emitted.at(-1)[1].name, 'ackParameterState');
assert.deepEqual(Array.from(emitted.at(-1)[1].params), ['9007199254740993', 7]);
listeners.__juce__complete({promiseId:emitted.at(-1)[1].resultId, result:true});

for (const state of [null, {}, {generation:7}, {publication:'3'},
  {generation:7,publication:3}, {generation:0.5,publication:'3'}])
  listeners.parameterStateChanged(state);
assert.equal(emitted.length, beforeState + 1, 'invalid states created acknowledgement requests');

// A current-state reply also releases an event missed by a newly loaded page.
const currentState = {generation:8, publication:'4', parameters:[{id:'level',value:0.75}]};
const reconnect = backend.getNativeFunction('getParameterState')();
const reconnectId = emitted.at(-1)[1].resultId;
listeners.__juce__complete({promiseId:reconnectId, result:currentState});
assert.equal(await reconnect, currentState, 'reconnect acknowledgement replaced authoritative state');
assert.equal(emitted.at(-1)[1].name, 'ackParameterState');
assert.deepEqual(Array.from(emitted.at(-1)[1].params), ['4', 8]);
listeners.__juce__complete({promiseId:emitted.at(-1)[1].resultId, result:false});

const beforeOrdinary = emitted.length;
const ordinary = set('level', 0.25);
listeners.__juce__complete({promiseId:emitted.at(-1)[1].resultId, result:currentState});
assert.equal(await ordinary, currentState);
assert.equal(emitted.length, beforeOrdinary + 1, 'ordinary command reply acknowledged unrelated data');
