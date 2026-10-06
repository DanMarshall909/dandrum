// Presentation data ported from the supplied Trigger reference.
export function modulationProjection(context) {
  let {k} = context;
  const R = (src, slot, amt, amtT, pol, curve, dest, live, on, fixed) => ({ src, slot, noSlot: !slot, amt: amtT, pol, curve, dest, live, on, fixed: !!fixed, a: amt });
  const rl = [
    R('Amp envelope', null, 1, '100%', 'Uni', 'Exp', 'Amp gain', 62, true, true),
    R('Filter envelope', 'A', 0.42, '+42%', 'Uni', 'Linear', 'Filter cutoff', 48, true),
    R('LFO 1', 'B', 0.05, '±5 ct', 'Bi', 'Linear', 'Pitch', 70, true),
    R('LFO 2', null, 0.18, '+18%', 'Bi', 'Linear', 'Filter drive', 0, false),
    R('Velocity', 'C', 0.3, '+30%', 'Uni', 'Exp', 'Amp gain', 58, true),
    R('Velocity', 'C', 0.15, '+15%', 'Uni', 'Linear', 'Filter cutoff', 58, true),
    R('Key track', null, 0.5, '+50%', 'Bi', 'Linear', 'Filter cutoff', 50, true),
    R('Aftertouch', null, 0.15, '+15%', 'Uni', 'Linear', 'Sample start', 0, true),
    R('Mod wheel · CC 1', null, 1, '+100%', 'Uni', 'Linear', 'LFO 1 depth', 22, true),
    R('Pitch bend', null, 1, '±2 st', 'Bi', 'Linear', 'Pitch', 50, true),
    R('Random per note', null, -0.12, '±12%', 'Bi', 'Linear', 'Pan', 34, true),
    R('Macro · Tone', 'D', 0.2, '+20%', 'Uni', 'Linear', 'Filter cutoff', 56, true),
    R('Macro · Tone', 'D', -0.1, '−10%', 'Uni', 'Linear', 'Filter resonance', 56, true),
    R('Macro · Space', null, 0.6, '+60%', 'Uni', 'S-curve', 'Send · Reverb', 32, true),
  ];
  const destOf = { Cutoff: 'Filter cutoff', Level: 'Amp gain', Start: 'Sample start', Tune: 'Pitch', Reso: 'Filter resonance', Drive: 'Filter drive' };
  Object.entries(this.state.removed || {}).forEach(([lab, arr]) => { for (let i = rl.length - 1; i >= 0; i--) if ((destOf[lab] || lab) === rl[i].dest && arr.includes(rl[i].src)) rl.splice(i, 1); });
  (this.state.extraRoutes || []).forEach(x => rl.push(R(x.src, x.slot || null, 0.25, '+25%', 'Uni', 'Linear', x.dest, 0, true)));
  const gone = this.state.routeGone || {};
  for (let i = rl.length - 1; i >= 0; i--) if (gone[rl[i].src + '|' + rl[i].dest]) rl.splice(i, 1);
  Object.entries(this.state.added || {}).forEach(([lab, arr]) => arr.forEach(m => rl.push(R(m.source, m.slot || null, 0.25, '+25%', 'Uni', 'Linear', destOf[lab] || lab, 0, true))));
  const colorOf = (s) => s ? 'var(--dd-mod-' + s.toLowerCase() + ')' : 'var(--dd-paper-3)';
  let lastSrc = null;
  const routes = rl.map((r, i) => {
    const a = Math.max(-1, Math.min(1, r.a)), l = a >= 0 ? 50 : 50 + a * 50, w = Math.abs(a) * 50;
    const head = r.src !== lastSrc && false; lastSrc = r.src;
    const s = i === (this.state.modRoute ?? 1);
    r.rk = r.src + '|' + r.dest; if ((this.state.routeOff || {})[r.rk]) { r.on = false; }
    return Object.assign(r, { select: () => this.setState({ modRoute: i, modInsp: false, editRoute: null }), l, w, c: colorOf(r.slot), op: r.on ? 1 : 0.5, head, bg: s ? 'var(--dd-vermilion-wash)' : 'transparent', edge: s ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none' });
  });
  const rOpen = this.state.routeOpen || {};
  const order = [], bySrc = {};
  routes.forEach((r, i) => { if (!bySrc[r.src]) { bySrc[r.src] = []; order.push(r.src); } bySrc[r.src].push(r); });
  const routeGroups = order.map(src => {
    const items = bySrc[src], multi = items.length > 1, open = rOpen[src] !== false;
    items.forEach((r, j) => { r.child = multi; r.top = !multi; r.indent = multi ? 22 : 0; });
    return { name: src, multi, show: !multi || open, chev: open ? 'chevron-down' : 'chevron-right', slot: items[0].slot, noSlot: !items[0].slot, count: items.length,
      summary: items.map(r => r.dest + ' ' + r.amt).join(' · '), live: items[0].live, items,
      toggle: () => this.setState({ routeOpen: Object.assign({}, rOpen, { [src]: !open }) }),
      addDest: (e) => { e.stopPropagation(); const p = this.winPt(e, 264, 300); this.setState({ dmenu: { x: p.x, y: p.y, src, slot: items[0].slot } }); } };
  });
  const selR = routes[Math.min(this.state.modRoute ?? 1, routes.length - 1)] || routes[0];
  const shapeSt = this.state.modShape || {};
  const isLfo = /^LFO/.test(selR.src), isEnv = /envelope/.test(selR.src), isRand = /Random/.test(selR.src);
  const meShapes = isLfo ? [{ id: 'sine', label: 'Sine' }, { id: 'tri', label: 'Tri' }, { id: 'saw', label: 'Saw' }, { id: 'sq', label: 'Square' }, { id: 'sh', label: 'S&H' }]
    : isEnv ? [{ id: 'adsr', label: 'ADSR' }, { id: 'ahdsr', label: 'AHDSR' }, { id: 'multi', label: 'Multi' }]
    : isRand ? [{ id: 'step', label: 'Steps' }, { id: 'smooth', label: 'Smooth' }]
    : [{ id: 'lin', label: 'Linear' }, { id: 'exp', label: 'Exp' }, { id: 'log', label: 'Log' }, { id: 's', label: 'S-curve' }];
  const meShape = shapeSt[selR.src] || (isLfo ? (selR.src === 'LFO 2' ? 'tri' : 'sine') : isEnv ? 'adsr' : isRand ? 'step' : ({ Exp: 'exp', 'S-curve': 's' }[selR.curve] || 'lin'));
  const fnOf = (sh) => {
    if (isLfo) return (t) => { const p = (t * 2) % 1; return sh === 'sine' ? Math.sin(p * 2 * Math.PI) : sh === 'tri' ? 1 - 4 * Math.abs(p - 0.5) : sh === 'saw' ? 1 - 2 * p : sh === 'sq' ? (p < 0.5 ? 1 : -1) : [0.6, -0.3, 0.9, -0.8, 0.2, -0.5, 0.75, -0.1][Math.floor(t * 8) % 8]; };
    if (isEnv) { const A = selR.src === 'Amp envelope' ? 0.04 : 0.01, D = 0.2, S = selR.src === 'Amp envelope' ? 0.5 : 0.3, R = 0.25, rel = 0.72; return (t) => t < A ? t / A : t < A + D ? 1 - (1 - S) * (t - A) / D : t < rel ? S : t < rel + R ? S * (1 - (t - rel) / R) : 0; }
    if (isRand) return (t) => { const v = [0.3, -0.7, 0.55, 0.1, -0.4, 0.85, -0.2, 0.6, -0.9, 0.25][Math.floor(t * 10) % 10]; if (sh === 'step') return v; const n = [0.3, -0.7, 0.55, 0.1, -0.4, 0.85, -0.2, 0.6, -0.9, 0.25, 0.3][Math.floor(t * 10) + 1] ?? v; const f = (t * 10) % 1; return v + (n - v) * (0.5 - 0.5 * Math.cos(f * Math.PI)); };
    return (t) => sh === 'exp' ? t * t : sh === 'log' ? Math.sqrt(t) : sh === 's' ? t * t * (3 - 2 * t) : t;
  };
  const meBi = selR.pol === 'Bi';
  const fn = fnOf(meShape), N = (isLfo && meShape === 'sq') || meShape === 'step' || meShape === 'sh' ? 400 : 120;
  const toY = (v, amt) => meBi ? 50 - v * 50 * amt : 100 - Math.max(0, meBi ? v : (isLfo || isRand ? (v + 1) / 2 : v)) * 100 * amt;
  const mkPts = (amt) => Array.from({ length: N + 1 }, (_, i) => { const t = i / N; return (t * 300).toFixed(1) + ',' + toY(fn(t), amt).toFixed(1); }).join(' ');
  const amtAbs = Math.min(1, Math.abs(selR.a));
  const kM = (label, v, t, bi) => k(label, v, t, bi);
  const me = {
    name: selR.src, dest: rl.filter(r => r.src === selR.src).length > 1 ? rl.filter(r => r.src === selR.src).map(r => r.dest).join(', ') : selR.dest,
    addDest: (e) => { const p = this.winPt(e, 264, 300); this.setState({ dmenu: { x: p.x, y: p.y, src: selR.src, slot: selR.slot } }); }, amt: selR.amt, pol: meBi ? 'bipolar' : 'unipolar', slot: selR.slot, bi: meBi,
    col: selR.slot ? 'var(--dd-mod-' + selR.slot.toLowerCase() + ')' : 'var(--dd-paper-2)',
    shapes: meShapes, shape: meShape, setShape: (v) => this.setState({ modShape: Object.assign({}, shapeSt, { [selR.src]: v }) }),
    ghost: mkPts(1), pts: mkPts(Math.max(0.04, amtAbs)), live: isEnv ? 0.42 : isLfo ? 0.31 : (selR.live || 40) / 100,
    yTop: meBi ? '+' + Math.round(amtAbs * 100) + '%' : '100%', yBot: meBi ? '−' + Math.round(amtAbs * 100) + '%' : '0',
    xLabel: isLfo ? '2 cycles · 2.40 Hz' : isEnv ? 'time · note on → release' : isRand ? 'per note' : ({ Velocity: 'velocity 1–127', 'Key track': 'key C-1–G9', Aftertouch: 'pressure 0–127', 'Mod wheel · CC 1': 'CC 1 0–127', 'Pitch bend': 'bend −8192–8191' }[selR.src] || 'macro 0–100%'),
    knobs: isLfo ? [kM('Rate', 0.45, '2.40 Hz'), kM('Phase', 0, '0°'), kM('Fade in', 0.2, '120 ms'), kM('Smooth', 0, '0%'), kM('Amount', amtAbs, selR.amt, meBi)]
      : isEnv ? [kM('Attack', 0.15, selR.src === 'Amp envelope' ? '12 ms' : '1 ms'), kM('Decay', 0.4, '380 ms'), kM('Sustain', selR.src === 'Amp envelope' ? 0.5 : 0.3, selR.src === 'Amp envelope' ? '−6.0 dB' : '30%'), kM('Release', 0.5, '1.20 s'), kM('Amount', amtAbs, selR.amt)]
      : isRand ? [kM('Smooth', meShape === 'smooth' ? 0.6 : 0, meShape === 'smooth' ? '60%' : '0%'), kM('Amount', amtAbs, selR.amt, meBi)]
      : [kM('In low', 0, '0%'), kM('In high', 1, '100%'), kM('Smooth', 0.1, '10 ms'), kM('Amount', amtAbs, selR.amt, meBi)],
    toggles: isLfo ? [{ label: 'Tempo sync', on: false }, { label: 'Retrigger on note', on: true }, { label: 'Per voice', on: selR.src === 'LFO 1' }]
      : isEnv ? [{ label: 'Retrigger', on: true }, { label: 'Velocity scales level', on: selR.src === 'Amp envelope' }, { label: 'Loop', on: false }]
      : isRand ? [{ label: 'New value per note', on: true }, { label: 'Per voice', on: true }]
      : [{ label: 'Invert', on: false }, { label: 'Per voice', on: !/Macro|Mod wheel|Pitch/.test(selR.src) }],
  };
  this.selRoute = selR;
  const srcCats = this.modCats();
  const openCats = this.state.openCats || {};
  const modSrcs = srcCats.map(([id, cat, list]) => {
    const open = !!openCats[id];
    const routesN = list.reduce((s, x) => s + x[2], 0);
    const selIn = list.some(x => x[0] === (this.state.pickedMod || 'Filter envelope'));
    return {
      cat, open, chev: open ? 'chevron-down' : 'chevron-right',
      slots: list.map(x => x[1]).filter(Boolean),
      summary: list.length + ' · ' + routesN, title: list.length + ' sources · ' + routesN + (routesN === 1 ? ' route' : ' routes'),
      hbg: selIn && !open ? 'var(--dd-vermilion-wash)' : 'transparent',
      toggle: () => this.setState(s => ({ openCats: Object.assign({}, s.openCats || {}, { [id]: !open }) })),
      items: list.map(([name, slot, n, live, scope]) => { const extra = Object.values(this.state.added || {}).reduce((c, arr) => c + arr.filter(m => m.source === name).length, 0); n = n + extra; const sel = name === (this.state.pickedMod || 'Filter envelope'); return { name, slot, noSlot: !slot, n, live, drag: (e) => { try { e.dataTransfer.setData('text/plain', name); e.dataTransfer.effectAllowed = 'link'; } catch (x) {} this.setState({ dragMod: { name, slot }, modNote: null }); }, tip: 'Applied to ' + scope + ' · click for details · drag onto a control to add a route', pick: () => this.setState({ pickedMod: name, modInsp: true, editRoute: null }), bg: sel ? 'var(--dd-vermilion-wash)' : 'transparent', edge: sel ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none' }; }),
    };
  });
  Object.assign(context, {R, rl, destOf, gone, colorOf, lastSrc, routes, rOpen, order, bySrc, routeGroups, selR, shapeSt, isLfo, isEnv, isRand, meShapes, meShape, fnOf, meBi, fn, N, toY, mkPts, amtAbs, kM, me, srcCats, openCats, modSrcs});
}
