// Presentation data ported from the supplied Trigger reference.
export function inspectorHistoryProjection(context) {
  let {page, empty, drop, missing, loading, loop, mode, nm, k, sliceNames, sp, sliceRows, zones, rels, isRRlike, vLo, vHi, vW, vCurve, hardRange, rl, me, macroSel} = context;
  const pr = (label, value, unit, ro, hint) => ({ p: true, label, value, unit: unit || '', ro: !!ro, hint: hint || '' });
  const hd = (label) => ({ h: true, label });
  const tx = (label) => ({ t: true, label });
  const bt = (label, icon, variant) => ({ b: true, label, icon: icon || undefined, variant: variant || 'secondary' });
  const md = (label, slot, depth, host) => ({ m: true, label, slot, depth, host: !!host });
  const kn = (list) => ({ knobs: true, list });
  let insp, crumbs;
  const C = (...a) => a.map((label, i) => ({ label, sep: i < a.length - 1, c: i === a.length - 1 ? 'var(--dd-paper-1)' : 'var(--dd-paper-3)' }));
  if (empty || drop) {
    crumbs = C('Untitled'); insp = { icon: 'layers', title: 'Untitled', type: 'Instrument', ann: 'Editor-only state: selection = instrument', rows: [hd('Instrument'), pr('Sounds', 'None'), pr('Voice limit', '32'), pr('Output', 'Main 1/2'), tx('Nothing is selected yet. Drop or browse a sample; the inspector then shows whatever you select in the workspace or tree.')] };
  } else if (macroSel) {
    crumbs = C('Felt Kit', 'Macros', 'Tone'); insp = { icon: 'variation', title: 'Tone', type: 'Macro 1', ann: 'Macro value Parameter + Bindings (MIDI CC, host) + Routes', rows: [
      pr('Value', '56', '%'), pr('MIDI', 'CC 74 · ch 1'), md('Host automation · Macro 1', null, 0, true), bt('Learn MIDI', 'midi'),
      hd('Destinations · 3'), md('Filter cutoff · 20–80%', 'D', 0.6), md('Filter resonance · 40–30%', 'D', -0.1), md('Send · Reverb · 0–25%', 'D', 0.25),
      tx('Each destination has its own range; drag past its start to invert. Bipolar macros map 50% to each destination’s current value.'),
      bt('Add destination…', 'plus'), hd('Macro'), pr('Mode', 'Unipolar'), pr('Name', 'Tone')] };
  } else if (page === 'sample') {
    crumbs = C('Felt Kit', 'Keys', 'Felt C4 f'); insp = { icon: missing ? 'file-missing' : 'level', title: 'felt_C4_f.wav', type: 'Sample', ann: 'Asset (identity, state) + Asset Region', rows: missing ? [
      hd('Sample'), pr('State', 'Missing'), pr('Last seen', 'Felt Piano folder', '', true), pr('Used by', '1 zone'), tx('Playback falls back to the soft layer for these keys. Locate the file or pick a replacement; the zone keeps its region and settings.'),
      bt('Locate file…', 'folder', 'primary'), bt('Search folder…', 'folder'), bt('Replace with…', 'reload'),
    ] : [
      hd('Sample'), pr('State', loading ? 'Loading · 62%' : 'Loaded · cached'), pr('Format', '48 kHz · 24-bit'), pr('Channels', 'Stereo'), pr('Used by', '1 zone'),
      hd('Region'), pr('Start', '0.058', 's'), pr('End', '4.531', 's'), pr('Loop', loop ? '2.651–3.952 s' : 'Off'), pr('Root', '60 C4'),
      hd('Analysis'), pr('Pitch', 'C4 +3¢', '', true), pr('Loudness', '−18.4 LUFS', '', true), pr('Zero crossings', 'Ready', '', true),
      bt('Replace…', 'folder'),
    ] };
  } else if (page === 'slices') {
    crumbs = C('Felt Kit', 'Break', '2 slices'); insp = { icon: 'slice', title: 'Snare 1, Snare 2', type: '2 slices', ann: 'Bulk edit over 2 Asset Regions + their Trigger Rules', rows: [
      hd('Playback'), pr('Notes', '26, 30'), pr('Play', 'Once'), pr('Tune', '0', 'st'), pr('Gain', '0.0', 'dB'), pr('Pan', 'Mixed', '', false, 'C · L4'),
      hd('Voice'), pr('Choke group', 'None'), pr('Output', 'Snare 5/6'), pr('Envelope', 'Group default'),
      tx('Values shared by all selected slices show normally; differing values show Mixed. Editing a Mixed value sets all.'),
    ] };
  } else if (page === 'mapping') {
    const z = zones.find(x => x.id === this.state.zoneSel) || zones.find(x => x.id === 'k60f') || zones[0] || {name:'No zone selected',lo:24,hi:24,velLo:1,velHi:127};
    const zr = z?rels.filter(r => r.a.id === z.id || r.b.id === z.id):[];
    const zx = zr.filter(r => r.kind === 'xf').map(r => this.rangeTxt(r.lo, r.hi) + ' ↔ ' + (r.a.id === z.id ? r.b : r.a).name);
    const zg = zr.filter(r => r.kind === 'gap').map(r => this.rangeTxt(r.lo, r.hi));
    crumbs = C('Felt Kit', 'Keys', z.name); insp = { icon: 'keyboard', title: z.name, type: 'Zone', ann: 'Trigger Rule (note, velocity) → Asset binding', rows: [
      hd('Keys'), pr('Low', z.lo + ' ' + nm(z.lo)), pr('High', z.hi + ' ' + nm(z.hi)), pr('Root', (z.root ?? z.lo) + ' ' + nm(z.root ?? z.lo)),
      hd('Velocity'), pr('Low', String(z.velLo)), pr('High', String(z.velHi)), pr('Vel crossfade', z.velLo === 81 ? '81–90' : 'Off'), hd('Neighbours'), pr('Key crossfade', zx.length ? zx.join(', ') : 'None'), pr('Gap', zg.length ? zg.join(', ') + ' silent' : 'None'), pr('Edge drag', 'Pushes neighbour', '', true, 'Hold Shift to overlap or leave a gap'),
      hd('Sample'), pr('File', z.source === 'Slice' ? 'amen_172.wav' : (z.name.replace(/ /g, '_').toLowerCase() + '.wav')), pr('Overlaps', z.velLo === 81 ? 'Felt ' + nm(z.root) + ' p' : 'None'), bt('Replace sample…', 'folder'),
    ] };
  } else if (page === 'layers') {
    crumbs = isRRlike ? C('Felt Kit', 'Drums', 'Snare', 'Hard', 'Hard B') : C('Felt Kit', 'Drums', 'Snare', 'Hard'); insp = isRRlike ? { icon: 'alternate', title: 'Hard B', type: 'Alternate', ann: 'Selector candidate (weight, order) → Asset binding', rows: [
      hd('Sample'), pr('File', 'snare_hard_b.wav'), pr('Gain', '−0.5', 'dB'), pr('Tune', '0', 'st'), pr('Pan', 'C'), hd('Selection'), pr('Order', '2 of 3'), pr('Weight', mode === 'wrand' ? '1.0' : 'Equal'), pr('Plays', '128 of 384 hits', '', true),
    ] } : { icon: 'layers', title: 'Hard', type: 'Layer', ann: 'Selector candidate that is itself a Group (round robin)', rows: [
      hd('Trigger'), pr('Note', '38 D2'), pr('Velocity', hardRange), pr('Crossfade', vW ? vLo + '–' + vHi + ' · ' + (vCurve === 'eq' ? 'equal power' : 'linear') : 'Off'), hd('Contents'), pr('Play', 'Round robin'), pr('Alternates', '3'), pr('Gain', '0.0', 'dB'), bt('Open alternates', 'chevron-right'),
    ] };
  } else if (page === 'voice') {
    crumbs = C('Felt Kit', 'Keys', 'Voice', 'Filter'); insp = { icon: 'settings', title: 'Filter', type: 'Module', ann: 'Module params + incoming Modulation Routes', rows: [
      hd('Cutoff · 2.40 kHz'), md('Filter envelope', 'A', 0.42), md('Velocity', 'C', 0.15), md('Macro · Tone', 'D', 0.2), md('Key track', null, 0.5), bt('Assign modulation…', 'modulate'),
      hd('Module'), pr('Type', 'Ladder'), pr('Slope', '24', 'dB/oct'), pr('Position', '2 of 3 in voice'), pr('Bypass', 'Off'),
    ] };
  } else if (page === 'mod') {
    const sr = this.selRoute;
    if (sr && sr.src !== 'Filter envelope') { crumbs = C('Felt Kit', 'Keys', 'Modulation', sr.src + ' → ' + sr.dest); insp = { icon: 'modulate', title: sr.src + ' → ' + sr.dest, type: 'Route', ann: 'Modulation Route', rows: [hd('Route'), md(sr.src, sr.slot, sr.a), pr('Destination', sr.dest), pr('Amount', sr.amt), pr('Polarity', sr.pol === 'Bi' ? 'Bipolar' : 'Unipolar'), pr('Curve', sr.curve), pr('Scope', 'Per voice'), hd('Source · ' + sr.src), pr('Shape', me.shapes.find(s => s.id === me.shape).label), pr('Used by', rl.filter(x => x.src === sr.src).length + ' route(s)')] }; } else
    crumbs = C('Felt Kit', 'Keys', 'Modulation', 'Filter envelope → Cutoff'), insp = { icon: 'modulate', title: 'Filter env → Cutoff', type: 'Route', ann: 'Modulation Route', rows: [
      hd('Route'), md('Filter envelope', 'A', 0.42), pr('Destination', 'Filter cutoff'), pr('Amount', '+42', '%'), pr('Polarity', 'Unipolar'), pr('Curve', 'Linear'), pr('Range', '0–100', '%'), pr('Scope', 'Per voice'),
      hd('Source · Filter envelope'), pr('Shape', 'ADSR'), pr('Trigger', 'Note gate'), pr('Used by', '1 route'), bt('Edit envelope', 'chevron-right'),
    ] };
  } else if (page === 'routing') {
    crumbs = C('Felt Kit', 'Routing', 'Snare'); insp = { icon: 'level', title: 'Snare', type: 'Sound', ann: 'Connection to Bus + send Connections', rows: [
      hd('Output'), pr('Bus', 'Snare'), pr('Plugin output', '5/6'), pr('Inherited from', 'Overrides Drums'), hd('Sends'), pr('Reverb', '−10.5', 'dB'), pr('Delay', '−15.0', 'dB'), pr('Send point', 'Post fader'),
    ] };
  } else {
    crumbs = C('Felt Kit', 'Drums', 'Inserts', 'Compressor'); insp = { icon: 'settings', title: 'Compressor', type: 'Processor', ann: 'Module (Compressor) in the Drums group chain', rows: [
      kn([k('Threshold', 0.7, '−18 dB'), k('Ratio', 0.18, '4:1'), k('Attack', 0.2, '10 ms'), k('Release', 0.3, '120 ms'), k('Makeup', 0.5, '+3 dB'), k('Mix', 1, '100%')]),
      hd('Gain reduction'), pr('Now', '−4.2', 'dB', true), pr('Detector', 'Peak'), pr('Sidechain', 'Self'), bt('Bypass', 'reset'),
    ] };
  }

  const isSampleish = page === 'sample' && !empty && !drop;
  const PV = this.state.pv, pvPos = this.state.pvPos || 0;
  const openSrc = Object.assign({}, this.state.openSrc || {});
  const activeSrc = page === 'sample' ? 'felt' : page === 'slices' ? 'amen' : page === 'layers' ? 'drum' : null;
  const srcDefs = [
    ['felt', 'Felt Piano', [['felt_C3_p.wav', '5.02 s', 'level'], ['felt_C3_f.wav', '5.10 s', 'level'], ['felt_G3_p.wav', 'missing', 'file-missing'], ['felt_G3_f.wav', '4.95 s', 'level'], ['felt_C4_p.wav', '4.61 s', 'level'], ['felt_C4_f.wav', '4.82 s', missing ? 'file-missing' : 'level'], ['felt_F#4_p.wav', '4.40 s', 'level'], ['felt_F#4_f.wav', '4.44 s', 'level'], ['felt_C5_p.wav', '4.10 s', 'level'], ['felt_C5_f.wav', '4.15 s', 'level']]],
    ['drum', 'Drum samples', [['kick.wav', '0.42 s', 'one-shot'], ['snare_soft.wav', '0.38 s', 'one-shot'], ['snare_hard_a.wav', '0.41 s', 'one-shot'], ['snare_hard_b.wav', '0.40 s', 'one-shot'], ['snare_hard_c.wav', '0.43 s', 'one-shot'], ['hat_closed.wav', '0.09 s', 'one-shot'], ['hat_open_a.wav', '0.62 s', 'one-shot']]],
    ['amen', 'amen_172.wav', sliceNames.map((n, i) => [n, 'slice ' + (i + 1), 'slice'])],
    ['patch', 'Dandrum patches', [['Lush', 'synth', 'variation']]],
  ];
  const playSlice = (i) => { const a = sp[i], b = sp[i + 1] ?? 1; this.setState({ selSlice: i }); this.play(sliceNames[i], { target: 'break', from: a, to: b, dur: (b - a) * 1.395 * 2 }); };
  const dragS = (name, kind) => (e) => { try { e.dataTransfer.setData('text/plain', name); e.dataTransfer.effectAllowed = 'copy'; } catch (x) {} this.setState({ dragSrc: { name, kind }, nodeOver: null, modNote: null }); };
  const srcGroups = srcDefs.map(([id, name, list]) => ({
    name, open: !!openSrc[id], hbg: id === activeSrc && !openSrc[id] ? 'var(--dd-vermilion-wash)' : 'transparent', chev: openSrc[id] ? 'chevron-down' : 'chevron-right', summary: list.length + (id === 'amen' ? ' slices' : ''),
    toggle: () => this.setState(s => ({ openSrc: Object.assign({}, s.openSrc || {}, { [id]: !openSrc[id] }) })),
    items: list.filter(([n]) => !(this.state.srcHidden || {})[n]).map(([n, d, icon], i) => {
      const playing = !!(PV && PV.name === n), sel = id === 'amen' ? (page === 'slices' && (this.state.selSlice ?? 2) === i) : (page === 'sample' && n === 'felt_C4_f.wav');
      return { name: n, detail: d, icon, ic: icon === 'file-missing' ? 'var(--dd-error)' : 'var(--dd-paper-3)', playing, pct: playing ? Math.round(pvPos * 100) : 0,
        bg: sel ? 'var(--dd-vermilion-wash)' : 'transparent', edge: sel ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none',
        drag: dragS(n, id === 'amen' ? 'slice' : 'sample'),
        play: () => id === 'amen' ? playSlice(i) : icon === 'file-missing' ? this.setState({ modNote: n + ' is missing and cannot be previewed' }) : this.play(n, n === 'felt_C4_f.wav' ? { target: 'sample', from: 0.012, to: 0.94, dur: 3 } : { dur: id === 'drum' ? 0.6 : 2.4 }) };
    }),
  }));
  sliceRows.forEach((r, i) => { r.drag = dragS(sliceNames[i], 'slice'); r.play = () => playSlice(i); if ((this.state.selSlice ?? null) != null) { const s = i === this.state.selSlice; r.bg = s ? 'var(--dd-vermilion-wash)' : 'transparent'; r.edge = s ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none'; } });

  const hOff = this.state.histOff || {}, hSel = this.state.histSel || null;
  let histDefs = [];
  if (page === 'sample' && isSampleish) histDefs = [
    ...(loop ? [['loop', 'Loop', 'loop', '2.651–3.952 s · 120 ms', [hd('Loop'), pr('Start', '2.651', 's'), pr('End', '3.952', 's'), pr('Crossfade', '120', 'ms'), pr('Mode', 'Sustain loop')]]] : []),
    ['fade', 'Fade', 'level', 'in 2 ms · out 380 ms', [hd('Fade'), pr('In', '2', 'ms'), pr('In curve', 'Exponential'), pr('Out', '380', 'ms'), pr('Out curve', 'Linear')]],
    ['norm', 'Normalize', 'variation', '−1.0 dBFS', [hd('Normalize'), pr('Target', '−1.0', 'dBFS'), pr('Mode', 'Peak'), pr('Gain applied', '+0.2', 'dB', true)]],
    ['trim', 'Trim', 'slice', '0.058–4.531 s', [hd('Trim'), pr('Start', '0.058', 's'), pr('End', '4.531', 's'), pr('Snap', 'Zero crossings'), tx('Drag the region edges on the waveform to change it.')]],
    ['src', 'Source', 'folder', 'felt_C4_f.wav', [hd('Source'), pr('File', 'felt_C4_f.wav'), pr('Format', '48 kHz · 24-bit'), pr('Length', '4.820', 's'), bt('Replace…', 'folder')]],
  ];
  if (page === 'slices') histDefs = [
    ['slice', 'Slice', 'slice', '8 · even', [hd('Slice'), pr('Method', 'Even'), pr('Count', '8'), pr('Snap', 'Zero crossings'), bt('Detect transients', 'slice')]],
    ['trim', 'Trim', 'slice', '0.000–1.395 s', [hd('Trim'), pr('Start', '0.000', 's'), pr('End', '1.395', 's')]],
    ['src', 'Source', 'folder', 'amen_172.wav', [hd('Source'), pr('File', 'amen_172.wav'), pr('Tempo', '172', 'BPM', true), pr('Length', '1.395', 's')]],
  ];
  if (hOff.norm === undefined && !this.state.histOff) hOff.norm = true;
  histDefs = histDefs.filter(d => !(this.state.histDel || {})[d[0]]);
  const hist = histDefs.map(([id, name, icon, detail], i) => {
    const base = id === 'src', on = base || !hOff[id], sel = hSel === id;
    return { id, name, detail, icon, on, grip: base ? '' : '⋮⋮', tOp: base ? 0.4 : 1, tTip: base ? 'The source is always on' : on ? 'Turn off' : 'Turn on',
      ic: on ? 'var(--dd-paper-2)' : 'var(--dd-paper-4)', tc: on ? 'var(--dd-paper-1)' : 'var(--dd-paper-4)',
      bg: sel ? 'var(--dd-vermilion-wash)' : 'transparent', edge: sel ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none',
      select: () => this.setState({ histSel: sel ? null : id, modInsp: false, editRoute: null }),
      toggle: (e) => { e.stopPropagation(); if (base) return; this.setState(s => ({ histOff: Object.assign({ norm: true }, s.histOff || {}, { [id]: on }) })); } };
  });
  const hOn = (id) => !histDefs.some(d => d[0] === id) || !hOff[id];
  if (hSel && histDefs.some(d => d[0] === hSel)) { const d = histDefs.find(x => x[0] === hSel); insp = Object.assign({}, insp, { title: d[1], type: 'Operation', icon: d[2], rows: d[4].concat([tx(hOn(hSel) ? 'Operations apply bottom to top. The source file is never changed.' : 'This operation is off and is skipped.')]) }); crumbs = crumbs.concat([]).slice(0, -1).concat([{ label: crumbs[crumbs.length - 1].label, sep: true, c: 'var(--dd-paper-3)' }, { label: d[1], sep: false, c: 'var(--dd-paper-1)' }]); }

  const flatMods = this.modCats().flatMap(c => c[2]);
  if (this.state.editRoute) {
    const er = this.state.editRoute, m = (this.knobMods[er.label] || []).find(x => x.source === er.source) || {}, fm = flatMods.find(x => x[0] === er.source);
    crumbs = C('Felt Kit', 'Modulation', er.source + ' → ' + er.label);
    insp = { icon: 'modulate', title: er.source + ' → ' + er.label, type: 'Route', ann: 'Modulation Route', rows: [hd('Route'), md(er.source, m.slot || null, m.depth || 0.25), pr('Destination', er.label), pr('Amount', (m.depth >= 0 ? '+' : '') + Math.round((m.depth || 0.25) * 100), '%'), pr('Polarity', 'Unipolar'), pr('Curve', 'Linear'), pr('Scope', 'Per voice'), hd('Source'), pr('Applied to', fm ? fm[4] : '—'), tx('Right-click the control again to remove this route or add another.')] };
  } else if (this.state.modInsp && this.state.pickedMod) {
    const nmM = this.state.pickedMod, fm = flatMods.find(x => x[0] === nmM) || [nmM, null, 0, 0, '—'];
    const used = rl.filter(r => r.src === nmM);
    crumbs = C('Felt Kit', 'Modulators', nmM);
    insp = { icon: 'modulate', title: nmM, type: 'Modulator', ann: 'Modulator (patch-level, shared across groups)', rows: [
      hd('Applied to'), pr('Groups', fm[4]), pr('Scope', /Velocity|Random|envelope|Key/.test(nmM) ? 'Per voice' : 'Global'),
      tx('A modulator belongs to the patch, so one envelope or LFO can drive controls in Keys, Drums and Break at once. Each route sets its own amount.'),
      hd('Routes · ' + used.length)].concat(used.map(r => md(r.dest + ' · ' + r.amt, r.slot, r.a)), [bt('Apply to group…', 'layers'), bt('Edit ' + nmM.toLowerCase(), 'settings')]) };
  }
  Object.assign(context, {pr, hd, tx, bt, md, kn, insp, crumbs, C, isSampleish, PV, pvPos, openSrc, activeSrc, srcDefs, playSlice, dragS, srcGroups, hOff, hSel, histDefs, hist, hOn, flatMods});
}
