import assert from 'node:assert/strict';
import test from 'node:test';
import { createPreparedParameterDocument } from './prepared-parameter-document.mjs';
import { admittedParameter, parseActualValue } from './parameter-value.mjs';

const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const tick = () => new Promise(resolve => setImmediate(resolve));
const document = (generation, minValue, maxValue, normalisedDefaultValue) => ({ generation,
  parameters: [{ id: 'pitch', minValue, maxValue, normalisedDefaultValue }] });

test('pending reload metadata cannot supply the old range or reset to the new generation', async () => {
  const old = deferred(), next = deferred(), late = deferred();
  const requests = [old, next, late];
  let prepared = null, count = 0;
  const fetch = createPreparedParameterDocument(() => requests[count++].promise,
    value => { prepared = value; }, reason => { throw reason; });
  const first = fetch.acceptGeneration(4);
  await tick();
  assert.equal(count, 1, 'accepting live state must request prepared control metadata');
  old.resolve(document(4, -1, 1, 0.2));
  await first;
  let state = { generation: 4, parameters: [{ id: 'pitch', value: 0.5 }] };
  assert.equal(admittedParameter(state, prepared, 'pitch').normalisedDefaultValue, 0.2);
  const pending = fetch.acceptGeneration(5);
  state = { ...state, generation: 5 };
  await tick();
  assert.equal(count, 2);
  assert.equal(admittedParameter(state, prepared, 'pitch'), null);
  next.resolve(document(5, 10, 20, 0.8));
  await pending;
  const control = admittedParameter(state, prepared, 'pitch');
  assert.equal(parseActualValue('16', control), 0.6);
  assert.equal(control.normalisedDefaultValue, 0.8);
  const delayed = fetch.refresh();
  await tick();
  late.resolve(document(4, -1, 1, 0.2));
  assert.equal(await delayed, false);
  assert.equal(admittedParameter(state, prepared, 'pitch').normalisedDefaultValue, 0.8);
  fetch.close();
});

test('a generation advance during one request coalesces and fetches current metadata afterward', async () => {
  const old = deferred(), current = deferred();
  const requests = [old, current];
  const emitted = []; let count = 0;
  const fetch = createPreparedParameterDocument(() => requests[count++].promise,
    value => emitted.push(value.generation), reason => { throw reason; });
  const pending = fetch.acceptGeneration(3);
  await tick();
  fetch.acceptGeneration(5);
  fetch.acceptGeneration(4);
  assert.equal(count, 1);
  old.resolve(document(3, 0, 1, 0.5));
  assert.equal(await pending, false);
  await tick();
  assert.equal(count, 2);
  current.resolve(document(5, 0, 1, 0.7));
  await fetch.refresh();
  assert.deepEqual(emitted, [5]);
  await fetch.acceptGeneration(5);
  await fetch.acceptGeneration(4);
  for (const invalid of [null, NaN, -1, 1.5]) await fetch.acceptGeneration(invalid);
  assert.equal(count, 2);
  fetch.close();
});

test('absent, invalid or failed metadata remains unavailable and can be refreshed without a queue', async () => {
  const replies = [null, { generation: NaN }, document(6, 0, 1, 0.4)];
  const emitted = [], errors = []; let count = 0;
  const fetch = createPreparedParameterDocument(async () => {
    if (count++ === 0) throw Error('metadata unavailable');
    return replies.shift();
  }, value => emitted.push(value.generation), reason => errors.push(reason.message));
  assert.equal(await fetch.acceptGeneration(6), false);
  assert.deepEqual(errors, ['metadata unavailable']);
  assert.equal(await fetch.refresh(), false);
  assert.equal(await fetch.refresh(), false);
  assert.equal(await fetch.refresh(), true);
  assert.deepEqual(emitted, [6]);
  fetch.close();
  assert.equal(await fetch.refresh(), false);
  await fetch.acceptGeneration(7);
  assert.equal(count, 4);
});

test('closing an editor ignores the late metadata response and error', async () => {
  for (const fail of [false, true]) {
    const reply = deferred(), emitted = [], errors = [];
    const fetch = createPreparedParameterDocument(() => reply.promise,
      value => emitted.push(value), reason => errors.push(reason));
    const pending = fetch.acceptGeneration(2);
    await tick();
    fetch.close();
    if (fail) reply.reject(Error('closed')); else reply.resolve(document(2, 0, 1, 0.1));
    assert.equal(await pending, false);
    assert.deepEqual(emitted, []);
    assert.deepEqual(errors, []);
  }
});
