import React from 'react';
import { Icon } from '../icons/Icon.jsx';
import { ModGlyph, MOD_SLOTS } from '../display/ModIndicator.jsx';

/**
 * Context menu / popup menu. Items: { label, icon, shortcut, disabled, danger, checked, submenu, header, separator, onSelect }.
 * Special item `{ type: 'assignment', slot, source, depth, onDepth, onRemove }` renders an inline depth slider row.
 * Keyboard: ↑/↓ move, Enter select, ←/→ adjust a focused depth row (Shift = fine), Delete removes it, Esc closes.
 */
export function ContextMenu({ items = [], title, x, y, width = 248, onClose, style }) {
  const [active, setActive] = React.useState(() => items.findIndex((i) => isFocusable(i)));
  const ref = React.useRef(null);
  React.useEffect(() => { ref.current && ref.current.focus(); }, []);
  const move = (dir) => {
    let i = active;
    for (let n = 0; n < items.length; n++) { i = (i + dir + items.length) % items.length; if (isFocusable(items[i])) break; }
    setActive(i);
  };
  const onKeyDown = (e) => {
    const it = items[active];
    if (e.key === 'ArrowDown') { move(1); e.preventDefault(); }
    else if (e.key === 'ArrowUp') { move(-1); e.preventDefault(); }
    else if (e.key === 'Escape') { onClose && onClose(); }
    else if (e.key === 'Enter' && it && !it.disabled && it.onSelect) { it.onSelect(); }
    else if (it && it.type === 'assignment' && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      const d = (e.key === 'ArrowRight' ? 1 : -1) * (e.shiftKey ? 0.01 : 0.05);
      it.onDepth && it.onDepth(Math.max(-1, Math.min(1, (it.depth || 0) + d))); e.preventDefault();
    } else if (it && it.type === 'assignment' && (e.key === 'Delete' || e.key === 'Backspace')) { it.onRemove && it.onRemove(); }
  };
  return (
    <div ref={ref} role="menu" tabIndex={-1} onKeyDown={onKeyDown}
      style={{
        position: x != null ? 'fixed' : 'relative', left: x, top: y, width, boxSizing: 'border-box', padding: '4px 0', outline: 'none', zIndex: 100,
        background: 'var(--surface-menu)', border: '1px solid var(--border-strong)', borderRadius: 'var(--radius-2)', boxShadow: 'var(--shadow-float)',
        fontFamily: 'var(--font-ui)', color: 'var(--text-primary)', ...style,
      }}>
      {title && <div style={{ padding: '4px 10px 6px', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase', color: 'var(--text-tertiary)', borderBottom: '1px solid var(--border-hairline)', marginBottom: 4 }}>{title}</div>}
      {items.map((it, i) => {
        if (it.separator) return <div key={i} style={{ height: 1, background: 'var(--border-hairline)', margin: '4px 0' }} />;
        if (it.header) return <div key={i} style={{ padding: '6px 10px 2px', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase', color: 'var(--text-tertiary)' }}>{it.header}</div>;
        if (it.note) return <div key={i} style={{ padding: '4px 10px 6px', fontSize: 'var(--type-micro)', lineHeight: 1.35, color: 'var(--text-tertiary)', textWrap: 'pretty' }}>{it.note}</div>;
        if (it.type === 'assignment') return <AssignmentRow key={i} it={it} active={i === active} onHover={() => setActive(i)} />;
        const isActive = i === active && !it.disabled;
        return (
          <div key={i} role="menuitem" aria-disabled={it.disabled || undefined}
            onMouseEnter={() => setActive(i)} onClick={() => !it.disabled && it.onSelect && it.onSelect()}
            style={{
              height: 26, display: 'flex', alignItems: 'center', gap: 8, padding: '0 10px', margin: '0 4px', borderRadius: 3, cursor: it.disabled ? 'default' : 'pointer',
              background: isActive ? 'var(--dd-ink-5)' : 'transparent', fontSize: 'var(--type-body)', fontWeight: 500,
              color: it.disabled ? 'var(--text-disabled)' : it.danger ? 'var(--dd-error)' : 'var(--text-primary)',
            }}>
            <span style={{ width: 16, display: 'flex', justifyContent: 'center', color: it.disabled ? 'var(--text-disabled)' : 'var(--text-secondary)' }}>
              {it.checked ? <Icon name="ok" size={14} /> : it.icon ? <Icon name={it.icon} size={16} /> : null}
            </span>
            <span style={{ flex: 1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{it.label}</span>
            {it.shortcut && <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-tertiary)' }}>{it.shortcut}</span>}
            {it.submenu && <Icon name="chevron-right" size={14} color="var(--text-tertiary)" />}
          </div>
        );
      })}
    </div>
  );
}

function isFocusable(i) { return i && !i.separator && !i.header && !i.note && !i.disabled; }

function AssignmentRow({ it, active, onHover }) {
  const slot = MOD_SLOTS[it.slot] || MOD_SLOTS.A;
  const d = it.depth || 0;
  const trackRef = React.useRef(null);
  const setFrom = (e) => { const r = trackRef.current.getBoundingClientRect(); const v = ((e.clientX - r.left) / r.width) * 2 - 1; it.onDepth && it.onDepth(Math.max(-1, Math.min(1, Math.round(v * 100) / 100))); };
  return (
    <div role="menuitem" onMouseEnter={onHover}
      style={{ display: 'grid', gridTemplateColumns: '16px 1fr auto 20px', alignItems: 'center', columnGap: 8, rowGap: 4, padding: '5px 6px 6px 10px', margin: '0 4px', borderRadius: 3, background: active ? 'var(--dd-ink-5)' : 'transparent' }}>
      <span style={{ display: 'flex', justifyContent: 'center' }}><ModGlyph slot={it.slot} size={10} /></span>
      <span style={{ fontSize: 'var(--type-body)', fontWeight: 500, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{it.source}</span>
      <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: slot.color }}>{d > 0 ? '+' : d < 0 ? '−' : ''}{Math.abs(Math.round(d * 100))}%</span>
      <button type="button" aria-label={`Remove ${it.source}`} title="Remove assignment" onClick={it.onRemove}
        style={{ width: 20, height: 20, padding: 0, border: 0, borderRadius: 2, background: 'transparent', color: 'var(--text-tertiary)', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
        onMouseEnter={(e) => { e.currentTarget.style.color = 'var(--dd-error)'; }} onMouseLeave={(e) => { e.currentTarget.style.color = 'var(--text-tertiary)'; }}>
        <Icon name="close" size={14} />
      </button>
      <span />
      <div ref={trackRef} onPointerDown={(e) => { e.currentTarget.setPointerCapture(e.pointerId); setFrom(e); }} onPointerMove={(e) => { if (e.buttons) setFrom(e); }}
        onDoubleClick={() => it.onDepth && it.onDepth(0)}
        style={{ gridColumn: '2 / 4', position: 'relative', height: 12, cursor: 'ew-resize', touchAction: 'none' }}>
        <div style={{ position: 'absolute', left: 0, right: 0, top: 5, height: 2, background: 'var(--dd-ink-0)', borderRadius: 1 }} />
        <div style={{ position: 'absolute', left: '50%', top: 2, width: 1, height: 8, background: 'var(--dd-paper-4)' }} />
        <div style={{ position: 'absolute', top: 5, height: 2, background: slot.color, left: `${50 + Math.min(0, d) * 50}%`, width: `${Math.abs(d) * 50}%` }} />
        <div style={{ position: 'absolute', top: 1, width: 4, height: 10, marginLeft: -2, borderRadius: 1, background: 'var(--dd-paper-1)', left: `${50 + d * 50}%` }} />
      </div>
      <span />
    </div>
  );
}

/** Dropdown menu trigger (closed state). Opens a ContextMenu anchored below. */
export function MenuButton({ label, value, onClick, compact = false, disabled = false, width, readOnly = false }) {
  return (
    <button type="button" onClick={onClick} disabled={disabled || readOnly}
      style={{
        height: compact ? 22 : 24, width, minWidth: 96, padding: '0 6px 0 8px', display: 'inline-flex', alignItems: 'center', gap: 6, boxSizing: 'border-box',
        background: readOnly ? 'transparent' : 'var(--surface-well)', border: `1px ${readOnly ? 'dashed' : 'solid'} var(--border-control)`, borderRadius: 'var(--radius-1)',
        color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)', cursor: disabled || readOnly ? 'default' : 'pointer',
        fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)', fontWeight: 500,
      }}>
      {label && <span style={{ color: 'var(--text-tertiary)' }}>{label}</span>}
      <span style={{ flex: 1, textAlign: 'left', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{value}</span>
      {!readOnly && <Icon name="chevron-down" size={14} color="var(--text-tertiary)" />}
    </button>
  );
}
