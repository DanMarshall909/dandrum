// Das Sampler — 4×4 pad grid with choke bracket.
(() => {
const { PadCell, Panel, Icon: PadIcon } = window.DandrumDesignSystem_3c2eab;

const PAD_ROWS = [[48, 49, 50, 51], [44, 45, 46, 47], [40, 41, 42, 43], [36, 37, 38, 39]];

function PadGrid({ patch, selected, onSelect, onTrigger, activity, compact, chokeFlash, missing }) {
  const size = compact ? 52 : 76, gap = compact ? 5 : 6;
  const pads = patch.kind === 'kit' ? patch.pads : null;
  const info = (n) => {
    if (pads) return pads[n];
    const i = n - 36; return i >= 0 && i < patch.slices.length ? { name: (patch.sliceNames && patch.sliceNames[i]) || 'Slice ' + (i + 1), short: (patch.sliceNames && patch.sliceNames[i]) || 'Slice ' + (i + 1), layers: [{ regions: [{}] }] } : null;
  };
  // choke bracket geometry: column 3 (index 2), rows of 46 (1) and 42 (2)
  const hasChoke = pads && pads[42] && pads[46];
  const bx = 3 * size + 2 * gap + gap / 2;
  const by0 = 1 * (size + gap) + size * 0.3, by1 = 2 * (size + gap) + size * 0.7;
  return (
    <Panel title="Pads" tag={patch.kind === 'kit' ? 'Notes 36–51' : 'Slices → 36–43'} compact={compact} style={{ flex: 'none' }} summary={`selected ${selected}`}>
      <div style={{ position: 'relative', display: 'grid', gridTemplateColumns: `repeat(4, ${size}px)`, gap }}>
        {PAD_ROWS.flat().map((n) => {
          const p = info(n);
          const a = activity[n] || {};
          if (!p) return <PadCell key={n} size={size} note={n} mapped={false} selected={selected === n} compact={compact} onSelect={onSelect} />;
          const layer = p.layers.length > 1 ? p.layers.length : 0;
          const alts = Math.max(...p.layers.map((l) => l.regions.length));
          return (
            <PadCell key={n} size={size} note={n} name={compact ? p.short : p.name} compact={compact} selected={selected === n}
              level={a.level || 0} velocity={a.velocity || 0} layers={layer} activeLayer={a.layer ?? -1}
              alternates={alts > 1 ? alts : 0} activeAlternate={a.alt ?? -1} chokeGroup={p.choke ?? undefined}
              choked={a.choked} missing={missing && n === 46}
              onSelect={onSelect} onTrigger={onTrigger} />
          );
        })}
        {hasChoke && (
          <div aria-hidden="true" style={{ position: 'absolute', left: bx - 1, top: by0, width: 2, height: by1 - by0, background: chokeFlash ? 'var(--dd-paper-1)' : 'var(--dd-line-3)', borderRadius: 1, transition: 'background 120ms' }}>
            <span style={{ position: 'absolute', left: -3, top: -1, width: 8, height: 2, background: 'inherit' }} />
            <span style={{ position: 'absolute', left: -3, bottom: -1, width: 8, height: 2, background: 'inherit' }} />
          </div>
        )}
      </div>
      {hasChoke && !compact && (
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 8, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-tertiary)' }}>
          <span style={{ display: 'flex', alignItems: 'center', gap: 2, padding: '1px 4px', borderRadius: 2, background: 'var(--dd-ink-0)', color: 'var(--text-secondary)', fontWeight: 600 }}><PadIcon name="choke" size={12} />1</span>
          Choke group 1: Closed Hat and Open Hat cut each other
        </div>
      )}
    </Panel>
  );
}

Object.assign(window, { PadGrid, PAD_ROWS });

})();
