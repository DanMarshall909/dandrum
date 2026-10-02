import React from 'react';
import { MOD_SLOTS, ModGlyph } from '../display/ModIndicator.jsx';

const SWEEP = 135; // degrees either side of 12 o'clock
const SIZES = { lg: 64, md: 48, sm: 36, xs: 28 };

function pt(c, r, deg) {
  const a = (deg - 90) * Math.PI / 180;
  return [c + r * Math.cos(a), c + r * Math.sin(a)];
}
function arc(c, r, d0, d1) {
  if (Math.abs(d1 - d0) < 0.01) return '';
  const a = Math.min(d0, d1), b = Math.max(d0, d1);
  const [x0, y0] = pt(c, r, a), [x1, y1] = pt(c, r, b);
  return `M${x0.toFixed(2)} ${y0.toFixed(2)}A${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}
const ang = (v) => -SWEEP + Math.max(0, Math.min(1, v)) * SWEEP * 2;
const clamp01 = (v) => Math.max(0, Math.min(1, v));

/**
 * Dandrum rotary knob. Anatomy (outside → in): modulation rings (≤2), value track + arc, graphite cap
 * carrying the combined-modulation arc. No pointer line — the value arc alone shows position.
 * All geometry is JUCE-drawable (Path::addCentredArc + fillEllipse).
 */
export function Knob({
  value = 0.5, defaultValue = 0.5, bipolar = false, size = 'md', label, valueText, unit,
  modulations = [], assigning = false, hostAutomated = false, focused = false, disabled = false,
  hovered: hoveredProp, active: activeProp, activeSlot, peak: peakProp, trough: troughProp, peakText, troughText,
  showPeakTrough = true, onResetPeaks, valueDisplay = 'hover', parseValue, popupOpen, onChange, onContextMenu, onFocus, showLabel = true, labelPosition = 'top', style,
}) {
  const px = typeof size === 'number' ? size : (SIZES[size] || 48);
  const [hoverLocal, setHover] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const [focusLocal, setFocusLocal] = React.useState(false);
  const hovered = hoveredProp ?? hoverLocal;
  const isFocused = focused || focusLocal;
  const drag = React.useRef(null);
  // Value popup: values are hidden on the panel and appear in an editable popup on hover / focus / drag.
  const [popHover, setPopHover] = React.useState(false);
  const [editing, setEditing] = React.useState(false);
  const [draft, setDraft] = React.useState('');
  const closeT = React.useRef(null);
  const holdOpen = () => { clearTimeout(closeT.current); setPopHover(true); };
  const letClose = () => { clearTimeout(closeT.current); closeT.current = setTimeout(() => setPopHover(false), 250); };
  React.useEffect(() => () => clearTimeout(closeT.current), []);
  const [nudged, setNudged] = React.useState(false);
  const nudgeT = React.useRef(null);
  const nudge = () => { setNudged(true); clearTimeout(nudgeT.current); nudgeT.current = setTimeout(() => setNudged(false), 600); };
  React.useEffect(() => () => clearTimeout(nudgeT.current), []);
  // "Being changed" = user dragging / keyboard / wheel nudge, or the host writing the value.
  const tweaking = !disabled && (activeProp ?? (dragging || nudged || hostAutomated));

  const c = px / 2;
  // Layout reserves the thick widths so nothing shifts when arcs thicken.
  const trackMax = px >= 60 ? 4 : px >= 44 ? 3 : 2.5;
  const trackMin = px >= 44 ? 1.5 : 1.25;
  const modMax = px >= 44 ? 2 : 1.5;
  const modMin = 1;
  // Only the element being changed thickens: the value arc when the base value moves; a mod ring when its
  // source is live or its depth is being edited (activeSlot). The background track never thickens.
  const trackW = trackMin;
  const valueW = tweaking ? trackMax : trackMin;
  const rMod1 = c - modMax / 2;
  const rMod2 = rMod1 - modMax - 1;
  const rTrack = c - (px >= 44 ? 6.5 : 5) - trackMax / 2;
  const rCap = rTrack - trackMax / 2 - (px >= 44 ? 2.5 : 2);
  const rInner = rCap - (px >= 44 ? 3 : 2.5);
  const innerW = px >= 44 ? 2 : 1.5;
  const origin = bipolar ? 0 : -SWEEP;
  const vA = ang(value);

  const valueColor = disabled ? 'var(--dd-paper-4)' : hostAutomated ? 'var(--dd-host)' : 'var(--color-value)';
  const capFill = disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-ink-6)' : hovered ? 'var(--dd-cap-hover)' : 'var(--dd-ink-5)';
  const shownMods = modulations.slice(0, 2);
  // Combined modulation (all assignments): range = value + Σ positive / negative depths; live = value + Σ depth·live
  const sumPos = modulations.reduce((a, m) => a + Math.max(0, m.depth || 0), 0);
  const sumNeg = modulations.reduce((a, m) => a + Math.min(0, m.depth || 0), 0);
  const anyLive = modulations.some((m) => m.live != null);
  const effective = value + modulations.reduce((a, m) => a + (m.depth || 0) * (m.live ?? 0), 0);
  // Peak / trough: extremes of the effective value since last reset (editor-held, decays on reset).
  const peakRef = React.useRef(null), troughRef = React.useRef(null);
  const [, bump] = React.useState(0);
  if (anyLive) {
    if (peakRef.current == null || effective > peakRef.current) peakRef.current = effective;
    if (troughRef.current == null || effective < troughRef.current) troughRef.current = effective;
  }
  const peak = peakProp ?? (modulations.length ? peakRef.current : null);
  const trough = troughProp ?? (modulations.length ? troughRef.current : null);
  const parse = parseValue || ((t) => { const n = parseFloat(String(t).replace('−', '-')); return Number.isFinite(n) ? n / 100 : null; });
  const commitDraft = () => { const v = parse(draft); if (v != null) setK(v); setEditing(false); };
  const resetPeaks = () => { peakRef.current = null; troughRef.current = null; bump((n) => n + 1); onResetPeaks && onResetPeaks(); };
  // Clip: modulation range reaches past the parameter range. Live clip = the effective value is pinned right now.
  const rangeClipHi = value + sumPos > 1.0001, rangeClipLo = value + sumNeg < -0.0001;
  const liveClipHi = anyLive && effective >= 1, liveClipLo = anyLive && effective <= 0;
  const clipWarn = !disabled && (rangeClipHi || rangeClipLo);
  const clipLive = !disabled && (liveClipHi || liveClipLo);
  const showInline = valueDisplay === 'always';
  const popOpen = !disabled && !showInline && (popupOpen ?? (popHover || editing || dragging || nudged || focusLocal));
  const combinedColor = modulations.length === 1 ? (MOD_SLOTS[modulations[0].slot] || MOD_SLOTS.A).color : 'var(--dd-paper-1)';

  const set = (v) => { if (!disabled && onChange) onChange(clamp01(v)); };
  const setK = (v) => { nudge(); set(v); };
  const onPointerDown = (e) => {
    if (disabled || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { y: e.clientY, v: value };
    setDragging(true);
  };
  const onPointerMove = (e) => {
    if (!drag.current) return;
    const sens = e.shiftKey ? 800 : 200; // px for full range; Shift = fine
    set(drag.current.v + (drag.current.y - e.clientY) / sens);
  };
  const end = () => { drag.current = null; setDragging(false); };
  const onKeyDown = (e) => {
    const step = e.shiftKey ? 0.01 : 0.05;
    if (e.key === 'ArrowUp' || e.key === 'ArrowRight') { setK(value + step); e.preventDefault(); }
    else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') { setK(value - step); e.preventDefault(); }
    else if (e.key === 'Home') setK(0); else if (e.key === 'End') setK(1);
    else if (e.key === 'Delete' || e.key === 'Backspace') setK(defaultValue);
    else if ((e.key === 'F10' && e.shiftKey) || e.key === 'ContextMenu' || e.key === 'm' || e.key === 'M') {
      e.preventDefault();
      if (onContextMenu) { const r = e.currentTarget.getBoundingClientRect(); onContextMenu({ clientX: r.left + r.width / 2, clientY: r.bottom, preventDefault() {}, fromKeyboard: true, target: e.currentTarget }); }
    }
  };

  const labelEl = showLabel && label && (
    <div style={{
      fontFamily: 'var(--font-ui)', fontSize: px <= 36 ? 'var(--type-micro)' : 'var(--type-label)', fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', lineHeight: 1.15, whiteSpace: 'nowrap',
      color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: 4,
    }}>
      {label}
      {modulations.length > 2 && <span style={{ fontFamily: 'var(--font-value)', fontSize: 10, color: 'var(--text-primary)', background: 'var(--dd-ink-0)', borderRadius: 2, padding: '0 3px', letterSpacing: 0 }}>+{modulations.length - 2}</span>}
    </div>
  );

  return (
    <div
      role="slider" tabIndex={disabled ? -1 : 0} aria-label={label} aria-valuemin={0} aria-valuemax={1}
      aria-valuenow={Number(value.toFixed(3))} aria-valuetext={valueText} aria-disabled={disabled || undefined}
      onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={end} onPointerCancel={end}
      onDoubleClick={() => setK(defaultValue)}
      onKeyDown={(e) => { if (e.key === 'Enter' && !disabled && !showInline) { e.preventDefault(); setDraft(String(valueText ?? Math.round(value * 100))); setEditing(true); return; } onKeyDown(e); }}
      onWheel={(e) => { if (disabled) return; setK(value + (e.deltaY < 0 ? 1 : -1) * (e.shiftKey ? 0.01 : 0.02)); }}
      onContextMenu={(e) => { e.preventDefault(); if (!disabled && onContextMenu) onContextMenu(e); }}
      onMouseEnter={() => { setHover(true); holdOpen(); }} onMouseLeave={() => { setHover(false); letClose(); }}
      onFocus={(e) => { setFocusLocal(true); onFocus && onFocus(e); }} onBlur={() => setFocusLocal(false)}
      style={{
        display: 'inline-flex', flexDirection: labelPosition === 'top' ? 'column' : 'column-reverse', alignItems: 'center',
        gap: px <= 36 ? 3 : 5, padding: px <= 36 ? '4px 4px' : '6px 6px', borderRadius: 'var(--radius-2)', outline: 'none',
        cursor: disabled ? 'default' : 'ns-resize', userSelect: 'none', touchAction: 'none', position: 'relative',
        minWidth: px + 12,
        background: assigning && !disabled ? 'var(--dd-mod-wash)' : 'transparent',
        boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : 'none',
        ...style,
      }}
    >
      {labelEl}
      <svg width={px} height={px} viewBox={`0 0 ${px} ${px}`} style={{ display: 'block', overflow: 'visible' }} aria-hidden="true">
        {assigning && !disabled && (
          <circle cx={c} cy={c} r={rMod1} style={{ fill: 'none', stroke: 'var(--dd-mod-a)', strokeWidth: 1.5, strokeDasharray: '3 3', opacity: 0.9 }} />
        )}
        {/* modulation rings */}
        {!disabled && shownMods.map((m, i) => {
          const r = i === 0 ? rMod1 : rMod2;
          const col = (MOD_SLOTS[m.slot] || MOD_SLOTS.A).color;
          const end = ang(value + (m.depth || 0));
          const live = m.live != null;
          const changing = live || m.slot === activeSlot;
          const w = changing ? modMax : modMin;
          return (
            <g key={i}>
              <path d={arc(c, r, -SWEEP, SWEEP)} style={{ fill: 'none', stroke: 'var(--dd-ink-3)', strokeWidth: modMin }} />
              <path d={arc(c, r, vA, end)} style={{ fill: 'none', stroke: col, strokeWidth: w, strokeLinecap: 'butt', opacity: changing ? 1 : 0.7 }} />
              {live && (() => { const [x, y] = pt(c, r, ang(value + (m.depth || 0) * m.live)); return <circle cx={x} cy={y} r={modMax / 2 + 1.2} style={{ fill: col, stroke: 'var(--surface-panel)', strokeWidth: 1 }} />; })()}
            </g>
          );
        })}
        {/* track + value */}
        <path d={arc(c, rTrack, -SWEEP, SWEEP)} style={{ fill: 'none', stroke: disabled ? 'var(--dd-ink-4)' : 'var(--color-track)', strokeWidth: trackW, strokeLinecap: 'round' }} />
        {!disabled && <path d={arc(c, rTrack, origin, vA)} style={{ fill: 'none', stroke: valueColor, strokeWidth: valueW, strokeLinecap: 'round' }} />}
        {/* origin tick */}
        {(() => { const [x0, y0] = pt(c, rTrack + trackW / 2 + 1, origin); const [x1, y1] = pt(c, rTrack + trackW / 2 + (px >= 44 ? 4 : 3), origin); return <line x1={x0} y1={y0} x2={x1} y2={y1} style={{ stroke: 'var(--dd-paper-3)', strokeWidth: 1.5, strokeLinecap: 'round', opacity: disabled ? 0.4 : 1 }} />; })()}
        {/* cap */}
        <circle cx={c} cy={c + 1.5} r={rCap} style={{ fill: 'rgba(0,0,0,0.45)' }} />
        <circle cx={c} cy={c} r={rCap} style={{ fill: capFill, stroke: 'var(--dd-line-3)', strokeWidth: 1, strokeOpacity: disabled ? 0.3 : 0.6 }} />
        <path d={arc(c, rCap - 1, -60, 60)} style={{ fill: 'none', stroke: 'rgba(255,255,255,0.09)', strokeWidth: 1 }} />
        {/* combined modulation inside the cap: faint full range, bright base→effective, dot at effective */}
        {!disabled && modulations.length > 0 && (
          <g>
            <path d={arc(c, rInner, ang(value + sumNeg), ang(value + sumPos))} style={{ fill: 'none', stroke: combinedColor, strokeWidth: innerW, strokeLinecap: 'butt', opacity: 0.3 }} />
            {anyLive && <path d={arc(c, rInner, vA, ang(effective))} style={{ fill: 'none', stroke: combinedColor, strokeWidth: innerW, strokeLinecap: 'butt' }} />}
            {anyLive && (() => { const [x, y] = pt(c, rInner, ang(effective)); return <circle cx={x} cy={y} r={innerW / 2 + 1} style={{ fill: combinedColor }} />; })()}
            {/* peak (outward wedge) + trough (inward wedge) ticks on the inner arc */}
            {showPeakTrough && [[peak, 1], [trough, -1]].map(([p, dir], i) => {
              if (p == null) return null;
              const a = ang(p), [x0, y0] = pt(c, rInner + dir * (innerW / 2 + 0.5), a), [x1, y1] = pt(c, rInner + dir * (innerW / 2 + (px >= 44 ? 3 : 2.5)), a);
              return <line key={i} x1={x0} y1={y0} x2={x1} y2={y1} style={{ stroke: 'var(--dd-paper-1)', strokeWidth: 1.25, strokeLinecap: 'round', opacity: 0.85 }} />;
            })}
          </g>
        )}
        {/* clip: range-end caps on the track. Hollow = range would exceed, filled = pinned right now */}
        {clipWarn && [[rangeClipHi, SWEEP, liveClipHi], [rangeClipLo, -SWEEP, liveClipLo]].map(([on, a, live], i) => {
          if (!on) return null;
          const [x, y] = pt(c, rTrack, a);
          return <circle key={'clip' + i} cx={x} cy={y} r={trackMax / 2 + 1.5} style={live ? { fill: 'var(--dd-error)' } : { fill: 'var(--surface-panel)', stroke: 'var(--dd-error)', strokeWidth: 1.5 }} />;
        })}
      </svg>
{showInline ? (<>
      <div style={{
        fontFamily: 'var(--font-value)', fontSize: px >= 60 ? 'var(--type-value-lg)' : px <= 36 ? 'var(--type-micro)' : 'var(--type-value)',
        fontWeight: 500, lineHeight: 1.1, whiteSpace: 'nowrap', minHeight: '1.1em', display: 'flex', alignItems: 'center', gap: 4,
        color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)',
      }}>
        {shownMods.length > 0 && !disabled && <span style={{ display: 'flex', gap: 2 }}>{shownMods.map((m, i) => <ModGlyph key={i} slot={m.slot} size={7} />)}</span>}
        <span>{valueText ?? Math.round(value * 100)}</span>
        {unit && <span style={{ color: 'var(--text-tertiary)' }}>{unit}</span>}
        {clipWarn && (
          <span title={clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range'} style={{ display: 'inline-flex', alignItems: 'center', gap: 2, height: 14, padding: '0 3px', borderRadius: 2, fontFamily: 'var(--font-ui)', fontSize: 10, fontWeight: 700, letterSpacing: '0.06em',
            background: clipLive ? 'var(--dd-error)' : 'transparent', color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)', boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)' }}>
            <svg width="8" height="8" viewBox="0 0 10 10" aria-hidden="true"><path d="M5 1 9.3 9H.7Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinejoin: 'round' }} /></svg>CLIP
          </span>
        )}
      </div>
      {showPeakTrough && !disabled && peak != null && trough != null && px >= 44 && (
        <button type="button" onClick={(e) => { e.stopPropagation(); resetPeaks(); }} onPointerDown={(e) => e.stopPropagation()} title="Peak / trough of the modulated value · click to reset"
          style={{ display: 'flex', gap: 6, padding: 0, border: 0, background: 'transparent', cursor: 'pointer', fontFamily: 'var(--font-value)', fontSize: 10, lineHeight: 1, color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>
          <span><span style={{ color: 'var(--text-secondary)' }}>▴</span>{peakText ?? Math.round(clamp01(peak) * 100)}</span>
          <span><span style={{ color: 'var(--text-secondary)' }}>▾</span>{troughText ?? Math.round(clamp01(trough) * 100)}</span>
        </button>
      )}
</>) : (
        <div style={{ height: 14, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{clipWarn && (
          <span title={clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range'} style={{ display: 'inline-flex', alignItems: 'center', gap: 2, height: 14, padding: '0 3px', borderRadius: 2, fontFamily: 'var(--font-ui)', fontSize: 10, fontWeight: 700, letterSpacing: '0.06em', flex: 'none',
            background: clipLive ? 'var(--dd-error)' : 'transparent', color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)', boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)' }}>
            <svg width="8" height="8" viewBox="0 0 10 10" aria-hidden="true"><path d="M5 1 9.3 9H.7Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinejoin: 'round' }} /></svg>CLIP
          </span>
        )}</div>
      )}
      {popOpen && (
        <div role="dialog" aria-label={(label || 'Value') + ' value'}
          onMouseEnter={holdOpen} onMouseLeave={letClose}
          onPointerDown={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()} onWheel={(e) => e.stopPropagation()} onKeyDown={(e) => e.stopPropagation()}
          style={{
            position: 'absolute', top: '100%', left: '50%', transform: 'translate(-50%, -2px)', zIndex: 30, minWidth: Math.max(104, px + 32),
            display: 'flex', flexDirection: 'column', gap: 5, padding: 6, boxSizing: 'border-box', cursor: 'default',
            background: 'var(--dd-ink-0)', border: '1px solid var(--border-strong)', borderRadius: 'var(--radius-2)', boxShadow: 'var(--shadow-float)',
          }}>
          <span style={{ position: 'absolute', top: -5, left: '50%', marginLeft: -4, width: 8, height: 8, transform: 'rotate(45deg)', background: 'var(--dd-ink-0)', borderLeft: '1px solid var(--border-strong)', borderTop: '1px solid var(--border-strong)' }} />
          <div onClick={() => { if (!editing) { setDraft(String(valueText ?? Math.round(value * 100))); setEditing(true); } }}
            title="Click to type a value"
            style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 4, height: 24, padding: '0 6px', borderRadius: 'var(--radius-1)', cursor: 'text',
              background: 'var(--dd-ink-2)', border: `1px solid ${editing ? 'var(--dd-paper-2)' : 'var(--border-control)'}`,
              fontFamily: 'var(--font-value)', fontSize: 'var(--type-value)', color: hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)', whiteSpace: 'nowrap' }}>
            {editing
              ? <input autoFocus value={draft} onChange={(e) => setDraft(e.target.value)} onBlur={commitDraft}
                  onKeyDown={(e) => { if (e.key === 'Enter') commitDraft(); if (e.key === 'Escape') setEditing(false); e.stopPropagation(); }}
                  onFocus={(e) => e.target.select()}
                  style={{ flex: 1, minWidth: 0, width: 56, background: 'transparent', border: 0, outline: 'none', padding: 0, color: 'var(--text-primary)', font: 'inherit' }} />
              : <span style={{ flex: 1 }}>{valueText ?? Math.round(value * 100)}</span>}
            {unit && <span style={{ color: 'var(--text-tertiary)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)' }}>{unit}</span>}
          </div>
          {(clipWarn || (showPeakTrough && peak != null && trough != null)) && (
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              {showPeakTrough && peak != null && trough != null && (
                <button type="button" onClick={resetPeaks} title="Peak / trough of the modulated value · click to reset"
                  style={{ display: 'flex', gap: 6, padding: 0, border: 0, background: 'transparent', cursor: 'pointer', fontFamily: 'var(--font-value)', fontSize: 10, lineHeight: 1, color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>
                  <span><span style={{ color: 'var(--text-secondary)' }}>▴</span>{peakText ?? Math.round(clamp01(peak) * 100)}</span>
                  <span><span style={{ color: 'var(--text-secondary)' }}>▾</span>{troughText ?? Math.round(clamp01(trough) * 100)}</span>
                </button>
              )}
              <span style={{ flex: 1 }} />
              {clipWarn && (
          <span title={clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range'} style={{ display: 'inline-flex', alignItems: 'center', gap: 2, height: 14, padding: '0 3px', borderRadius: 2, fontFamily: 'var(--font-ui)', fontSize: 10, fontWeight: 700, letterSpacing: '0.06em', flex: 'none',
            background: clipLive ? 'var(--dd-error)' : 'transparent', color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)', boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)' }}>
            <svg width="8" height="8" viewBox="0 0 10 10" aria-hidden="true"><path d="M5 1 9.3 9H.7Z" style={{ fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, strokeLinejoin: 'round' }} /></svg>CLIP
          </span>
        )}
            </div>
          )}
          {modulations.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2, paddingTop: 4, borderTop: '1px solid var(--border-hairline)' }}>
              {modulations.map((m, i) => (
                <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
                  <ModGlyph slot={m.slot} size={8} /><span style={{ flex: 1 }}>{m.source || (MOD_SLOTS[m.slot] || MOD_SLOTS.A).name}</span>
                  <span style={{ fontFamily: 'var(--font-value)', color: (MOD_SLOTS[m.slot] || MOD_SLOTS.A).color }}>{(m.depth > 0 ? '+' : m.depth < 0 ? '−' : '') + Math.abs(Math.round((m.depth || 0) * 100))}%</span>
                </div>
              ))}
            </div>
          )}
          {hostAutomated && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', color: 'var(--dd-host)' }}>Host automation</span>}
        </div>
      )}
    </div>
  );
}
