import assert from 'node:assert/strict';
import test from 'node:test';
import { acceptedKnobWrite, knobSnapshotValue, createKnobCommand } from './knob-reconciliation.mjs';
test('only accepted valid value writes advance knob admission', () => {
  const current = { generation: 2, sequence: 4, value: 0.5 };
  for (const reply of [null, {}, { status: 'queueFull' }, { status: 'accepted', generation: 2, sequence: 3 },
    { status: 'accepted', generation: 1, sequence: 100 }, { status: 'accepted', generation: 2.5, sequence: 5 },
    { status: 'accepted', generation: 2, sequence: 5.5 }])
    assert.equal(acceptedKnobWrite(current, reply, 0.75), current);
  for (const value of [NaN, undefined, -1, 2])
    assert.equal(acceptedKnobWrite(current, { status: 'accepted', generation: 2, sequence: 5 }, value), current);
  assert.deepEqual(acceptedKnobWrite(current, { status: 'accepted', generation: 2, sequence: 5 }, 0.75),
    { generation: 2, sequence: 5, value: 0.75 });
  assert.deepEqual(acceptedKnobWrite(null, { status: 'accepted', generation: 1, sequence: 0 }, 0),
    { generation: 1, sequence: 0, value: 0 });
});
test('snapshots cover the latest admitted write before reconciling its display', () => {
  const write = { generation: 2, sequence: 4, value: 0.75 };
  assert.equal(knobSnapshotValue({ generation: 2, sequence: 3, value: 0.6 }, write), 0.75);
  assert.equal(knobSnapshotValue({ generation: 2, value: 0.6 }, write), 0.75);
  assert.equal(knobSnapshotValue({ generation: 2, sequence: 4, value: 0.4 }, write), 0.4);
  assert.equal(knobSnapshotValue({ generation: 3, sequence: 0, value: 0.2 }, write), 0.2);
  assert.equal(knobSnapshotValue({ value: 0.5 }, null), 0.5);
  assert.equal(knobSnapshotValue(null, write), 0);
});

test('value replies retain admission and late failures retain their submitting interaction', async () => {
  let owner = 1, reject;
  const late = new Promise((resolve, fail) => { reject = fail; }), writes = [], restores = [], calls = [];
  const reply = { status: 'accepted', generation: 2, sequence: 4 };
  const command = createKnobCommand((...args) => {
    calls.push(args); return args[0] === 'endGesture' ? late : Promise.resolve(reply);
  }, () => owner, (...args) => writes.push(args), failedOwner => restores.push(failedOwner));
  assert.equal(await command('beginGesture', undefined, 2), reply);
  assert.deepEqual(writes, []);
  assert.equal(await command('setParameter', 0.75, 2), reply);
  assert.deepEqual(writes, [[reply, 0.75]]);
  const closing = command('endGesture', undefined, 2); owner = 2;
  const error = new Error('noGesture'); reject(error);
  await assert.rejects(closing, reason => reason === error);
  assert.deepEqual(restores, [1]);
  assert.deepEqual(calls, [['beginGesture', undefined, 2], ['setParameter', 0.75, 2], ['endGesture', undefined, 2]]);
});


test('queued command failures retain their originating owner without passing it to the host', async () => {
  const error = new Error('noGesture'), restores = [], calls = [];
  const command = createKnobCommand((...args) => { calls.push(args); throw error; },
    () => 9, () => {}, failedOwner => restores.push(failedOwner));
  await assert.rejects(command('endGesture', undefined, 2, 3), reason => reason === error);
  assert.deepEqual(restores, [3]);
  assert.deepEqual(calls, [['endGesture', undefined, 2]]);
});
