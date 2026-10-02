import assert from 'node:assert/strict';
import test from 'node:test';
import { preparedWaveformView, paintPreparedWaveform } from './prepared-waveform.mjs';

const first = 9007199254740993n;
const frame = offset => String(first + BigInt(offset));
const source = {
  id: 'drums', sampleRateHz: 48000, channelCount: 2, frameCount: frame(96000),
  regions: [{ id: 'kick', startFrame: frame(0), endFrame: frame(48000),
              fadeInMs: 250, fadeOutMs: 125,
              loop: { startFrame: frame(12000), endFrame: frame(36000) } }],
  slices: [{ id: 'beat', startFrame: frame(24000), endFrame: frame(30000) }],
};
const document = { generation: 7, sources: [source] };
const snapshot = {
  generation: 7, state: 'ready',
  result: { sourceId: 'drums', regionId: 'kick', sampleRateHz: 48000, channel: 0,
    startFrame: frame(0), endFrame: frame(48000),
    buckets: [
      { startFrame: frame(0), endFrame: frame(12000), minimum: -0.75, maximum: 0.5 },
      { startFrame: frame(12000), endFrame: frame(24000), minimum: 0, maximum: 0 },
    ] },
};

test('prepared Canvas view uses exact source-frame offsets, signed extrema and source rate at both sizes', () => {
  for (const width of [1200, 820]) {
    const view = preparedWaveformView(document, 'drums', 'kick', snapshot, width, 200);
    assert.ok(view);
    assert.equal(view.durationSeconds, 1);
    assert.equal(view.sampleRateHz, 48000);
    assert.deepEqual(view.buckets[0], {
      x: width / 8, top: 50, bottom: 175,
    });
    assert.deepEqual(view.markers.map(marker => [marker.kind, marker.x]), [
      ['regionStart', 0], ['regionEnd', width], ['fadeInEnd', width / 4],
      ['fadeOutStart', width * 7 / 8], ['loopStart', width / 4],
      ['loopEnd', width * 3 / 4], ['sliceStart', width / 2],
      ['sliceEnd', width * 5 / 8],
    ]);
  }
});

test('stale, malformed and mismatched results cannot paint an old instrument', () => {
  assert.equal(preparedWaveformView(document, 'drums', 'kick',
    { ...snapshot, generation: 6 }, 820, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'kick',
    { ...snapshot, state: 'running' }, 820, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'kick',
    { ...snapshot, result: { ...snapshot.result, sourceId: 'other' } }, 820, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'kick',
    { ...snapshot, result: { ...snapshot.result, channel: 2 } }, 820, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'missing', snapshot, 820, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'kick', snapshot, 0, 200), null);
  assert.equal(preparedWaveformView(document, 'drums', 'kick',
    { ...snapshot, result: { ...snapshot.result, buckets: [
      { startFrame: frame(0), endFrame: frame(12000), minimum: 0.5, maximum: -0.5 },
    ] } }, 820, 200), null);
});

test('Canvas can inspect either prepared source channel', () => {
  const right = { ...snapshot, result: { ...snapshot.result, channel: 1 } };
  assert.deepEqual(preparedWaveformView(document, 'drums', 'kick', right, 820, 200)?.buckets,
    preparedWaveformView(document, 'drums', 'kick', snapshot, 820, 200)?.buckets);
});

test('Canvas paints numeric envelope and prepared marker columns', () => {
  const view = preparedWaveformView(document, 'drums', 'kick', snapshot, 820, 200);
  const calls = [];
  const context = {
    fillStyle: '',
    fillRect(x, y, width, height) { calls.push([this.fillStyle, x, y, width, height]); },
  };
  paintPreparedWaveform(context, view);
  assert.ok(calls.some(call => call[0] === '#7ce0aa' && call[1] === 103
    && call[2] === 50 && call[4] === 126));
  assert.ok(calls.some(call => call[0] === '#e2bf72' && call[1] === 205));
  assert.ok(calls.some(call => call[0] === '#ab9ee9' && call[1] === 410));
  assert.ok(calls.some(call => call[0] === '#8da79a' && call[1] === 819));
});
