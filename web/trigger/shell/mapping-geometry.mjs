import React from 'react';

export const mappingGeometry = {
getZones() {
    if(this.state.zones)return this.state.zones;
    const patch=this.props.store.getSnapshot().patch;
    return patch.rules.map(rule=>({...rule,g:rule.group,lo:rule.noteLo,hi:rule.noteHi,sourceId:rule.source,
      source:!rule.source?'Empty':patch.regions.find(r=>r.id===rule.source)?.kind==='slice'?'Slice':patch.selectors[rule.source]?'Selector':'Sample'}));
  },
sameLayer(a, b) { return a.g === b.g && a.velLo === b.velLo && a.velHi === b.velHi; },
rangeTxt(lo, hi) { return lo === hi ? this.nn(lo) : this.nn(lo) + '–' + this.nn(hi); },
uid() { this._uid = (this._uid || 0) + 1; return 'z' + Date.now().toString(36) + this._uid; },
commit(zones, note, extra) {
    const rules=zones.map(({g,lo,hi,sourceId,source,...rule})=>({...rule,group:g,noteLo:lo,noteHi:hi,source:sourceId??null}));
    this.setState({zones,mapNote:note||null,menu:null,...extra});
    return this.props.store.operation(note||'Edit mapping','setRules',rules)
      .catch(error=>this.setState({modNote:error.message,mapNote:error.message}))
      .finally(()=>this.setState({zones:null}));
  },
chZones(next, id) {
    if (!this.snap) this.snap = this.getZones();
    const snap = this.snap, orig = snap.find(z => z.id === id), nz = next.find(z => z.id === id);
    if (!orig || !nz) { this.setState({ zones: next }); return; }
    if (this.shift) { this.setState({ zones: snap.map(z => z.id === id ? nz : z), dragShift: true }); return; }
    const L = snap.find(z => z.id !== id && this.sameLayer(z, orig) && z.hi === orig.lo - 1);
    const R = snap.find(z => z.id !== id && this.sameLayer(z, orig) && z.lo === orig.hi + 1);
    const dl = nz.lo - orig.lo, dh = nz.hi - orig.hi;
    let lo = nz.lo, hi = nz.hi, root = nz.root;
    if (dl === dh && dl !== 0) {
      let d = dl;
      if (L && d < 0) d = Math.max(d, L.lo + 1 - orig.lo);
      if (R && d > 0) d = Math.min(d, R.hi - 1 - orig.hi);
      lo = orig.lo + d; hi = orig.hi + d; if (orig.root != null) root = orig.root + d;
    } else {
      if (L && dl) lo = Math.max(lo, L.lo + 1);
      if (R && dh) hi = Math.min(hi, R.hi - 1);
    }
    const sameVel = nz.velLo === orig.velLo && nz.velHi === orig.velHi;
    this.setState({ dragShift: false, zones: snap.map(z => {
      if (z.id === id) return Object.assign({}, nz, { lo, hi, root });
      if (sameVel && L && z.id === L.id) return Object.assign({}, L, { hi: lo - 1 });
      if (sameVel && R && z.id === R.id) return Object.assign({}, R, { lo: hi + 1 });
      return z;
    }) });
  },
gridGeom() {
    const wrap = this.mapRef && this.mapRef.current; if (!wrap) return null;
    const scroll = [...wrap.querySelectorAll('div')].find(d => d.style.overflowY === 'hidden' && d.style.overflowX);
    if (!scroll || !scroll.firstElementChild) return null;
    const g = scroll.firstElementChild.children;
    return { grid: g[1].getBoundingClientRect(), kb: g[5] ? g[5].getBoundingClientRect() : null };
  },
pointNote(e) {
    const G = this.gridGeom(); if (!G) return null;
    const r = G.grid, x = e.clientX - r.left;
    if (x < 0 || x > r.width) return null;
    const note = Math.max(24, Math.min(96, 24 + Math.floor(x / (r.width / 73))));
    const inGrid = e.clientY >= r.top && e.clientY <= r.bottom;
    const inKb = G.kb && e.clientY >= G.kb.top && e.clientY <= G.kb.bottom;
    if (!inGrid && !inKb) return null;
    return { note, vel: inGrid ? Math.max(1, Math.min(127, 127 - Math.floor((e.clientY - r.top) / r.height * 127))) : null };
  },
onMapMove(e) { const p = this.pointNote(e); if (p) this.hoverNote = p.note; },
onMapCtx(e) {
    const p = this.pointNote(e); if (!p) return;
    e.preventDefault();
    const zones = this.getZones();
    const hits = zones.filter(z => p.note >= z.lo && p.note <= z.hi && (p.vel == null || (p.vel >= z.velLo && p.vel <= z.velHi)));
    const hit = hits.find(z => z.id === this.state.zoneSel) || hits[hits.length - 1] || null;
    const wr = this.winRef.current.getBoundingClientRect(), sc = wr.width / this.winRef.current.offsetWidth;
    let x = (e.clientX - wr.left) / sc + 2, y = (e.clientY - wr.top) / sc + 2;
    const W = this.winRef.current.offsetWidth, H = this.winRef.current.offsetHeight;
    if (x + 264 > W) x -= 266; if (y + 330 > H) y = Math.max(4, H - 334);
    this.setState({ menu: { x, y, note: p.note, vel: p.vel, zid: hit && hit.id }, zoneSel: hit ? hit.id : this.state.zoneSel });
  }
};
