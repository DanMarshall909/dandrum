Layer list for one zone or pad. Every layer triggers on each hit; each is a source followed by an inline chain of modules (filter, gain, saturation…) processed left to right. Pair with KeyMap (zone selection drives the stack).

```jsx
<LayerStack subtitle="Kick · 36 C2" onAddModule={openModulePicker} layers={[
  { id: 'sub', name: 'Kick sub', source: 'Synth', detail: 'Sine · 48 Hz · pitch env 120 ms', level: -3,
    modules: [{ id: 'f', type: 'Filter', params: [
      { id: 'mode', label: 'Mode', value: 'LP', options: ['LP', 'HP', 'BP'], summary: true },
      { id: 'cut', label: 'Cutoff', value: 120, min: 20, max: 20000, unit: 'Hz', log: true, summary: true },
      { id: 'res', label: 'Reso', value: 10, min: 0, max: 100, unit: '%' },
    ] }] },
  { id: 'top', name: 'Kick mid-high', source: 'Sample', detail: 'kick_top.wav', level: -6,
    modules: [{ id: 'f', type: 'Filter', value: 'HP 120 Hz' }, { id: 'g', type: 'Gain', value: '+2.0 dB' }] },
]} onChange={setLayers} />
```

- No built-in EQ or band split: tone shaping is whatever modules the layer carries.
- Module block: bypass dot (filled = on, hollow = bypassed), type in caps, main value in mono. Selected module: ember outline. × appears on hover.
- Click the source block to open the source editor in the same slot: samples show a WaveformPanel (`waveform` props; a `start` param drives the start-offset marker) above their `params`; synths show engine `params`. Only one editor is open per layer.
- Click a module to open its editor under the layer row: knobs (`valueDisplay="always"`) for continuous params, segmented control for `options`. Click the module again, × or Esc to close. Block summary is built from params flagged `summary`.
- Dashed + slot at the end of each chain calls `onAddModule(layerId)`.
- Source tag names the engine type; the stack works the same for synths, samples and nested patches.
- Muted layer: amber M, chain dims.

### Routing
- Pass `fxBusses` to show a send knob per bus (letter above, −∞…+6 dB, double-click resets to off) and `outputs` to show an output menu per row.
- FX busses are the same component: rows with `source: 'FX bus'`, a module chain, level and output. Leave `fxBusses` empty on the bus stack (no bus-to-bus sends).
- Output busses use **OutputBusses** (channel pair per bus, meters, what feeds it).
