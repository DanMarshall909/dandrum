import assert from 'node:assert/strict';
import test from 'node:test';
import { createPreparedPolling } from './prepared-polling.mjs';
const clocks = () => { const tasks = new Map(); let id = 0; return {
  setInterval(fn) { tasks.set(++id, fn); return id; }, clearInterval(id) { tasks.delete(id); },
  tasks, async tick() { for (const fn of tasks.values()) await fn(); },
}; };

test('terminal results retire timers while partial readiness keeps polling', async () => {
  const clock = clocks(), packets = []; let callback, polls = 0;
  const factory = onState => { callback = onState; return { async select() { callback({ status: { state: 'running' } }); return true; },
    async poll() { polls++; if (polls === 2) callback({ status: { state: 'ready', result: { columns: [1] } } }); }, async close() {} }; };
  const polling = createPreparedPolling(factory, packet => packets.push(packet), assert.fail, clock);
  await polling.select({}); assert.equal(clock.tasks.size, 1);
  await clock.tick(); assert.equal(clock.tasks.size, 1);
  await clock.tick(); assert.equal(clock.tasks.size, 0);
  assert.equal(packets.at(-1).status.state, 'ready');
  await polling.close();
});

test('failure, admission rejection and close remove timers and ignore late states', async () => {
  for (const terminal of ['failed', 'cancelled', 'stale']) {
    const clock = clocks(); let callback;
    const polling = createPreparedPolling(onState => { callback = onState; return {
      async select() { callback({ status: { state: terminal } }); return true; }, async close() {}, async poll() {} };
    }, () => {}, assert.fail, clock);
    await polling.select({}); assert.equal(clock.tasks.size, 0); await polling.close();
  }
  for (const admit of [false, new Error('admission')]) {
    const clock = clocks(), errors = [];
    const polling = createPreparedPolling(() => ({ async select() { if (admit instanceof Error) throw admit; return admit; }, async close() {} }),
      () => {}, error => errors.push(error), clock);
    await polling.select({}); assert.equal(clock.tasks.size, 0);
    assert.equal(errors.length, admit === false ? 0 : 1); await polling.close();
  }
  const clock = clocks(), packets = [], errors = []; let callback;
  const polling = createPreparedPolling(onState => { callback = onState; return {
    async select() { return true; }, async poll() { throw new Error('poll'); }, async close() {} };
  }, packet => packets.push(packet), error => errors.push(error), clock);
  await polling.select({}); await clock.tick(); assert.equal(errors.length, 1); assert.equal(clock.tasks.size, 0);
  await polling.close(); callback({ status: { state: 'ready' } });
  assert.equal(packets.length, 0); assert.equal(await polling.select({}), false);
});

test('new polling owner restarts selection after visibility restoration or retry', async () => {
  const clock = clocks(); let closed = 0, selected = 0;
  const open = () => createPreparedPolling(() => ({ async select() { selected++; return true; },
    async poll() {}, async close() { closed++; } }), () => {}, assert.fail, clock);
  const hidden = open(); await hidden.select({}); await hidden.close(); await hidden.close();
  assert.equal(clock.tasks.size, 0); assert.equal(closed, 1);
  const shown = open(); await shown.select({}); assert.equal(clock.tasks.size, 1);
  assert.equal(selected, 2); await shown.close();
});

test('actual waveform transport keeps its timer after admission and retires on ready', async () => {
  const { createWaveformTransport } = await import('./waveform-transport.mjs');
  const clock = clocks(), packets = [];
  const polling = createPreparedPolling(onState => createWaveformTransport(async name =>
    name === 'requestWaveform' ? { status: 'accepted', job_id: '7', generation: 1 }
      : { state: 'ready', generation: 1, result: {} }, onState), packet => packets.push(packet), assert.fail, clock);
  await polling.select({ sourceId: 'drums', regionId: 'kick', channel: 0, buckets: 512, generation: 1 });
  assert.equal(clock.tasks.size, 1);
  await clock.tick(); assert.equal(packets.at(-1).status.state, 'ready');
  assert.equal(clock.tasks.size, 0); await polling.close();
});

test('actual spectrum transport keeps polling through partial ready pages', async () => {
  const { createSpectrumTransport } = await import('./spectrum-transport.mjs');
  const clock = clocks(), packets = [];
  const invoke = async (name, id, offset = 0) => name === 'requestSpectrogram'
    ? { status: 'accepted', job_id: '19', generation: 7 }
    : { state: 'ready', generation: 7, job_id: '19', result: {
      sourceId: 'drums', regionId: 'body', sampleRateHz: 48000, channel: 1,
      startFrame: '0', endFrame: '4608', contentRevision: 'ab'.repeat(32),
      settings: { fftSize: 1024, hopFrames: '256', window: 'periodicHann',
        scaling: 'oneSidedPeakDbFS', channelPolicy: 'selectedChannel', floorDbFS: -120 },
      frequencyHz: Array.from({ length: 513 }, (_, i) => i * 46.875), totalColumns: 18, columnOffset: offset,
      columns: Array.from({ length: Math.min(16, 18 - offset) }, (_, i) => ({
        startFrame: String((offset + i) * 256), endFrame: String(Math.min(4608, 1024 + (offset + i) * 256)),
        magnitudeDbFS: Array.from({ length: 513 }, () => -120) })) } };
  const polling = createPreparedPolling(onState => createSpectrumTransport(invoke, onState),
    packet => packets.push(packet), assert.fail, clock);
  await polling.select({ sourceId: 'drums', regionId: 'body', channel: 1, generation: 7 });
  await clock.tick(); assert.equal(clock.tasks.size, 1); assert.equal(packets.at(-1).status.state, 'running');
  await clock.tick(); assert.equal(clock.tasks.size, 0);
  assert.equal(packets.at(-1).status.result.columns.length, 18); await polling.close();
});
