// Presentation data ported from the supplied Trigger reference.
import React from 'react';
import {parameterUnits} from '../parameter-units.mjs';
export function sampleMappingProjection(context) {
  let {page, missing, nm} = context;
  const addedMods = this.state.added || {};
  const removedMods = this.state.removed || {};
  this.knobMods = {};
  const kAll = (label, mods) => { const all = (mods || []).concat(addedMods[label] || []).filter(m => !(removedMods[label] || []).includes(m.source)); this.knobMods[label] = all; return all; };
  const kvS = this.state.kv || {}; this.kDefaults = this.kDefaults || {}; let kCtx = '';
  const k = (label, v, t, bi, mods, host, pop) => {
    const key = kCtx + label;
    if (this.kDefaults[key] === undefined) this.kDefaults[key] = v;
    const param = this.props.store.getSnapshot().patch.params[key];
    const cv = param?.value ?? kvS[key] ?? v;
    const set = param
      ? value => this.changeParameter(key, value)
      : value => this.setState(s => ({kv: {...s.kv, [key]: value}}));
    const focused = this.state.dropOver === key || this.state.kmenu?.label === key || this.state.flashKey === key;
    const units=parameterUnits(key);
    return {
      label, key, v: cv, default:param?.default??v, t:units.format(cv), parse:units.parse,
      bi: !!bi, mods: kAll(key, mods).filter(m => m.slot), host: !!host,
      pop: !!pop && !this.state.dragMod && !this.state.kmenu, set,
      ring: focused ? '0 0 0 1px var(--dd-vermilion)' : 'none',
    };
  };
  const kL = (...a) => { kCtx = 'Lush '; const r = k(...a); kCtx = ''; return r; };
  const sampleKnobGroups = [
    { title: 'Region', ann: 'Asset Region', knobs: [k('Start', 0.012, '0.058 s', 0, [{ slot: 'D', depth: 0.15, source: 'Aftertouch' }]), k('End', 0.94, '4.531 s'), k('Fade in', 0.04, '2 ms'), k('Fade out', 0.3, '380 ms')] },
    { title: 'Pitch', ann: 'Parameter · AssetPlayer', knobs: [k('Tune', 0.5, '0 st', 1), k('Fine', 0.47, '−3 ct', 1)] },
    { title: 'Output', ann: 'Parameter · Gain', knobs: [k('Gain', 0.72, '−1.5 dB'), k('Pan', 0.5, 'C', 1)] },
  ];
  const meta = [{ k: 'Format', v: '48 kHz · 24-bit · stereo' }, { k: 'Length', v: '4.820 s' }, { k: 'Peak', v: '−1.2 dBFS' }, { k: 'Loudness', v: '−18.4 LUFS' }, { k: 'Pitch', v: 'C4 +3¢' }, { k: 'Source', v: missing ? 'missing' : 'external · cached' }];

  const sliceNames = ['Kick 1','Hat 1','Snare 1','Ghost','Kick 2','Hat 2','Snare 2','Hat 3'];
  const sp = [0, .125, .25, .3125, .5, .625, .75, .875];
  const slices = sp.map((pos, i) => ({ pos, name: sliceNames[i] }));
  const len = 1.395;
  const sliceRows = sp.map((p, i) => {
    const s = i === 2 || i === 6;
    return { n: i + 1, name: sliceNames[i], note: (24 + i) + ' ' + nm(24 + i), range: (p * len).toFixed(3) + '–' + ((sp[i + 1] ?? 1) * len).toFixed(3), tune: i === 3 ? '+2 st' : '0 st', gain: i === 3 ? '−6.0 dB' : '0.0 dB', pan: i === 1 || i === 5 ? 'L18' : 'C', mode: sliceNames[i].startsWith('Hat') ? 'Gated' : 'Once', choke: sliceNames[i].startsWith('Hat') ? '1' : '—', out: s ? 'Snare' : 'Inherit · Main', bg: s ? 'var(--dd-vermilion-wash)' : 'transparent', edge: s ? 'inset 2px 0 0 var(--dd-vermilion)' : 'none' };
  });
  const tpos = [0, .0625, .125, .1875, .25, .3125, .375, .4375, .5, .5625, .625, .6875, .75, .8125, .875];
  const transients = tpos.map((x, i) => { const st = [1, .3, .7, .35, .95, .6, .4, .25, .9, .3, .65, .45, .95, .32, .7][i]; return { x: x * 100 + 0.3, h: Math.round(6 + st * 18), c: st > 0.4 ? 'var(--dd-paper-1)' : 'var(--dd-paper-4)' }; });

  const zones = this.getZones();
  this.curPage = page;
  if (!this.mapRef) this.mapRef = React.createRef(); if (!this.winRef) this.winRef = React.createRef();
  const rels = this.relations(zones);
  const XFS = { bg: 'repeating-linear-gradient(135deg, var(--dd-vermilion) 0 2px, var(--dd-vermilion-wash) 2px 5px)', bd: '1px solid var(--dd-vermilion)' };
  const GAPS = { bg: 'transparent', bd: '1px dashed var(--dd-warn)' };
  const relSegs = rels.map(r => Object.assign({ l: (r.lo - 24) / 73 * 100, w: (r.hi - r.lo + 1) / 73 * 100, title: (r.kind === 'xf' ? 'Crossfade ' : 'Gap ') + this.rangeTxt(r.lo, r.hi) }, r.kind === 'xf' ? XFS : GAPS));
  const relList = rels.slice(0, 4).map(r => Object.assign({ kind: r.kind === 'xf' ? 'Crossfade' : 'Gap', text: r.kind === 'xf' ? this.rangeTxt(r.lo, r.hi) + ' · ' + r.a.name + ' ↔ ' + r.b.name : this.rangeTxt(r.lo, r.hi) + ' · silent' }, r.kind === 'xf' ? XFS : GAPS));
  if (rels.length > 4) relList.push({ kind: '+' + (rels.length - 4), text: 'more', bg: 'transparent', bd: '1px solid transparent' });
  const M = this.state.menu;
  let menuItems = [], menuTitle = '';
  if (M) {
    const mz = M.zid ? zones.find(z => z.id === M.zid) : null, at = nm(M.note), c = this.state.clip;
    const pasteItems = [
      { label: c ? 'Paste ' + c.label + ' at ' + at : 'Paste key range', shortcut: 'Ctrl+V', disabled: !c, onSelect: () => this.paste(M.note, false) },
      { label: 'Paste as layer at ' + at, shortcut: 'Ctrl+Shift+V', disabled: !c, onSelect: () => this.paste(M.note, true) },
    ];
    if (mz) {
      const gName = { keys: 'Keys', drums: 'Drums', break: 'Break' }[mz.g];
      const n = zones.filter(o => o.g === mz.g && o.lo >= mz.lo && o.hi <= mz.hi).length;
      menuTitle = mz.name + ' · ' + this.rangeTxt(mz.lo, mz.hi);
      menuItems = [
        { label: 'Insert zone at ' + at, icon: 'plus', disabled: mz.lo === mz.hi, onSelect: () => this.insertAt(mz, M.note) },
        { separator: true },
        { label: 'Copy key range ' + this.rangeTxt(mz.lo, mz.hi) + ' (' + n + ')', shortcut: 'Ctrl+C', onSelect: () => this.copyRange(mz) },
        { label: 'Copy group ' + gName, onSelect: () => this.copyGroup(mz.g) },
      ].concat(pasteItems, [
        { separator: true },
        { label: 'Delete zone', shortcut: 'Delete', danger: true, onSelect: () => this.del(mz, false) },
        { label: 'Delete, leave gap', shortcut: 'Shift+Del', danger: true, onSelect: () => this.del(mz, true) },
      ]);
    } else {
      menuTitle = 'Key ' + M.note + ' ' + at + (M.vel != null ? ' · vel ' + M.vel : '');
      menuItems = [{ label: 'Add zone in empty keys', icon: 'plus', onSelect: () => this.addInGap(M.note, M.vel) }, { separator: true }].concat(pasteItems);
    }
    menuItems = menuItems.map(it => it.onSelect ? Object.assign({}, it, { onSelect: () => { it.onSelect(); } }) : it);
  }
  const dropRules = [
    { k: 'File on a zone', v: 'Replace its sample. Shift adds as a new layer on the same keys.' },
    { k: 'File on empty keys', v: 'New zone at that key, root = key. Drag height sets velocity range.' },
    { k: 'Several files', v: 'Spread from the drop key. Roots from file names when found.' },
    { k: 'Zone edges · Shift', v: 'Edges push the neighbouring zone. Shift-drag to overlap (crossfade) or leave a gap (silent).' },
    { k: 'Right-click', v: 'Insert a zone, copy the key range, paste it at another key, delete.' },
  ];
  Object.assign(context, {addedMods, removedMods, kAll, kvS, kCtx, k, kL, sampleKnobGroups, meta, sliceNames, sp, slices, len, sliceRows, tpos, transients, zones, rels, XFS, GAPS, relSegs, relList, M, menuItems, menuTitle, dropRules});
}
