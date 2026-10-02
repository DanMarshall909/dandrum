import React from 'react';
import { Knob } from '../controls/Knob.jsx';
import { WaveformPanel } from './WaveformPanel.jsx';
import { ContextMenu, MenuButton } from '../navigation/ContextMenu.jsx';
const lsSendNorm = (db) => db == null || db <= -60 ? 0 : (db + 60) / 66;
const lsSendDb = (n) => n <= 0.001 ? null : Math.round((-60 + n * 66) * 10) / 10;
const lsSendText = (db) => db == null ? '−∞ dB' : (db === 0 ? '0.0 dB' : `${db < 0 ? '−' : '+'}${Math.abs(db).toFixed(1)} dB`);

const lsLabel = { fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: '0.08em', textTransform: 'uppercase', color: 'var(--text-tertiary)' };
const lsValue = { fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', color: 'var(--text-secondary)', whiteSpace: 'nowrap' };
const lsFmt = (p, v = p.value) => {
  if (p.options) return String(v);
  if (p.unit === 'Hz') return v >= 1000 ? `${(v / 1000).toFixed(v >= 10000 ? 0 : 1)} kHz` : `${Math.round(v)} Hz`;
  if (p.unit === 'dB') return v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;
  if (p.unit === '%') return `${Math.round(v)}%`;
  if (p.unit === 'ms') return `${Math.round(v)} ms`;
  return `${(+v).toFixed(2)}${p.unit ? ' ' + p.unit : ''}`;
};
const lsNorm = (p, v) => p.log ? Math.log(v / p.min) / Math.log(p.max / p.min) : (v - p.min) / (p.max - p.min);
const lsDenorm = (p, n) => p.log ? p.min * Math.pow(p.max / p.min, n) : p.min + n * (p.max - p.min);
const lsSummary = (m) => {
  const s = (m.params || []).filter((p) => p.summary);
  return s.length ? s.map((p) => lsFmt(p)).join(' ') : m.value;
};
const lsDb = (v) => v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;

/**
 * Layers that all trigger together for one zone or pad. Each layer is a source (synth engine, sample, patch)
 * followed by an inline chain of modules (filter, gain, saturation…) processed left to right.
 * Optional routing per row: a send knob for each FX bus (fxBusses) and an output menu (outputs). Use the same component for FX busses (rows with source "FX bus").
 * Click the source block to open the source editor (sample: waveform + playback params; synth: engine params).
 * Click a module to open its editor under the layer (click again or Esc to close); its dot bypasses, × removes; the dashed slot adds a module.
 */
export function LayerStack({ layers = [], selectedId, onSelect, onChange, onAddModule, fxBusses = [], outputs = [], countLabel, title = 'Layers', subtitle, editable = true, style }) {
  const [menu, setMenu] = React.useState(null);
  const [local, setLocal] = React.useState(layers);
  const ref = React.useRef(layers);
  React.useEffect(() => { setLocal(layers); ref.current = layers; }, [layers]);
  const [sel, setSel] = React.useState(selectedId ?? layers[0]?.id);
  React.useEffect(() => { if (selectedId !== undefined) setSel(selectedId); }, [selectedId]);
  const [selMod, setSelMod] = React.useState(null);
  const [hoverMod, setHoverMod] = React.useState(null);

  const select = (id) => { setSel(id); onSelect && onSelect(id); };
  const commit = (next, id) => { ref.current = next; setLocal(next); onChange && onChange(next, id); };
  const patchLayer = (lid, fn) => commit(ref.current.map((l) => l.id === lid ? fn(l) : l), lid);
  const toggleMute = (l) => patchLayer(l.id, (o) => ({ ...o, muted: !o.muted }));
  const toggleBypass = (l, m) => patchLayer(l.id, (o) => ({ ...o, modules: o.modules.map((x) => x.id === m.id ? { ...x, bypassed: !x.bypassed } : x) }));
  const setParam = (l, m, pid, v) => patchLayer(l.id, (o) => ({ ...o, modules: o.modules.map((x) => x.id === m.id ? { ...x, params: x.params.map((p) => p.id === pid ? { ...p, value: v } : p) } : x) }));
  const setSrcParam = (l, pid, v) => patchLayer(l.id, (o) => ({ ...o, params: o.params.map((p) => p.id === pid ? { ...p, value: v } : p) }));
  const removeMod = (l, m) => patchLayer(l.id, (o) => ({ ...o, modules: o.modules.filter((x) => x.id !== m.id) }));

  const stop = (e) => e.stopPropagation();
  const box = { boxSizing: 'border-box', height: 38, borderRadius: 'var(--radius-2, 4px)', display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 3, padding: '0 8px', flex: 'none' };
  const wire = <span style={{ width: 8, height: 1, background: 'var(--dd-line-3)', flex: 'none' }} />;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minWidth: 0, userSelect: 'none', ...style }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, minHeight: 18 }}>
        <span style={lsLabel}>{title}</span>
        {subtitle && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap' }}>{subtitle}</span>}
        <span style={{ ...lsValue, color: 'var(--text-tertiary)', marginLeft: 'auto' }}>{countLabel ?? `${local.length} ${local.length === 1 ? 'layer' : 'layers'} · all trigger`}</span>
      </div>
      {local.map((l) => {
        const isSel = l.id === sel;
        const srcKey = l.id + '::src', srcOpen = selMod === srcKey;
        const mod = (l.modules || []).find((m) => selMod === l.id + m.id);
        const openMod = srcOpen ? { id: '::src', type: `${l.source || 'Source'} · ${l.name}`, params: l.params, src: true } : mod;
        const setP = (pid, v) => srcOpen ? setSrcParam(l, pid, v) : setParam(l, mod, pid, v);
        const startP = srcOpen && (l.params || []).find((p) => p.id === 'start');
        const hasSrcEditor = !!(l.params || l.waveform);
        const outName = (outputs.find((o) => o.id === l.output) || outputs[0] || {}).name;
        return (
          <div key={l.id} style={{ display: 'flex', flexDirection: 'column' }}>
          <div role="button" tabIndex={0} aria-pressed={isSel} onPointerDown={() => select(l.id)} onKeyDown={(e) => { if (e.target === e.currentTarget && (e.key === ' ' || e.key === 'Enter')) { e.preventDefault(); select(l.id); } }}
            style={{ display: 'flex', alignItems: 'center', gap: 12, padding: 6, background: isSel ? 'var(--dd-ink-4)' : 'var(--dd-ink-3)', border: `1px solid ${isSel ? 'var(--dd-vermilion)' : 'var(--border-hairline)'}`, borderRadius: openMod ? '4px 4px 0 0' : 'var(--radius-2, 4px)', cursor: 'pointer', outline: 'none' }}>
            <div style={{ flex: 1, minWidth: 0, display: 'flex', alignItems: 'center', overflowX: 'auto', opacity: l.muted ? 0.5 : 1 }}>
              <div role="button" tabIndex={0} aria-expanded={srcOpen} aria-label={`Edit ${l.source || 'source'} ${l.name}`}
                onPointerDown={(e) => { stop(e); select(l.id); if (hasSrcEditor) setSelMod(srcOpen ? null : srcKey); }}
                onKeyDown={(e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); stop(e); setSelMod(srcOpen ? null : srcKey); } if (e.key === 'Escape') setSelMod(null); }}
                onPointerEnter={() => setHoverMod(srcKey)} onPointerLeave={() => setHoverMod(null)}
                style={{ ...box, minWidth: 190, maxWidth: 280, cursor: hasSrcEditor ? 'pointer' : 'default', background: hoverMod === srcKey && hasSrcEditor ? 'var(--dd-ink-3)' : 'var(--dd-ink-2)', border: `1px solid ${srcOpen ? 'var(--dd-vermilion)' : 'var(--border-control)'}`, outline: 'none' }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  {l.source && <span style={{ ...lsLabel, padding: '1px 4px', background: 'var(--dd-ink-0)', borderRadius: 2, color: 'var(--text-secondary)', flex: 'none' }}>{l.source}</span>}
                  <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', fontWeight: 600, color: 'var(--text-primary)', whiteSpace: 'nowrap', flex: 'none' }}>{l.name}</span>
                </span>
                {l.detail && <span style={{ ...lsValue, color: 'var(--text-tertiary)', overflow: 'hidden', textOverflow: 'ellipsis' }}>{l.detail}</span>}
              </div>
              {(l.modules || []).map((m) => {
                const key = l.id + m.id, mSel = selMod === key, hot = hoverMod === key;
                return (
                  <React.Fragment key={m.id}>
                    {wire}
                    <div role="button" tabIndex={0} aria-label={`${m.type}${m.bypassed ? ', bypassed' : ''}`} onPointerDown={(e) => { stop(e); select(l.id); setSelMod(mSel ? null : key); }}
                      onKeyDown={(e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); stop(e); setSelMod(mSel ? null : key); } if (e.key === 'Escape') setSelMod(null); }}
                      aria-expanded={mSel}
                      onPointerEnter={() => setHoverMod(key)} onPointerLeave={() => setHoverMod(null)}
                      style={{ ...box, position: 'relative', minWidth: 88, background: hot ? 'var(--surface-control-hover)' : 'var(--surface-control)', border: `1px solid ${mSel ? 'var(--dd-vermilion)' : 'var(--border-control)'}`, outline: 'none', opacity: m.bypassed ? 0.55 : 1 }}>
                      <span style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
                        <button type="button" aria-label={m.bypassed ? 'Enable module' : 'Bypass module'} aria-pressed={!m.bypassed} onPointerDown={stop} onClick={(e) => { stop(e); toggleBypass(l, m); }}
                          title={m.bypassed ? 'Bypassed' : 'On'}
                          style={{ width: 8, height: 8, padding: 0, borderRadius: 8, cursor: 'pointer', background: m.bypassed ? 'transparent' : 'var(--dd-paper-1)', border: `1px solid ${m.bypassed ? 'var(--dd-paper-3)' : 'var(--dd-paper-1)'}`, flex: 'none' }} />
                        <span style={{ ...lsLabel, color: 'var(--text-secondary)', textDecoration: m.bypassed ? 'line-through' : 'none' }}>{m.type}</span>
                        {editable && hot && (
                          <button type="button" aria-label={`Remove ${m.type}`} onPointerDown={stop} onClick={(e) => { stop(e); removeMod(l, m); }}
                            style={{ marginLeft: 'auto', width: 14, height: 14, padding: 0, background: 'transparent', border: 'none', color: 'var(--text-tertiary)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', lineHeight: 1, cursor: 'pointer' }}>×</button>
                        )}
                      </span>
                      <span style={{ ...lsValue, color: 'var(--text-primary)' }}>{lsSummary(m)}</span>
                    </div>
                  </React.Fragment>
                );
              })}
              {editable && <>
                {wire}
                <button type="button" aria-label={`Add module to ${l.name}`} title="Add module…" onPointerDown={stop} onClick={(e) => { stop(e); onAddModule && onAddModule(l.id); }}
                  style={{ ...box, width: 30, padding: 0, alignItems: 'center', background: 'transparent', border: '1px dashed var(--border-strong)', color: 'var(--text-tertiary)', fontFamily: 'var(--font-value)', fontSize: 'var(--type-body)', cursor: 'pointer' }}>+</button>
              </>}
            </div>
            {fxBusses.length > 0 && (
              <div style={{ display: 'flex', gap: 8, flex: 'none', paddingLeft: 4, borderLeft: '1px solid var(--border-hairline)' }} onPointerDown={stop}>
                {fxBusses.map((b) => {
                  const db = (l.sends || {})[b.id];
                  return (
                    <div key={b.id} title={`Send to ${b.name}`} style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 2 }}>
                      <span style={{ ...lsLabel, color: db == null ? 'var(--text-disabled)' : 'var(--text-secondary)' }}>{b.letter || b.name}</span>
                      <Knob size="xs" showLabel={false} label={`Send ${b.name}`} value={lsSendNorm(db)} defaultValue={0} valueText={lsSendText(db)}
                        onChange={(n) => patchLayer(l.id, (o) => ({ ...o, sends: { ...(o.sends || {}), [b.id]: lsSendDb(n) } }))} />
                    </div>
                  );
                })}
              </div>
            )}
            {outputs.length > 0 && (
              <span onPointerDown={stop} style={{ flex: 'none' }}
                onClickCapture={(e) => { const r = e.currentTarget.getBoundingClientRect(); setMenu(menu && menu.id === l.id ? null : { id: l.id, x: r.left, y: r.bottom + 4 }); }}>
                <MenuButton compact width={124} value={outName} />
              </span>
            )}
            <span style={{ ...lsValue, width: 56, textAlign: 'right', color: 'var(--text-primary)', flex: 'none' }}>{lsDb(l.level ?? 0)}</span>
            <button type="button" aria-label={`${l.muted ? 'Unmute' : 'Mute'} ${l.name}`} aria-pressed={!!l.muted} onPointerDown={stop} onClick={(e) => { stop(e); toggleMute(l); }}
              style={{ width: 22, height: 20, padding: 0, flex: 'none', background: l.muted ? 'var(--dd-warn)' : 'var(--surface-control)', color: l.muted ? 'var(--text-on-accent)' : 'var(--text-secondary)', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-2, 4px)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 700, cursor: 'pointer' }}>M</button>
          </div>
          {openMod && (
            <div role="region" aria-label={`${openMod.type} editor`} onKeyDown={(e) => { if (e.key === 'Escape') setSelMod(null); }}
              style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: '10px 12px 12px', background: 'var(--dd-ink-2)', border: '1px solid var(--dd-vermilion)', borderTop: '1px solid var(--border-hairline)', borderRadius: '0 0 4px 4px', opacity: openMod.bypassed ? 0.6 : 1 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                {!openMod.src && <button type="button" aria-label={openMod.bypassed ? 'Enable module' : 'Bypass module'} aria-pressed={!openMod.bypassed} onClick={() => toggleBypass(l, openMod)}
                  style={{ width: 10, height: 10, padding: 0, borderRadius: 10, cursor: 'pointer', background: openMod.bypassed ? 'transparent' : 'var(--dd-paper-1)', border: `1px solid ${openMod.bypassed ? 'var(--dd-paper-3)' : 'var(--dd-paper-1)'}` }} />}
                <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', fontWeight: 600, letterSpacing: '0.06em', textTransform: 'uppercase', color: 'var(--text-primary)' }}>{openMod.src ? (l.source || 'Source') : openMod.type}</span>
                <span style={{ ...lsValue, color: 'var(--text-tertiary)' }}>{openMod.src ? (l.detail || l.name) : l.name}{openMod.bypassed ? ' · bypassed' : ''}</span>
                <button type="button" aria-label="Close editor" onClick={() => setSelMod(null)}
                  style={{ marginLeft: 'auto', width: 20, height: 20, padding: 0, background: 'var(--surface-control)', border: '1px solid var(--border-control)', borderRadius: 'var(--radius-2, 4px)', color: 'var(--text-secondary)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', lineHeight: 1, cursor: 'pointer' }}>×</button>
              </div>
              {openMod.src && l.waveform && (
                <WaveformPanel height={96} {...l.waveform} startOffset={startP ? startP.value / 100 : l.waveform.startOffset} />
              )}
              {(openMod.params || []).length ? (
                <div style={{ display: 'flex', alignItems: 'flex-start', gap: 20, flexWrap: 'wrap' }}>
                  {openMod.params.map((p) => p.options ? (
                    <div key={p.id} style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                      <span style={lsLabel}>{p.label}</span>
                      <div role="radiogroup" aria-label={p.label} style={{ display: 'flex', background: 'var(--dd-ink-0)', borderRadius: 'var(--radius-2, 4px)', padding: 2, gap: 2 }}>
                        {p.options.map((o) => (
                          <button key={o} type="button" role="radio" aria-checked={p.value === o} onClick={() => setP(p.id, o)}
                            style={{ height: 22, padding: '0 8px', border: 'none', borderRadius: 3, cursor: 'pointer', background: p.value === o ? 'var(--dd-ink-5)' : 'transparent', boxShadow: p.value === o ? 'var(--shadow-cap)' : 'none', color: p.value === o ? 'var(--text-primary)' : 'var(--text-tertiary)', fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)', whiteSpace: 'nowrap', flex: 'none' }}>{o}</button>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <Knob key={p.id} size="md" label={p.label} bipolar={p.bipolar} valueDisplay="always"
                      value={lsNorm(p, p.value)} defaultValue={p.default != null ? lsNorm(p, p.default) : undefined} valueText={lsFmt(p)}
                      onChange={(n) => setP(p.id, lsDenorm(p, n))} />
                  ))}
                </div>
              ) : <span style={{ ...lsValue, color: 'var(--text-tertiary)' }}>{openMod.src ? 'This source has no editable parameters.' : 'This module has no parameters.'}</span>}
            </div>
          )}
          </div>
        );
      })}
      {menu && (() => {
        const ml = local.find((x) => x.id === menu.id);
        if (!ml) return null;
        return <>
          <div onPointerDown={() => setMenu(null)} style={{ position: 'fixed', inset: 0, zIndex: 99 }} />
          <ContextMenu title="Output" x={menu.x} y={menu.y} width={200} onClose={() => setMenu(null)}
            items={outputs.map((o) => ({ label: o.name, shortcut: o.note, checked: (ml.output ?? outputs[0].id) === o.id, onSelect: () => { patchLayer(ml.id, (x) => ({ ...x, output: o.id })); setMenu(null); } }))} />
        </>;
      })()}
    </div>
  );
}
