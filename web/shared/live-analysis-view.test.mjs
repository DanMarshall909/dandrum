import test from 'node:test';
import assert from 'node:assert/strict';
import { liveAnalysisView, paintLiveAnalysis } from './live-analysis-view.mjs';

const origin = 9007199254740993n;
function packet() {
  return { generation: 7, sequence: '12', bus: 'master', sampleRateHz: 96000,
    streamId: '4', selectionId: '5', captureSequence: '6', channelMask: 3, gap: true,
    startFrame: String(origin), endFrame: String(origin + 1024n),
    settings: { window: 'periodicHann', scaling: 'oneSidedPeakDbFS',
      channelPolicy: 'selectedChannel', fftSize: 1024, hopFrames: '256', floorDbFS: -120 },
    frequencyHz: Array.from({ length: 513 }, (_, i) => i * 93.75),
    channels: [0, 1].map(channel => ({ channel,
      scope: Array.from({ length: 128 }, (_, i) => ({
        startFrame: String(origin + BigInt(i * 8)), endFrame: String(origin + BigInt((i + 1) * 8)),
        minimum: channel ? -0.5 : 0.25, maximum: channel ? -0.25 : 0.5 })),
      magnitudeDbFS: Array.from({ length: 513 }, (_, i) => i === 64 ? -6.020599913 : -120) })) };
}

test('live views preserve exact stream frames, selected signed channels and host-rate frequency', () => {
  const input = packet();
  const left = liveAnalysisView(input, 0, 900, 160, 'scope');
  assert.ok(left, 'complete current live measurement must produce a view');
  assert.equal(left.startFrame, String(origin)); assert.equal(left.endFrame, String(origin + 1024n));
  assert.equal(left.sampleRateHz, 96000); assert.equal(left.gap, true);
  assert.equal(left.durationMs, 1024 / 96); assert.equal(left.channel, 0);
  assert.deepEqual(left.buckets[0], { x: 900 / 256, top: 40, bottom: 60 });
  assert.deepEqual(left.buckets[127], { x: 900 * 255 / 256, top: 40, bottom: 60 });
  const right = liveAnalysisView(input, 1, 900, 160, 'scope');
  assert.deepEqual(right.buckets[0], { x: 900 / 256, top: 100, bottom: 120 });
  const spectrum = liveAnalysisView(input, 0, 900, 160, 'spectrum');
  assert.equal(spectrum.minimumHz, 93.75); assert.equal(spectrum.maximumHz, 48000);
  assert.equal(spectrum.points.length, 512); assert.equal(spectrum.points[63].frequencyHz, 6000);
  assert.equal(spectrum.points[63].x, 600);
  assert.ok(Math.abs(spectrum.points[63].y - 8.02746655) < 1e-7);
  assert.equal(spectrum.points[0].x, 0); assert.equal(spectrum.points.at(-1).x, 900);
  input.channels[0].scope[0].maximum = 9;
  assert.equal(left.buckets[0].top, 40, 'accepted view owns its derived geometry');
  const over = liveAnalysisView(input, 0, 900, 160, 'scope');
  assert.equal(over.buckets[0].top, 0, 'only display coordinates clamp finite overrange audio');
  input.channels[0].magnitudeDbFS[64] = 4;
  assert.equal(liveAnalysisView(input, 0, 900, 160, 'spectrum').points[63].y, 0);
  input.channelMask = 2; input.channels.shift();
  assert.ok(liveAnalysisView(input, 1, 900, 160, 'scope'));
  assert.equal(liveAnalysisView(input, 0, 900, 160, 'scope'), null);
});

test('live views reject incoherent or unbounded measurements before painting', () => {
  for (const mutate of [
    p => { p.bus = 'imaginary'; }, p => { p.generation = 0; }, p => { p.generation = 2 ** 32; },
    p => { p.sequence = '0'; }, p => { p.streamId = '18446744073709551616'; },
    p => { p.selectionId = 1; }, p => { p.captureSequence = '00'; },
    p => { p.startFrame = 'x'; }, p => { p.endFrame = p.startFrame; },
    p => { p.sampleRateHz = 0; }, p => { p.sampleRateHz = 48000.5; },
    p => { p.channelMask = 0; }, p => { p.channelMask = 4; }, p => { p.gap = 'false'; },
    p => { p.settings = null; }, p => { p.settings.window = 'rect'; },
    p => { p.settings.scaling = 'power'; }, p => { p.settings.channelPolicy = 'sum'; },
    p => { p.settings.fftSize = 2048; }, p => { p.settings.hopFrames = '512'; },
    p => { p.settings.floorDbFS = -90; }, p => { p.frequencyHz.pop(); },
    p => { p.frequencyHz[64] = 3000; }, p => { p.frequencyHz[3] = NaN; },
    p => { p.channels.pop(); }, p => { p.channels[1].channel = 0; },
    p => { p.channels[1].scope.pop(); }, p => { p.channels[1].scope[3].startFrame = '0'; },
    p => { p.channels[1].scope[3].endFrame = '0'; },
    p => { p.channels[0].scope[3].startFrame = String(origin + 23n); },
    p => { p.channels[0].scope[3].endFrame = String(origin + 31n); },
    p => { p.channels[1].scope[3].minimum = 2; }, p => { p.channels[1].scope[3].maximum = Infinity; },
    p => { p.channels[1].magnitudeDbFS.pop(); }, p => { p.channels[1].magnitudeDbFS[64] = NaN; },
    p => { delete p.channels; }, p => { delete p.frequencyHz; },
    p => { p.channels[0] = null; }, p => { p.channels[0].scope = null; },
    p => { p.channels[0].scope[0] = null; }, p => { p.channels[0].magnitudeDbFS = null; },
  ]) {
    const input = packet(); mutate(input);
    assert.equal(liveAnalysisView(input, 0, 900, 160, 'scope'), null, String(mutate));
  }
  assert.equal(liveAnalysisView(null, 0, 900, 160, 'scope'), null);
  for (const args of [[0, 0, 160, 'scope'], [0, 900, NaN, 'scope'],
    [2, 900, 160, 'scope'], [0.5, 900, 160, 'scope'], [0, 900, 160, 'fake']])
    assert.equal(liveAnalysisView(packet(), ...args), null);
});

test('live painters draw signed scope and log spectrum with labelled levels', () => {
  const operations = [];
  const context = { fillStyle: '', strokeStyle: '', lineWidth: 0,
    fillRect(...args) { operations.push([this.fillStyle, ...args]); },
    beginPath() { operations.push(['begin']); }, moveTo(...args) { operations.push(['move', ...args]); },
    lineTo(...args) { operations.push(['line', ...args]); }, stroke() { operations.push(['stroke', this.strokeStyle]); } };
  paintLiveAnalysis(null, {}); paintLiveAnalysis(context, null); assert.equal(operations.length, 0);
  paintLiveAnalysis(context, liveAnalysisView(packet(), 1, 900, 160, 'scope'));
  assert.deepEqual(operations[0], ['#161616', 0, 0, 900, 160]);
  assert.deepEqual(operations[1], ['#444444', 0, 80, 900, 1]);
  assert.deepEqual(operations[2], ['#f6aa69', 4, 100, 1, 21]);
  assert.equal(operations.length, 130);
  operations.length = 0;
  paintLiveAnalysis(context, liveAnalysisView(packet(), 0, 900, 160, 'spectrum'));
  assert.deepEqual(operations[1], ['#444444', 0, 80, 900, 1]);
  assert.deepEqual(operations[3], ['move', 0, 160]);
  assert.equal(operations[66][0], 'line'); assert.equal(operations[66][1], 600);
  assert.deepEqual(operations.at(-1), ['stroke', '#f6aa69']);
});
