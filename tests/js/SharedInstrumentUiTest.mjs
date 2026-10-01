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
const backend = {
  getNativeFunction: name => (...arguments_) => {
    calls.push([name, ...arguments_]);
    if (name === 'getParameters')
      return Promise.resolve([{id: 'kick.tune_hz', name: 'Tune', value: 0.5},
                              {id: 'kick.decay_ms', name: 'Decay', value: 0.25}]);
    return Promise.resolve(name === 'setParameter' ? setResult : undefined);
  },
  addEventListener: (name, callback) => { events[name] = callback; },
};
const window = {__JUCE__: {backend}};
vm.runInNewContext(script, {document, window});
await new Promise(resolve => setImmediate(resolve));

assert.equal(elements.controls.children.length, 2);
assert.equal(elements.controls.children[0].children[1].textContent, 'Tune');
assert.equal(elements.controls.children[1].children[1].textContent, 'Decay');
assert.equal(elements.keys.children.length, 19);

const firstKnob = elements.controls.children[0].children[0].children[0];
firstKnob.onpointerdown({pointerId: 1, clientY: 100});
firstKnob.onpointermove({clientY: 83});
await new Promise(resolve => setImmediate(resolve));
assert.equal(calls.at(-1)[0], 'setParameter');
assert.equal(calls.at(-1)[1], 'kick.tune_hz');
assert.ok(Math.abs(calls.at(-1)[2] - 0.6) < 1e-9);
firstKnob.onpointerup();

events.parameterValuesChanged([{id: 'kick.tune_hz', name: 'Tune', value: 0.2},
                               {id: 'kick.decay_ms', name: 'Decay', value: 0.25}]);
assert.equal(firstKnob.children[0].style.transform, 'rotate(-81deg)');
events.parameterValuesChanged([{id: 'filter.cutoff', name: 'Cutoff', value: 0.3}]);
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

events.parameterValuesChanged([]);
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
    return Promise.resolve(name === 'getParameters' ? [] : undefined);
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
