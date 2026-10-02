import React from 'react';
import { Icon } from '../icons/Icon.jsx';

/**
 * List row (sample regions, layers, alternates, presets). 26px (22 compact).
 * Selected = vermilion 2px edge bar + raised face. `active` shows a filled dot (playing now).
 */
export function ListRow({ label, detail, value, icon, leading, selected = false, active = false, disabled = false, compact = false, onClick, trailing }) {
  const [hover, setHover] = React.useState(false);
  return (
    <div role="option" aria-selected={selected} tabIndex={disabled ? -1 : 0} onClick={disabled ? undefined : onClick}
      onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
      onKeyDown={(e) => { if ((e.key === 'Enter' || e.key === ' ') && onClick) { e.preventDefault(); onClick(); } }}
      style={{
        position: 'relative', height: compact ? 22 : 26, display: 'flex', alignItems: 'center', gap: 8, padding: '0 8px 0 10px', boxSizing: 'border-box',
        background: selected ? 'var(--dd-ink-4)' : hover && !disabled ? 'var(--dd-ink-3)' : 'transparent', borderRadius: 'var(--radius-1)',
        cursor: disabled || !onClick ? 'default' : 'pointer', outline: 'none',
        fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)', fontWeight: 500,
        color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)',
      }}
      onFocus={(e) => { e.currentTarget.style.boxShadow = 'inset 0 0 0 2px var(--color-focus)'; }}
      onBlur={(e) => { e.currentTarget.style.boxShadow = 'none'; }}>
      <span style={{ position: 'absolute', left: 0, top: 4, bottom: 4, width: 2, borderRadius: 1, background: selected ? 'var(--dd-vermilion)' : 'transparent' }} />
      <span style={{ width: 6, height: 6, borderRadius: 6, flex: 'none', background: active ? 'var(--dd-paper-1)' : 'transparent', boxShadow: active ? 'none' : 'inset 0 0 0 1px var(--dd-line-3)' }} />
      {leading}
      {icon && <Icon name={icon} size={compact ? 14 : 16} color="var(--text-secondary)" />}
      <span style={{ flex: 1, minWidth: 0, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{label}</span>
      {detail && <span style={{ color: 'var(--text-tertiary)', whiteSpace: 'nowrap' }}>{detail}</span>}
      {value != null && <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>{value}</span>}
      {trailing}
    </div>
  );
}

/**
 * Label/value pair for settings. `prepared` = read-only patch data (lock glyph, dashed underline,
 * secondary text, no hover); live values are primary text in mono.
 */
export function PropertyRow({ label, value, unit, prepared = false, compact = false, hint }) {
  return (
    <div title={prepared ? 'Prepared setting — change in the patch and reload' : hint}
      style={{ display: 'flex', alignItems: 'baseline', gap: 8, minHeight: compact ? 20 : 22, padding: '2px 0', fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)' }}>
      <span style={{ color: 'var(--text-tertiary)', whiteSpace: 'nowrap', flex: 'none', minWidth: compact ? 64 : 92 }}>{label}</span>
      <span style={{ flex: 1, minWidth: 0, fontFamily: 'var(--font-value)', fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)', color: prepared ? 'var(--text-secondary)' : 'var(--text-primary)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        <span style={{ borderBottom: prepared ? '1px dotted var(--dd-line-3)' : 'none' }}>{value}</span>
        {unit && <span style={{ color: 'var(--text-tertiary)', marginLeft: 3 }}>{unit}</span>}
      </span>
      {prepared && <Icon name="lock" size={12} color="var(--text-tertiary)" style={{ alignSelf: 'center' }} />}
    </div>
  );
}

/** Stacked parameter label + value for read-outs that are not themselves controls. */
export function ParamLabel({ label, value, unit, align = 'left', hostAutomated = false, size = 'md' }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: align === 'center' ? 'center' : align === 'right' ? 'flex-end' : 'flex-start', gap: 2 }}>
      <span style={{ fontFamily: 'var(--font-ui)', fontSize: size === 'sm' ? 'var(--type-micro)' : 'var(--type-label)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-secondary)' }}>{label}</span>
      <span style={{ fontFamily: 'var(--font-value)', fontSize: size === 'lg' ? 'var(--type-value-lg)' : size === 'sm' ? 'var(--type-micro)' : 'var(--type-value)', color: hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)' }}>
        {value}{unit && <span style={{ color: 'var(--text-tertiary)', marginLeft: 3 }}>{unit}</span>}
      </span>
    </div>
  );
}
