// Das Sampler — waveform panel with Region / Slices views.
(() => {
const { Panel, Tabs, WaveformPanel, EmptyState, NumericField, ListRow, Button, Icon, SegmentedControl } = window.DandrumDesignSystem_3c2eab;

const tagStyle = { display: 'inline-flex', alignItems: 'center', gap: 4, height: 20, padding: '0 6px', borderRadius: 2, border: '1px solid var(--border-control)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', textTransform: 'uppercase', color: 'var(--text-secondary)', whiteSpace: 'nowrap' };

function WaveView({ patch, view, onView, pad, region, regionIdx, onRegion, cursor, startOffset, hostStart, startMod, compact, height, slice, onSlice, onLoadBreak, onLoadKit, missingRegion }) {
  const isBreak = patch.kind === 'break';
  const allRegions = pad ? pad.layers.flatMap((l) => l.regions.map((r) => ({ ...r, layer: l.name, multi: pad.layers.length > 1 }))) : [];
  const tabs = [{ id: 'region', label: 'Region' }, { id: 'slices', label: 'Slices', badge: 'Break patch' }];
  const h = height ?? (compact ? 150 : 250);
  return (
    <Panel compact={compact} padding={0} style={{ flex: 'none' }}
      summary={view === 'slices' ? 'Slices' : region ? region.file : 'Region'}
      title={<div data-panel-action="" style={{ marginLeft: compact ? -6 : -8 }}><Tabs compact={compact} value={view} onChange={onView} items={tabs} /></div>}
      actions={view === 'region' && pad && !isBreak ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          {!compact && <span style={tagStyle}><Icon name={pad.mode === 'Gated' ? 'gate' : 'one-shot'} size={12} />{pad.mode}</span>}
          {allRegions.length > 1 && (
            <SegmentedControl compact value={String(regionIdx)} onChange={(v) => onRegion(Number(v))}
              options={allRegions.map((r, i) => ({ id: String(i), label: r.multi ? (r.layer + (r.alt ? ' ' + r.alt : '')) : 'Alt ' + r.alt }))} />
          )}
        </div>
      ) : null}>
      <div style={{ padding: compact ? 8 : 12, display: 'flex', flexDirection: 'column', gap: 8 }}>
        {view === 'region' && !isBreak && (pad && region ? (
          <>
            <WaveformPanel kind={region.kind} height={h} regionStart={region.start} regionEnd={region.end}
              fadeIn={region.fadeIn / region.len} fadeOut={region.fadeOut / region.len} cursor={cursor} startOffset={startOffset}
              hostAutomated={hostStart} startModulation={startMod} compact={compact} missing={missingRegion}
              label={`${region.file} · ${region.len} ms`} />
            {!compact && (
              <div style={{ display: 'flex', gap: 16, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-tertiary)' }}>
                <span>Region <b style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-value)', fontWeight: 500 }}>{Math.round(region.start * region.len)}–{Math.round(region.end * region.len)} ms</b></span>
                <span>Fade in <b style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-value)', fontWeight: 500 }}>{region.fadeIn} ms</b></span>
                <span>Fade out <b style={{ color: 'var(--text-secondary)', fontFamily: 'var(--font-value)', fontWeight: 500 }}>{region.fadeOut} ms</b></span>
                <span>Loop <b style={{ color: 'var(--text-secondary)', fontWeight: 500 }}>none</b></span>
                <div style={{ flex: 1 }} />
                <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}><Icon name="lock" size={12} />Markers show prepared data</span>
              </div>
            )}
          </>
        ) : (
          <div style={{ height: h + (compact ? 0 : 24), display: 'flex' }}><div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
            <EmptyState icon="plus" title="No sound on this pad" compact={compact}>This note is available for a future mapping. Add it to the patch, then reload.</EmptyState>
          </div></div>
        ))}
        {view === 'region' && isBreak && (
          <div style={{ height: h + 24, display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
            <EmptyState icon="slice" title="This patch plays slices" compact={compact} action={<Button size="sm" onClick={() => onView('slices')}>Open Slices</Button>}>Example Break is one prepared sample divided into 8 slices.</EmptyState>
          </div>
        )}
        {view === 'slices' && !isBreak && (
          <div style={{ height: h + (compact ? 0 : 24), display: 'flex', flexDirection: 'column', justifyContent: 'center' }}>
            <EmptyState icon="slice" title="Slices belong to sliced-break patches" compact={compact} action={<Button size="sm" icon="folder" onClick={onLoadBreak}>Load example break patch</Button>}>
              Reference Drum Kit has no break sample. Its pads play separate sample regions.
            </EmptyState>
          </div>
        )}
        {view === 'slices' && isBreak && (
          <>
            <WaveformPanel kind="break" height={h} slices={patch.slices.map((pos, i) => ({ pos, name: patch.sliceNames && patch.sliceNames[i] }))} selectedSlice={slice} cursor={cursor} compact={compact} label={`${patch.file} · ${patch.len} ms`} />
            <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
              <NumericField label="Slice index" value={slice + 1} min={1} max={patch.slices.length} onChange={(v) => onSlice(v - 1)} compact={compact} />
              <div style={{ display: 'flex', flexDirection: 'column', gap: 3, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-tertiary)' }}>
                <span><b style={{ color: 'var(--text-primary)', fontWeight: 600 }}>{patch.sliceNames ? patch.sliceNames[slice] : ''}</b> · slice {slice + 1} of {patch.slices.length} · note <b style={{ fontFamily: 'var(--font-value)', color: 'var(--text-secondary)', fontWeight: 500 }}>{36 + slice}</b> · {Math.round(patch.slices[slice] * patch.len)}–{Math.round((patch.slices[slice + 1] ?? 1) * patch.len)} ms</span>
                <span>Slice index is a host parameter; mapped notes 36–43 also trigger slices.</span>
              </div>
              <div style={{ flex: 1 }} />
              {!compact && <Button size="sm" variant="ghost" onClick={onLoadKit}>Back to Reference Drum Kit</Button>}
            </div>
          </>
        )}
      </div>
    </Panel>
  );
}

Object.assign(window, { WaveView });
})();
