// Presentation data ported from the supplied Trigger reference.
import {selectedSample} from '../sample-model.mjs';
export function layoutTreeProjection(context) {
  const S = [
    ['empty','01 Empty','sample',{empty:1}], ['loaded','02 Single sample','sample',{browser:1}], ['sample','03 Sample editing','sample',{}],
    ['loop','04 Loop editing','sample',{loop:1}], ['slices','05 Slice editing','slices',{}], ['transients','06 Transient analysis','slices',{tr:'done'}],
    ['mapping','07 Key map','mapping',{}], ['vel','08 Velocity layers','layers',{mode:'vel'}], ['rr','09 Round robin','layers',{mode:'rr'}],
    ['voice','10 Voice shaping','voice',{}], ['mod','11 Modulation','mod',{}], ['macro','12 Macro editing','voice',{macro:1}],
    ['routing','13 Routing','routing',{}], ['fx','14 Effects','fx',{}], ['missing','15 Missing asset','sample',{missing:1}],
    ['unsupported','16 Unsupported asset','sample',{unsup:1}], ['loading','17 Loading','sample',{loading:1}], ['running','18 Analysis running','slices',{tr:'run'}],
    ['failed','19 Analysis failed','slices',{tr:'fail'}], ['min','20 Minimum 820×560','sample',{layout:'min'}], ['default','21 Default 1200×800','mapping',{layout:'default'}],
    ['exp','22 Expanded 1600×1000','voice',{layout:'exp'}], ['drop','23 Multi-file drop','sample',{drop:1}], ['perf','24 Compact · macros only','voice',{perf:1}],
  ];
  const cur = S.find(s => s[0] === this.state.st) || S[2];
  const f = cur[3], page = cur[2];
  const layout = this.state.layout || f.layout || 'default';
  const isMin = layout === 'min' && !(this.state.perf ?? !!f.perf), isExp = layout === 'exp' && !(this.state.perf ?? !!f.perf);
  const perf = this.state.perf ?? !!f.perf;
  const W = perf ? 900 : isMin ? 820 : isExp ? 1600 : 1200, H = perf ? 44 + 82 : isMin ? 560 : isExp ? 1000 : 800;
  const avail = Math.max(320, (typeof window !== 'undefined' ? window.innerWidth : 1300) - 32);
  const scale = Math.min(1, avail / W);
  const browser = this.state.browser != null ? this.state.browser : !!(f.browser);
  const selected=selectedSample(this);
  const empty=selected.patch.regions.length===0&&!selected.asset,drop=!!this.state.importDialog,missing=selected.asset?.state==='missing',unsup=selected.asset?.state==='unsupported',loading=selected.asset?.state==='loading',loop=!!selected.region?.loop;
  const mode = this.state.rrMode || f.mode || 'vel';
  const tr = f.tr || null;
  const treeW = isMin ? 36 : isExp ? 248 : 208, inspW = isMin ? 232 : isExp ? 320 : 272;
  const pageCanon = { sample: 'sample', slices: 'slices', mapping: 'mapping', layers: 'vel', voice: 'voice', mod: 'mod', routing: 'routing', fx: 'fx' };
  const A = (o) => o;

  const pianoPeaks = this.peaks(600, (t, r) => (t < 0.004 ? t / 0.004 : 1) * (Math.exp(-t * 2.4) * 0.85 + 0.06 * Math.exp(-t * 0.6)) * (0.78 + 0.22 * r()), 11);
  const seamA = this.peaks(80, (t, r) => 0.32 * (0.6 + 0.4 * Math.abs(Math.sin(t * 40))) * (0.85 + 0.15 * r()), 3);
  const seamB = this.peaks(80, (t, r) => 0.34 * (0.6 + 0.4 * Math.abs(Math.sin(t * 40 + 1))) * (0.85 + 0.15 * r()), 5);
  const partialPeaks = pianoPeaks.map((v, i) => i < 372 ? v : 0);

  const nm = this.nn.bind(this);
  const tree = [];
  const sel = { sample: 'k60', slices: 'break', mapping: 'keys', layers: mode === 'rr' ? 'snareH' : 'snare', voice: 'keys', mod: 'keys', routing: 'inst', fx: 'drums' }[page];
  const assigned = this.state.assigned || {}, DS = this.state.dragSrc, NO = this.state.nodeOver;
  const T = (id, label, detail, depth, icon, go, extra) => { const elig = DS && this.dropInfo(id); T0(id, label, detail, depth, icon, go, Object.assign({ drop: elig ? (NO === id ? 'inset 0 0 0 1px var(--dd-vermilion)' : 'inset 0 0 0 1px var(--dd-line-3)') : 'none', dbg: elig && NO === id ? 'var(--dd-vermilion-wash)' : 'transparent' }, extra || {})); (assigned[id] || []).forEach((a, i) => T0(id + '_a' + i, a.name, a.role === 'new pad' ? (39 + i) + ' ' + this.nn(39 + i) : a.role === 'alternate' ? 'alt' : 'layer', depth + 1, a.kind === 'slice' ? 'slice' : 'level', go, { drop: 'none', dbg: 'transparent' })); };
  const tHidden = this.state.treeHidden || {};
  const T0 = (id, label, detail, depth, icon, go, extra) => tHidden[id] ? null : tree.push(Object.assign({ id, label, detail, pad: 4 + depth * 14, icon, sel: id === sel && !empty, active: false, dim: false, go: () => this.go(go) }, extra || {}));
  if (empty || drop) { T('inst', 'Untitled', 'no sounds', 0, 'layers', 'empty'); }
  else {
    T('inst', 'Felt Kit', '3 groups', 0, 'layers', 'routing');
    T('keys', 'Keys', '10 zones', 1, 'keyboard', 'mapping');
    [[48,'Felt C3'],[55,'Felt G3'],[60,'Felt C4'],[66,'Felt F♯4'],[72,'Felt C5']].forEach(([n, l]) => T('k' + n, l, 'p · f', 2, missing && n === 60 ? 'file-missing' : 'level', 'sample', { active: n === 60 && page === 'sample' }));
    T('drums', 'Drums', '4 sounds', 1, 'layers', 'fx');
    T('kick', 'Kick', '36 C2', 2, 'one-shot', 'vel');
    T('snare', 'Snare', '38 · vel', 2, 'layers', 'vel');
    T('snareH', 'Hard', '×3', 3, 'alternate', 'rr');
    T('chat', 'Closed Hat', '42', 2, 'choke', 'vel');
    T('ohat', 'Open Hat', '46', 2, 'choke', 'vel');
    T('break', 'Break', '8 slices', 1, 'slice', 'slices');
  }

  const assets = [
    { head: true, label: 'Recent' },
    { row: true, icon: 'level', label: 'felt_C4_f.wav', dur: '4.82 s', sel: true },
    { row: true, icon: 'level', label: 'felt_C4_p.wav', dur: '4.61 s' },
    { row: true, icon: 'level', label: 'amen_172.wav', dur: '1.40 s' },
    { head: true, label: 'Felt Piano · 24 files' },
    { row: true, icon: 'level', label: 'felt_C3_f.wav', dur: '5.10 s' },
    { row: true, icon: 'level', label: 'felt_C3_p.wav', dur: '5.02 s' },
    { row: true, icon: 'level', label: 'felt_G3_f.wav', dur: '4.95 s' },
    { row: true, icon: 'file-missing', label: 'felt_G3_p.wav · missing', dur: '—' },
    { row: true, icon: 'level', label: 'felt_F#4_f.wav', dur: '4.44 s' },
    { head: true, label: 'Impulse responses' },
    { row: true, icon: 'level', label: 'plate_small.wav', dur: '1.80 s' },
    { row: true, icon: 'level', label: 'room_wood.wav', dur: '0.92 s' },
  ];
  Object.assign(context, {S, cur, f, page, layout, isMin, isExp, perf, W, H, avail, scale, browser, empty, drop, missing, unsup, loading, loop, mode, tr, treeW, inspW, pageCanon, A, pianoPeaks, seamA, seamB, partialPeaks, nm, tree, sel, assigned, DS, NO, T, tHidden, T0, assets});
}
