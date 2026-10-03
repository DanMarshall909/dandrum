import assert from 'node:assert/strict';
import test from 'node:test';
import { createSpectrumTransport } from './spectrum-transport.mjs';

const selection = { sourceId: 'drums', regionId: 'body', channel: 1, generation: 7 };
const accepted = (id = '19', generation = 7) => ({ status: 'accepted', job_id: id, generation });
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };
const page = (offset = 0, total = 18) => ({ state: 'ready', generation: 7, job_id: '19', result: {
  sourceId: 'drums', regionId: 'body', sampleRateHz: 48000, channel: 1,
  startFrame: '768', endFrame: String(768 + total * 256), contentRevision: 'ab'.repeat(32),
  settings: { fftSize: 1024, hopFrames: '256', window: 'periodicHann',
    scaling: 'oneSidedPeakDbFS', channelPolicy: 'selectedChannel', floorDbFS: -120 },
  frequencyHz: Array.from({ length: 513 }, (_, i) => i * 46.875), totalColumns: total, columnOffset: offset,
  columns: Array.from({ length: Math.min(16, total - offset) }, (_, i) => ({
    startFrame: String(768 + (offset + i) * 256), endFrame: String(Math.min(768 + total * 256, 1792 + (offset + i) * 256)),
    magnitudeDbFS: Array.from({ length: 513 }, (_, bin) => bin === 64 ? -6.020599913 : -120) })) } });

test('bounded numeric pages assemble one coherent current spectrum', async () => {
  const calls = [], states = [];
  const transport = createSpectrumTransport(async (name, ...args) => {
    calls.push([name, ...args]);
    return name === 'requestSpectrogram' ? accepted() : page(args[1]);
  }, packet => states.push(packet));
  await transport.select(selection);
  assert.equal(transport.activeJobId(), '19');
  assert.deepEqual(calls[0], ['requestSpectrogram', 'drums', 'body', 1, 7]);
  await transport.poll();
  assert.equal(states.at(-1).status.state, 'running', 'partial columns became visible');
  await transport.poll();
  assert.deepEqual(calls.slice(1), [['getSpectrogramJobStatus', '19', 0], ['getSpectrogramJobStatus', '19', 16]]);
  const result = states.at(-1).status.result;
  assert.equal(result.columns.length, 18);
  assert.equal(result.columns[16].startFrame, '4864');
  assert.equal(result.columns[17].endFrame, '5376');
  assert.equal(result.columns[16].magnitudeDbFS[64], -6.020599913);
  assert.equal(result.frequencyHz[64], 3000);
  assert.equal(result.settings.floorDbFS, -120);
  assert.equal(transport.activeJobId(), null);
  assert.equal(await transport.poll(), false);
});

test('late admission coalesces selections and closure cancels accepted work', async () => {
  const admission = deferred(), calls = [], states = [];
  const transport = createSpectrumTransport(async (name, ...args) => {
    calls.push([name, ...args]);
    return name === 'requestSpectrogram' ? (args[1] === 'body' ? admission.promise : accepted('21')) : true;
  }, state => states.push(state));
  const first = transport.select(selection);
  const middle = transport.select({ ...selection, regionId: 'middle' });
  const last = transport.select({ ...selection, regionId: 'last' });
  admission.resolve(accepted()); await Promise.all([first, middle, last]);
  assert.deepEqual(calls, [['requestSpectrogram', 'drums', 'body', 1, 7],
    ['cancelSpectrogram', '19'], ['requestSpectrogram', 'drums', 'last', 1, 7]]);
  assert.equal(states.length, 1); assert.equal(states[0].selection.regionId, 'last');
  await transport.close();
  assert.deepEqual(calls.at(-1), ['cancelSpectrogram', '21']);
});

test('closing during admission cancels the late job without publishing', async () => {
  const admission = deferred(), calls = [], states = [];
  const transport = createSpectrumTransport(async (name, ...args) => {
    calls.push([name, ...args]); return name === 'requestSpectrogram' ? admission.promise : true;
  }, state => states.push(state));
  const pending = transport.select(selection);
  await transport.close(); admission.resolve(accepted()); await pending;
  assert.deepEqual(calls.at(-1), ['cancelSpectrogram', '19']);
  assert.equal(states.length, 0);
  assert.equal(await transport.select(selection), false);
});

test('a stalled status request cannot multiply or overwrite a replacement selection', async () => {
  const status = deferred(), calls = [], states = [];
  const transport = createSpectrumTransport(async (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'requestSpectrogram') return accepted(args[1] === 'body' ? '19' : '21');
    if (name === 'getSpectrogramJobStatus') return status.promise;
    return true;
  }, state => states.push(state));
  await transport.select(selection);
  const pending = transport.poll();
  assert.equal(await transport.poll(), false);
  await transport.select({ ...selection, regionId: 'replacement' });
  status.resolve(page(0, 1)); assert.equal(await pending, false);
  assert.equal(states.at(-1).selection.regionId, 'replacement');
  assert.equal(states.at(-1).status.state, 'running');
  assert.equal(calls.filter(c => c[0] === 'getSpectrogramJobStatus').length, 1);
  await transport.close(); assert.deepEqual(calls.at(-1), ['cancelSpectrogram', '21']);
});

test('malformed or changed pages are rejected without unbounded retained results', async () => {
  for (const corrupt of [
    () => page(0, 1025).result, // Coherent coordinates; only the resource cap is violated.
    data => ({ ...data, totalColumns: 1025 }), data => ({ ...data, columns: [] }),
    data => ({ ...data, columns: Array(17).fill(data.columns[0]) }),
    data => ({ ...data, columnOffset: 1 }), data => ({ ...data, frequencyHz: [] }),
    data => ({ ...data, settings: null }), data => ({ ...data, sampleRateHz: 0 }),
    data => ({ ...data, startFrame: 'wrong' }),
    data => ({ ...data, columns: [{ ...data.columns[0], startFrame: '769' }, ...data.columns.slice(1)] }),
    data => ({ ...data, columns: [{ ...data.columns[0], magnitudeDbFS: [] }] })]) {
    const states = [], calls = [];
    const transport = createSpectrumTransport(async (name, ...args) => {
      calls.push([name, ...args]);
      return name === 'requestSpectrogram' ? accepted()
        : name === 'getSpectrogramJobStatus' ? { ...page(), result: corrupt(page().result) } : true;
    }, state => states.push(state));
    await transport.select(selection); await transport.poll();
    assert.equal(states.at(-1).status.state, 'failed');
    assert.match(states.at(-1).status.error, /page/i);
    assert.equal(transport.activeJobId(), null);
    assert.deepEqual(calls.at(-1), ['cancelSpectrogram', '19']);
  }
  const states = [];
  const transport = createSpectrumTransport(async (name, id, offset) => name === 'requestSpectrogram' ? accepted()
    : name === 'getSpectrogramJobStatus' ? { ...page(offset), result: { ...page(offset).result,
      contentRevision: offset ? 'cd'.repeat(32) : 'ab'.repeat(32) } } : true, state => states.push(state));
  await transport.select(selection); await transport.poll(); await transport.poll();
  assert.equal(states.at(-1).status.state, 'failed', 'different contents were joined across pages');
});

test('running, wrong-identity and interrupted polls retain no obsolete result', async () => {
  for (const response of [
    { state: 'running', job_id: '19', generation: 7 },
    { ...page(), job_id: '20' }, { ...page(), generation: 6 }, new Error('status unavailable')]) {
    const states = [];
    const transport = createSpectrumTransport(async name => {
      if (name === 'requestSpectrogram') return accepted();
      if (name === 'cancelSpectrogram') throw new Error('disconnected');
      if (response instanceof Error) throw response;
      return response;
    }, state => states.push(state));
    await transport.select(selection); await transport.poll();
    assert.equal(states.at(-1).status.state, response.state === 'running' ? 'running' : 'failed');
    assert.equal(states.at(-1).status.result, undefined);
    await transport.close();
  }
  const pending = deferred(), states = [];
  const transport = createSpectrumTransport(async name => name === 'requestSpectrogram'
    ? accepted() : name === 'getSpectrogramJobStatus' ? pending.promise : true, state => states.push(state));
  await transport.select(selection); const poll = transport.poll(); await transport.close();
  pending.resolve(page()); assert.equal(await poll, false);
  assert.equal(states.length, 1); assert.equal(states[0].status.state, 'running');
});

test('terminal states, request failures and invalid identities allow explicit retry', async () => {
  for (const state of ['failed', 'cancelled', 'stale']) {
    const states = [];
    const transport = createSpectrumTransport(async name => name === 'requestSpectrogram' ? accepted()
      : { state, generation: 7, job_id: '19', error: 'read failed' }, packet => states.push(packet));
    await transport.select(selection); await transport.poll();
    assert.equal(states.at(-1).status.state, state);
    assert.equal(transport.activeJobId(), null);
  }
  for (const response of [accepted('19', 6), { status: 'rejected' }, new Error('offline')]) {
    const calls = [], states = [];
    let fail = true;
    const transport = createSpectrumTransport(async (name, ...args) => {
      calls.push([name, ...args]);
      if (name === 'requestSpectrogram' && fail) {
        fail = false; if (response instanceof Error) throw response; return response;
      }
      return name === 'requestSpectrogram' ? accepted() : page(0, 1);
    }, state => states.push(state));
    await transport.select(selection);
    assert.equal(states.at(-1).status.state, 'failed');
    if (response.generation === 6) assert.deepEqual(calls.at(-1), ['cancelSpectrogram', '19']);
    await transport.select(selection); await transport.poll();
    assert.equal(states.at(-1).status.state, 'ready');
  }
  let calls = 0;
  const transport = createSpectrumTransport(async () => { ++calls; return accepted(); }, () => {});
  for (const patch of [{ sourceId: '' }, { regionId: '' }, { channel: -1 }, { channel: 65536 },
    { generation: 1.5 }, { generation: 4294967296 }])
    assert.equal(await transport.select({ ...selection, ...patch }), false);
  assert.equal(calls, 0); await transport.close(); await transport.close();
});
