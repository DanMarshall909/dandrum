// Das Sampler — live control strip (public host parameters) + modulation context menu wiring.
(() => {
const { Panel, Knob, Tooltip, ModIndicator, Icon } = window.DandrumDesignSystem_3c2eab;
const D = window.SAMPLER_DATA;

function LiveControls({ values, onChange, mods, liveMod, assigning, hostParam, focusedId, onMenu, onAssignTo, compact, hoverId, setHoverId }) {
  return (
    <Panel title="Live controls" compact={compact} style={{ flex: 1 }} summary={D.liveParams.map((p) => p.fmt(values[p.id]) + (p.unit ? ' ' + p.unit : '')).join(' · ')}
      actions={!compact && <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-tertiary)' }}><Icon name="host" size={14} />Patch-wide host parameters · automate in your DAW</span>}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, minmax(0, 1fr))', alignItems: 'start', height: '100%' }}>
        {D.liveParams.map((p, i) => {
          const m = (mods[p.id] || []).map((a) => ({ ...a, live: liveMod[a.slot] }));
          const isAssigningHere = assigning && !(mods[p.id] || []).some((a) => a.slot === assigning.slot);
          return (
            <div key={p.id} style={{ position: 'relative', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4, borderLeft: i ? '1px solid var(--border-hairline)' : 'none' }}
              onMouseEnter={() => setHoverId(p.id)} onMouseLeave={() => setHoverId(null)}
              onClickCapture={(e) => { if (assigning && isAssigningHere) { e.stopPropagation(); onAssignTo(p.id); } }}>
              <Knob size={compact ? 'md' : 'lg'} label={p.label} bipolar={p.bipolar} value={values[p.id]} defaultValue={p.def}
                valueText={p.fmt(values[p.id])} unit={p.unit} modulations={m.map((a) => ({ ...a, source: a.source }))} assigning={isAssigningHere} parseValue={p.parse}
                hostAutomated={hostParam === p.id} focused={focusedId === p.id}
                onChange={(v) => onChange(p.id, v)} onContextMenu={(e) => onMenu(p.id, e)} />
              {!compact && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', color: 'var(--text-disabled)' }}>{p.range}</span>}
            </div>
          );
        })}
      </div>
    </Panel>
  );
}

function buildModMenu({ param, value, mods, setDepth, remove, startAssign, reset, hostParam, close, sub, setSub }) {
  const p = D.liveParams.find((x) => x.id === param);
  const list = mods[param] || [];
  const used = new Set(list.map((a) => a.slot));
  if (sub) {
    const items = [{ label: 'Back', icon: 'chevron-left', onSelect: () => setSub(false) }, { separator: true }, { header: 'Choose a source' }];
    D.modSources.forEach((s) => items.push({ label: s.name, shortcut: s.slot, disabled: used.has(s.slot), onSelect: () => startAssign(param, s) }));
    items.push({ separator: true }, { note: 'After choosing, eligible controls are highlighted. Click more to add the same source; Esc finishes.' });
    return { title: `Assign modulation · ${p.label}`, items };
  }
  const items = [{ header: 'In-plugin modulation' }];
  list.forEach((a) => items.push({ type: 'assignment', slot: a.slot, source: a.source, depth: a.depth, onDepth: (d) => setDepth(param, a.slot, d), onRemove: () => remove(param, a.slot) }));
  if (!list.length) items.push({ note: 'No assignments yet.' });
  items.push({ label: 'Assign modulation…', icon: 'modulate', submenu: true, disabled: used.size >= D.modSources.length, onSelect: () => setSub(true) });
  items.push({ separator: true });
  items.push({ label: 'Reset to default', icon: 'reset', shortcut: 'Dbl-click', onSelect: () => { reset(param); close(); } });
  items.push({ label: 'Type value…', shortcut: 'Enter', onSelect: close });
  if (list.length > 1) items.push({ label: 'Remove all modulation', danger: true, onSelect: () => { list.forEach((a) => remove(param, a.slot)); } });
  items.push({ separator: true });
  items.push({ note: hostParam === param ? 'Your DAW is automating this parameter. Host automation and DAW modulation are edited in the DAW.' : 'Host automation and DAW-side modulation are edited in your DAW, not here.' });
  return { title: `${p.label} · ${p.fmt(value)}${p.unit ? ' ' + p.unit : ''}`, items };
}

Object.assign(window, { LiveControls, buildModMenu });
})();
