import assert from 'node:assert/strict';
import test from 'node:test';
import { createParameterGesture } from './parameter-gesture.mjs';

const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const tick = () => new Promise(resolve => setImmediate(resolve));

test('pointer drag admits one gesture and retains the latest value while the host stalls', async () => {
  const calls = [];
  const stalled = deferred();
  const gesture = createParameterGesture(async (name, value, generation) => {
    calls.push([name, value, generation]);
    if (name === 'setParameter' && value === 0.2) await stalled.promise;
  }, () => {});
  gesture.begin(4);
  gesture.change(0.2);
  await tick();
  gesture.change(0.3);
  gesture.change(0.8);
  const finished = gesture.end();
  assert.deepEqual(calls, [['beginGesture', undefined, 4], ['setParameter', 0.2, 4]]);
  stalled.resolve();
  await finished;
  assert.deepEqual(calls, [
    ['beginGesture', undefined, 4], ['setParameter', 0.2, 4],
    ['setParameter', 0.8, 4], ['endGesture', undefined, 4],
  ]);
});

test('keyboard and typed values each get host gesture boundaries', async () => {
  const calls = [];
  const gesture = createParameterGesture(async (...args) => { calls.push(args); }, () => {});
  gesture.change(0.4, 7);
  await gesture.flush();
  gesture.commit(0.9, 7);
  await gesture.flush();
  assert.deepEqual(calls, [
    ['beginGesture', undefined, 7], ['setParameter', 0.4, 7],
    ['endGesture', undefined, 7], ['beginGesture', undefined, 7],
    ['setParameter', 0.9, 7], ['endGesture', undefined, 7],
  ]);
});

test('non-pointer edits retain only the latest pending value under backpressure', async () => {
  const calls = [];
  const stalled = deferred();
  const gesture = createParameterGesture(async (name, value) => {
    calls.push([name, value]);
    if (name === 'setParameter' && value === 0.1) await stalled.promise;
  }, () => {});
  gesture.change(0.1, 2);
  await tick();
  gesture.change(0.2, 2);
  gesture.change(0.7, 2);
  assert.deepEqual(calls, [['beginGesture', undefined], ['setParameter', 0.1]]);
  stalled.resolve();
  await gesture.flush();
  assert.deepEqual(calls, [
    ['beginGesture', undefined], ['setParameter', 0.1], ['endGesture', undefined],
    ['beginGesture', undefined], ['setParameter', 0.7], ['endGesture', undefined],
  ]);
});

test('a rejected value reports failure and still closes its gesture', async () => {
  const calls = [];
  const failures = [];
  const gesture = createParameterGesture(async name => {
    calls.push(name);
    if (name === 'setParameter') throw new Error('rejected');
  }, reason => failures.push(reason.message));
  gesture.commit(0.5, 3);
  await gesture.flush();
  assert.deepEqual(calls, ['beginGesture', 'setParameter', 'endGesture']);
  assert.deepEqual(failures, ['rejected']);
  await gesture.end();
});

test('flush observes an active pointer write', async () => {
  const stalled = deferred();
  const calls = [];
  const gesture = createParameterGesture(async name => {
    calls.push(name);
    if (name === 'setParameter') await stalled.promise;
  }, () => {});
  gesture.begin(1);
  gesture.change(0.6);
  await tick();
  let flushed = false;
  const pending = gesture.flush().then(() => { flushed = true; });
  await tick();
  assert.equal(flushed, false);
  stalled.resolve();
  await pending;
  await gesture.end();
  assert.deepEqual(calls, ['beginGesture', 'setParameter', 'endGesture']);
});

test('a second drag waits for the closing gesture and keeps its own final value', async () => {
  const calls = [];
  const stalled = deferred();
  const gesture = createParameterGesture(async (name, value, generation) => {
    calls.push([name, value, generation]);
    if (name === 'setParameter' && value === 0.2) await stalled.promise;
  }, () => {});
  gesture.begin(5);
  gesture.change(0.2);
  await tick();
  const firstEnd = gesture.end();
  gesture.begin(5);
  gesture.change(0.8);
  const secondEnd = gesture.end();
  stalled.resolve();
  await Promise.all([firstEnd, secondEnd]);
  assert.deepEqual(calls, [
    ['beginGesture', undefined, 5], ['setParameter', 0.2, 5],
    ['endGesture', undefined, 5], ['beginGesture', undefined, 5],
    ['setParameter', 0.8, 5], ['endGesture', undefined, 5],
  ]);
});

test('typed entry during pointer closure waits for its own gesture boundary', async () => {
  const calls = [];
  const stalled = deferred();
  const gesture = createParameterGesture(async (name, value) => {
    calls.push([name, value]);
    if (name === 'setParameter' && value === 0.2) await stalled.promise;
  }, () => {});
  gesture.begin(5);
  gesture.change(0.2);
  await tick();
  const closing = gesture.end();
  gesture.commit(0.7, 5);
  stalled.resolve();
  await closing;
  await gesture.flush();
  assert.deepEqual(calls, [
    ['beginGesture', undefined], ['setParameter', 0.2], ['endGesture', undefined],
    ['beginGesture', undefined], ['setParameter', 0.7], ['endGesture', undefined],
  ]);
});

test('keyboard slider change during pointer closure retains its own host gesture', async () => {
  const calls = [];
  const stalled = deferred();
  const gesture = createParameterGesture(async (name, value) => {
    calls.push([name, value]);
    if (name === 'setParameter' && value === 0.2) await stalled.promise;
  }, () => {});
  gesture.begin(1);
  gesture.change(0.2, 1);
  await tick();
  const closing = gesture.end();
  gesture.change(0.8, 1);
  stalled.resolve();
  await closing;
  await gesture.flush();
  assert.deepEqual(calls, [
    ['beginGesture', undefined], ['setParameter', 0.2], ['endGesture', undefined],
    ['beginGesture', undefined], ['setParameter', 0.8], ['endGesture', undefined],
  ]);
});


test('queued and coalesced gestures retain origin ownership before prior writes complete', async () => {
  const calls = [], stalled = deferred(); let owner = 1;
  const gesture = createParameterGesture(async (...args) => {
    calls.push(args);
    if (args[0] === 'setParameter' && args[1] === 0.25) await stalled.promise;
  }, () => {}, () => owner);
  gesture.begin(2); owner = 2; gesture.change(0.25, 2); await tick();
  owner = 3; const firstEnd = gesture.end();
  owner = 4; gesture.begin(2);
  owner = 5; gesture.change(0.7, 2);
  owner = 6; gesture.change(0.9, 2);
  owner = 7; const secondEnd = gesture.end();
  owner = 8; gesture.commit(0.2, 2);
  owner = 9; stalled.resolve();
  await Promise.all([firstEnd, secondEnd]); await gesture.flush();
  assert.deepEqual(calls, [
    ['beginGesture', undefined, 2, 1], ['setParameter', 0.25, 2, 2],
    ['endGesture', undefined, 2, 3], ['beginGesture', undefined, 2, 4],
    ['setParameter', 0.9, 2, 6], ['endGesture', undefined, 2, 7],
    ['beginGesture', undefined, 2, 8], ['setParameter', 0.2, 2, 8], ['endGesture', undefined, 2, 8],
  ]);
});
