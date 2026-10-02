import assert from 'node:assert/strict';
import test from 'node:test';
import { meterView } from './meter-view.mjs';

test('stereo meter uses host display levels, clip latches and incomplete state', () => {
  const view = meterView({
    generation: 7,
    display_valid: true,
    display_complete: false,
    display_peak: [0.5, 0.25],
    display_rms: [0.4, 0.125],
    display_clipped: [false, true],
    clip_ticket: ['0', '9007199254740993'],
  });
  assert.deepEqual(view.channels, [
    { name: 'L', peak: 0.5, rms: 0.4, clipped: false, ticket: '0' },
    { name: 'R', peak: 0.25, rms: 0.125, clipped: true,
      ticket: '9007199254740993' },
  ]);
  assert.equal(view.complete, false);
  assert.equal(view.valid, true);
  assert.equal(view.generation, 7);
});

test('absent or malformed meter packets show safe empty bars', () => {
  const empty = meterView(null);
  assert.equal(empty.valid, false);
  assert.deepEqual(empty.channels.map(channel => channel.peak), [0, 0]);
  const malformed = meterView({
    display_valid: true,
    display_peak: [2, -1],
    display_rms: [Number.NaN, 0.5],
    display_clipped: [true, false],
    clip_ticket: [34, '-1'],
  });
  assert.deepEqual(malformed.channels.map(channel => [channel.peak, channel.rms]),
                   [[1, 0], [0, 0.5]]);
  assert.deepEqual(malformed.channels.map(channel => channel.ticket), ['0', '0']);
});
