import assert from 'node:assert/strict';
import test from 'node:test';
import { preparedSpectrumView, paintPreparedSpectrum, spectralColour } from './prepared-spectrum.mjs';

const start = 9007199254740993n;
const frame = offset => String(start + BigInt(offset));
const source = { id: 'sine', sampleRateHz: 48000, channelCount: 2, frameCount: frame(4096),
  slices: [{ id: 'hit', startFrame: frame(1024), endFrame: frame(2048) }],
  regions: [{ id: 'body', startFrame: frame(0), endFrame: frame(4096), fadeInMs: 2,
    fadeOutMs: 4, loop: { startFrame: frame(512), endFrame: frame(3072) } }] };
const document = { generation: 7, sources: [source] };
const magnitude = Array(513).fill(-120); magnitude[0] = 0;
magnitude[64] = -6.020599913; magnitude[63] = -12.041199826;
const result = { sourceId: 'sine', regionId: 'body', sampleRateHz: 48000, channel: 0,
  startFrame: frame(0), endFrame: frame(4096),
  settings: { fftSize: 1024, hopFrames: '256', window: 'periodicHann',
    scaling: 'oneSidedPeakDbFS', channelPolicy: 'selectedChannel', floorDbFS: -120 },
  frequencyHz: Array.from({ length: 513 }, (_, i) => i * 46.875),
  columns: Array.from({ length: 16 }, (_, i) => ({ startFrame: frame(i * 256),
    endFrame: frame(Math.min(4096, i * 256 + 1024)), magnitudeDbFS: [...magnitude] })) };
const snapshot = { state: 'ready', generation: 7, result };
const ramp = ['#130F0C', '#41362C', '#B0662F', '#E08A4E', '#F2E6D3'];

test('prepared spectrum uses log-frequency and exact source-frame columns at full and compact widths', () => {
  for (const width of [1200, 820]) {
    const view = preparedSpectrumView(document, 'sine', 'body', snapshot, width, 180);
    assert.ok(view, 'valid prepared spectrum unavailable');
    assert.equal(view.sampleRateHz, 48000);
    assert.equal(view.durationSeconds, 4096 / 48000);
    assert.equal(view.minimumHz, 46.875); assert.equal(view.maximumHz, 24000);
    assert.equal(view.columns[1].left, width / 16);
    assert.equal(view.columns[1].right, width / 8);
    assert.equal(view.columns[15].right, width);
    assert.ok(Math.abs(view.columns[0].rows[60] + 6.020599913) < 0.00001);
    assert.equal(view.columns[0].rows[179], -120, 'DC must not paint at 47 Hz');
    assert.deepEqual(view.markers.map(m => m.x), [0, width, width*96/4096,
      width*3904/4096, width/8, width*3/4, width/4, width/2]);
  }
});

test('prepared spectral palette and Canvas use declared finite dBFS with prepared overlays', () => {
  assert.equal(spectralColour(-120, -120, ramp), 'rgb(19,15,12)');
  assert.equal(spectralColour(0, -120, ramp), 'rgb(242,230,211)');
  assert.equal(spectralColour(-30, -120, ramp), 'rgb(224,138,78)');
  assert.equal(spectralColour(-200, -120, ramp), 'rgb(19,15,12)');
  assert.equal(spectralColour(6, -120, ramp), 'rgb(242,230,211)');
  const view = preparedSpectrumView(document, 'sine', 'body', snapshot, 820, 180);
  const calls = [];
  const context = { fillStyle: '', fillRect(...rect) { calls.push([this.fillStyle, ...rect]); } };
  paintPreparedSpectrum(context, view, ramp);
  assert.ok(calls.some(c => c[0] === 'rgb(238,212,184)' && c[1] === 0 && c[2] === 60), 'known 0.5 sine row not painted with absolute colour');
  assert.ok(calls.some(c => c[0] === '#e2bf72' && c[1] === 103 && c[4] === 180), 'loop overlay missing');
  assert.ok(calls.some(c => c[0] === '#ab9ee9' && c[1] === 205), 'slice overlay missing');
  assert.ok(calls.some(c => c[0] === '#8da79a' && c[1] === 819), 'end marker not clipped to last pixel');
});

test('spectral view rejects obsolete and incoherent numeric data', () => {
  const view = (next, doc = document, width = 820, height = 180) => preparedSpectrumView(doc, 'sine', 'body', next, width, height);
  assert.equal(view({ ...snapshot, generation: 6 }), null);
  assert.equal(view({ ...snapshot, state: 'running' }), null);
  assert.equal(view(snapshot, document, 0), null);
  for (const patch of [{ sampleRateHz: 44100 }, { channel: 2 }, { regionId: 'missing' },
    { endFrame: frame(4095) }, { columns: [] }, { frequencyHz: [] },
    { settings: { ...result.settings, fftSize: 2048 } },
    { settings: { ...result.settings, floorDbFS: NaN } },
    { settings: { ...result.settings, floorDbFS: 0 } },
    { settings: { ...result.settings, window: 'unknown' } },
    { settings: { ...result.settings, hopFrames: '0' } },
    { columns: [{ ...result.columns[0], startFrame: 'invalid' }] },
    { columns: [{ ...result.columns[0], magnitudeDbFS: [NaN] }] },
    { columns: [result.columns[1], result.columns[0]] }])
    assert.equal(view({ ...snapshot, result: { ...result, ...patch } }), null, JSON.stringify(patch));
  assert.equal(view(snapshot, { ...document, sources: [{ ...source, regions: [{ ...source.regions[0], fadeInMs: -1 }] }] }), null);
  paintPreparedSpectrum(null, null, ramp);
});
