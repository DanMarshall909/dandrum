import React from 'react';

const SM_NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
const noteName = (n) => SM_NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1);

const SM_BLACK = [1, 3, 6, 8, 10];
const smIsBlack = (n) => SM_BLACK.includes(((n % 12) + 12) % 12);
const SM_WHITE_IDX = { 0: 0, 2: 1, 4: 2, 5: 3, 7: 4, 9: 5, 11: 6 };
const smClamp = (v, a, b) => Math.max(a, Math.min(b, v));
const SM_AXIS = 30;
const smLabel = { fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--text-tertiary)' };
const smValue = { fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-primary)', whiteSpace: 'nowrap' };

/**
 * Key × velocity map with an aligned, zoomable piano keyboard.
 * Each zone routes a key/velocity range to a source: a synth engine, a sample, or any compatible Dandrum patch.
 * Overlapping zones layer; identical zones draw as an offset stack and the header lists every layer under the selection.
 * Zones are rectangles: x = MIDI key range, y = velocity range (top = 127).
 * Zoom: − / + / Fit in the header, or Ctrl/Cmd + wheel (anchored at the pointer). Scroll horizontally when zoomed.
 * Drag a zone body to move it, drag an edge or corner to resize; arrows nudge (Shift resizes the high edge).
 * Play keys on the keyboard: lower on the key = louder. Matching zones light up.
 */
export function KeyMap({
  zones = [], selectedId, onSelect, onChange, onNoteOn, onNoteOff,
  lowNote = 24, highNote = 96, keyWidth, gridHeight = 140, keyboardHeight = 56,
  editable = true, title = 'Key map', zoom: zoomProp, onZoomChange, minZoom = 1, maxZoom = 8, style,
}) {
  const [local, setLocal] = React.useState(zones);
  const localRef = React.useRef(zones);
  React.useEffect(() => { setLocal(zones); localRef.current = zones; }, [zones]);
  const [selLocal, setSelLocal] = React.useState(selectedId ?? zones[0]?.id);
  React.useEffect(() => { if (selectedId !== undefined) setSelLocal(selectedId); }, [selectedId]);
  const [hover, setHover] = React.useState(null);
  const [held, setHeld] = React.useState(null);
  const [lit, setLit] = React.useState([]);
  const [drag, setDrag] = React.useState(null);
  const [focusId, setFocusId] = React.useState(null);
  const wrapRef = React.useRef(null);
  const scrollRef = React.useRef(null);
  const anchorRef = React.useRef(null);
  const [zoomLocal, setZoomLocal] = React.useState(1);
  const zoom = zoomProp ?? zoomLocal;
  const setZoom = (z, anchorX) => {
    const nz = smClamp(Math.round(z * 100) / 100, minZoom, maxZoom);
    if (nz === zoom) return;
    const el = scrollRef.current;
    if (el) {
      const ax = anchorX ?? el.clientWidth / 2;
      anchorRef.current = { ax, note: (el.scrollLeft + ax - SM_AXIS) / k };
    }
    setZoomLocal(nz); onZoomChange && onZoomChange(nz);
  };
  const [avail, setAvail] = React.useState(800);

  React.useLayoutEffect(() => {
    if (!wrapRef.current) return;
    const ro = new ResizeObserver(([e]) => setAvail(e.contentRect.width));
    ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);

  const count = highNote - lowNote + 1;
  const k = (keyWidth ?? Math.max(4, (avail - SM_AXIS) / count)) * zoom;
  const W = count * k, H = gridHeight;
  const xOf = (n) => (n - lowNote) * k;
  const yTop = (v) => ((127 - v) / 127) * H;
  const yBot = (v) => ((128 - v) / 127) * H;

  React.useLayoutEffect(() => {
    const a = anchorRef.current, el = scrollRef.current;
    if (!a || !el) return;
    el.scrollLeft = a.note * k + SM_AXIS - a.ax; anchorRef.current = null;
  }, [k]);
  const zoomRef = React.useRef(); zoomRef.current = { zoom, setZoom };
  React.useEffect(() => {
    const el = scrollRef.current; if (!el) return;
    const wheel = (e) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      e.preventDefault();
      const r = el.getBoundingClientRect();
      const { zoom: z, setZoom: sz } = zoomRef.current;
      sz(z * Math.exp(-e.deltaY * 0.004), e.clientX - r.left);
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => el.removeEventListener('wheel', wheel);
  }, []);

  const select = (id) => { setSelLocal(id); onSelect && onSelect(id); };
  const update = (id, patch) => {
    const next = localRef.current.map((z) => (z.id === id ? { ...z, ...patch } : z));
    localRef.current = next; setLocal(next); onChange && onChange(next, id);
  };

  const geom = (o, mode, dn, dv) => {
    if (mode === 'move') {
      const span = o.hi - o.lo, vspan = o.velHi - o.velLo;
      const lo = smClamp(o.lo + dn, lowNote, highNote - span);
      const velLo = smClamp(o.velLo + dv, 1, 127 - vspan);
      const p = { lo, hi: lo + span, velLo, velHi: velLo + vspan };
      if (o.root != null) p.root = smClamp(o.root + (lo - o.lo), 0, 127);
      return p;
    }
    const p = {};
    if (mode.includes('l')) p.lo = smClamp(o.lo + dn, lowNote, o.hi);
    if (mode.includes('r')) p.hi = smClamp(o.hi + dn, o.lo, highNote);
    if (mode.includes('t')) p.velHi = smClamp(o.velHi + dv, o.velLo, 127);
    if (mode.includes('b')) p.velLo = smClamp(o.velLo + dv, 1, o.velHi);
    return p;
  };

  React.useEffect(() => {
    if (!drag) return;
    const move = (e) => {
      const dn = Math.round((e.clientX - drag.x0) / k);
      const dv = Math.round((-(e.clientY - drag.y0) / H) * 127);
      update(drag.id, geom(drag.orig, drag.mode, dn, dv));
    };
    const up = () => setDrag(null);
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up);
    return () => { window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); };
  }, [drag, k, H]);

  const startDrag = (e, z, mode) => {
    if (e.button !== 0) return;
    e.stopPropagation(); e.preventDefault();
    select(z.id);
    if (!editable) return;
    setDrag({ id: z.id, mode, x0: e.clientX, y0: e.clientY, orig: { ...z } });
  };

  const onZoneKey = (e, z) => {
    if (!editable) return;
    const d = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] }[e.key];
    if (!d) return;
    e.preventDefault();
    const step = e.altKey ? 8 : 1;
    const mode = e.shiftKey ? (d[0] ? 'r' : 't') : 'move';
    update(z.id, geom(z, mode, d[0] * step, d[1] * step));
  };

  const gridPoint = (e) => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - r.left, y = e.clientY - r.top;
    return { note: smClamp(Math.floor(x / k) + lowNote, lowNote, highNote), vel: smClamp(127 - Math.floor((y / H) * 127), 1, 127) };
  };

  const noteOn = (e, n) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture && e.currentTarget.setPointerCapture(e.pointerId);
    const r = e.currentTarget.getBoundingClientRect();
    const vel = smClamp(Math.round(24 + ((e.clientY - r.top) / r.height) * 103), 1, 127);
    const hits = localRef.current.filter((z) => n >= z.lo && n <= z.hi && vel >= z.velLo && vel <= z.velHi);
    setHeld({ note: n, vel }); setLit(hits.map((z) => z.id));
    if (hits.length) select(hits[hits.length - 1].id);
    onNoteOn && onNoteOn(n, vel);
  };
  const noteOff = () => {
    if (!held) return;
    onNoteOff && onNoteOff(held.note);
    setHeld(null); setLit([]);
  };

  const selZone = local.find((z) => z.id === selLocal);
  const ordered = [...local.filter((z) => z.id !== selLocal), ...(selZone ? [selZone] : [])];
  const sameRect = (a, b) => a.lo === b.lo && a.hi === b.hi && a.velLo === b.velLo && a.velHi === b.velHi;
  const stackIdx = (z) => { const i = local.indexOf(z); return local.slice(0, i).filter((o) => sameRect(o, z)).length; };
  const stackCount = (z) => local.filter((o) => sameRect(o, z)).length;
  const layered = selZone ? local.filter((o) => o.lo <= selZone.hi && o.hi >= selZone.lo && o.velLo <= selZone.velHi && o.velHi >= selZone.velLo) : [];
  const inSel = (n) => selZone && n >= selZone.lo && n <= selZone.hi;
  const mapped = (n) => local.some((z) => n >= z.lo && n <= z.hi);
  const notes = Array.from({ length: count }, (_, i) => lowNote + i);
  const whites = [];
  for (let n = lowNote - 1; n <= highNote + 1; n++) if (!smIsBlack(n) && n >= 0 && n <= 127) whites.push(n);
  const whiteLeft = (n) => (Math.floor(n / 12) * 12 + (SM_WHITE_IDX[n % 12] * 12) / 7 - lowNote) * k;
  const whiteW = (12 / 7) * k;
  const readout = hover ? hover : held;
  const scroll = W + SM_AXIS > avail + 0.5;
  const zb = { width: 20, height: 20, padding: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'var(--surface-control)', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-2, 4px)', color: 'var(--text-secondary)', fontFamily: 'var(--font-value)', fontSize: 'var(--type-label)', lineHeight: 1, cursor: 'pointer' };
  const cursorFor = (m) => ({ move: drag ? 'grabbing' : 'grab', l: 'ew-resize', r: 'ew-resize', t: 'ns-resize', b: 'ns-resize', lt: 'nwse-resize', rb: 'nwse-resize', rt: 'nesw-resize', lb: 'nesw-resize' })[m];
  const range = (z) => z.lo === z.hi ? `${z.lo} ${noteName(z.lo)}` : `${z.lo} ${noteName(z.lo)}–${z.hi} ${noteName(z.hi)}`;

  return (
    <div ref={wrapRef} style={{ display: 'flex', flexDirection: 'column', gap: 6, minWidth: 0, userSelect: 'none', ...style }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, minHeight: 18, flexWrap: 'wrap' }}>
        <span style={smLabel}>{title}</span>
        {!editable && <span title="Prepared setting. Edit the patch and reload to change." style={{ ...smLabel, padding: '1px 4px', border: '1px dashed var(--border-strong)', borderRadius: 2 }}>Prepared</span>}
        {selZone && (
          <span style={{ display: 'flex', gap: 10, alignItems: 'baseline', flexWrap: 'wrap' }}>
            <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap' }}>{selZone.name}</span>
            {selZone.source && <span style={{ ...smLabel, padding: '1px 4px', background: 'var(--dd-ink-0)', borderRadius: 2, color: 'var(--text-secondary)' }}>{selZone.source}</span>}
            <span style={{ display: 'flex', gap: 4, alignItems: 'baseline', whiteSpace: 'nowrap' }}><span style={smLabel}>Key</span><span style={smValue}>{range(selZone)}</span></span>
            <span style={{ display: 'flex', gap: 4, alignItems: 'baseline', whiteSpace: 'nowrap' }}><span style={smLabel}>Vel</span><span style={smValue}>{selZone.velLo}–{selZone.velHi}</span></span>
            {selZone.root != null && <span style={{ display: 'flex', gap: 4, alignItems: 'baseline', whiteSpace: 'nowrap' }}><span style={smLabel}>Root</span><span style={smValue}>{selZone.root} {noteName(selZone.root)}</span></span>}
          </span>
        )}
        <span style={{ ...smValue, marginLeft: 'auto', color: 'var(--text-tertiary)', minWidth: 110, textAlign: 'right' }}>
          {readout ? `${readout.note} ${noteName(readout.note)} · vel ${readout.vel}` : ''}
        </span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 4 }} title="Zoom. Ctrl/Cmd + wheel zooms at the pointer.">
          <button type="button" aria-label="Zoom out" disabled={zoom <= minZoom} onClick={() => setZoom(zoom / 1.5)} style={{ ...zb, opacity: zoom <= minZoom ? 0.4 : 1 }}>−</button>
          <span style={{ ...smValue, color: 'var(--text-secondary)', minWidth: 34, textAlign: 'center' }}>{Math.round(zoom * 100)}%</span>
          <button type="button" aria-label="Zoom in" disabled={zoom >= maxZoom} onClick={() => setZoom(zoom * 1.5)} style={{ ...zb, opacity: zoom >= maxZoom ? 0.4 : 1 }}>+</button>
          <button type="button" aria-label="Zoom to fit" onClick={() => setZoom(1)} style={{ ...zb, width: 'auto', padding: '0 6px', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: '0.08em', textTransform: 'uppercase' }}>Fit</button>
        </span>
      </div>

      {layered.length > 1 && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
          <span style={smLabel}>Layered</span>
          {layered.map((z) => (
            <button key={z.id} type="button" onClick={() => select(z.id)} aria-pressed={z.id === selLocal}
              style={{ display: 'flex', alignItems: 'center', gap: 5, height: 20, padding: '0 6px', background: z.id === selLocal ? 'var(--dd-vermilion-wash)' : 'var(--surface-control)', border: `1px solid ${z.id === selLocal ? 'var(--dd-vermilion)' : 'var(--border-control)'}`, borderRadius: 'var(--radius-2, 4px)', cursor: 'pointer', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap' }}>
              {z.source && <span style={{ ...smLabel, color: 'var(--text-tertiary)' }}>{z.source}</span>}{z.name}
            </button>
          ))}
        </div>
      )}
      <div ref={scrollRef} style={{ overflowX: scroll ? 'auto' : 'hidden', overflowY: 'hidden', paddingBottom: scroll ? 4 : 0 }}>
        <div style={{ display: 'grid', gridTemplateColumns: `${SM_AXIS}px ${W}px`, rowGap: 0 }}>
          <div style={{ position: 'sticky', left: 0, zIndex: 2, height: H, background: 'var(--surface-panel)' }}>
            {[127, 96, 64, 32, 1].map((v) => (
              <span key={v} style={{ position: 'absolute', right: 6, top: smClamp(yTop(v) - 6, 0, H - 12), fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', lineHeight: '12px', color: 'var(--text-tertiary)' }}>{v}</span>
            ))}
          </div>
          <div
            onPointerMove={(e) => setHover(gridPoint(e))} onPointerLeave={() => setHover(null)}
            style={{ position: 'relative', width: W, height: H, background: 'var(--dd-ink-1)', borderRadius: 'var(--radius-1, 2px)', overflow: 'hidden', boxShadow: 'inset 0 0 0 1px var(--border-hairline)', cursor: drag ? cursorFor(drag.mode) : 'default' }}>
            {notes.map((n) => (
              <div key={n} style={{ position: 'absolute', left: xOf(n), top: 0, width: k, height: H, background: smIsBlack(n) ? 'var(--dd-ink-0)' : 'transparent', borderLeft: n % 12 === 0 ? '1px solid var(--dd-line-2)' : 'none', boxSizing: 'border-box', pointerEvents: 'none' }} />
            ))}
            {[96, 64, 32].map((v) => <div key={v} style={{ position: 'absolute', left: 0, right: 0, top: yTop(v), height: 1, background: 'var(--dd-line-1)', pointerEvents: 'none' }} />)}
            {(hover || held) && <div style={{ position: 'absolute', left: xOf((held || hover).note), top: 0, width: k, height: H, background: 'var(--dd-paper-1)', opacity: 0.06, pointerEvents: 'none' }} />}

            {ordered.map((z) => {
              const isSel = z.id === selLocal, on = lit.includes(z.id);
              const si = stackIdx(z), sc = stackCount(z), off = Math.min(4, k / 5);
              const left = xOf(z.lo) + si * off, width = (z.hi - z.lo + 1) * k - (sc - 1) * off, top = yTop(z.velHi) + si * off, height = yBot(z.velLo) - yTop(z.velHi) - (sc - 1) * off;
              const e = Math.min(5, Math.max(2, width / 4)), ev = Math.min(5, Math.max(2, height / 4));
              const h = (m, s) => editable && <div key={m} onPointerDown={(ev2) => startDrag(ev2, z, m)} style={{ position: 'absolute', cursor: cursorFor(m), ...s }} />;
              return (
                <div key={z.id} role="button" tabIndex={0} aria-label={`${z.name}${z.source ? ' (' + z.source + ')' : ''}, key ${range(z)}, velocity ${z.velLo} to ${z.velHi}`} aria-pressed={isSel}
                  onPointerDown={(ev2) => startDrag(ev2, z, 'move')} onKeyDown={(ev2) => onZoneKey(ev2, z)}
                  onFocus={() => { setFocusId(z.id); select(z.id); }} onBlur={() => setFocusId(null)}
                  style={{
                    position: 'absolute', left, top, width, height, boxSizing: 'border-box', outline: 'none', overflow: 'hidden',
                    cursor: editable ? cursorFor('move') : 'pointer',
                    background: isSel ? 'color-mix(in srgb, var(--dd-vermilion) 24%, transparent)' : 'color-mix(in srgb, var(--dd-ink-6) 72%, transparent)',
                    border: `1px ${editable ? 'solid' : 'dashed'} ${isSel ? 'var(--dd-vermilion)' : 'var(--dd-line-3)'}`,
                    borderRadius: 2,
                    boxShadow: focusId === z.id ? '0 0 0 2px var(--dd-ink-1), 0 0 0 4px var(--color-focus)' : 'none',
                  }}>
                  <div style={{ position: 'absolute', inset: 0, background: 'var(--dd-paper-1)', opacity: on ? 0.28 : 0, transition: on ? 'none' : 'opacity 400ms ease-out', pointerEvents: 'none' }} />
                  {width >= 34 && height >= 16 && (sc === 1 || isSel || (!ordered.some((o) => o !== z && sameRect(o, z) && o.id === selLocal) && si === sc - 1)) && (
                    <span style={{ position: 'absolute', left: 4, top: 2, right: 4, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, lineHeight: '12px', color: isSel ? 'var(--text-primary)' : 'var(--text-secondary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', pointerEvents: 'none' }}>{z.name}{z.source && height >= 30 && <span style={{ display: 'block', fontWeight: 500, color: 'var(--text-tertiary)' }}>{z.source}</span>}</span>
                  )}
                  {h('l', { left: 0, top: ev, bottom: ev, width: e })}
                  {h('r', { right: 0, top: ev, bottom: ev, width: e })}
                  {h('t', { top: 0, left: e, right: e, height: ev })}
                  {h('b', { bottom: 0, left: e, right: e, height: ev })}
                  {h('lt', { left: 0, top: 0, width: e, height: ev })}
                  {h('rt', { right: 0, top: 0, width: e, height: ev })}
                  {h('lb', { left: 0, bottom: 0, width: e, height: ev })}
                  {h('rb', { right: 0, bottom: 0, width: e, height: ev })}
                </div>
              );
            })}
            {held && <div style={{ position: 'absolute', left: xOf(held.note) - 2, width: k + 4, top: yTop(held.vel), height: 2, background: 'var(--dd-paper-1)', pointerEvents: 'none' }} />}
          </div>

          <div style={{ position: 'sticky', left: 0, zIndex: 2, background: 'var(--surface-panel)' }} />
          <div style={{ position: 'relative', width: W, height: 4, margin: '3px 0 2px' }}>
            {notes.map((n) => mapped(n) && <div key={n} style={{ position: 'absolute', left: xOf(n), width: k, top: 1, height: 2, background: inSel(n) ? 'var(--dd-vermilion)' : 'var(--dd-paper-4)', boxShadow: inSel(n) ? '0 -1px 0 var(--dd-vermilion), 0 1px 0 var(--dd-vermilion)' : 'none' }} />)}
          </div>

          <div style={{ position: 'sticky', left: 0, zIndex: 2, background: 'var(--surface-panel)' }} />
          <div onPointerUp={noteOff} onPointerCancel={noteOff} style={{ position: 'relative', width: W, height: keyboardHeight, overflow: 'hidden', background: 'var(--dd-ink-0)', borderRadius: '0 0 2px 2px', touchAction: 'none' }}>
            {whites.map((n) => {
              const down = held && held.note === n, sel = inSel(n), root = selZone && selZone.root === n;
              return (
                <div key={n} role="button" aria-label={`Key ${n} ${noteName(n)}`} onPointerDown={(e) => noteOn(e, n)}
                  style={{
                    position: 'absolute', left: whiteLeft(n), top: 0, width: whiteW, height: keyboardHeight, boxSizing: 'border-box',
                    background: down ? 'var(--dd-vermilion)' : sel ? 'color-mix(in srgb, var(--dd-paper-2) 74%, var(--dd-vermilion))' : 'var(--dd-paper-2)',
                    borderRight: '1px solid var(--dd-ink-2)', borderRadius: '0 0 2px 2px', cursor: 'pointer',
                    display: 'flex', flexDirection: 'column', justifyContent: 'flex-end', alignItems: 'center', gap: 3, paddingBottom: 3,
                  }}>
                  {root && <span title="Root key" style={{ width: 5, height: 5, background: 'var(--dd-ink-0)', borderRadius: 1 }} />}
                  {n % 12 === 0 && whiteW >= 14 && <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', lineHeight: 1, color: 'var(--dd-ink-3)', pointerEvents: 'none' }}>{noteName(n)}</span>}
                </div>
              );
            })}
            {notes.filter(smIsBlack).map((n) => {
              const down = held && held.note === n, sel = inSel(n), root = selZone && selZone.root === n;
              return (
                <div key={n} role="button" aria-label={`Key ${n} ${noteName(n)}`} onPointerDown={(e) => noteOn(e, n)}
                  style={{
                    position: 'absolute', left: xOf(n), top: 0, width: k, height: Math.round(keyboardHeight * 0.6), boxSizing: 'border-box',
                    background: down ? 'var(--dd-vermilion)' : sel ? 'color-mix(in srgb, var(--dd-ink-1) 62%, var(--dd-vermilion))' : 'var(--dd-ink-1)',
                    border: '1px solid var(--dd-ink-0)', borderTop: 'none', borderRadius: '0 0 2px 2px', cursor: 'pointer',
                    display: 'flex', alignItems: 'flex-end', justifyContent: 'center', paddingBottom: 3,
                  }}>
                  {root && <span title="Root key" style={{ width: 4, height: 4, background: 'var(--dd-paper-1)', borderRadius: 1 }} />}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
