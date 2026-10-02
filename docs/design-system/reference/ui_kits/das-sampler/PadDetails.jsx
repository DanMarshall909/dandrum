// Das Sampler — selected pad details (prepared, read-only).
(() => {
const { Panel, Rollout, SectionHeading, PropertyRow, ListRow, StatusMessage, Button, EmptyState } = window.DandrumDesignSystem_3c2eab;
// noteName is lowercase, so the compiled bundle doesn't expose it on the namespace — keep a local copy (C4 = 60).
const NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
const noteName = (n) => NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1);

function PadDetails({ note, pad, regionIdx, onRegion, activeRegion, compact, missingRegionId, patch }) {
  if (patch.kind === 'break') {
    const i = note - 36; const ok = i >= 0 && i < patch.slices.length;
    return (
      <Panel title="Pad details" tag="Prepared" prepared compact={compact} style={{ flex: 1 }}>
        {ok ? (<>
          <PropertyRow label="Note" value={`${note} · ${noteName(note)}`} prepared compact={compact} />
          <PropertyRow label="Plays" value={`Slice ${i + 1}${patch.sliceNames ? ' · ' + patch.sliceNames[i] : ''}`} prepared compact={compact} />
          <PropertyRow label="Sample" value={patch.file} prepared compact={compact} />
          <PropertyRow label="Voice limit" value="4" prepared compact={compact} />
          <PropertyRow label="Steal" value="Oldest" prepared compact={compact} />
          <PropertyRow label="Choke" value="Slices cut each other" prepared compact={compact} />
        </>) : <EmptyState compact title="No slice on this note">Notes 36–43 play slices 1–8.</EmptyState>}
      </Panel>
    );
  }
  if (!pad) {
    return (
      <Panel title="Pad details" tag="Prepared" prepared compact={compact} style={{ flex: 1 }}>
        <PropertyRow label="Note" value={`${note} · ${noteName(note)}`} prepared compact={compact} />
        <div style={{ marginTop: 12 }}><EmptyState compact icon="plus" title="Empty pad">Available for a future mapping in the patch.</EmptyState></div>
      </Panel>
    );
  }
  const regions = pad.layers.flatMap((l) => l.regions.map((r) => ({ ...r, layerName: l.name, vel: l.vel, policy: l.policy })));
  const r = regions[regionIdx] || regions[0];
  const altPolicy = pad.layers.some((l) => l.policy) ? pad.layers.find((l) => l.policy).policy : 'None';
  const g2 = { display: 'grid', gridTemplateColumns: compact ? 'minmax(0,1fr)' : 'minmax(0,1fr) minmax(0,1fr)', columnGap: 12 };
  return (
    <Panel title="Pad details" tag="Prepared" prepared compact={compact} style={{ flex: 1 }} summary={`${pad.name} · ${note}`} bodyStyle={{ display: 'flex', flexDirection: 'column', gap: 2, overflowY: 'auto', paddingTop: compact ? 4 : 6 }}>
      <Rollout title="Mapping" prepared compact={compact} summary={`${pad.name} · ${note} ${noteName(note)} · ${pad.mode}`}>
        <div style={g2}>
          <PropertyRow label="Sound" value={pad.name} prepared compact />
          <PropertyRow label="Key" value={`${note} · ${noteName(note)}`} prepared compact />
          <PropertyRow label="Velocity" value={pad.layers.length > 1 ? '2 layers' : '1–127'} prepared compact />
          <PropertyRow label="Play mode" value={pad.mode} prepared compact />
        </div>
      </Rollout>
      <Rollout title="Voices & choke" prepared compact={compact} defaultCollapsed={compact} summary={`${pad.voices} voices · ${pad.steal} · ${pad.choke ? 'choke ' + pad.choke : 'no choke'}`}>
        <div style={g2}>
          <PropertyRow label="Voice limit" value={String(pad.voices)} prepared compact />
          <PropertyRow label="Steal" value={pad.steal} prepared compact />
          <PropertyRow label="Choke" value={pad.choke ? `Group ${pad.choke}` : 'None'} prepared compact />
          <PropertyRow label="Alternates" value={altPolicy} prepared compact />
        </div>
      </Rollout>
      <Rollout title="Layers & alternates" prepared compact={compact} defaultCollapsed={compact} summary={`${regions.length} region${regions.length > 1 ? 's' : ''}${pad.layers.length > 1 ? ' · ' + pad.layers.length + ' layers' : ''} · ${altPolicy}`}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1, marginLeft: -10 }}>
          {regions.map((x, i) => (
            <ListRow key={x.id} compact selected={i === regionIdx} active={x.id === activeRegion} onClick={() => onRegion(i)}
              label={x.layerName + (x.alt ? ` · alt ${x.alt}` : '')}
              detail={pad.layers.length > 1 ? `vel ${x.vel[0]}–${x.vel[1]}` : null}
              value={x.id === missingRegionId ? 'missing' : x.weight ? `w ${x.weight}` : x.alt ? 'RR' : null}
              disabled={x.id === missingRegionId} />
          ))}
        </div>
      </Rollout>
      {r && (
        <Rollout title={'Region · ' + r.file} prepared compact={compact} defaultCollapsed={compact} summary={`${r.gain} dB · ${r.pan} · ${r.pitch}×`}>
          <div style={{ display: 'grid', gridTemplateColumns: compact ? 'minmax(0,1fr)' : 'repeat(3, minmax(0,1fr))', columnGap: 12 }}>
            <PropertyRow label="Gain" value={r.gain} unit="dB" prepared compact />
            <PropertyRow label="Pan" value={r.pan} prepared compact />
            <PropertyRow label="Pitch" value={r.pitch} unit="×" prepared compact />
          </div>
        </Rollout>
      )}
    </Panel>
  );
}

Object.assign(window, { PadDetails });
})();
