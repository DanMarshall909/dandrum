import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const root = new URL('../../src/juce-plugin/', import.meta.url);
const pageHeader = fs.readFileSync(new URL('Tb303WebUi.h', root), 'utf8');
const sharedHeader = fs.readFileSync(new URL('SharedInstrumentUi.h', root), 'utf8');
const html = pageHeader.match(/R"HTML\(([\s\S]*?)\)HTML"/)?.[1];
const shared = sharedHeader.match(/R"JS\(([\s\S]*?)\)JS"/)?.[1];
assert.ok(html && shared);
assert.ok(html.includes('<script src="/shared-instrument-ui.js"></script>'),
  '303 page does not load the shared control script');
const inline = html.match(/<script>([\s\S]*?)<\/script>/)?.[1];
assert.ok(inline, '303 page has no Sound Lab/presentation script');

const canvasContext = Object.fromEntries(
  ['clearRect', 'fillRect', 'beginPath', 'moveTo', 'lineTo', 'stroke', 'fillText', 'setLineDash']
    .map(name => [name, () => {}]));
class Element {
  constructor(tag = 'div') {
    this.tag = tag; this.children = []; this.style = {}; this.dataset = {};
    this.textContent = ''; this.innerHTML = ''; this.loadCount = 0;
    this.classes = new Set();
    this.classList = {
      add: name => this.classes.add(name),
      remove: name => this.classes.delete(name),
      contains: name => this.classes.has(name),
      toggle: name => this.classes.has(name) ? this.classes.delete(name) : this.classes.add(name),
    };
  }
  set className(value) { this.classes = new Set(value.split(/\s+/).filter(Boolean)); }
  get className() { return [...this.classes].join(' '); }
  appendChild(child) { this.children.push(child); return child; }
  append(...children) { this.children.push(...children); }
  replaceChildren(...children) { this.children = [...children]; }
  setPointerCapture() {}
  removeAttribute(name) { delete this[name]; }
  load() { ++this.loadCount; }
  getContext() { return canvasContext; }
}

const ids = ['controls', 'keys', 'error', 'soundLabStatus', 'renderSoundLab',
  'chooseSoundLabReference', 'matchSoundLab', 'cancelSoundLab', 'acceptSoundLabMatch',
  'requestGraphProposal', 'soundLabMeta', 'soundLabScore', 'soundLabProgress',
  'soundLabReference', 'soundLabProposal', 'soundLabAudio', 'soundLabReferenceAudio',
  'soundLabCandidateAudio', 'soundLabPlot', 'soundLabLegend', 'steps', 'functions', 'status'];
const elements = Object.fromEntries(ids.map(id => [id, new Element()]));
elements.soundLabPlot.width = 900; elements.soundLabPlot.height = 194;
const document = {
  getElementById: id => elements[id],
  createElement: tag => new Element(tag),
  querySelectorAll: selector => selector === '.step' ? elements.steps.children : [],
};
const calls = [];
const listeners = {};
const backend = {
  getNativeFunction: name => (...args) => {
    calls.push([name, ...args]);
    if (name === 'getParameters')
      return Promise.resolve([{id: 'filter.cutoff', name: 'Cutoff', value: 0.5},
                              {id: 'filter.resonance', name: 'Resonance', value: 0.25}]);
    if (name === 'getSoundLabAnalysis') return Promise.resolve({state: 'idle', generation: 0});
    return Promise.resolve();
  },
  addEventListener: (name, listener) => { listeners[name] = listener; },
};
const context = vm.createContext({document, window: {__JUCE__: {backend}}});
vm.runInContext(shared, context);
vm.runInContext(inline, context);
await new Promise(resolve => setImmediate(resolve));

assert.equal(elements.controls.children.length, 2);
assert.equal(elements.controls.children[0].children[1].textContent, 'Cutoff');
assert.equal(elements.keys.children.length, 19);
assert.equal(elements.steps.children.length, 16);
assert.equal(elements.soundLabStatus.textContent, 'IDLE');

elements.steps.children[3].onclick();
assert.equal(elements.status.textContent, '04');
assert.equal(elements.steps.children[3].classList.contains('on'), true);

elements.renderSoundLab.onclick();
assert.equal(elements.soundLabStatus.dataset.state, 'rendering');
await new Promise(resolve => setImmediate(resolve));
assert.equal(calls.at(-1)[0], 'renderSoundLab');

listeners.soundLabAnalysisChanged({state: 'ready', generation: 2,
  sample_rate_hz: 48000, duration_seconds: 1,
  metrics: [{time_seconds: 0, rms: 0.1, peak: 0.2, spectral_centroid_hz: 300}],
  audio_url: '/sound-lab.wav?generation=2'});
assert.equal(elements.soundLabStatus.textContent, 'READY');
assert.equal(elements.soundLabAudio.src, '/sound-lab.wav?generation=2');
assert.equal(elements.soundLabCandidateAudio.src, '/sound-lab.wav?generation=2');

listeners.soundLabAnalysisChanged({state: 'rendering', generation: 3});
assert.equal(elements.soundLabAudio.src, undefined);
assert.equal(elements.soundLabReferenceAudio.src, undefined);
assert.equal(elements.soundLabCandidateAudio.src, undefined);

listeners.soundLabAnalysisChanged({state: 'matched', generation: 4,
  completed_evaluations: 2, max_evaluations: 2, reference_name: 'reference.wav',
  best_parameters: [{id: 'filter.cutoff', best: 0.7}],
  comparison_metrics: [{time_seconds: 0, reference_rms: 0.1, candidate_rms: 0.2,
    reference_peak: 0.2, candidate_peak: 0.3,
    reference_spectral_centroid_hz: 400, candidate_spectral_centroid_hz: 500}],
  reference_audio_url: '/sound-lab-reference.wav?generation=4',
  candidate_audio_url: '/sound-lab-candidate.wav?generation=4',
  manifest: {seed: 303, reference_sha256: 'abc', best_score: {total: 0.1,
    spectral: 0.2, rms: 0.3, centroid: 0.4}}});
assert.equal(elements.acceptSoundLabMatch.disabled, false);
assert.equal(elements.soundLabReferenceAudio.src, '/sound-lab-reference.wav?generation=4');
assert.equal(elements.soundLabCandidateAudio.src, '/sound-lab-candidate.wav?generation=4');
assert.match(elements.soundLabMeta.textContent, /reference abc/);
elements.acceptSoundLabMatch.onclick();
await new Promise(resolve => setImmediate(resolve));
assert.equal(calls.at(-1)[0], 'acceptSoundLabMatch');
assert.match(elements.soundLabMeta.textContent, /accepted into the active TB-303 controls/);

listeners.parameterValuesChanged([{id: 'filter.cutoff', name: 'Cutoff', value: 0.75}]);
assert.equal(elements.controls.children.length, 1);
