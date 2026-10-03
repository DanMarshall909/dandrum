import assert from 'node:assert/strict';
import test from 'node:test';
import { createLiveAnalysisTransport } from './live-analysis-transport.mjs';

const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};

test('live subscription bounds requests through a stalled packet and acknowledgement', async () => {
  const packet = deferred(), acknowledgement = deferred();
  const calls = [], received = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'getLiveAnalysisPacket') return packet.promise;
    if (name === 'ackLiveAnalysisPacket') return acknowledgement.promise;
    return Promise.resolve(true);
  };
  const transport = createLiveAnalysisTransport(invoke, value => received.push(value));
  const starts = Array.from({ length: 100 }, () => transport.start(7, 3));
  const wrongGeneration = transport.start(8, 3), wrongMask = transport.start(7, 1);
  assert.equal(await wrongGeneration, false);
  assert.equal(await wrongMask, false);
  assert.ok((await Promise.all(starts)).every(Boolean));
  assert.equal(await transport.start(7, 3), true);
  assert.deepEqual(calls, [['subscribeLiveAnalysis', 7, 3]]);
  const first = transport.tick();
  const stalledRequests = Array.from({ length: 1000 }, () => transport.tick());
  assert.equal(calls.filter(([name]) => name === 'getLiveAnalysisPacket').length, 1);
  await Promise.all(stalledRequests);
  assert.equal(transport.inFlight(), true);
  packet.resolve({ generation: 7, sequence: '9007199254740993' });
  await Promise.resolve();
  assert.deepEqual(received, [{ generation: 7, sequence: '9007199254740993' }]);
  assert.deepEqual(calls.at(-1), ['ackLiveAnalysisPacket', '9007199254740993', 7]);
  const stalledAcks = Array.from({ length: 1000 }, () => transport.tick());
  assert.equal(calls.filter(([name]) => name === 'getLiveAnalysisPacket').length, 1);
  await Promise.all(stalledAcks);
  acknowledgement.resolve(true);
  assert.equal(await first, true);
  assert.equal(transport.inFlight(), false);
  assert.equal(await transport.close(), true);
  assert.equal(await transport.close(), true);
  assert.equal(calls.filter(([name]) => name === 'unsubscribeLiveAnalysis').length, 1);
  assert.equal(await transport.start(7, 3), false);
  assert.equal(await transport.tick(), false);
});

test('hidden startup and serialized visibility keep a single newest demand', async () => {
  const hidden = deferred(), calls = [], received = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'setLiveAnalysisVisible' && !args[0]) return hidden.promise;
    if (name === 'getLiveAnalysisPacket') return Promise.resolve({ generation: 8, sequence: '42' });
    return Promise.resolve(true);
  };
  const transport = createLiveAnalysisTransport(invoke, value => received.push(value));
  await transport.setVisible(false);
  const starting = transport.start(8, 1);
  await Promise.resolve();
  const reopening = transport.setVisible(true);
  await Promise.all(Array.from({ length: 1000 }, () => transport.tick()));
  assert.deepEqual(calls, [['subscribeLiveAnalysis', 8, 1], ['setLiveAnalysisVisible', false, 8]]);
  hidden.resolve(true);
  assert.equal(await starting, true);
  await reopening;
  assert.deepEqual(calls.at(-1), ['setLiveAnalysisVisible', true, 8]);
  assert.equal(await transport.tick(), true);
  assert.equal(received.length, 1);
  await transport.setVisible(false);
  assert.equal(await transport.tick(), false);
  await transport.close();
});

test('a packet requested before hide/show cannot paint after the demand changes', async () => {
  const delayed = deferred(), received = [], calls = [];
  let gets = 0;
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'getLiveAnalysisPacket') return ++gets === 1 ? delayed.promise
      : Promise.resolve({ generation: 4, sequence: '23' });
    return Promise.resolve(true);
  };
  const transport = createLiveAnalysisTransport(invoke, value => received.push(value));
  await transport.start(4, 3);
  const pending = transport.tick();
  await transport.setVisible(false);
  await transport.setVisible(true);
  delayed.resolve({ generation: 4, sequence: '22' });
  await pending;
  assert.deepEqual(received, []);
  assert.deepEqual(calls.at(-1), ['ackLiveAnalysisPacket', '22', 4]);
  await transport.tick();
  assert.deepEqual(received, [{ generation: 4, sequence: '23' }]);
  await transport.close();
});

test('closing releases admitted demand without waiting for a subscription or packet reply', async () => {
  const subscription = deferred(), packet = deferred(), received = [], calls = [];
  let admitted = false;
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'subscribeLiveAnalysis') { admitted = true; return subscription.promise; }
    if (name === 'unsubscribeLiveAnalysis') { admitted = false; return Promise.resolve(true); }
    if (name === 'getLiveAnalysisPacket') return packet.promise;
    return Promise.resolve(true);
  };
  const transport = createLiveAnalysisTransport(invoke, value => received.push(value));
  const starting = transport.start(9, 2);
  assert.equal(admitted, true);
  assert.equal(await transport.close(), true);
  assert.equal(admitted, false);
  subscription.resolve(true);
  assert.equal(await starting, false);
  assert.equal(await transport.tick(), false);
  assert.deepEqual(received, []);
  const other = createLiveAnalysisTransport(invoke, value => received.push(value));
  await other.start(9, 3);
  const pending = other.tick();
  await other.close();
  packet.resolve({ generation: 9, sequence: '31' });
  assert.equal(await pending, false);
  assert.deepEqual(received, []);
  assert.equal(calls.filter(([name]) => name === 'ackLiveAnalysisPacket').length, 0);
});

test('stale/duplicate packets and renderer failures cannot strand acknowledgement', async () => {
  const calls = [], received = [];
  let next = { generation: 3, sequence: '7' };
  let acknowledged = true;
  const invoke = async (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'ackLiveAnalysisPacket') return acknowledged;
    return name === 'getLiveAnalysisPacket' ? next : true;
  };
  const transport = createLiveAnalysisTransport(invoke, value => received.push(value));
  await transport.start(3, 3);
  await transport.tick();
  await transport.tick();
  next = { generation: 2, sequence: '8' };
  acknowledged = false;
  assert.equal(await transport.tick(), false);
  assert.deepEqual(received, [{ generation: 3, sequence: '7' }]);
  assert.deepEqual(calls.at(-1), ['ackLiveAnalysisPacket', '8', 2]);
  next = { generation: 3, sequence: '8' }; acknowledged = true;
  assert.equal(await transport.tick(), true);
  assert.deepEqual(received.map(value => value.sequence), ['7', '8']);
  await transport.close();
  const throwing = createLiveAnalysisTransport(invoke, () => { throw new Error('live paint failed'); });
  next = { generation: 3, sequence: '9' };
  await throwing.start(3, 3);
  await assert.rejects(throwing.tick(), /live paint failed/);
  assert.deepEqual(calls.at(-1), ['ackLiveAnalysisPacket', '9', 3]);
  assert.equal(throwing.inFlight(), false);
  await throwing.close();
});

test('closure also bypasses a pending visibility reply and stops its reconciliation', async () => {
  const visibility = deferred(), calls = [];
  const invoke = (name, ...args) => {
    calls.push([name, ...args]);
    return name === 'setLiveAnalysisVisible' ? visibility.promise : Promise.resolve(true);
  };
  const transport = createLiveAnalysisTransport(invoke, () => {});
  await transport.setVisible(false);
  const starting = transport.start(6);
  await Promise.resolve();
  const reopening = transport.setVisible(true);
  assert.equal(await transport.close(), true);
  assert.deepEqual(calls.at(-1), ['unsubscribeLiveAnalysis', 6]);
  visibility.resolve(true);
  assert.equal(await starting, false);
  await reopening;
  await transport.setVisible(false);
  assert.equal(calls.filter(([name]) => name === 'setLiveAnalysisVisible').length, 1);
  assert.equal(await transport.tick(), false);
});

test('invalid admission and packet identities fail explicitly without extra calls', async () => {
  const calls = [];
  let accepted = false, next = null, visibility = true;
  const invoke = async (name, ...args) => {
    calls.push([name, ...args]);
    if (name === 'subscribeLiveAnalysis') return accepted;
    if (name === 'getLiveAnalysisPacket') return next;
    if (name === 'setLiveAnalysisVisible') return visibility;
    return true;
  };
  const transport = createLiveAnalysisTransport(invoke, () => {});
  for (const generation of [0, -1, 1.5, '1', NaN, Infinity, 4294967296])
    assert.equal(await transport.start(generation, 3), false);
  for (const mask of [0, 4, 1.5, '3', NaN]) assert.equal(await transport.start(1, mask), false);
  assert.deepEqual(calls, []);
  assert.equal(await transport.tick(), false);
  assert.equal(await transport.start(1, 3), false);
  accepted = true;
  assert.equal(await transport.start(1, 3), true);
  assert.equal(await transport.start(2, 3), false);
  assert.equal(await transport.start(1, 1), false);
  assert.equal(await transport.tick(), false);
  for (const packet of [{ generation: 1, sequence: 1 }, { generation: 1, sequence: '0' },
      { generation: 1, sequence: '18446744073709551616' }, { generation: 1, sequence: '100000000000000000000' },
      { generation: 1, sequence: '01' }, { generation: 1, sequence: '-1' },
      { generation: 1, sequence: '1.5' }, { generation: 0, sequence: '2' },
      { generation: '1', sequence: '2' }, { generation: 4294967296, sequence: '2' }]) {
    next = packet;
    await assert.rejects(transport.tick(), /Invalid live analysis packet identity/);
    assert.equal(transport.inFlight(), false);
  }
  visibility = false;
  await assert.rejects(transport.setVisible(false), /Live analysis visibility was rejected/);
  await transport.close();
  const untouched = createLiveAnalysisTransport(invoke, () => {});
  const before = calls.length;
  assert.equal(await untouched.close(), false);
  assert.equal(calls.length, before);
});
