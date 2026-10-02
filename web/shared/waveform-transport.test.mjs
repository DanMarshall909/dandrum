import assert from 'node:assert/strict';
import test from 'node:test';
import { createWaveformTransport } from './waveform-transport.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};

const region = (name, generation = 7) => ({
  sourceId: 'drums', regionId: name, channel: 0, buckets: 512, generation,
});

test('rapid region changes cancel superseded jobs and admit only the latest selection', async () => {
  const first = deferred();
  const calls = [];
  const states = [];
  let nextJob = 2;
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'requestWaveform') return args[1] === 'kick' ? first.promise
      : Promise.resolve({ status: 'accepted', job_id: String(nextJob++), generation: args[4] });
    return Promise.resolve(true);
  };
  const transport = createWaveformTransport(invoke, state => states.push(state));
  const selection = transport.select(region('kick'));
  for (let i = 0; i < 100; ++i)
    void transport.select(region(`alternate-${i}`));
  first.resolve({ status: 'accepted', job_id: '1', generation: 7 });
  await selection;
  assert.deepEqual(calls.filter(([name]) => name === 'requestWaveform')
    .map(([, , selected]) => selected), ['kick', 'alternate-99']);
  assert.ok(calls.some(([name, id]) => name === 'cancelWaveform' && id === '1'));
  assert.equal(transport.activeJobId(), '2');
  assert.deepEqual(states.map(state => state.selection.regionId), ['alternate-99']);
});

test('one in-flight poll publishes only current-generation ready data', async () => {
  const pending = deferred();
  const calls = [];
  const states = [];
  let packet = pending.promise;
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'requestWaveform')
      return Promise.resolve({ status: 'accepted', job_id: String(args[4]), generation: args[4] });
    if (name === 'getWaveformJobStatus') return packet;
    return Promise.resolve(true);
  };
  const transport = createWaveformTransport(invoke, state => states.push(state));
  await transport.select(region('kick'));
  const poll = transport.poll();
  await Promise.all(Array.from({ length: 1000 }, () => transport.poll()));
  assert.equal(calls.filter(([name]) => name === 'getWaveformJobStatus').length, 1);
  pending.resolve({ state: 'ready', generation: 6, result: { sourceId: 'drums' } });
  await poll;
  assert.equal(states.some(state => state.status.state === 'ready'), false);
  packet = Promise.resolve({ state: 'ready', generation: 7,
    result: { sourceId: 'drums', regionId: 'kick' } });
  await transport.poll();
  assert.equal(states.at(-1).status.state, 'ready');
  assert.equal(transport.activeJobId(), null);
  await transport.poll();
  assert.equal(calls.filter(([name]) => name === 'getWaveformJobStatus').length, 2);
});

test('reload and close prevent delayed results and cancel active or late-accepted work', async () => {
  const delayed = deferred();
  const calls = [];
  const states = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'requestWaveform')
      return args[4] === 8 ? delayed.promise
        : Promise.resolve({ status: 'accepted', job_id: '7', generation: 7 });
    if (name === 'getWaveformJobStatus')
      return Promise.resolve({ state: 'ready', generation: 7, result: { regionId: 'kick' } });
    return Promise.resolve(true);
  };
  const transport = createWaveformTransport(invoke, state => states.push(state));
  await transport.select(region('kick'));
  const replacement = transport.select(region('snare', 8));
  assert.ok(calls.some(([name, id]) => name === 'cancelWaveform' && id === '7'));
  for (let i = 0; i < 10 && !calls.some(([name, , , , , generation]) =>
    name === 'requestWaveform' && generation === 8); ++i)
    await Promise.resolve();
  assert.ok(calls.some(([name, , , , , generation]) =>
    name === 'requestWaveform' && generation === 8));
  await transport.close();
  delayed.resolve({ status: 'accepted', job_id: '8', generation: 8 });
  await replacement;
  await transport.poll();
  assert.ok(calls.some(([name, id]) => name === 'cancelWaveform' && id === '8'));
  assert.equal(states.some(state => state.status.state === 'ready'), false);
  assert.equal(transport.activeJobId(), null);
});

test('admission errors are bounded and a later region request can retry', async () => {
  const states = [];
  let attempt = 0;
  const transport = createWaveformTransport(async name => {
    if (name !== 'requestWaveform') return true;
    if (++attempt === 1) throw new Error('worker unavailable');
    if (attempt === 2) return 'Waveform request unavailable';
    return { status: 'accepted', job_id: '10', generation: 7 };
  }, state => states.push(state));
  await transport.select(region('kick'));
  await transport.select(region('snare'));
  await transport.select(region('hat'));
  assert.deepEqual(states.map(state => state.status.state),
    ['failed', 'failed', 'running']);
  assert.match(states[0].status.error, /worker unavailable/);
  assert.match(states[1].status.error, /Waveform request unavailable/);
  assert.equal(transport.activeJobId(), '10');
  await transport.close();
});

test('wrong-generation accepted jobs are cancelled instead of abandoned', async () => {
  const calls = [];
  const transport = createWaveformTransport(async (name, ...args) => {
    calls.push([name, ...args]);
    return name === 'requestWaveform'
      ? { status: 'accepted', job_id: '44', generation: 6 } : true;
  }, () => {});
  await transport.select(region('kick'));
  assert.deepEqual(calls.at(-1), ['cancelWaveform', '44']);
  assert.equal(transport.activeJobId(), null);
});

test('terminal stale status retires an old job after host reload', async () => {
  const calls = [];
  const states = [];
  const transport = createWaveformTransport(async (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'requestWaveform')
      return { status: 'accepted', job_id: '45', generation: 7 };
    if (name === 'getWaveformJobStatus')
      return { state: 'stale', generation: 6, error: 'reloaded' };
    return true;
  }, state => states.push(state));
  await transport.select(region('kick'));
  await transport.poll();
  await transport.poll();
  assert.equal(transport.activeJobId(), null);
  assert.equal(calls.filter(([name]) => name === 'getWaveformJobStatus').length, 1);
  assert.equal(states.at(-1).status.state, 'stale');
});
