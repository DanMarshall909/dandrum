import assert from 'node:assert/strict';
import test from 'node:test';
import { acceptParameterState, createParameterController } from './parameter-controller.mjs';
const state = (generation = 1, sequence = 0, value = 0.5) => ({ generation, sequence,
  parameters: [{ id: 'level', name: 'Level', value }] });
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };
const settle = async () => { for (let i = 0; i < 20; ++i) await Promise.resolve(); };

test('equal snapshots retain identity while automation, metadata and ordering remain authoritative', () => {
  const current = state(2, 4);
  assert.equal(acceptParameterState(current, structuredClone(current)), current);
  for (const invalid of [null, {}, state(1, 100), state(2, 3), { ...current, parameters: null }, { ...current, sequence: 1.5 }])
    assert.equal(acceptParameterState(current, invalid), current);
  assert.equal(acceptParameterState(current, state(2, 4), 5), current);
  for (const next of [state(2, 4, 0.75), state(2, 5), state(3, 0),
    { ...current, parameters: [] }, { ...current, parameters: [{ id: 'other', name: 'Level', value: 0.5 }] },
    { ...current, parameters: [{ id: 'level', name: 'Renamed', value: 0.5 }] }])
    assert.equal(acceptParameterState(current, next), next);
  const initial = state(); assert.equal(acceptParameterState(null, initial), initial);
});

test('writes progress while reads stall, with one read and one replacement refresh', async () => {
  const read = deferred(); const calls = [], seen = [], errors = [];
  let sequence = 0;
  const controller = createParameterController(async name => {
    calls.push(name);
    if (name === 'getParameterState') return read.promise;
    return { status: 'accepted', generation: 1, sequence: ++sequence };
  }, next => seen.push(next), reason => errors.push(reason));
  controller.accept(state());
  await controller.command('beginGesture', 'level', 1);
  assert.deepEqual(calls, ['beginGesture']);
  await controller.command('setParameter', 'level', 0.6, 1);
  for (let i = 0; i < 100; i++) await controller.command('setParameter', 'level', 0.7, 1);
  await controller.command('endGesture', 'level', 1);
  assert.equal(calls.filter(name => name === 'getParameterState').length, 1);
  assert.equal(calls.at(-1), 'endGesture');
  read.resolve(state(1, sequence, 0.7)); await settle();
  assert.equal(calls.filter(name => name === 'getParameterState').length, 2);
  assert.equal(seen.at(-1).parameters[0].value, 0.7);
  assert.equal(errors.filter(Boolean).length, 0);
  controller.close(); await controller.refresh();
});

test('rejection recovers authoritative state and stale replies cannot restore old generations', async () => {
  const read = deferred(), write = deferred(), seen = [], errors = [];
  let reads = 0;
  const controller = createParameterController(name => name === 'getParameterState'
    ? (++reads === 1 ? read.promise : Promise.resolve(state(2, 0, 0.9))) : write.promise,
  next => seen.push(next), reason => errors.push(String(reason)));
  controller.accept(state()); const pending = controller.command('setParameter');
  controller.accept(state(1, 0, 0.1)); assert.equal(seen.length, 1);
  controller.accept(state(2, 0, 0.9));
  write.resolve({ status: 'staleGeneration', generation: 1 });
  await assert.rejects(pending, /staleGeneration/);
  read.resolve(state(1, 99, 0.1)); await settle();
  assert.equal(seen.at(-1).generation, 2);
  assert.ok(errors.some(error => error.includes('staleGeneration')));
  await controller.refresh(); assert.equal(seen.at(-1).parameters[0].value, 0.9);
  controller.close();
});

test('admitted sequences reject stale reads, errors are reported, and closing ignores late results', async () => {
  const read = deferred(), errors = [], seen = [];
  let sequence = 5;
  const controller = createParameterController(async name => {
    if (name === 'getParameterState') return read.promise;
    if (name === 'bad') return 'native failure';
    return { status: 'accepted', generation: 1, sequence: sequence++ };
  }, next => seen.push(next), reason => errors.push(reason));
  controller.accept(state()); await controller.command('setParameter');
  read.resolve(state(1, 4, 0.2)); await settle(); assert.equal(seen.length, 1);
  await assert.rejects(controller.command('bad'), /native failure/);
  assert.ok(errors.some(reason => String(reason).includes('native failure')));
  const late = deferred();
  const other = createParameterController(() => late.promise, next => seen.push(next), reason => errors.push(reason));
  const pending = other.refresh(); other.close(); other.accept(state(8));
  late.resolve(state(8)); await pending; assert.equal(seen.length, 1);
  const broken = createParameterController(() => Promise.reject(new Error('read failure')), () => {}, reason => errors.push(reason));
  await broken.refresh(); assert.ok(errors.some(reason => String(reason).includes('read failure')));
  broken.close(); controller.close();
});

test('accepted commands can omit sequence and closed controllers ignore errors from pending work', async () => {
  const errors = [];
  const controller = createParameterController(async () => ({ status: 'accepted', generation: 1 }), () => {}, error => errors.push(error));
  controller.accept(state()); await controller.command('beginGesture'); controller.close();
  const read = deferred(), failed = createParameterController(() => read.promise, assert.fail, assert.fail);
  const pending = failed.refresh(); failed.close(); read.reject(new Error('late read')); await pending;
  const write = deferred(), closing = createParameterController(() => write.promise, assert.fail, assert.fail);
  const command = closing.command('setParameter'); closing.close(); write.reject(new Error('late write'));
  await assert.rejects(command, /late write/);
});
