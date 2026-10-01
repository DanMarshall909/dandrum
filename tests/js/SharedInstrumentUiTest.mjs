import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const header = fs.readFileSync(new URL('../../src/juce-plugin/SharedInstrumentUi.h', import.meta.url), 'utf8');
const script = header.match(/R"JS\(([\s\S]*?)\)JS"/)?.[1];
assert.ok(script, 'shared UI asset was not embedded in the resource header');

class Element {
  constructor(tag = 'div') {
    this.tag = tag;
    this.children = [];
    this.style = {};
    this.dataset = {};
    this.textContent = '';
    this.classes = new Set();
    this.classList = {
      add: name => this.classes.add(name),
      remove: name => this.classes.delete(name),
      contains: name => this.classes.has(name),
    };
  }
  set className(value) { this.classes = new Set(value.split(/\s+/).filter(Boolean)); }
  get className() { return [...this.classes].join(' '); }
  appendChild(child) { this.children.push(child); return child; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = [...children]; }
  setPointerCapture() {}
}

const elements = Object.fromEntries(['controls', 'keys', 'error'].map(id => [id, new Element()]));
const document = {
  getElementById: id => elements[id],
  createElement: tag => new Element(tag),
};
const events = {};
const calls = [];
let setResult;
let commandSequence = 0;
let currentGeneration = 1;
const backend = {
  getNativeFunction: name => (...arguments_) => {
    calls.push([name, ...arguments_]);
    if (name === 'getParameterState')
      return Promise.resolve({generation: currentGeneration, sequence: commandSequence,
        parameters: [{id: 'kick.tune_hz', name: 'Tune', value: 0.5},
                     {id: 'kick.decay_ms', name: 'Decay', value: 0.25}]});
    if (name === 'setParameter' && setResult)
      return Promise.resolve(setResult);
    return Promise.resolve({status: 'accepted', generation: currentGeneration,
                            sequence: ++commandSequence});
  },
  addEventListener: (name, callback) => { events[name] = callback; },
};
const window = {__JUCE__: {backend}};
vm.runInNewContext(script, {document, window});
await new Promise(resolve => setImmediate(resolve));

assert.equal(elements.controls.children.length, 2);
assert.equal(calls[0][0], 'getParameterState');
assert.equal(elements.controls.children[0].children[1].textContent, 'Tune');
assert.equal(elements.controls.children[1].children[1].textContent, 'Decay');
assert.equal(elements.keys.children.length, 19);

const firstKnob = elements.controls.children[0].children[0].children[0];
firstKnob.onpointerdown({pointerId: 1, clientY: 100});
assert.deepEqual(calls.at(-1), ['beginGesture', 'kick.tune_hz', 1]);
firstKnob.onpointermove({clientY: 83});
await new Promise(resolve => setImmediate(resolve));
assert.equal(calls.at(-1)[0], 'setParameter');
assert.equal(calls.at(-1)[1], 'kick.tune_hz');
assert.ok(Math.abs(calls.at(-1)[2] - 0.6) < 1e-9);
assert.equal(calls.at(-1)[3], 1);
firstKnob.onpointerup();
assert.deepEqual(calls.at(-1), ['endGesture', 'kick.tune_hz', 1]);
await new Promise(resolve => setImmediate(resolve));

events.parameterStateChanged({generation: 1, sequence: 0,
  parameters: [{id: 'kick.tune_hz', name: 'Tune', value: 0.2},
               {id: 'kick.decay_ms', name: 'Decay', value: 0.25}]});
assert.equal(firstKnob.children[0].style.transform, 'rotate(27deg)',
             'a stale timer echo overrode a newer local edit');
events.parameterStateChanged({generation: 1, sequence: commandSequence,
  parameters: [{id: 'kick.tune_hz', name: 'Tune', value: 0.2},
               {id: 'kick.decay_ms', name: 'Decay', value: 0.25}]});
assert.equal(firstKnob.children[0].style.transform, 'rotate(-81deg)');
events.parameterStateChanged({generation: 0, sequence: 100,
  parameters: [{id: 'kick.tune_hz', name: 'Tune', value: 0.8}]});
assert.equal(firstKnob.children[0].style.transform, 'rotate(-81deg)',
             'an obsolete instrument generation changed the current control');
let prevented = false;
firstKnob.onkeydown({key: 'ArrowUp', preventDefault() { prevented = true; }});
await new Promise(resolve => setImmediate(resolve));
assert.equal(prevented, true);
assert.equal(calls.at(-1)[0], 'setParameter');
assert.equal(calls.at(-1)[1], 'kick.tune_hz');
assert.ok(Math.abs(calls.at(-1)[2] - 0.21) < 1e-9);
assert.equal(calls.at(-1)[3], 1);
const firstEntry = elements.controls.children[0].children[2];
const callsBeforeInvalidEntry = calls.length;
firstEntry.value = 'not a number';
firstEntry.onchange();
assert.equal(calls.length, callsBeforeInvalidEntry);
assert.equal(firstEntry.value, '0.210');
firstEntry.value = '0.75';
firstEntry.onchange();
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(calls.at(-1), ['setParameter', 'kick.tune_hz', 0.75, 1]);
assert.equal(firstKnob.children[0].style.transform, 'rotate(67.5deg)');
firstKnob.onkeydown({key: 'End', preventDefault() {}});
assert.deepEqual(calls.at(-1), ['setParameter', 'kick.tune_hz', 1, 1]);
firstKnob.onkeydown({key: 'Home', preventDefault() {}});
assert.deepEqual(calls.at(-1), ['setParameter', 'kick.tune_hz', 0, 1]);
currentGeneration = 2;
events.parameterStateChanged({generation: 2, sequence: commandSequence,
  parameters: [{id: 'filter.cutoff', name: 'Cutoff', value: 0.3}]});
assert.equal(elements.controls.children.length, 1);
assert.equal(elements.controls.children[0].children[1].textContent, 'Cutoff');

const firstKey = elements.keys.children[0];
firstKey.onpointerdown();
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(calls.at(-1), ['noteOn', 48, 0.9]);
firstKey.onpointerup();
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(calls.at(-1), ['noteOff', 48]);

setResult = 'Unknown public parameter';
const changedKnob = elements.controls.children[0].children[0].children[0];
changedKnob.onpointerdown({pointerId: 2, clientY: 100});
changedKnob.onpointermove({clientY: 83});
await new Promise(resolve => setImmediate(resolve));
assert.equal(elements.error.textContent, 'Unknown public parameter');
changedKnob.onpointercancel();
assert.deepEqual(calls.at(-1), ['endGesture', 'filter.cutoff', 2]);

events.parameterStateChanged({generation: 2, sequence: commandSequence, parameters: []});
assert.equal(elements.controls.children.length, 1);
assert.equal(elements.controls.children[0].textContent, 'NO PUBLIC PARAMETERS');

const drumElements = Object.fromEntries(['controls', 'keys', 'error'].map(id => [id, new Element()]));
drumElements.keys.dataset.drumNotes = '36:Kick,38:Snare,42:Closed Hat,46:Open Hat';
const drumDocument = {
  getElementById: id => drumElements[id],
  createElement: tag => new Element(tag),
};
const drumCalls = [];
const drumBackend = {
  getNativeFunction: name => (...arguments_) => {
    drumCalls.push([name, ...arguments_]);
    return Promise.resolve(name === 'getParameterState'
      ? {generation: 1, sequence: 0, parameters: []} : undefined);
  },
  addEventListener: () => {},
};
vm.runInNewContext(script, {document: drumDocument, window: {__JUCE__: {backend: drumBackend}}});
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(drumElements.keys.children.map(key => key.textContent),
                 ['Kick', 'Snare', 'Closed Hat', 'Open Hat']);
drumElements.keys.children[2].onpointerdown();
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(drumCalls.at(-1), ['noteOn', 42, 0.9]);
drumElements.keys.children[2].onpointerup();
await new Promise(resolve => setImmediate(resolve));
assert.deepEqual(drumCalls.at(-1), ['noteOff', 42]);
