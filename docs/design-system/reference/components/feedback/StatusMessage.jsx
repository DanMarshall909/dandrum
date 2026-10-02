import React from 'react';
import { Icon } from '../icons/Icon.jsx';

const KINDS = {
  ok: { icon: 'ok', color: 'var(--dd-ok)', wash: 'var(--dd-ok-wash)' },
  warn: { icon: 'warning', color: 'var(--dd-warn)', wash: 'var(--dd-warn-wash)' },
  error: { icon: 'error', color: 'var(--dd-error)', wash: 'var(--dd-error-wash)' },
  info: { icon: 'info', color: 'var(--dd-host)', wash: 'var(--dd-host-wash)' },
  busy: { icon: 'reload', color: 'var(--dd-paper-2)', wash: 'var(--dd-ink-3)' },
};

/**
 * Status message: icon (shape) + text + optional action. `inline` = status-bar form (no box);
 * default = banner with tinted wash and a 1px border in the status colour.
 */
export function StatusMessage({ kind = 'info', title, children, action, inline = false, compact = false }) {
  const k = KINDS[kind] || KINDS.info;
  if (inline) {
    return (
      <span role="status" style={{ display: 'inline-flex', alignItems: 'center', gap: 6, fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)', color: 'var(--text-secondary)', whiteSpace: 'nowrap', minWidth: 0 }}>
        <Icon name={k.icon} size={14} color={k.color} />
        {title && <span style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{title}</span>}
        {children && <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{children}</span>}
        {action}
      </span>
    );
  }
  return (
    <div role={kind === 'error' ? 'alert' : 'status'} style={{
      display: 'flex', alignItems: 'flex-start', gap: 8, padding: compact ? '6px 8px' : '8px 10px', borderRadius: 'var(--radius-2)',
      background: k.wash, border: `1px solid ${k.color}`, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-body)', lineHeight: 'var(--leading-body)',
    }}>
      <Icon name={k.icon} size={16} color={k.color} style={{ marginTop: 1 }} />
      <div style={{ flex: 1, minWidth: 0 }}>
        {title && <div style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{title}</div>}
        {children && <div style={{ color: 'var(--text-secondary)', textWrap: 'pretty' }}>{children}</div>}
      </div>
      {action && <div style={{ flex: 'none', alignSelf: 'center' }}>{action}</div>}
    </div>
  );
}

/** Tooltip bubble. Opens after 500 ms hover or immediately on keyboard focus; shows name, value, and modulation detail. */
export function Tooltip({ title, value, children, placement = 'top', style }) {
  return (
    <div role="tooltip" style={{
      display: 'inline-flex', flexDirection: 'column', gap: 4, padding: '6px 8px', maxWidth: 240, boxSizing: 'border-box',
      background: 'var(--dd-ink-0)', border: '1px solid var(--border-strong)', borderRadius: 'var(--radius-2)', boxShadow: 'var(--shadow-float)',
      fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-secondary)', lineHeight: 1.3, position: 'relative', ...style,
    }}>
      {(title || value) && (
        <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', gap: 12 }}>
          {title && <span style={{ color: 'var(--text-primary)', fontWeight: 600, fontSize: 'var(--type-body)' }}>{title}</span>}
          {value && <span style={{ fontFamily: 'var(--font-value)', color: 'var(--text-primary)', fontSize: 'var(--type-value)' }}>{value}</span>}
        </div>
      )}
      {children}
    </div>
  );
}

/** Empty state for panels and pads with nothing mapped. Short, factual, one optional action. */
export function EmptyState({ icon = 'info', title, children, action, compact = false }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 8, padding: compact ? 12 : 24, textAlign: 'center', border: '1px dashed var(--border-control)', borderRadius: 'var(--radius-2)', fontFamily: 'var(--font-ui)' }}>
      <Icon name={icon} size={compact ? 16 : 20} color="var(--text-tertiary)" />
      {title && <div style={{ fontSize: 'var(--type-body)', fontWeight: 600, color: 'var(--text-secondary)' }}>{title}</div>}
      {children && <div style={{ fontSize: 'var(--type-label)', color: 'var(--text-tertiary)', maxWidth: 280, lineHeight: 1.35, textWrap: 'pretty' }}>{children}</div>}
      {action}
    </div>
  );
}
