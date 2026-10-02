// Mock patch data for the Das Sampler UI kit. Musician-facing names only.
window.SAMPLER_DATA = {
  kit: {
    name: 'Reference Drum Kit',
    kind: 'kit',
    regions: 7,
    pads: {
      36: {
        name: 'Kick', short: 'Kick', mode: 'One-shot', voices: 8, steal: 'Oldest', choke: null,
        layers: [{ name: 'Kick', vel: [1, 127], policy: null, regions: [{ id: 'kick', file: 'kick.wav', kind: 'kick', len: 410, gain: '0.0', pan: 'C', pitch: '1.000', fadeIn: 0, fadeOut: 30, start: 0.01, end: 0.92 }] }],
      },
      38: {
        name: 'Snare', short: 'Snare', mode: 'One-shot', voices: 8, steal: 'Oldest', choke: null,
        layers: [
          { name: 'Soft', vel: [1, 95], policy: null, regions: [{ id: 'snare-soft', file: 'snare_soft.wav', kind: 'snare-soft', len: 280, gain: '−2.0', pan: 'C', pitch: '1.000', fadeIn: 0, fadeOut: 25, start: 0.015, end: 0.85 }] },
          { name: 'Hard', vel: [96, 127], policy: 'Round-robin', regions: [
            { id: 'snare-hard-a', alt: 'A', file: 'snare_hard_a.wav', kind: 'snare', len: 320, gain: '0.0', pan: 'C', pitch: '1.000', fadeIn: 0, fadeOut: 30, start: 0.01, end: 0.88 },
            { id: 'snare-hard-b', alt: 'B', file: 'snare_hard_b.wav', kind: 'snare', len: 335, gain: '−0.5', pan: 'C', pitch: '0.995', fadeIn: 0, fadeOut: 30, start: 0.012, end: 0.9 },
          ] },
        ],
      },
      42: {
        name: 'Closed Hat', short: 'Cl Hat', mode: 'One-shot', voices: 8, steal: 'Oldest', choke: 1,
        layers: [{ name: 'Closed Hat', vel: [1, 127], policy: null, regions: [{ id: 'hat-closed', file: 'hat_closed.wav', kind: 'hat-closed', len: 120, gain: '−3.0', pan: 'R8', pitch: '1.000', fadeIn: 0, fadeOut: 15, start: 0.005, end: 0.6 }] }],
      },
      46: {
        name: 'Open Hat', short: 'Op Hat', mode: 'Gated', voices: 8, steal: 'Oldest', choke: 1,
        layers: [{ name: 'Open Hat', vel: [1, 127], policy: 'Weighted', regions: [
          { id: 'hat-open-a', alt: 'A', weight: '1.0', file: 'hat_open_a.wav', kind: 'hat-open', len: 620, gain: '−3.5', pan: 'R8', pitch: '1.000', fadeIn: 0, fadeOut: 120, start: 0.01, end: 0.9 },
          { id: 'hat-open-b', alt: 'B', weight: '0.6', file: 'hat_open_b.wav', kind: 'hat-open', len: 590, gain: '−4.0', pan: 'R8', pitch: '1.000', fadeIn: 0, fadeOut: 120, start: 0.01, end: 0.88 },
        ] }],
      },
    },
  },
  breakPatch: {
    name: 'Example Break (sliced)',
    kind: 'break',
    file: 'break_120bpm.wav',
    len: 2000,
    slices: [0, 0.125, 0.25, 0.3125, 0.5, 0.625, 0.75, 0.875],
    sliceNames: ['Kick', 'Hat', 'Snare', 'Ghost snare', 'Kick', 'Hat open', 'Snare', 'Fill'],
  },
  modSources: [
    { slot: 'A', name: 'Velocity' },
    { slot: 'B', name: 'Random per hit' },
    { slot: 'C', name: 'Mod wheel' },
    { slot: 'D', name: 'Aftertouch' },
  ],
  liveParams: [
    { id: 'pitch', label: 'Pitch ratio', bipolar: true, def: 0.5, fmt: (v) => (Math.pow(2, (v - 0.5) * 2)).toFixed(3), unit: '×', range: '0.500–2.000×', parse: (t) => { const r = parseFloat(t); return r > 0 ? Math.log2(r) / 2 + 0.5 : null; } },
    { id: 'start', label: 'Start offset', bipolar: false, def: 0, fmt: (v) => String(Math.round(v * 100)), unit: 'ms', range: '0–100 ms', parse: (t) => { const n = parseFloat(t); return Number.isFinite(n) ? n / 100 : null; } },
    { id: 'level', label: 'Level', bipolar: false, def: 0.8, fmt: (v) => v < 0.01 ? '−∞' : ((v - 0.8) * 30).toFixed(1).replace('-', '−'), unit: 'dB', range: '−∞…+6 dB', parse: (t) => { const n = parseFloat(String(t).replace('−', '-')); return Number.isFinite(n) ? n / 30 + 0.8 : null; } },
    { id: 'pan', label: 'Pan', bipolar: true, def: 0.5, fmt: (v) => { const p = Math.round((v - 0.5) * 100); return p === 0 ? 'C' : p < 0 ? 'L' + -p : 'R' + p; }, unit: '', range: 'L50…R50', parse: (t) => { const s = String(t).trim().toUpperCase(); if (s === 'C') return 0.5; const n = parseFloat(s.slice(1)); if (!Number.isFinite(n)) return null; return 0.5 + (s[0] === 'L' ? -n : n) / 100; } },
    { id: 'variation', label: 'Variation', bipolar: false, def: 0, fmt: (v) => String(Math.round(v * 100)), unit: '%', range: 'alternate spread', parse: (t) => { const n = parseFloat(t); return Number.isFinite(n) ? n / 100 : null; } },
  ],
};
