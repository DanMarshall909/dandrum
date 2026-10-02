import React from 'react';
import { Icon } from '../icons/Icon.jsx';

const dbToPos = (db) => Math.max(0, Math.min(1, (db + 60) / 63)); // −60 … +3 dBFS

/** Peak meter (vertical or horizontal; mono or stereo). Zones: ok < −12 dB, warn −12…0, clip > 0 with latching LED. */
export function Meter({ levels = [-18, -20], peak, clip = false, orientation = 'vertical', length = 96, thickness = 6, compact = false, label, onResetClip }) {
  const vertical = orientation === 'vertical';
  const zones = [
    { to: dbToPos(-12), c: 'var(--dd-ok)' },
    { to: dbToPos(0), c: 'var(--dd-warn)' },
    { to: 1, c: 'var(--dd-error)' },
  ];
  const bar = (db, i) => {
    const p = dbToPos(db);
    const segs = []; let from = 0;
    for (const z of zones) { const a = from, b = Math.min(z.to, p); if (b > a) segs.push({ a, b, c: z.c }); from = z.to; }
    return (
      <div key={i} style={{ position: 'relative', width: vertical ? thickness : length, height: vertical ? length : thickness, background: 'var(--dd-ink-0)', borderRadius: 1, overflow: 'hidden' }}>
        {segs.map((s, j) => <div key={j} style={vertical ? { position: 'absolute', left: 0, right: 0, bottom: `${s.a * 100}%`, height: `${(s.b - s.a) * 100}%`, background: s.c } : { position: 'absolute', top: 0, bottom: 0, left: `${s.a * 100}%`, width: `${(s.b - s.a) * 100}%`, background: s.c }} />)}
        <div style={vertical ? { position: 'absolute', left: 0, right: 0, bottom: `${dbToPos(0) * 100}%`, height: 1, background: 'var(--dd-ink-2)' } : { position: 'absolute', top: 0, bottom: 0, left: `${dbToPos(0) * 100}%`, width: 1, background: 'var(--dd-ink-2)' }} />
        {peak != null && <div style={vertical ? { position: 'absolute', left: 0, right: 0, bottom: `${dbToPos(Array.isArray(peak) ? peak[i] : peak) * 100}%`, height: 2, background: 'var(--dd-paper-1)' } : { position: 'absolute', top: 0, bottom: 0, left: `${dbToPos(Array.isArray(peak) ? peak[i] : peak) * 100}%`, width: 2, background: 'var(--dd-paper-1)' }} />}
      </div>
    );
  };
  return (
    <div style={{ display: 'inline-flex', flexDirection: vertical ? 'column' : 'row', alignItems: 'center', gap: 4 }}>
      {vertical && (
        <button type="button" onClick={onResetClip} title={clip ? 'Clipped — click to reset' : 'No clipping'} aria-label="Clip indicator"
          style={{ width: thickness * levels.length + 2 * (levels.length - 1), height: 4, padding: 0, border: 0, borderRadius: 1, background: clip ? 'var(--dd-error)' : 'var(--dd-ink-0)', cursor: 'pointer' }} />
      )}
      <div style={{ display: 'flex', flexDirection: vertical ? 'row' : 'column', gap: 2 }}>{levels.map(bar)}</div>
      {label && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', color: 'var(--text-tertiary)' }}>{label}</span>}
    </div>
  );
}
