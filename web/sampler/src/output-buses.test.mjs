import assert from 'node:assert/strict';
import { after, afterEach, test } from 'node:test';
import React, { act } from 'react';
import { JSDOM } from 'jsdom';
import { meterView } from '../../shared/meter-view.mjs';
import { createOutputBuses, acknowledgedOutputClip, acknowledgeOutputClip } from './output-buses.mjs';

const dom = new JSDOM('<!doctype html><html><body></body></html>');
Object.assign(globalThis, { window: dom.window, document: dom.window.document, IS_REACT_ACT_ENVIRONMENT: true });
Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
const { createRoot } = await import('react-dom/client');
const OutputBuses = createOutputBuses(React);
const buses = [
  { id: 'output:0', name: 'Main mix', channels: ['L', 'R'], main: true, meterBusId: 'master' },
  { id: 'output:1', name: 'Mono monitor', channels: ['C'], main: false, meterBusId: '' },
  { id: 'output:2', name: 'Surround room', channels: ['L', 'R', 'C', 'Lfe', 'Ls', 'Rs'], main: false, meterBusId: '' },
  { id: 'output:3', name: 'Disabled cue', channels: [], main: false, meterBusId: '' },
];
const packet = { generation: 9, display_valid: true, display_complete: true,
  display_peak: [0.75, 0.125], display_rms: [0.5, 0.0625], display_clipped: [true, true], clip_ticket: ['17', '19'] };
let mounted;
async function mount(overrides = {}) {
  document.body.innerHTML = '<div id="root"></div>';
  const root = createRoot(document.querySelector('#root'));
  const acknowledgements = [], edits = [];
  let props = { generation: 9, buses, meter: meterView(null), onChange: (...args) => edits.push(args),
    onAcknowledge: (...args) => acknowledgements.push(args), ...overrides };
  const update = async changes => {
    props = { ...props, ...changes };
    await act(async () => root.render(React.createElement(OutputBuses, props)));
  };
  mounted = { update, acknowledgements, edits, unmount: async () => { await act(async () => root.unmount()); mounted = null; } };
  await update({});
  return mounted;
}
afterEach(async () => { if (mounted) await mounted.unmount(); });
after(() => dom.window.close());
const row = id => document.querySelector(`[data-output-bus="${id}"]`);
const meters = () => [...document.querySelectorAll('[role="meter"]')];
const clips = () => [...document.querySelectorAll('[aria-label^="Acknowledge"]')];

test('actual named output layouts are inspected without fabricated routes or local edits', async () => {
  const data = structuredClone(buses), before = structuredClone(data);
  const fixture = await mount({ buses: data });
  assert.ok(row('output:0'), 'actual named output bindings must be shown');
  assert.match(row('output:0').textContent, /Main mix.*2 channels · L\/R/s);
  assert.match(row('output:1').textContent, /Mono monitor.*1 channel · C/s);
  assert.match(row('output:2').textContent, /Surround room.*6 channels · L\/R\/C\/Lfe\/Ls\/Rs/s);
  assert.match(row('output:3').textContent, /Disabled cue.*Disabled/s);
  assert.equal(document.querySelectorAll('[data-output-bus]').length, 4);
  assert.equal(document.querySelectorAll('[data-output-bus="output:2"] [data-channel-index]').length, 6);
  assert.match(document.body.textContent, /Feed details unavailable/);
  assert.doesNotMatch(document.body.textContent, /Nothing routed|15\/16|3\/4|stereo outputs/);
  assert.equal(document.querySelector('input,select,[aria-label^="Mute"],[aria-label^="Add"]'), null);
  await act(async () => row('output:2').focus());
  assert.equal(document.activeElement, row('output:2'));
  assert.deepEqual(data, before);
  assert.deepEqual(fixture.edits, []);
});

test('measured peak and RMS keep actual bus, channel and generation identities', async () => {
  await mount({ meter: meterView(packet) });
  assert.match(row('output:0').textContent, /LIVE/);
  assert.deepEqual(meters().map(el => Number(el.getAttribute('aria-valuenow'))), [0.75, 0.125]);
  assert.deepEqual(meters().map(el => el.dataset.rms), ['0.5', '0.0625']);
  assert.deepEqual(meters().map(el => el.dataset.meterBus), ['master', 'master']);
  assert.deepEqual(meters().map(el => el.dataset.generation), ['9', '9']);
  assert.deepEqual(meters().map(el => el.dataset.channelIndex), ['0', '1']);
  assert.deepEqual(meters().map(el => el.querySelector('.dd-bus-peak').style.width), ['75%', '12.5%']);
  assert.match(row('output:1').textContent, /Measurements unavailable/);
  assert.equal(row('output:1').querySelector('[role="meter"]'), null);
});

test('waiting, stale generation, partial history and measured silence are distinct', async () => {
  const fixture = await mount();
  assert.match(row('output:0').textContent, /WAITING FOR AUDIO/);
  assert.equal(meters().length, 0, 'waiting is not measured zero');
  await fixture.update({ meter: meterView({ ...packet, generation: 8 }) });
  assert.equal(meters().length, 0, 'retired generation levels must not appear on current output bindings');
  assert.ok(clips().every(el => el.disabled));
  await fixture.update({ meter: meterView({ ...packet, display_complete: false }) });
  assert.match(row('output:0').textContent, /HISTORY GAP/);
  assert.equal(meters().length, 2);
  await fixture.update({ meter: meterView({ ...packet, display_peak: [0, 0], display_rms: [0, 0], display_clipped: [false, false] }) });
  assert.match(row('output:0').textContent, /LIVE/);
  assert.deepEqual(meters().map(el => Number(el.getAttribute('aria-valuenow'))), [0, 0]);
});

test('clip acknowledgement targets only the measured output channel, generation and ticket', async () => {
  const fixture = await mount({ meter: meterView(packet) });
  const buttons = row('output:0').querySelectorAll('button');
  await act(async () => buttons[0].click());
  assert.deepEqual(fixture.acknowledgements, [['output:0', 0, 9, '17']]);
  const cleared = acknowledgedOutputClip(meterView(packet), 0, 9, '17');
  await fixture.update({ meter: cleared });
  assert.equal(row('output:0').querySelectorAll('button')[0].disabled, true);
  assert.equal(row('output:0').querySelectorAll('button')[1].disabled, false, 'other channel remains latched');
  assert.deepEqual(meters().map(el => Number(el.getAttribute('aria-valuenow'))), [0.75, 0.125]);
});

test('an old acknowledgement cannot clear a newer clip or generation', () => {
  const old = meterView(packet), newerClip = meterView({ ...packet, clip_ticket: ['21', '19'] });
  const newerGeneration = meterView({ ...packet, generation: 10 });
  assert.equal(acknowledgedOutputClip(newerClip, 0, 9, '17'), newerClip);
  assert.equal(acknowledgedOutputClip(newerGeneration, 0, 9, '17'), newerGeneration);
  assert.equal(acknowledgedOutputClip(old, 9, 9, '17'), old);
  assert.equal(acknowledgedOutputClip(old, 0, 9, '0'), old);
});

test('native clip replies preserve newer readings and rejected acknowledgements', async () => {
  let display = meterView(packet);
  const requests = [];
  let reply;
  const invoke = (...args) => {
    requests.push(args);
    return new Promise(resolve => { reply = resolve; });
  };
  const update = change => { display = change(display); };
  const pending = acknowledgeOutputClip(invoke, update, 0, 9, '17');
  assert.deepEqual(requests, [['ackMeterClip', 0, 9, '17']], 'clip requests preserve the observed native identity');
  display = meterView({ ...packet, clip_ticket: ['21', '19'] });
  reply(true);
  await pending;
  assert.equal(display.channels[0].clipped, true, 'late native success cannot clear a newer clip');
  await assert.rejects(acknowledgeOutputClip(async () => false, update, 0, 9, '21'),
    /acknowledgement was rejected/, 'native rejection must not clear the current clip');
  assert.equal(display.channels[0].clipped, true);
  await assert.rejects(acknowledgeOutputClip(async () => { throw Error('Host unavailable'); }, update, 0, 9, '21'),
    /Host unavailable/);
  assert.equal(display.channels[0].clipped, true);
  await acknowledgeOutputClip(async () => true, update, 0, 9, '21');
  assert.deepEqual(display.channels.map(channel => channel.clipped), [false, true]);
  assert.deepEqual(display.channels.map(channel => channel.peak), [0.75, 0.125]);
});

test('missing acknowledgement capability and mismatched channel layouts stay unavailable', async () => {
  const fixture = await mount({ meter: meterView(packet), onAcknowledge: undefined });
  assert.ok(clips().every(el => el.disabled));
  await fixture.update({ onAcknowledge: (...args) => fixture.acknowledgements.push(args),
    meter: meterView({ ...packet, clip_ticket: ['0', '19'] }) });
  assert.equal(clips()[0].disabled, true, 'a missing clip ticket cannot authorize acknowledgement');
  assert.equal(clips()[1].disabled, false);
  await act(async () => clips()[0].click());
  assert.deepEqual(fixture.acknowledgements, []);
  await fixture.update({ buses: [{ ...buses[2], meterBusId: 'master' }] });
  assert.equal(meters().length, 0, 'a six-channel output cannot borrow the stereo master tap');
  assert.match(document.body.textContent, /Measurements unavailable/);
});

test('missing output metadata does not invent a default stereo binding', async () => {
  await mount({ buses: [] });
  assert.equal(document.querySelector('[data-output-bus]'), null);
  assert.match(document.body.textContent, /Output bindings unavailable/);
  assert.equal(meters().length, 0);
});
