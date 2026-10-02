// Das Sampler — header bar and status bar.
(() => {
const { Button, IconButton, SegmentedControl, Meter, StatusMessage, MenuButton, ModGlyph } = window.DandrumDesignSystem_3c2eab;

function SamplerHeader({ patch, status, compact, onLayout, onReload, onPatchMenu, levels }) {
  return (
    <header style={{ height: compact ? 36 : 44, flex: 'none', display: 'flex', alignItems: 'center', gap: compact ? 8 : 12, padding: compact ? '0 8px' : '0 12px', background: 'var(--dd-ink-0)', borderBottom: '1px solid var(--border-hairline)' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, whiteSpace: 'nowrap' }}>
        <span style={{ fontFamily: 'var(--font-display)', fontWeight: 700, fontSize: compact ? 15 : 'var(--type-brand)', letterSpacing: '-0.01em', color: 'var(--text-primary)' }}>dandrum</span>
        <span style={{ width: 6, height: 6, borderRadius: 6, background: 'var(--dd-vermilion)', alignSelf: 'center' }} />
        <span style={{ fontFamily: 'var(--font-ui)', fontWeight: 700, fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)', letterSpacing: 'var(--tracking-heading)', textTransform: 'uppercase', color: 'var(--text-secondary)' }}>Das Sampler</span>
      </div>
      <div style={{ width: 1, height: 20, background: 'var(--border-hairline)' }} />
      <MenuButton label="Patch" value={patch.name} width={compact ? 180 : 240} onClick={onPatchMenu} compact={compact} />
      {!compact && <StatusMessage inline kind={status.kind} title={status.title}>{status.text}</StatusMessage>}
      {compact && <StatusMessage inline compact kind={status.kind} title={status.short || status.title} />}
      <div style={{ flex: 1 }} />
      <Button variant={status.kind === 'ok' ? 'secondary' : 'primary'} icon="reload" size={compact ? 'sm' : 'md'} onClick={onReload}>Reload Patch</Button>
      <SegmentedControl compact value={compact ? 'compact' : 'full'} onChange={onLayout} options={[{ id: 'full', label: 'Full', title: '1200 × 800' }, { id: 'compact', label: 'Compact', title: '820 × 560' }]} />
      <Meter levels={levels} thickness={4} length={compact ? 24 : 30} />
    </header>
  );
}

function SamplerStatusBar({ assigning, onEndAssign, voices, compact, focusHint }) {
  return (
    <footer style={{ height: compact ? 24 : 26, flex: 'none', display: 'flex', alignItems: 'center', gap: 12, padding: '0 12px', background: 'var(--dd-ink-0)', borderTop: '1px solid var(--border-hairline)', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-label)', color: 'var(--text-tertiary)', whiteSpace: 'nowrap', overflow: 'hidden' }}>
      {assigning ? (
        <>
          <span style={{ display: 'flex', alignItems: 'center', gap: 6, color: 'var(--text-primary)', fontWeight: 600 }}><ModGlyph slot={assigning.slot} size={9} />Assigning {assigning.name}</span>
          <span>Click highlighted controls to add · Esc to finish</span>
          <Button size="sm" variant="ghost" onClick={onEndAssign} style={{ height: 20 }}>Done</Button>
        </>
      ) : (
        <span>{focusHint || 'Right-click a live control, or focus it and press M, to assign modulation'}</span>
      )}
      <div style={{ flex: 1 }} />
      <span style={{ fontFamily: 'var(--font-value)', fontSize: 'var(--type-micro)' }}>Voices {voices}</span>
    </footer>
  );
}

Object.assign(window, { SamplerHeader, SamplerStatusBar });

})();
