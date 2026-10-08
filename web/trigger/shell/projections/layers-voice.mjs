// Presentation data ported from the supplied Trigger reference.
import {selectedSelector} from '../layers-model.mjs';
export function layersVoiceProjection(context) {
  let {isMin, isExp, mode, k, kL} = context;
  const modeList = [['stack','Stack'],['vel','Velocity'],['rr','Round robin'],['alt','Alternate'],['rand','Random'],['wrand','Weighted']];
  const selModes = modeList.map(([id, label]) => ({ label, sel: id === mode, go: () => this.setState({ rrMode: id }) }));
  const modeDescs = { stack: 'All samples play on every hit.', vel: 'One sample per hit, chosen by velocity range. Ranges may overlap to crossfade.', rr: 'One sample per hit, in order. Each note keeps its own position.', alt: 'Two samples, swapping on every hit.', rand: 'One sample per hit at random; never the same twice in a row.', wrand: 'One sample per hit at random, in proportion to each weight.' };
  const isRRlike = ['rr','alt','rand','wrand'].includes(mode);
  const {selector,target}=selectedSelector(this),draft=this.state.selectorDraft?.target===target?this.state.selectorDraft:null;
  const vLo = draft?.hardLo??selector?.hardLo??90, vHi = draft?.softHi??selector?.softHi??101, vW = Math.max(0, vHi - vLo + 1), vCurve = selector?.curve==='linear'?'lin':'eq';
  const vx = (v) => (v - 1) / 126 * 100;
  const ramp = (a, b, up) => { if (b <= a) return up ? [[a, 100], [a, 0]] : [[a, 0], [a, 100]]; const pts = []; for (let i = 0; i <= 12; i++) { const f = i / 12, g = vCurve === 'eq' ? (up ? Math.sin(f * Math.PI / 2) : Math.cos(f * Math.PI / 2)) : (up ? f : 1 - f); pts.push([a + (b - a) * f, 100 - g * 100]); } return pts; };
  const xa = vLo - 1.5, xb = vHi - 0.5;
  const ptsStr = (arr) => arr.map(p => p[0].toFixed(2) + ',' + p[1].toFixed(1)).join(' ');
  const vSoftPts = ptsStr([[0, 100], [0, 0]].concat(vW ? ramp(xa, xb, false) : [[vHi - 0.5, 0], [vHi - 0.5, 100]]));
  const vHardPts = ptsStr((vW ? ramp(xa, xb, true) : [[vLo - 1.5, 100], [vLo - 1.5, 0]]).concat([[126, 0], [126, 100]]));
  const gSoft = Math.pow(10, -2.0 / 20), gHard = Math.pow(10, 0 / 20);
  const gStops = (up, g) => Array.from({ length: 13 }, (_, i) => { const f = i / 12, c = !vW ? (up ? (f > 0 ? 1 : 0) : (f < 1 ? 1 : 0)) : vCurve === 'eq' ? (up ? Math.sin(f * Math.PI / 2) : Math.cos(f * Math.PI / 2)) : (up ? f : 1 - f); return { o: f.toFixed(3), a: (c * g).toFixed(3) }; });
  const softRange = '1–' + vHi, hardRange = vLo + '–127';
  const cands = isRRlike ? [
    { ord: 1, name: 'Hard A', file: 'snare_hard_a.wav', vel: '96–127', w: mode === 'wrand' ? '2.0' : '33%', wpct: mode === 'wrand' ? 50 : 33, gain: '0.0 dB', pan: 'C', tune: '0 st', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-4)', bg: 'transparent', edge: 'none' },
    { ord: 2, name: 'Hard B', file: 'snare_hard_b.wav', vel: '96–127', w: mode === 'wrand' ? '1.0' : '33%', wpct: mode === 'wrand' ? 25 : 33, gain: '−0.5 dB', pan: 'C', tune: '0 st', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-1)', bg: 'var(--dd-vermilion-wash)', edge: 'inset 2px 0 0 var(--dd-vermilion)' },
    { ord: 3, name: 'Hard C', file: 'snare_hard_c.wav', vel: '96–127', w: mode === 'wrand' ? '1.0' : '33%', wpct: mode === 'wrand' ? 25 : 33, gain: '0.0 dB', pan: 'C', tune: '+8 ct', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-4)', bg: 'transparent', edge: 'none' },
  ] : [
    { ord: 1, name: 'Soft', file: 'snare_soft.wav', vel: softRange, w: '—', wpct: 0, gain: '−2.0 dB', pan: 'C', tune: '0 st', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-1)', bg: 'transparent', edge: 'none' },
    { ord: 2, name: 'Hard', file: '3 alternates · round robin', vel: hardRange, w: '—', wpct: 0, gain: '0.0 dB', pan: 'C', tune: '0 st', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-4)', bg: 'var(--dd-vermilion-wash)', edge: 'inset 2px 0 0 var(--dd-vermilion)' },
  ];
  ((this.state.assigned || {}).snareH || []).forEach((a, i) => cands.push({ ord: 4 + i, name: a.name, file: a.kind === 'slice' ? 'amen_172.wav · slice' : a.name, vel: '96–127', w: '25%', wpct: 25, gain: '0.0 dB', pan: 'C', tune: '0 st', out: 'Inherit · Snare', mc: 'var(--dd-paper-4)', dot: 'var(--dd-paper-4)', bg: 'transparent', edge: 'none' }));
  const rrSeq = [
    { label: 'Hard A', tag: '', bg: 'var(--dd-ink-4)', bd: 'var(--dd-line-2)', tc: 'var(--dd-paper-3)' },
    { label: 'Hard B', tag: 'Last', bg: 'var(--dd-ink-5)', bd: 'var(--dd-paper-2)', tc: 'var(--dd-paper-2)' },
    { label: 'Hard C', tag: 'Next', bg: 'var(--dd-ink-4)', bd: 'var(--dd-vermilion)', tc: 'var(--dd-vermilion)' },
  ];

  const chain = [
    { label: 'Sample player', bg: 'var(--dd-ink-4)', bd: 'var(--dd-line-2)', arrow: true, sep: '+' },
    { label: 'Lush', bg: 'var(--dd-ink-4)', bd: 'var(--dd-line-2)', arrow: true, sep: '→' },
    { label: 'Filter', bg: 'var(--dd-vermilion-wash)', bd: 'var(--dd-vermilion)', arrow: true, sep: '→' },
    { label: 'Amplifier', bg: 'var(--dd-ink-4)', bd: 'var(--dd-line-2)', arrow: true, sep: '→' },
    { label: 'Group out', bg: 'transparent', bd: 'var(--dd-line-2)', arrow: false },
  ];
  const cutoffMods = [{ slot: 'A', depth: 0.26, source: 'Filter envelope', live: 0.6 }, { slot: 'D', depth: 0.12, source: 'Macro · Tone' }, { slot: 'C', depth: 0.08, source: 'Velocity' }];
  const voiceSecs = [
    { title: 'Source', sub: 'Sample player', ann: 'Module · AssetPlayer', bd: 'var(--dd-line-1)', hasSeg: true, segV: 'once', seg: [{ id: 'once', label: 'Once' }, { id: 'gated', label: 'Gated' }, { id: 'loop', label: 'Loop' }], knobs: [k('Start', 0.012, '0.058 s', 0, [{ slot: 'D', depth: 0.15, source: 'Aftertouch' }]), k('Gain', 0.72, '−1.5 dB')] },
    { title: 'Source', sub: 'Lush · Dandrum synth', ann: 'Module · Patch (another Dandrum instrument as a source)', bd: 'var(--dd-line-1)', hasSeg: true, segV: 'note', seg: [{ id: 'note', label: 'Follows notes' }, { id: 'drone', label: 'Drone' }], knobs: [kL('Level', 0.55, '−6.0 dB'), kL('Pan', 0.5, 'C', 1), kL('Tune', 0.5, '0 st', 1), kL('Detune', 0.34, '34%', 0, [{ slot: 'B', depth: 0.1, source: 'LFO 1' }]), kL('Brightness', 0.6, '60%', 0, [{ slot: 'D', depth: 0.18, source: 'Macro · Tone' }]), kL('Width', 0.7, '70%')] },
    { title: 'Pitch', sub: '', ann: 'Parameters · AssetPlayer pitch', bd: 'var(--dd-line-1)', hasSeg: false, knobs: [k('Tune', 0.5, '0 st', 1, [{ slot: 'B', depth: 0.05, source: 'LFO 1' }]), k('Fine', 0.47, '−3 ct', 1), k('Key track', 1, '100%'), k('Bend', 0.17, '±2 st')] },
    { title: 'Filter', sub: 'Ladder 24 dB', ann: 'Module · Filter', bd: 'var(--dd-vermilion)', hasSeg: true, segV: 'lp', seg: [{ id: 'lp', label: 'LP' }, { id: 'bp', label: 'BP' }, { id: 'hp', label: 'HP' }, { id: 'nt', label: 'Notch' }], knobs: [k('Cutoff', 0.42, '2.40 kHz', 0, cutoffMods), k('Reso', 0.22, '22%'), k('Drive', 0.1, '1.5 dB', 0, []), k('Key trk', 0.5, '50%'), k('Env amt', 0.71, '+42%', 1)] },
    { title: 'Amplitude', sub: '', ann: 'Module · Gain', bd: 'var(--dd-line-1)', hasSeg: false, knobs: [k('Level', 0.62, '−0.8 dB', 0, [{ slot: 'C', depth: 0.2, source: 'Velocity' }]), k('Pan', 0.5, 'C', 1, []), k('Vel sens', 0.3, '30%')] },
  ];
  const envH = isMin ? 44 : isExp ? 72 : 56;
  const envs = [
    { title: 'Amp envelope', slot: false, dest: '→ Amp gain', ann: 'Envelope modulator → Route (fixed) → Gain', pts: '0,100 6,0 60,32 220,32 300,100', sx: 220, handles: [{ x: 0.02, y: 0 }, { x: 0.2, y: 0.32 }, { x: 0.733, y: 0.32 }, { x: 1, y: 1 }], live: 0.42, vals: [{ k: 'Attack', v: '12 ms' }, { k: 'Decay', v: '380 ms' }, { k: 'Sustain', v: '−6.0 dB' }, { k: 'Release', v: '1.20 s' }] },
    { title: 'Filter envelope', slot: true, dest: '→ Cutoff +42%', ann: 'Envelope modulator → Route → Filter cutoff', pts: '0,100 2,0 40,70 220,70 260,100', sx: 220, handles: [{ x: 0.007, y: 0 }, { x: 0.133, y: 0.7 }, { x: 0.733, y: 0.7 }, { x: 0.867, y: 1 }], live: 0.42, vals: [{ k: 'Attack', v: '1 ms' }, { k: 'Decay', v: '240 ms' }, { k: 'Sustain', v: '30%' }, { k: 'Release', v: '420 ms' }] },
  ];
  Object.assign(context, {modeList, selModes, modeDescs, isRRlike, vLo, vHi, vW, vCurve, vx, ramp, xa, xb, ptsStr, vSoftPts, vHardPts, gSoft, gHard, gStops, softRange, hardRange, cands, rrSeq, chain, cutoffMods, voiceSecs, envH, envs});
}
