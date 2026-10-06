// Presentation data ported from the supplied Trigger reference.
export function routingMacrosProjection(context) {
  let {f} = context;
  const routeRows = [
    ['Felt Kit', 'Instrument', 'Main 1/2', 0.0, 0.0, 0],
    ['Keys', 'Group', 'Inherit · Main', 0.35, 0.1, 1],
    ['Drums', 'Group', 'Drums 3/4', 0.12, 0.0, 1],
    ['Kick', 'Sound · note 36', 'Inherit · Drums', 0, 0, 2],
    ['Snare', 'Sound · note 38', 'Snare 5/6', 0.3, 0.18, 2],
    ['Closed Hat', 'Sound · note 42', 'Inherit · Drums', 0, 0, 2],
    ['Open Hat', 'Sound · note 46', 'Inherit · Drums', 0, 0.2, 2],
    ['Break', 'Group', 'Inherit · Main', 0.15, 0, 1],
    ['Snare 1 · slice 3', 'Slice · note 26', 'Snare 5/6', 0, 0, 2],
  ].map(([name, kind, bus, sa, sb, d], i) => ({ name, kind, bus, sa, sb, pad: d * 14, bg: i === 4 ? 'var(--dd-vermilion-wash)' : 'transparent', edge: i === 4 ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none' }));
  const busses = [
    { id: 'main', name: 'Main', channels: '1/2', main: true, feeds: ['Keys', 'Break', 'Reverb', 'Delay'], levels: [-14, -15], level: 0 },
    { id: 'drums', name: 'Drums', channels: '3/4', feeds: ['Kick', 'Closed Hat', 'Open Hat'], levels: [-10, -10], level: -1.5 },
    { id: 'snare', name: 'Snare', channels: '5/6', feeds: ['Snare', 'Snare 1 · slice 3'], levels: [-12, -12.5], level: 0 },
  ];
  const outputs = [{ id: 'main', name: 'Main', note: '1/2' }, { id: 'drums', name: 'Drums', note: '3/4' }, { id: 'snare', name: 'Snare', note: '5/6' }];
  const P = (id, label, value, min, max, unit, extra) => Object.assign({ id, label, value, min, max, unit }, extra || {});
  const insertLayers = [
    { id: 'drums', name: 'Drums', source: 'Group', detail: 'Kick · Snare · Hats', output: 'drums', level: -1.5, modules: [
      { id: 'eq', type: 'EQ', params: [P('lo', 'Low', 2, -12, 12, 'dB', { bipolar: true, summary: true }), P('hi', 'High', 1.5, -12, 12, 'dB', { bipolar: true })] },
      { id: 'comp', type: 'Compressor', params: [P('thr', 'Threshold', -18, -60, 0, 'dB', { summary: true }), P('ratio', 'Ratio', 4, 1, 20, ':1', { summary: true }), P('att', 'Attack', 10, 0.1, 100, 'ms'), P('rel', 'Release', 120, 10, 1000, 'ms')] },
      { id: 'sat', type: 'Saturate', params: [P('drv', 'Drive', 30, 0, 100, '%', { summary: true }), P('mix', 'Mix', 60, 0, 100, '%')] },
    ] },
    { id: 'keys', name: 'Keys', source: 'Group', detail: 'Felt C3–C5 · 10 zones', output: 'main', level: 0, modules: [
      { id: 'eq2', type: 'EQ', params: [P('lo', 'Low', -1.5, -12, 12, 'dB', { bipolar: true, summary: true })] },
    ] },
    { id: 'brk', name: 'Break', source: 'Group', detail: 'amen_172.wav · 8 slices', output: 'main', level: -3, modules: [
      { id: 'crush', type: 'Bitcrush', params: [P('bits', 'Bits', 12, 1, 24, 'bit', { summary: true })], bypassed: true },
    ] },
  ];
  const fxLayers = [
    { id: 'rev', name: 'Reverb', source: 'FX bus', detail: 'Send A · plate_small.wav', output: 'main', level: -4, modules: [
      { id: 'conv', type: 'Convolution', params: [P('pre', 'Predelay', 12, 0, 200, 'ms', { summary: true }), P('mix', 'Mix', 100, 0, 100, '%')] },
      { id: 'eqr', type: 'EQ', params: [P('hp', 'Low cut', 180, 20, 1000, 'Hz', { log: true, summary: true })] },
    ] },
    { id: 'dly', name: 'Delay', source: 'FX bus', detail: 'Send B · 1/8 dotted', output: 'main', level: -8, modules: [
      { id: 'delay', type: 'Delay', params: [P('time', 'Time', 261, 1, 2000, 'ms', { summary: true }), P('fb', 'Feedback', 35, 0, 100, '%')] },
      { id: 'flt', type: 'Filter', params: [P('cut', 'Cutoff', 3200, 20, 20000, 'Hz', { log: true, summary: true })] },
    ] },
  ];

  const macroSel = !!f.macro;
  const macros = [['Tone', 0.56, '3 destinations'], ['Body', 0.4, '2 destinations'], ['Space', 0.32, 'Host automated', 1], ['Drive', 0.18, '2 destinations'], ['Snap', 0.64, '1 destination'], ['Width', 0.5, '1 destination'], ['Macro 7', 0, 'Unassigned', 0, 1], ['Macro 8', 0, 'Unassigned', 0, 1]].map(([name, v0, d, host, off], i) => {
    const s = macroSel && i === 0;
    this.mDefaults = this.mDefaults || {}; if (this.mDefaults[name] === undefined) this.mDefaults[name] = v0;
    const v = (this.state.mv || {})[name] ?? v0;
    return { name, v, d, set: (nv) => this.setState(st => ({ mv: Object.assign({}, st.mv || {}, { [name]: nv }) })), host: !!host, off: !!off, c: off ? 'var(--dd-paper-4)' : 'var(--dd-paper-1)', bg: s ? 'var(--dd-vermilion-wash)' : 'transparent', edge: s ? 'inset 0 0 0 1px var(--dd-vermilion)' : 'none' };
  });
  Object.assign(context, {routeRows, busses, outputs, P, insertLayers, fxLayers, macroSel, macros});
}
