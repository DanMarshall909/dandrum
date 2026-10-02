import React from 'react';
import { MOD_SLOTS } from '../display/ModIndicator.jsx';

const clamp01 = (v) => Math.max(0, Math.min(1, v));

/**
 * Linear slider, horizontal or vertical. Track 4px, thumb 12px square-ish (radius 2).
 * Modulation range = thin bar(s) beside the track in the slot colour; host automation tints fill blue.
 */
export function Slider({
  value = 0.5, defaultValue = 0.5, orientation = 'horizontal', length, bipolar = false, label, valueText,
  modulations = [], assigning = false, hostAutomated = false, focused = false, disabled = false, compact = false,
  onChange, onContextMenu, style,
}) {
  const vertical = orientation === 'vertical';
  const L = length ?? (vertical ? (compact ? 88 : 120) : (compact ? 112 : 160));
  const thick = compact ? 20 : 24;
  const [focusLocal, setFocusLocal] = React.useState(false);
  const [hover, setHover] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const ref = React.useRef(null);
  const thumb = 12;
  const usable = L - thumb;
  const pos = (v) => thumb / 2 + clamp01(v) * usable;
  const set = (v) => { if (!disabled && onChange) onChange(clamp01(v)); };
  const fromEvent = (e) => {
    const r = ref.current.getBoundingClientRect();
    return vertical ? 1 - (e.clientY - r.top - thumb / 2) / usable : (e.clientX - r.left - thumb / 2) / usable;
  };
  const onPointerDown = (e) => { if (disabled || e.button !== 0) return; e.currentTarget.setPointerCapture(e.pointerId); setDragging(true); set(fromEvent(e)); };
  const onPointerMove = (e) => { if (dragging) set(fromEvent(e)); };
  const end = () => setDragging(false);
  const onKeyDown = (e) => {
    const step = e.shiftKey ? 0.01 : 0.05;
    if (['ArrowUp', 'ArrowRight'].includes(e.key)) { set(value + step); e.preventDefault(); }
    else if (['ArrowDown', 'ArrowLeft'].includes(e.key)) { set(value - step); e.preventDefault(); }
    else if (e.key === 'Delete' || e.key === 'Backspace') set(defaultValue);
    else if ((e.key === 'F10' && e.shiftKey) || e.key === 'ContextMenu' || e.key === 'm' || e.key === 'M') {
      e.preventDefault();
      if (onContextMenu) { const r = e.currentTarget.getBoundingClientRect(); onContextMenu({ clientX: r.left, clientY: r.bottom, preventDefault() {}, fromKeyboard: true }); }
    }
  };
  const isFocused = focused || focusLocal;
  const fillColor = disabled ? 'var(--dd-ink-6)' : hostAutomated ? 'var(--dd-host)' : 'var(--color-value)';
  const o = bipolar ? pos(0.5) : pos(0);
  const p = pos(value);
  const lo = Math.min(o, p), hi = Math.max(o, p);
  const mid = thick / 2;

  const seg = (a, b, off, w, color, opacity = 1, key) => vertical
    ? <div key={key} style={{ position: 'absolute', left: mid + off - w / 2, width: w, bottom: Math.min(a, b), height: Math.abs(b - a), background: color, opacity, borderRadius: 1 }} />
    : <div key={key} style={{ position: 'absolute', top: mid + off - w / 2, height: w, left: Math.min(a, b), width: Math.abs(b - a), background: color, opacity, borderRadius: 1 }} />;

  return (
    <div style={{ display: 'inline-flex', flexDirection: vertical ? 'column' : 'column', alignItems: vertical ? 'center' : 'stretch', gap: 4, ...style }}>
      {label && (
        <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8, fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
          <span>{label}</span>
          {!vertical && <span style={{ fontFamily: 'var(--font-value)', textTransform: 'none', letterSpacing: 0, fontWeight: 500, fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)', color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)' }}>{valueText ?? Math.round(value * 100)}</span>}
        </div>
      )}
      <div
        ref={ref} role="slider" tabIndex={disabled ? -1 : 0} aria-label={label} aria-orientation={orientation}
        aria-valuemin={0} aria-valuemax={1} aria-valuenow={Number(value.toFixed(3))} aria-valuetext={valueText}
        onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={end} onPointerCancel={end}
        onDoubleClick={() => set(defaultValue)} onKeyDown={onKeyDown}
        onContextMenu={(e) => { e.preventDefault(); if (!disabled && onContextMenu) onContextMenu(e); }}
        onFocus={() => setFocusLocal(true)} onBlur={() => setFocusLocal(false)}
        onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        style={{
          position: 'relative', width: vertical ? thick : L, height: vertical ? L : thick, outline: 'none', touchAction: 'none',
          cursor: disabled ? 'default' : vertical ? 'ns-resize' : 'ew-resize', borderRadius: 'var(--radius-1)',
          background: assigning && !disabled ? 'var(--dd-mod-wash)' : 'transparent',
          boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : assigning && !disabled ? 'inset 0 0 0 1px var(--dd-mod-a)' : 'none',
        }}
      >
        {seg(thumb / 2, L - thumb / 2, 0, 4, disabled ? 'var(--dd-ink-4)' : 'var(--color-track)')}
        {!disabled && seg(lo, hi, 0, 4, fillColor)}
        {bipolar && seg(o - 0.75, o + 0.75, 0, 10, 'var(--dd-paper-3)')}
        {!disabled && modulations.slice(0, 2).map((m, i) => {
          const col = (MOD_SLOTS[m.slot] || MOD_SLOTS.A).color;
          const e = pos(value + (m.depth || 0));
          const off = (i === 0 ? -1 : 1) * 6.5;
          const nodes = [seg(p, e, off, 2, col, m.live != null ? 1 : 0.6, 'r' + i)];
          if (m.live != null) { const lp = pos(value + (m.depth || 0) * m.live); nodes.push(seg(lp - 1.5, lp + 1.5, off, 4, col, 1, 'l' + i)); }
          return nodes;
        })}
        <div style={{
          position: 'absolute', width: vertical ? 16 : thumb, height: vertical ? thumb : 16, borderRadius: 2,
          left: vertical ? mid - 8 : p - thumb / 2, top: vertical ? undefined : mid - 8, bottom: vertical ? p - thumb / 2 : undefined,
          background: disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-ink-6)' : hover ? 'var(--dd-cap-hover)' : 'var(--dd-ink-5)',
          border: '1px solid var(--dd-line-3)', boxShadow: disabled ? 'none' : 'var(--shadow-cap), var(--highlight-cap)', boxSizing: 'border-box',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <div style={{ width: vertical ? 8 : 2, height: vertical ? 2 : 8, background: disabled ? 'var(--dd-paper-4)' : 'var(--dd-paper-1)', borderRadius: 1 }} />
        </div>
      </div>
      {vertical && <div style={{ fontFamily: 'var(--font-value)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)', color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)', textAlign: 'center' }}>{valueText ?? Math.round(value * 100)}</div>}
    </div>
  );
}
