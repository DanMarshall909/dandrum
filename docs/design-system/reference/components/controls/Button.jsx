import React from 'react';
import { Icon } from '../icons/Icon.jsx';

const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/** Text button. Variants: primary (vermilion, one per view), secondary (graphite), ghost (text only). */
export function Button({ children, variant = 'secondary', size = 'md', icon, selected = false, disabled = false, focused = false, pressed: pressedProp, onClick, title, style }) {
  const [hover, setHover] = React.useState(false);
  const [down, setDown] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const pressed = pressedProp ?? down;
  const h = size === 'sm' ? 24 : 28;
  let bg, fg, border;
  if (variant === 'primary') {
    bg = disabled ? 'var(--dd-ink-4)' : pressed ? 'var(--dd-vermilion-lo)' : hover ? 'var(--dd-vermilion-hi)' : 'var(--dd-vermilion)';
    fg = disabled ? 'var(--text-disabled)' : 'var(--text-on-accent)'; border = 'transparent';
  } else if (variant === 'ghost') {
    bg = pressed ? 'var(--dd-ink-5)' : hover && !disabled ? 'var(--dd-ink-4)' : 'transparent';
    fg = disabled ? 'var(--text-disabled)' : selected ? 'var(--dd-vermilion)' : 'var(--text-secondary)'; border = 'transparent';
  } else {
    bg = disabled ? 'var(--dd-ink-3)' : pressed ? 'var(--dd-ink-6)' : hover ? 'var(--dd-ink-5)' : 'var(--dd-ink-4)';
    fg = disabled ? 'var(--text-disabled)' : 'var(--text-primary)';
    border = selected ? 'var(--dd-vermilion)' : 'var(--border-control)';
  }
  return (
    <button
      type="button" disabled={disabled} title={title} onClick={onClick}
      onMouseEnter={() => setHover(true)} onMouseLeave={() => { setHover(false); setDown(false); }}
      onMouseDown={() => setDown(true)} onMouseUp={() => setDown(false)}
      onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
      style={{
        height: h, padding: size === 'sm' ? '0 8px' : '0 12px', display: 'inline-flex', alignItems: 'center', justifyContent: 'center', gap: 6,
        background: bg, color: fg, border: `1px solid ${border}`, borderRadius: 'var(--radius-2)', boxSizing: 'border-box',
        fontFamily: 'var(--font-ui)', fontSize: size === 'sm' ? 'var(--type-label)' : 'var(--type-body)', fontWeight: 600,
        letterSpacing: '0.02em', whiteSpace: 'nowrap', cursor: disabled ? 'default' : 'pointer', outline: 'none',
        boxShadow: (focus || focused) ? focusRing : (variant === 'secondary' && !disabled && !pressed ? 'var(--highlight-cap)' : 'none'),
        transform: pressed && !disabled ? 'translateY(1px)' : 'none', ...style,
      }}
    >
      {icon && <Icon name={icon} size={size === 'sm' ? 14 : 16} />}
      {children}
    </button>
  );
}

/** Square icon-only button. Always pass `label` (tooltip + accessible name). */
export function IconButton({ icon, label, size = 'md', variant = 'secondary', selected = false, disabled = false, focused = false, onClick, style }) {
  const [hover, setHover] = React.useState(false);
  const [down, setDown] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const s = size === 'sm' ? 24 : 28;
  const ghost = variant === 'ghost';
  const bg = disabled ? (ghost ? 'transparent' : 'var(--dd-ink-3)') : down ? 'var(--dd-ink-6)' : hover ? 'var(--dd-ink-5)' : ghost ? 'transparent' : 'var(--dd-ink-4)';
  return (
    <button
      type="button" aria-label={label} title={label} disabled={disabled} aria-pressed={selected || undefined} onClick={onClick}
      onMouseEnter={() => setHover(true)} onMouseLeave={() => { setHover(false); setDown(false); }}
      onMouseDown={() => setDown(true)} onMouseUp={() => setDown(false)}
      onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
      style={{
        width: s, height: s, padding: 0, display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        background: selected ? 'var(--dd-vermilion-wash)' : bg, borderRadius: 'var(--radius-2)', boxSizing: 'border-box',
        border: `1px solid ${selected ? 'var(--dd-vermilion)' : ghost ? 'transparent' : 'var(--border-control)'}`,
        color: disabled ? 'var(--text-disabled)' : selected ? 'var(--dd-vermilion)' : 'var(--text-primary)',
        cursor: disabled ? 'default' : 'pointer', outline: 'none', boxShadow: (focus || focused) ? focusRing : 'none',
        transform: down && !disabled ? 'translateY(1px)' : 'none', ...style,
      }}
    >
      <Icon name={icon} size={size === 'sm' ? 14 : 16} />
    </button>
  );
}
