import { preparedSource, visibleParameters } from './model.mjs';

// Adapt the supplied LayerStack to copied sample assignments. Source rows are
// inspection controls; declared parameters retain the existing live bindings.
export function createLayerStack(React) {
  const { useLayoutEffect, useRef, useState } = React;
  const h = React.createElement;
  const declared = (value, unit = '') => value == null ? 'Not declared' : `${value}${unit}`;
  const frames = value => `${value.startFrame}..${value.endFrame}`;
  return function PreparedLayerStack({ document, pad, selectedZoneId, onSelect, readOnlyIcon }) {
    const [open, setOpen] = useState(null);
    const returnFocus = useRef(null);
    const rows = (pad?.zoneIds ?? []).map(zoneId => ({ zoneId,
      assignment: preparedSource(document, pad, zoneId) }));
    const matches = open && open.generation === document.generation
      && open.padId === pad?.id && open.mapId === pad?.mapId && open.zoneId === selectedZoneId;
    const active = matches ? rows.find(row => row.zoneId === open.zoneId)?.assignment : null;
    const validOpen = Boolean(active);
    useLayoutEffect(() => {
      if (open && (!matches || !active)) {
        setOpen(null); returnFocus.current = null;
      }
    }, [open, matches, validOpen]);
    const close = () => { setOpen(null); returnFocus.current?.focus(); };
    const toggle = (row, event) => {
      returnFocus.current = event.currentTarget;
      onSelect(pad, row.zoneId);
      setOpen(matches && open.zoneId === row.zoneId ? null : {
        generation: document.generation, padId: pad.id, mapId: pad.mapId, zoneId: row.zoneId,
      });
    };
    const keyboard = (row, event) => {
      if (event.key === 'Escape') { event.preventDefault(); close(); return; }
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      if (!event.repeat) toggle(row, event);
    };
    const detail = (name, value) => h(React.Fragment, { key: name },
      h('dt', null, name), h('dd', null, value));
    const parameters = active ? visibleParameters(document, pad, open.zoneId) : [];
    const alternatives = rows.length > 1 && !pad.simultaneousLayers;
    return h('section', { className: 'panel dd-layer-stack', 'aria-label': 'Prepared sources' },
      h('div', { className: 'section-heading' },
        h('h2', null, alternatives ? 'Alternatives' : 'Sources'),
        h('span', null, rows.length ? `${rows.length} · ${pad.selectionMode.replaceAll('_', ' ')}` : 'INSPECT ONLY')),
      rows.length === 0 ? h('p', { className: 'details', role: 'status' }, 'No mapped sample selected')
        : rows.map(row => h('div', { key: row.zoneId, className: 'dd-source-row' },
          h('button', { type: 'button', className: 'dd-source-block', 'data-source-zone': row.zoneId,
            disabled: !row.assignment, 'aria-expanded': Boolean(active && open.zoneId === row.zoneId),
            'aria-pressed': selectedZoneId === row.zoneId,
            onClick: event => toggle(row, event), onKeyDown: event => keyboard(row, event) },
            h('span', { className: 'dd-source-title' }, row.assignment ? 'Sample' : 'Unavailable',
              h('strong', null, row.zoneId.replaceAll('_', ' '))),
            h('span', { className: 'dd-source-summary' }, row.assignment
              ? `${row.assignment.source.id}.${row.assignment.region.id}` : 'Source assignment unavailable')),
          active && open.zoneId === row.zoneId ? h('div', { className: 'dd-source-details',
            role: 'region', 'aria-label': 'Prepared source details',
            'data-generation': document.generation, 'data-source-id': active.source.id,
            'data-region-id': active.region.id, 'data-region-start': active.region.startFrame,
            'data-region-end': active.region.endFrame,
            onKeyDown: event => { if (event.key === 'Escape') { event.preventDefault(); close(); } } },
            h('div', { className: 'dd-source-detail-heading' }, h('strong', null, active.region.id),
              h('button', { type: 'button', 'aria-label': 'Close source details', onClick: close }, '×')),
            h('dl', null,
              detail('Source', active.source.id),
              detail('Asset', `${active.source.sampleRateHz} Hz · ${active.source.channelCount} ${active.source.channelCount === 1 ? 'channel' : 'channels'} · ${active.source.frameCount} frames`),
              detail('Region frames', frames(active.region)),
              detail('Mapped frames', frames(active.zone)),
              detail('Root note', declared(active.region.rootNote)),
              detail('Region gain', declared(active.region.gainDb, ' dB')),
              detail('Region pan', declared(active.region.pan)),
              detail('Direction', active.region.reverse ? 'Reverse' : 'Forward'),
              detail('Fades', `${active.region.fadeInMs} ms in · ${active.region.fadeOutMs} ms out`),
              detail('Loop', active.region.loop ? `${active.region.loop.mode.replaceAll('_', ' ')} · ${frames(active.region.loop)} · ${active.region.loop.crossfadeMs} ms crossfade` : 'None'),
              detail('Keys / velocity', `${active.zone.keyLow}..${active.zone.keyHigh} / ${active.zone.velocityLow}..${active.zone.velocityHigh}`),
              detail('Mapping', `weight ${active.zone.weight} · choke ${active.zone.chokeGroup || 'None'} · gain ${declared(active.zone.gainDb, ' dB')} · pan ${declared(active.zone.pan)} · pitch ${declared(active.zone.pitchSemitones, ' st')}`),
              detail('Slices', active.source.slices.length ? active.source.slices.map(slice => `${slice.id}: ${frames(slice)}`).join(' · ') : 'None')),
            h('a', { href: '#sampler-public-controls' }, 'Public controls'),
            h('ul', { className: 'dd-source-bindings', 'aria-label': 'Declared public bindings' },
              parameters.map(parameter => h('li', { key: parameter.id, 'data-public-binding': parameter.id }, parameter.id)))) : null)),
      h('p', { className: 'dd-source-capabilities' },
        'Synth layers unavailable · Nested patches unavailable · Module chain unavailable'),
      h('p', { className: 'dd-source-capabilities' }, readOnlyIcon,
        ' Structure is read only. Public controls remain live.'));
  };
}
