import React from 'react';

// Modulation slots pair colour + glyph + letter so meaning never relies on colour alone.
export const MOD_SLOTS = {
  A: { color: 'var(--dd-mod-a)', glyph: 'circle', name: 'Mod A' },
  B: { color: 'var(--dd-mod-b)', glyph: 'triangle', name: 'Mod B' },
  C: { color: 'var(--dd-mod-c)', glyph: 'square', name: 'Mod C' },
  D: { color: 'var(--dd-mod-d)', glyph: 'diamond', name: 'Mod D' },
};

export function ModGlyph({ slot = 'A', size = 8, color, hollow = false }) {
  const s = MOD_SLOTS[slot] || MOD_SLOTS.A;
  const c = color || s.color;
  const st = hollow ? { fill: 'none', stroke: c, strokeWidth: 1.5 } : { fill: c };
  let shape;
  if (s.glyph === 'circle') shape = <circle cx="5" cy="5" r="3.6" style={st} />;
  else if (s.glyph === 'triangle') shape = <path d="M5 1.2 9 8.6H1Z" style={st} />;
  else if (s.glyph === 'square') shape = <rect x="1.6" y="1.6" width="6.8" height="6.8" style={st} />;
  else shape = <path d="M5 .8 9.2 5 5 9.2.8 5Z" style={st} />;
  return <svg width={size} height={size} viewBox="0 0 10 10" style={{ display: 'block', flex: 'none' }} aria-hidden="true">{shape}</svg>;
}

/**
 * Compact chip naming a modulation source + depth. Used in tooltips, context menus,
 * and the pad-detail panel. Host = DAW-owned automation (blue, plug glyph, "HOST").
 */
export function ModIndicator({ slot = 'A', depth, source, host = false, active = false, compact = false }) {
  const color = host ? 'var(--dd-host)' : (MOD_SLOTS[slot] || MOD_SLOTS.A).color;
  const depthText = depth == null ? null : `${depth > 0 ? '+' : depth < 0 ? '−' : ''}${Math.abs(Math.round(depth * 100))}%`;
  return (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 5, height: compact ? 18 : 20, padding: compact ? '0 5px' : '0 6px',
      borderRadius: 'var(--radius-1)', background: 'var(--dd-ink-0)', border: `1px solid ${active ? color : 'var(--border-control)'}`,
      fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)',
      color: 'var(--text-secondary)', whiteSpace: 'nowrap', textTransform: 'uppercase',
    }}>
      {host
        ? <svg width="9" height="9" viewBox="0 0 10 10" aria-hidden="true"><path d="M3 1v3M7 1v3M1.5 4h7v1.5a3.5 3.5 0 0 1-7 0Z" style={{ fill: 'none', stroke: color, strokeWidth: 1.4 }} /></svg>
        : <ModGlyph slot={slot} size={9} />}
      <span style={{ color: 'var(--text-primary)' }}>{host ? 'Host' : (source || slot)}</span>
      {depthText && <span style={{ fontFamily: 'var(--font-value)', letterSpacing: 0, color }}>{depthText}</span>}
    </span>
  );
}
