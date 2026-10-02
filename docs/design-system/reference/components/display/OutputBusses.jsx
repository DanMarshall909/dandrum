import React from 'react';
import { Meter } from './Meter.jsx';
import { ContextMenu, MenuButton } from '../navigation/ContextMenu.jsx';

const obLabel = { fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--text-tertiary)' };
const obValue = { fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-secondary)', whiteSpace: 'nowrap' };
const obDb = (v) => v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;

/**
 * Output busses: each maps to a plugin output channel pair (Main 1/2, 3/4…), shows what feeds it, a stereo meter, level and mute.
 * Layers and FX busses pick one of these as their output.
 */
export function OutputBusses({ busses = [], channelOptions = ['1/2', '3/4', '5/6', '7/8', '9/10', '11/12', '13/14', '15/16'], onChange, title = 'Output busses', style }) {
  const [local, setLocal] = React.useState(busses);
  const ref = React.useRef(busses);
  React.useEffect(() => { setLocal(busses); ref.current = busses; }, [busses]);
  const [menu, setMenu] = React.useState(null);
  const patch = (id, p) => { const next = ref.current.map((b) => b.id === id ? { ...b, ...p } : b); ref.current = next; setLocal(next); onChange && onChange(next, id); };
  const used = (ch, id) => local.some((b) => b.id !== id && b.channels === ch);
  const grid = { display: 'grid', gridTemplateColumns: 'minmax(110px, 1fr) 84px minmax(120px, 2fr) 120px 56px 22px', alignItems: 'center', columnGap: 12 };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minWidth: 0, userSelect: 'none', ...style }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, minHeight: 18 }}>
        <span style={obLabel}>{title}</span>
        <span style={{ ...obValue, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>{local.length} of {channelOptions.length} stereo outputs</span>
      </div>
      <div style={{ ...grid, padding: '0 9px' }}>
        <span style={obLabel}>Bus</span><span style={obLabel}>Plugin out</span><span style={obLabel}>Fed by</span><span style={obLabel}>Level</span><span /><span />
      </div>
      {local.map((b) => (
        <div key={b.id} style={{ ...grid, padding: '6px 8px', background: 'var(--dd-ink-3)', border: '1px solid var(--border-hairline)', borderRadius: 'var(--radius-2, 4px)', opacity: b.muted ? 0.55 : 1 }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 6, minWidth: 0 }}>
            {b.main && <span style={{ ...obLabel, padding: '1px 4px', background: 'var(--dd-ink-0)', borderRadius: 2, color: 'var(--text-secondary)' }}>Main</span>}
            <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap' }}>{b.name}</span>
          </span>
          <span onClickCapture={(e) => { const r = e.currentTarget.getBoundingClientRect(); setMenu(menu && menu.id === b.id ? null : { id: b.id, x: r.left, y: r.bottom + 4 }); }}>
            <MenuButton compact width={84} value={`Out ${b.channels}`} />
          </span>
          <span style={{ ...obValue, color: (b.feeds || []).length ? 'var(--text-secondary)' : 'var(--text-disabled)', overflow: 'hidden', textOverflow: 'ellipsis' }} title={(b.feeds || []).join(', ')}>
            {(b.feeds || []).length ? b.feeds.join(' · ') : 'Nothing routed'}
          </span>
          <Meter orientation="horizontal" compact levels={b.levels || [-90, -90]} length={120} thickness={4} />
          <span style={{ ...obValue, textAlign: 'right', color: 'var(--text-primary)' }}>{obDb(b.level ?? 0)}</span>
          <button type="button" aria-label={`${b.muted ? 'Unmute' : 'Mute'} ${b.name}`} aria-pressed={!!b.muted} onClick={() => patch(b.id, { muted: !b.muted })}
            style={{ width: 22, height: 20, padding: 0, background: b.muted ? 'var(--dd-warn)' : 'var(--surface-control)', color: b.muted ? 'var(--text-on-accent)' : 'var(--text-secondary)', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-2, 4px)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 700, cursor: 'pointer' }}>M</button>
        </div>
      ))}
      {menu && (() => {
        const b = local.find((x) => x.id === menu.id);
        return <>
          <div onPointerDown={() => setMenu(null)} style={{ position: 'fixed', inset: 0, zIndex: 99 }} />
          <ContextMenu title={`${b.name} · plugin output`} x={menu.x} y={menu.y} width={200} onClose={() => setMenu(null)}
            items={channelOptions.map((ch) => ({ label: `Out ${ch}`, checked: b.channels === ch, shortcut: used(ch, b.id) ? 'shared' : undefined, onSelect: () => { patch(b.id, { channels: ch }); setMenu(null); } }))} />
        </>;
      })()}
    </div>
  );
}
