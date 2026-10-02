import assert from 'node:assert/strict';
import test from 'node:test';
import { admittedParameter, actualValue, formatActualValue, parseActualValue } from './parameter-value.mjs';
import { dragValue, keyValue, wheelValue } from './parameter-value.mjs';

const descriptor = { id: 'fixture.level', name: 'Level', minValue: -1, maxValue: 1,
  normalisedDefaultValue: 0.2 };
const document = { generation: 4, parameters: [descriptor] };
const state = { generation: 4, parameters: [{ id: 'fixture.level', value: 0.75 }] };

test('a typed actual value uses the admitted instrument range while commands stay normalized', () => {
  const parameter = admittedParameter(state, document, 'fixture.level');
  assert.ok(parameter, 'matching prepared and live controls must become available');
  assert.equal(parameter.generation, 4);
  assert.equal(parameter.value, 0.75);
  assert.equal(actualValue(parameter.value, parameter), 0.5);
  assert.equal(formatActualValue(parameter.value, parameter), '0.5');
  assert.equal(parseActualValue(' −0.6 ', parameter), 0.2);
  assert.equal(parseActualValue('1', parameter), 1);
  assert.equal(parseActualValue('-1', parameter), 0);
  assert.equal(parameter.normalisedDefaultValue, 0.2);
});

test('obsolete or missing metadata cannot lend its range or default to a newer control', () => {
  for (const [live, prepared, id] of [
    [null, document, 'fixture.level'], [state, null, 'fixture.level'],
    [{ ...state, generation: 5 }, document, 'fixture.level'],
    [state, { ...document, generation: 5 }, 'fixture.level'],
    [{ ...state, generation: NaN }, { ...document, generation: NaN }, 'fixture.level'],
    [state, document, 'missing'], [{ ...state, parameters: [] }, document, 'fixture.level'],
    [state, { ...document, parameters: [] }, 'fixture.level'],
    [{ ...state, parameters: [{ id: 'fixture.level', value: Infinity }] }, document, 'fixture.level'],
  ]) assert.equal(admittedParameter(live, prepared, id), null);
  const changed = admittedParameter({ ...state, generation: 5 }, {
    generation: 5, parameters: [{ ...descriptor, minValue: 10, maxValue: 20,
      normalisedDefaultValue: 0.8 }],
  }, 'fixture.level');
  assert.equal(actualValue(changed.value, changed), 17.5);
  assert.equal(changed.normalisedDefaultValue, 0.8);
});

test('actual entry accepts complete decimal numbers and rejects malformed or out-of-range text', () => {
  assert.equal(parseActualValue('+.5', descriptor), 0.75);
  assert.equal(parseActualValue('5e-1', descriptor), 0.75);
  for (const text of ['', ' ', '.5 Hz', 'NaN', 'Infinity', '1e999', '0x1', '--1', '2', '-2', null])
    assert.equal(parseActualValue(text, descriptor), null, String(text));
  for (const range of [null, { minValue: 1, maxValue: 1 }, { minValue: 2, maxValue: 1 },
    { minValue: -Infinity, maxValue: 1 }, { minValue: 0, maxValue: NaN }]) {
    assert.equal(actualValue(0.5, range), null);
    assert.equal(formatActualValue(0.5, range), '');
    assert.equal(parseActualValue('.5', range), null);
  }
  for (const value of [NaN, Infinity, -0.1, 1.1]) {
    assert.equal(actualValue(value, descriptor), null);
    assert.equal(admittedParameter({ ...state, parameters: [{ id: 'fixture.level', value }] },
      document, 'fixture.level'), null);
  }
  assert.equal(formatActualValue(1 / 9, { minValue: 0.5, maxValue: 5 }), '1');
  assert.equal(formatActualValue(0.5, { minValue: 0.02, maxValue: 0.9 }), '0.46');
});

test('knob inputs follow the supplied vertical drag, fine steps, endpoints and loaded reset', () => {
  assert.equal(dragValue(0.5, 100, 50, false), 0.75);
  assert.equal(dragValue(0.5, 100, 50, true), 0.5625);
  assert.equal(dragValue(0.5, 100, -200, false), 1);
  assert.equal(dragValue(0.5, 100, 400, false), 0);
  for (const key of ['ArrowUp', 'ArrowRight']) {
    assert.equal(keyValue(0.5, key, false, 0.2), 0.55);
    assert.equal(keyValue(0.5, key, true, 0.2), 0.51);
  }
  for (const key of ['ArrowDown', 'ArrowLeft']) {
    assert.equal(keyValue(0.5, key, false, 0.2), 0.45);
    assert.equal(keyValue(0.5, key, true, 0.2), 0.49);
  }
  for (const key of ['Backspace', 'Delete']) {
    assert.equal(keyValue(0.8, key, false, 0.2), 0.2);
    for (const unavailable of [undefined, NaN, -1, 2])
      assert.equal(keyValue(0.8, key, false, unavailable), null);
  }
  assert.equal(keyValue(0.5, 'Home', false, 0.2), 0);
  assert.equal(keyValue(0.5, 'End', false, 0.2), 1);
  assert.equal(keyValue(0.99, 'ArrowUp', false, 0.2), 1);
  assert.equal(keyValue(0.01, 'ArrowDown', false, 0.2), 0);
  assert.equal(keyValue(0.5, 'Tab', false, 0.2), null);
  assert.equal(wheelValue(0.5, -100, false), 0.52);
  assert.equal(wheelValue(0.5, 100, false), 0.48);
  assert.equal(wheelValue(0.5, -100, true), 0.51);
  assert.equal(wheelValue(0.5, 100, true), 0.49);
  assert.equal(wheelValue(0.99, -1, false), 1);
  assert.equal(wheelValue(0.01, 1, false), 0);
  assert.equal(wheelValue(0.5, 0, false), null);
});
