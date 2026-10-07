import assert from 'node:assert/strict';
import test from 'node:test';
import { createMeterTransport } from './meter-transport.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};

test('stalled browser requests and acknowledgements retain one in-flight meter packet', async () => {
  const request = deferred();
  const ack = deferred();
  const calls = [];
  const packets = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'subscribeMeter') return Promise.resolve(true);
    if (name === 'getMeterPacket') return request.promise;
    if (name === 'ackMeterPacket') return ack.promise;
    return Promise.resolve(true);
  };
  const transport = createMeterTransport(invoke, packet => packets.push(packet));
  await Promise.all (Array.from({ length: 100 }, () => transport.start(7)));
  assert.equal(calls.filter(([name]) => name === 'subscribeMeter').length, 1);

  const firstTick = transport.tick();
  await Promise.all(Array.from({ length: 1000 }, () => transport.tick()));
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 1);
  request.resolve({ sequence: '9007199254740993', generation: 7, peak: [0.5, 0.25] });
  await Promise.resolve();
  assert.deepEqual(packets.map(packet => packet.sequence), ['9007199254740993']);
  assert.deepEqual(calls.find(([name]) => name === 'ackMeterPacket'),
                   ['ackMeterPacket', '9007199254740993', 7]);
  await Promise.all(Array.from({ length: 1000 }, () => transport.tick()));
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 1);
  ack.resolve(true);
  await firstTick;
  assert.equal(transport.inFlight(), false);
});

test('hidden, reopened and stale-generation browser states remain bounded', async () => {
  const visibility = deferred();
  const calls = [];
  let packet = { sequence: '42', generation: 8, rms: [0.5, 0] };
  const received = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'setMeterVisible' && args[0] === true) return visibility.promise;
    if (name === 'getMeterPacket') return Promise.resolve(packet);
    return Promise.resolve(true);
  };
  const transport = createMeterTransport(invoke, value => received.push(value));
  await transport.start(8);
  await transport.setVisible(false);
  await transport.tick();
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 0);

  const reopen = transport.setVisible(true);
  await Promise.all(Array.from({ length: 1000 }, () => transport.tick()));
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 0);
  visibility.resolve(true);
  await reopen;
  await transport.tick();
  assert.equal(received.length, 1);
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 1);

  packet = { sequence: '43', generation: 7, rms: [1, 1] };
  await transport.tick();
  assert.equal(received.length, 1);
  assert.deepEqual(calls.at(-1), ['ackMeterPacket', '43', 7]);
  await transport.setVisible(false);
  await transport.tick();
  assert.equal(calls.filter(([name]) => name === 'getMeterPacket').length, 2);
});

test('a renderer error still acknowledges the consumed packet', async () => {
  const calls = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'getMeterPacket')
      return Promise.resolve({ sequence: '57', generation: 3 });
    return Promise.resolve(true);
  };
  const transport = createMeterTransport(invoke, () => { throw new Error('render failed'); });
  await transport.start(3);
  await assert.rejects(transport.tick(), /render failed/);
  assert.deepEqual(calls.at(-1), ['ackMeterPacket', '57', 3]);
  assert.equal(transport.inFlight(), false);
});

test('identical meter displays still acknowledge every packet sequence', async () => {
  const { meterView } = await import('./meter-view.mjs');
  let display = meterView(null), sequence = 0; const acknowledgements = [], displays = [];
  const transport = createMeterTransport(async (name, ...args) => {
    if (name === 'getMeterPacket') return { sequence: String(++sequence), generation: 7,
      display_valid: true, display_peak: [0.5, 0.25] };
    if (name === 'ackMeterPacket') acknowledgements.push(args);
    return true;
  }, packet => { display = meterView(packet, display); displays.push(display); });
  await transport.start(7); await transport.tick(); await transport.tick();
  assert.equal(displays[0], displays[1]);
  assert.deepEqual(acknowledgements, [['1', 7], ['2', 7]]);
  await transport.setVisible(false);
});
