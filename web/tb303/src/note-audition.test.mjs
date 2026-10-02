import assert from 'node:assert/strict';
import test from 'node:test';

import { createNoteAudition } from './note-audition.mjs';

test('a release waits for a pending note-on and cannot be lost', async () => {
  const calls = [];
  let admitNoteOn;
  const audition = createNoteAudition((name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'noteOn') return new Promise(resolve => { admitNoteOn = resolve; });
    return Promise.resolve();
  });
  const pressing = audition.press(60, 0.9);
  assert.deepEqual(calls, [['noteOn', 60, 0.9]]);
  const releasing = audition.release(60);
  assert.equal(audition.isPressed(60), false);
  assert.deepEqual(calls, [['noteOn', 60, 0.9]]);
  admitNoteOn();
  await Promise.all([pressing, releasing]);
  assert.deepEqual(calls, [['noteOn', 60, 0.9], ['noteOff', 60]]);
});

test('rapid repeated presses stay bounded and page cleanup releases keys', async () => {
  const calls = [];
  const audition = createNoteAudition(async (name, ...args) => { calls.push([name, ...args]); });
  await Promise.all(Array.from({length: 256}, () => audition.press(48, 0.8)));
  assert.deepEqual(calls, [['noteOn', 48, 0.8]]);
  await audition.keepAlive();
  assert.deepEqual(calls.at(-1), ['noteHeartbeat']);
  await audition.releaseAll();
  assert.deepEqual(calls.at(-1), ['noteOff', 48]);
  const count = calls.length;
  await audition.keepAlive();
  assert.equal(calls.length, count);
});

test('a stalled browser transport keeps at most one heartbeat in flight', async () => {
  let finishHeartbeat;
  const calls = [];
  const audition = createNoteAudition((name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'noteHeartbeat')
      return new Promise(resolve => { finishHeartbeat = resolve; });
    return Promise.resolve();
  });
  await audition.press(60);
  const heartbeat = audition.keepAlive();
  const duplicates = Array.from({length: 256}, () => audition.keepAlive());
  assert.equal(calls.filter(([name]) => name === 'noteHeartbeat').length, 1);
  finishHeartbeat();
  await Promise.all([heartbeat, ...duplicates]);
  await audition.release(60);
});

test('a rejected note command reports the error and still allows cleanup', async () => {
  const calls = [];
  const errors = [];
  const audition = createNoteAudition(async (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'noteOn') throw new Error('rejected');
  }, error => errors.push(error.message));
  await audition.press(60);
  await audition.release(60);
  assert.deepEqual(errors, ['rejected']);
  assert.deepEqual(calls, [['noteOn', 60, 0.9], ['noteOff', 60]]);
});

test('release retains the instrument generation from its matching press', async () => {
  const calls = [];
  const audition = createNoteAudition(async (name, ...args) => { calls.push([name, ...args]); });
  await audition.press(60, 0.9, 4);
  await audition.release(60);
  assert.deepEqual(calls, [['noteOn', 60, 0.9, 4], ['noteOff', 60, 4]]);
});
