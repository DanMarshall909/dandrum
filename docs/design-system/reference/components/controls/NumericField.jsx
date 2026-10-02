import React from 'react';

const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/**
 * Numeric field: drag vertically to scrub, click to type, arrows/wheel to step, double-click resets.
 * Mono value, unit in tertiary. `readOnly` renders prepared (non-live) values with a dotted outline.
 */
export function NumericField({
  value = 0, min = 0, max = 100, step = 1, defaultValue, unit, label, format, width, compact = false,
  readOnly = false, disabled = false, focused = false, hostAutomated = false, onChange, onContextMenu,
}) {
  const [editing, setEditing] = React.useState(false);
  const [text, setText] = React.useState('');
  const [focus, setFocus] = React.useState(false);
  const [hover, setHover] = React.useState(false);
  const drag = React.useRef(null);
  const fmt = format || ((v) => (Number.isInteger(step) ? String(Math.round(v)) : v.toFixed(2)));
  const clamp = (v) => Math.max(min, Math.min(max, v));
  const set = (v) => { if (!readOnly && !disabled && onChange) onChange(clamp(Math.round(v / step) * step)); };
  const live = !readOnly && !disabled;
  const h = compact ? 22 : 24;

  const commit = () => { const n = parseFloat(text); if (!Number.isNaN(n)) set(n); setEditing(false); };
  return (
    <div style={{ display: 'inline-flex', flexDirection: 'column', gap: 3 }}>
      {label && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)' }}>{label}</span>}
      <div
        role="spinbutton" tabIndex={disabled ? -1 : 0} aria-label={label} aria-valuenow={value} aria-valuemin={min} aria-valuemax={max} aria-readonly={readOnly || undefined}
        onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
        onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        onPointerDown={(e) => { if (!live || editing || e.button !== 0) return; e.currentTarget.setPointerCapture(e.pointerId); drag.current = { y: e.clientY, v: value, moved: false }; }}
        onPointerMove={(e) => { if (!drag.current) return; const dy = drag.current.y - e.clientY; if (Math.abs(dy) > 2) drag.current.moved = true; set(drag.current.v + Math.round(dy / (e.shiftKey ? 12 : 4)) * step); }}
        onPointerUp={() => { const d = drag.current; drag.current = null; if (d && !d.moved && live) { setText(fmt(value)); setEditing(true); } }}
        onDoubleClick={() => defaultValue != null && set(defaultValue)}
        onKeyDown={(e) => {
          if (!live || editing) return;
          if (e.key === 'ArrowUp') { set(value + step * (e.shiftKey ? 10 : 1)); e.preventDefault(); }
          else if (e.key === 'ArrowDown') { set(value - step * (e.shiftKey ? 10 : 1)); e.preventDefault(); }
          else if (e.key === 'Enter') { setText(fmt(value)); setEditing(true); }
          else if ((e.key === 'F10' && e.shiftKey) || e.key === 'ContextMenu') { e.preventDefault(); onContextMenu && onContextMenu({ clientX: e.currentTarget.getBoundingClientRect().left, clientY: e.currentTarget.getBoundingClientRect().bottom, preventDefault() {}, fromKeyboard: true }); }
        }}
        onContextMenu={(e) => { e.preventDefault(); if (live && onContextMenu) onContextMenu(e); }}
        style={{
          height: h, width: width ?? (compact ? 60 : 72), boxSizing: 'border-box', padding: '0 6px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 4,
          background: readOnly ? 'transparent' : 'var(--surface-well)', borderRadius: 'var(--radius-1)', outline: 'none',
          border: readOnly ? '1px dashed var(--border-control)' : `1px solid ${editing ? 'var(--dd-paper-2)' : hover && live ? 'var(--border-strong)' : 'var(--border-control)'}`,
          boxShadow: (focus || focused) ? focusRing : readOnly ? 'none' : 'var(--inset-well)',
          cursor: live ? (editing ? 'text' : 'ns-resize') : 'default', touchAction: 'none',
        }}
      >
        {editing ? (
          <input autoFocus value={text} onChange={(e) => setText(e.target.value)} onBlur={commit}
            onKeyDown={(e) => { if (e.key === 'Enter') commit(); if (e.key === 'Escape') setEditing(false); }}
            style={{ width: '100%', background: 'transparent', border: 0, outline: 'none', color: 'var(--text-primary)', fontFamily: 'var(--font-value)', fontSize: 'var(--type-value)', padding: 0 }} />
        ) : (
          <span style={{ fontFamily: 'var(--font-value)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)', color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : readOnly ? 'var(--text-secondary)' : 'var(--text-primary)', whiteSpace: 'nowrap' }}>{fmt(value)}</span>
        )}
        {unit && !editing && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', color: 'var(--text-tertiary)' }}>{unit}</span>}
      </div>
    </div>
  );
}
