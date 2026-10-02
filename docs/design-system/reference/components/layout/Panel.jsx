import React from 'react';
import { Icon } from '../icons/Icon.jsx';

function Chevron({ open, size = 12 }) {
  return <Icon name="chevron-right" size={size} color="var(--text-tertiary)" style={{ transform: open ? 'rotate(90deg)' : 'none', transition: 'transform 90ms ease-out' }} />;
}

/**
 * Functional panel. Header (28px, 24 compact) carries a chevron, caps heading, optional tag and actions.
 * Every panel is collapsible: click the header (or Enter/Space when focused). Collapsed panels shrink to
 * their header and give their space to siblings. Panels butt together with a 4px seam; no outer shadow.
 */
export function Panel({
  title, tag, prepared = false, actions, children, compact = false, padding, style, bodyStyle, tone = 'default',
  collapsible = true, collapsed: collapsedProp, defaultCollapsed = false, onToggle, summary,
}) {
  const [local, setLocal] = React.useState(defaultCollapsed);
  const collapsed = collapsedProp ?? local;
  const toggle = () => { if (!collapsible) return; const n = !collapsed; if (collapsedProp == null) setLocal(n); onToggle && onToggle(n); };
  const [focus, setFocus] = React.useState(false);
  return (
    <section style={{
      display: 'flex', flexDirection: 'column', minWidth: 0, minHeight: 0, borderRadius: 'var(--radius-3)', overflow: 'hidden',
      background: tone === 'sunken' ? 'var(--dd-ink-1)' : 'var(--surface-panel)', border: '1px solid var(--border-hairline)', ...style,
      ...(collapsed ? { flex: 'none' } : null),
    }}>
      {title && (
        <header
          role={collapsible ? 'button' : undefined} tabIndex={collapsible ? 0 : undefined} aria-expanded={collapsible ? !collapsed : undefined}
          onClick={(e) => { if (e.target.closest('[data-panel-action]')) return; toggle(); }}
          onKeyDown={(e) => { if (e.target !== e.currentTarget) return; if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } }}
          onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
          style={{
            height: compact ? 24 : 28, flex: 'none', display: 'flex', alignItems: 'center', gap: compact ? 6 : 8, padding: compact ? '0 8px 0 6px' : '0 12px 0 8px',
            background: 'var(--surface-panel-header)', borderBottom: collapsed ? 'none' : '1px solid var(--border-hairline)',
            cursor: collapsible ? 'pointer' : 'default', outline: 'none', userSelect: 'none',
            boxShadow: focus ? 'inset 0 0 0 2px var(--color-focus)' : 'none',
          }}>
          {collapsible && <Chevron open={!collapsed} />}
          <SectionHeading compact={compact} prepared={prepared}>{title}</SectionHeading>
          {tag && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 10, fontWeight: 600, letterSpacing: '0.08em', textTransform: 'uppercase', padding: '1px 4px', borderRadius: 2, border: '1px solid var(--border-strong)', color: 'var(--text-secondary)' }}>{tag}</span>}
          {collapsed && summary && <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-tertiary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', minWidth: 0 }}>{summary}</span>}
          <div style={{ flex: 1 }} />
          {actions && <div data-panel-action="" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>{actions}</div>}
        </header>
      )}
      {!collapsed && <div className="dd-scroll" style={{ flex: 1, minHeight: 0, padding: padding ?? (compact ? 8 : 12), ...bodyStyle }}>{children}</div>}
    </section>
  );
}

/**
 * Rollout — collapsible region inside a panel (3ds Max-style). 22px header: chevron, caps title,
 * hairline rule, and a one-line summary when collapsed so key values stay visible.
 */
export function Rollout({ title, summary, children, collapsed: collapsedProp, defaultCollapsed = false, onToggle, prepared = false, compact = false, actions }) {
  const [local, setLocal] = React.useState(defaultCollapsed);
  const collapsed = collapsedProp ?? local;
  const toggle = () => { const n = !collapsed; if (collapsedProp == null) setLocal(n); onToggle && onToggle(n); };
  const [focus, setFocus] = React.useState(false);
  return (
    <div style={{ display: 'flex', flexDirection: 'column' }}>
      <div role="button" tabIndex={0} aria-expanded={!collapsed}
        onClick={(e) => { if (e.target.closest('[data-panel-action]')) return; toggle(); }}
        onKeyDown={(e) => { if (e.target !== e.currentTarget) return; if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } if (e.key === 'ArrowLeft' && !collapsed) toggle(); if (e.key === 'ArrowRight' && collapsed) toggle(); }}
        onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
        onMouseEnter={(e) => { e.currentTarget.style.background = 'var(--dd-ink-3)'; }} onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; }}
        style={{
          height: compact ? 20 : 22, display: 'flex', alignItems: 'center', gap: 6, padding: '0 4px', margin: '0 -4px', borderRadius: 'var(--radius-1)',
          cursor: 'pointer', outline: 'none', userSelect: 'none', boxShadow: focus ? 'inset 0 0 0 2px var(--color-focus)' : 'none',
        }}>
        <Chevron open={!collapsed} size={12} />
        <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 700, letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
          {prepared && <Icon name="lock" size={11} color="var(--text-tertiary)" />}{title}
        </span>
        {collapsed && summary
          ? <span style={{ flex: 1, minWidth: 0, fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-tertiary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{summary}</span>
          : <span style={{ flex: 1, height: 1, background: 'var(--border-hairline)' }} />}
        {actions && <div data-panel-action="" style={{ display: 'flex', alignItems: 'center', gap: 4 }}>{actions}</div>}
      </div>
      {!collapsed && <div style={{ padding: compact ? '2px 0 6px 18px' : '4px 0 8px 18px' }}>{children}</div>}
    </div>
  );
}

/** Caps section heading. `prepared` adds a lock glyph to mark read-only patch settings. */
export function SectionHeading({ children, compact = false, prepared = false, color }) {
  return (
    <h3 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: 6, fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)', fontWeight: 700, letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase', color: color || 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
      {prepared && <Icon name="lock" size={12} color="var(--text-tertiary)" />}
      {children}
    </h3>
  );
}
