import React from 'react';

const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/** On/off toggle (pill track + square-ish thumb). Use for binary live options. Label sits right. */
export function Toggle({ checked = false, label, disabled = false, focused = false, hostAutomated = false, onChange, compact = false }) {
  const [focus, setFocus] = React.useState(false);
  const w = compact ? 28 : 32, h = compact ? 16 : 18, t = h - 6;
  const on = checked && !disabled;
  return (
    <label style={{ display: 'inline-flex', alignItems: 'center', gap: 8, minHeight: 24, cursor: disabled ? 'default' : 'pointer', userSelect: 'none' }}>
      <button
        type="button" role="switch" aria-checked={checked} disabled={disabled}
        onClick={() => onChange && onChange(!checked)} onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
        style={{
          width: w, height: h, padding: 0, position: 'relative', borderRadius: 'var(--radius-round)', outline: 'none', boxSizing: 'border-box',
          background: on ? (hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)') : 'var(--dd-ink-0)',
          border: `1px solid ${on ? 'transparent' : 'var(--border-control)'}`, cursor: 'inherit',
          boxShadow: (focus || focused) ? focusRing : 'none', opacity: disabled ? 0.5 : 1,
        }}
      >
        <span style={{
          position: 'absolute', top: 2, left: checked ? w - t - 4 : 2, width: t, height: t, borderRadius: 'var(--radius-round)',
          background: on ? 'var(--dd-ink-1)' : disabled ? 'var(--dd-ink-5)' : 'var(--dd-paper-3)', transition: 'left 90ms ease-out',
        }} />
      </button>
      {label && <span style={{ fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)', fontWeight: 500, color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)' }}>{label}</span>}
    </label>
  );
}

/**
 * Hardware-style latching switch: a labelled key with an LED bar. Use for mode switches that
 * read like instrument buttons (Reverse, Loop, Mono). Pairs LED + text so state never relies on colour.
 */
export function Switch({ on = false, label, disabled = false, focused = false, onChange, compact = false, width }) {
  const [hover, setHover] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  return (
    <button
      type="button" aria-pressed={on} disabled={disabled} onClick={() => onChange && onChange(!on)}
      onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
      onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
      style={{
        height: compact ? 24 : 28, minWidth: width ?? (compact ? 52 : 64), padding: '0 8px', display: 'inline-flex', alignItems: 'center', gap: 6,
        background: disabled ? 'var(--dd-ink-3)' : hover ? 'var(--dd-ink-5)' : 'var(--dd-ink-4)', boxSizing: 'border-box',
        border: '1px solid var(--border-control)', borderRadius: 'var(--radius-2)', outline: 'none', cursor: disabled ? 'default' : 'pointer',
        boxShadow: (focus || focused) ? focusRing : 'var(--highlight-cap)',
      }}
    >
      <span style={{ width: 3, height: compact ? 10 : 12, borderRadius: 1, background: disabled ? 'var(--dd-ink-6)' : on ? 'var(--dd-vermilion)' : 'var(--dd-ink-0)', boxShadow: on && !disabled ? '0 0 0 1px var(--dd-vermilion-lo)' : 'inset 0 0 0 1px var(--dd-line-2)' }} />
      <span style={{ fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: disabled ? 'var(--text-disabled)' : on ? 'var(--text-primary)' : 'var(--text-secondary)' }}>{label}</span>
    </button>
  );
}
