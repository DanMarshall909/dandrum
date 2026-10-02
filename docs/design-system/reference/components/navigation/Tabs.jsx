import React from 'react';
import { Icon } from '../icons/Icon.jsx';

const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/**
 * Panel tabs. Selected tab = primary text + 2px vermilion bar under the label (position + colour).
 * `badge` renders a small caps tag after the label (e.g. "BREAK" to scope a view to a patch type).
 */
export function Tabs({ items = [], value, onChange, compact = false, style }) {
  const [focusIdx, setFocusIdx] = React.useState(-1);
  return (
    <div role="tablist" style={{ display: 'flex', alignItems: 'stretch', gap: 2, height: compact ? 26 : 32, ...style }}
      onKeyDown={(e) => {
        const i = items.findIndex((t) => t.id === value);
        if (e.key === 'ArrowRight') { const n = items[(i + 1) % items.length]; onChange && onChange(n.id); }
        if (e.key === 'ArrowLeft') { const n = items[(i - 1 + items.length) % items.length]; onChange && onChange(n.id); }
      }}>
      {items.map((t, i) => {
        const sel = t.id === value;
        return (
          <button key={t.id} role="tab" aria-selected={sel} disabled={t.disabled} tabIndex={sel ? 0 : -1}
            onClick={() => onChange && onChange(t.id)} onFocus={() => setFocusIdx(i)} onBlur={() => setFocusIdx(-1)}
            style={{
              position: 'relative', display: 'flex', alignItems: 'center', gap: 6, padding: compact ? '0 8px' : '0 12px',
              background: 'transparent', border: 0, outline: 'none', cursor: t.disabled ? 'default' : 'pointer', borderRadius: 'var(--radius-1)',
              fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)', fontWeight: 600,
              letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase',
              color: t.disabled ? 'var(--text-disabled)' : sel ? 'var(--text-primary)' : 'var(--text-tertiary)',
              boxShadow: focusIdx === i ? focusRing : 'none',
            }}
            onMouseEnter={(e) => { if (!sel && !t.disabled) e.currentTarget.style.color = 'var(--text-secondary)'; }}
            onMouseLeave={(e) => { if (!sel && !t.disabled) e.currentTarget.style.color = 'var(--text-tertiary)'; }}
          >
            {t.icon && <Icon name={t.icon} size={compact ? 14 : 16} />}
            {t.label}
            {t.badge && <span style={{ fontSize: 10, letterSpacing: '0.08em', padding: '1px 4px', borderRadius: 2, border: '1px solid var(--border-strong)', color: 'var(--text-secondary)' }}>{t.badge}</span>}
            <span style={{ position: 'absolute', left: compact ? 8 : 12, right: compact ? 8 : 12, bottom: 0, height: 2, borderRadius: 1, background: sel ? 'var(--dd-vermilion)' : 'transparent' }} />
          </button>
        );
      })}
    </div>
  );
}

/** Segmented control: 2–5 mutually exclusive short options in one well. Selected = raised graphite face + primary text. */
export function SegmentedControl({ options = [], value, onChange, compact = false, disabled = false, hostAutomated = false, fullWidth = false, style }) {
  const h = compact ? 22 : 24;
  return (
    <div role="radiogroup" style={{ display: fullWidth ? 'flex' : 'inline-flex', height: h, padding: 2, gap: 2, boxSizing: 'content-box', background: 'var(--surface-well)', borderRadius: 'var(--radius-2)', border: '1px solid var(--border-hairline)', ...style }}>
      {options.map((o) => {
        const opt = typeof o === 'string' ? { id: o, label: o } : o;
        const sel = opt.id === value;
        return (
          <button key={opt.id} role="radio" aria-checked={sel} disabled={disabled} title={opt.title}
            onClick={() => onChange && onChange(opt.id)}
            style={{
              flex: fullWidth ? 1 : 'none', minWidth: compact ? 32 : 40, padding: '0 8px', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 5,
              border: 0, borderRadius: 3, cursor: disabled ? 'default' : 'pointer', outline: 'none',
              background: sel ? (disabled ? 'var(--dd-ink-4)' : 'var(--dd-ink-5)') : 'transparent',
              boxShadow: sel && !disabled ? 'var(--shadow-cap), var(--highlight-cap)' : 'none',
              color: disabled ? 'var(--text-disabled)' : sel ? (hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)') : 'var(--text-tertiary)',
              fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase',
            }}
            onFocus={(e) => { e.currentTarget.style.boxShadow = focusRing; }}
            onBlur={(e) => { e.currentTarget.style.boxShadow = sel && !disabled ? 'var(--shadow-cap), var(--highlight-cap)' : 'none'; }}
          >
            {opt.icon && <Icon name={opt.icon} size={14} />}
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
