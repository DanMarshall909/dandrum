import React from 'react';

export const mappingClipboard = {
insertAt(z, note) {
    const zones = this.getZones();
    if (z.lo === z.hi) return;
    let a, b;
    if (note <= z.lo) { a = { lo: z.lo, hi: z.lo }; b = { lo: z.lo + 1, hi: z.hi }; }
    else { a = { lo: note, hi: z.hi }; b = { lo: z.lo, hi: note - 1 }; }
    const nz = { id: this.uid(), g: z.g, name: 'New zone', source: 'Empty', lo: a.lo, hi: a.hi, velLo: z.velLo, velHi: z.velHi, root: a.lo };
    const out = zones.map(o => o.id === z.id ? Object.assign({}, o, b) : o); out.push(nz);
    return this.commit(out, 'Inserted zone ' + this.rangeTxt(nz.lo, nz.hi) + ' · ' + z.name + ' now ' + this.rangeTxt(b.lo, b.hi), { zoneSel: nz.id });
  },
addInGap(note, vel) {
    const zones = this.getZones();
    const band = zones.filter(z => vel == null || (vel >= z.velLo && vel <= z.velHi));
    const left = band.filter(z => z.hi < note).reduce((m, z) => Math.max(m, z.hi), 23) + 1;
    const right = band.filter(z => z.lo > note).reduce((m, z) => Math.min(m, z.lo), 97) - 1;
    const near = band.filter(z => z.hi === left - 1 || z.lo === right + 1)[0];
    let lo = left, hi = right;
    if (hi - lo > 11) { lo = note; hi = note; }
    const nz = { id: this.uid(), g: near ? near.g : this.props.store.getSnapshot().patch.nodes.find(n=>n.kind==='group')?.id??'root', name: 'New zone', source: 'Empty', lo, hi, velLo: near ? near.velLo : 1, velHi: near ? near.velHi : 127, root: note };
    return this.commit(zones.concat([nz]), 'Added zone ' + this.rangeTxt(lo, hi) + ' · drop a sample on it to fill it', { zoneSel: nz.id });
  },
copyRange(z) {
    const zs = this.getZones().filter(o => o.g === z.g && o.lo <= z.hi && o.hi >= z.lo)
      .map(o=>({...o,lo:Math.max(o.lo,z.lo),hi:Math.min(o.hi,z.hi)}));
    this.setState({ clip: { zones: zs.map(o => Object.assign({}, o)), lo: z.lo, hi: z.hi, label: this.rangeTxt(z.lo, z.hi) + ' · ' + zs.length + (zs.length === 1 ? ' zone' : ' zones') }, menu: null, mapNote: 'Copied ' + this.rangeTxt(z.lo, z.hi) + ' (' + zs.length + (zs.length === 1 ? ' zone' : ' zones') + ', all velocity layers)' });
  },
copyGroup(g) {
    const zs = this.getZones().filter(o => o.g === g);
    const lo = Math.min(...zs.map(o => o.lo)), hi = Math.max(...zs.map(o => o.hi));
    const nmG = { keys: 'Keys', drums: 'Drums', break: 'Break' }[g] || g;
    this.setState({ clip: { zones: zs.map(o => Object.assign({}, o)), lo, hi, label: nmG + ' ' + this.rangeTxt(lo, hi) + ' · ' + zs.length + ' zones' }, menu: null, mapNote: 'Copied group ' + nmG + ' (' + zs.length + ' zones)' });
  },
paste(note, asLayer) {
    const c = this.state.clip; if (!c) return;
    let off = note - c.lo;
    off = Math.min(off, 96 - c.hi); off = Math.max(off, 24 - c.lo);
    const nm = this.nn.bind(this);
    const pasted = c.zones.map(z => {
      const root = z.root != null ? z.root + off : undefined;
      const name = z.root != null && z.name.includes(nm(z.root)) ? z.name.replace(nm(z.root), nm(root)) : z.name + ' copy';
      return Object.assign({}, z, { id: this.uid(), lo: z.lo + off, hi: z.hi + off, root, name });
    });
    let zones = this.getZones(), trimmed = 0, removed = 0;
    if (!asLayer) {
      const out = [];
      zones.forEach(E => {
        let pieces = [{ lo: E.lo, hi: E.hi, velLo:E.velLo, velHi:E.velHi }];
        pasted.forEach(P => {
          pieces = pieces.flatMap(s => {
            if (P.hi<s.lo||P.lo>s.hi||P.velLo>s.velHi||P.velHi<s.velLo)return [s];
            const lo=Math.max(s.lo,P.lo),hi=Math.min(s.hi,P.hi),r=[];
            if(s.lo<lo)r.push({...s,hi:lo-1});if(s.hi>hi)r.push({...s,lo:hi+1});
            if(s.velLo<P.velLo)r.push({...s,lo,hi,velHi:P.velLo-1});
            if(s.velHi>P.velHi)r.push({...s,lo,hi,velLo:P.velHi+1});return r;
          });
        });
        if (!pieces.length) { removed++; return; }
        if (pieces.length !== 1 || pieces[0].lo !== E.lo || pieces[0].hi !== E.hi) trimmed++;
        pieces.forEach((s, i) => out.push(Object.assign({}, E, s, {id:i?this.uid():E.id})));
      });
      zones = out;
    }
    const tail = asLayer ? ' as a layer · overlaps crossfade' : (removed || trimmed ? ' · replaced ' + removed + ', trimmed ' + trimmed : '');
    return this.commit(zones.concat(pasted), 'Pasted ' + pasted.length + (pasted.length === 1 ? ' zone' : ' zones') + ' at ' + this.rangeTxt(c.lo + off, c.hi + off) + tail, { zoneSel: pasted[pasted.length - 1].id });
  },
del(z, leaveGap) {
    const zones = this.getZones();
    const L = zones.find(o => o.id !== z.id && this.sameLayer(o, z) && o.hi === z.lo - 1);
    const R = zones.find(o => o.id !== z.id && this.sameLayer(o, z) && o.lo === z.hi + 1);
    let lh = null, rl = null;
    if (!leaveGap) {
      if (L && R) { lh = Math.floor((z.lo + z.hi) / 2); rl = lh + 1; }
      else if (L) lh = z.hi; else if (R) rl = z.lo;
    }
    const out = zones.filter(o => o.id !== z.id).map(o => (L && o.id === L.id && lh != null) ? Object.assign({}, o, { hi: lh }) : (R && o.id === R.id && rl != null) ? Object.assign({}, o, { lo: rl }) : o);
    const how = leaveGap ? ' · keys ' + this.rangeTxt(z.lo, z.hi) + ' are now silent' : (L || R) ? ' · neighbours filled ' + this.rangeTxt(z.lo, z.hi) : '';
    return this.commit(out, 'Deleted ' + z.name + how, { zoneSel: (L || R || out[0] || {}).id });
  },
relations(zones) {
    const segs = [], groups = {};
    zones.forEach(z => { const k = z.g + ':' + z.velLo + '-' + z.velHi; (groups[k] = groups[k] || []).push(z); });
    Object.values(groups).forEach(list => {
      const s = list.slice().sort((a, b) => a.lo - b.lo || a.hi - b.hi);
      for (let i = 0; i < s.length - 1; i++) {
        const a = s[i], b = s[i + 1];
        if (b.lo <= a.hi) segs.push({ kind: 'xf', lo: b.lo, hi: Math.min(a.hi, b.hi), a, b });
        else if (b.lo > a.hi + 1 ) segs.push({ kind: 'gap', lo: a.hi + 1, hi: b.lo - 1, a, b });
      }
    });
    return segs;
  }
};
