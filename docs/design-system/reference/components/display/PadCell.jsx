import React from 'react';
import { Icon } from '../icons/Icon.jsx';

const NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
/** MIDI note → name using the C4 = 60 convention (36 = C2). */
export function noteName(n) { return NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1); }

/**
 * Playable pad. Top: MIDI note + name. Middle: sound name. Bottom: velocity bar + alternate dots.
 * `level` (0–1) is the live activity brightness the editor decays each frame after a hit.
 * Click position sets velocity (top of pad = 127, bottom = 1).
 */
export function PadCell({
  note = 36, name, mapped = true, selected = false, level = 0, velocity = 0, alternates = 0, activeAlternate = -1,
  layers = 0, activeLayer = -1, chokeGroup, choked = false, missing = false, focused = false, disabled = false,
  compact = false, size, onTrigger, onSelect, onRelease, style,
}) {
  const [hover, setHover] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const s = size ?? (compact ? 52 : 76);
  const isFocused = focused || focus;
  const lit = Math.max(0, Math.min(1, level));
  const trigger = (e) => {
    if (disabled || e.button !== 0) return;
    const r = e.currentTarget.getBoundingClientRect();
    const v = Math.max(0.05, Math.min(1, 1 - (e.clientY - r.top) / r.height + 0.15));
    onSelect && onSelect(note);
    if (mapped && onTrigger) onTrigger(note, v);
  };

  if (!mapped) {
    return (
      <button type="button" aria-label={`Pad ${note} ${noteName(note)}, empty`} onClick={() => onSelect && onSelect(note)}
        onFocus={() => setFocus(true)} onBlur={() => setFocus(false)} onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
        style={{
          width: s, height: s, padding: compact ? 5 : 7, boxSizing: 'border-box', display: 'flex', flexDirection: 'column', justifyContent: 'space-between',
          background: hover ? 'var(--dd-ink-3)' : 'transparent', border: `1px dashed ${selected ? 'var(--dd-vermilion)' : 'var(--dd-line-2)'}`, borderRadius: 'var(--radius-2)',
          outline: 'none', cursor: 'pointer', boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : 'none', ...style,
        }}>
        <PadHeader note={note} compact={compact} dim />
        {!compact && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', color: 'var(--text-disabled)', textAlign: 'left' }}>Empty</span>}
      </button>
    );
  }

  return (
    <div role="button" tabIndex={disabled ? -1 : 0} aria-label={`Pad ${note} ${noteName(note)}, ${name}`} aria-pressed={selected}
      onPointerDown={trigger} onPointerUp={() => onRelease && onRelease(note)} onPointerLeave={() => { setHover(false); }}
      onMouseEnter={() => setHover(true)}
      onKeyDown={(e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); onSelect && onSelect(note); onTrigger && onTrigger(note, 0.8); } }}
      onFocus={() => setFocus(true)} onBlur={() => setFocus(false)}
      style={{
        width: s, height: s, padding: compact ? 5 : 7, boxSizing: 'border-box', position: 'relative', overflow: 'hidden',
        display: 'flex', flexDirection: 'column', justifyContent: 'space-between', userSelect: 'none', touchAction: 'none',
        background: disabled ? 'var(--dd-ink-3)' : hover ? 'var(--dd-pad-hover)' : 'var(--dd-ink-4)', borderRadius: 'var(--radius-2)', outline: 'none',
        cursor: disabled ? 'default' : 'pointer',
        border: `1px solid ${selected ? 'var(--dd-vermilion)' : 'var(--dd-line-2)'}`,
        boxShadow: [
          isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : null,
          selected ? 'inset 0 0 0 1px var(--dd-vermilion)' : null,
          'var(--shadow-cap)', 'var(--highlight-cap)',
        ].filter(Boolean).join(', '),
        ...style,
      }}>
      {/* activity light: a top-lit wash whose opacity follows level */}
      <div style={{ position: 'absolute', inset: 0, background: 'var(--dd-paper-1)', opacity: lit * 0.16, pointerEvents: 'none' }} />
      <div style={{ position: 'absolute', left: 0, right: 0, top: 0, height: 3, background: 'var(--dd-paper-1)', opacity: lit, pointerEvents: 'none' }} />
      <PadHeader note={note} compact={compact} chokeGroup={chokeGroup} />
      <div style={{ position: 'relative', fontFamily: 'var(--font-ui)', fontSize: compact ? 'var(--type-label)' : 'var(--type-body)', fontWeight: 600, lineHeight: 1.1, color: missing ? 'var(--dd-error)' : choked ? 'var(--text-tertiary)' : 'var(--text-primary)', textAlign: 'left', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', display: 'flex', alignItems: 'center', gap: 4 }}>
        {missing && <Icon name="file-missing" size={14} />}
        <span style={{ textDecoration: choked ? 'line-through' : 'none' }}>{name}</span>
      </div>
      <div style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 4, height: 6 }}>
        <div style={{ flex: 1, height: 3, borderRadius: 1, background: 'var(--dd-ink-0)', overflow: 'hidden' }}>
          <div style={{ width: `${velocity * 100}%`, height: '100%', background: 'var(--dd-paper-2)', opacity: 0.4 + lit * 0.6 }} />
        </div>
        {layers > 1 && !compact && (
          <div title={`${layers} velocity layers`} style={{ display: 'flex', flexDirection: 'column-reverse', gap: 1 }}>
            {Array.from({ length: layers }).map((_, i) => <span key={i} style={{ width: 6, height: 2, borderRadius: 1, background: i === activeLayer ? 'var(--dd-paper-1)' : 'var(--dd-ink-6)' }} />)}
          </div>
        )}
        {alternates > 1 && (
          <div title={`${alternates} alternates`} style={{ display: 'flex', gap: 2 }}>
            {Array.from({ length: alternates }).map((_, i) => <span key={i} style={{ width: 4, height: 4, borderRadius: 4, background: i === activeAlternate ? 'var(--dd-paper-1)' : 'transparent', boxShadow: 'inset 0 0 0 1px var(--dd-paper-3)' }} />)}
          </div>
        )}
      </div>
    </div>
  );
}

function PadHeader({ note, compact, dim, chokeGroup }) {
  return (
    <div style={{ position: 'relative', display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 4, fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', lineHeight: 1, color: dim ? 'var(--text-disabled)' : 'var(--text-tertiary)' }}>
      <span style={{ display: 'flex', gap: 5 }}><span style={{ color: dim ? 'var(--text-disabled)' : 'var(--text-secondary)' }}>{note}</span>{!compact && <span>{noteName(note)}</span>}</span>
      {chokeGroup != null && (
        <span title={`Choke group ${chokeGroup}`} style={{ display: 'flex', alignItems: 'center', gap: 2, padding: '1px 3px', borderRadius: 2, background: 'var(--dd-ink-0)', color: 'var(--text-secondary)', fontFamily: 'var(--font-ui)', fontWeight: 600 }}>
          <Icon name="choke" size={11} />{chokeGroup}
        </span>
      )}
    </div>
  );
}
