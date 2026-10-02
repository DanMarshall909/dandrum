// The sampler displays prepared selection facts. Round-robin alternatives
// share one audition range; overlapping zones are never assumed to layer.
export function preparedPads(document) {
  const pads = [];
  const byAlternative = new Map();
  for (const map of document?.maps ?? [])
    for (const zone of map.zones ?? []) {
      const alternative = zone.roundRobinGroup
        ? `${map.id}:${zone.roundRobinGroup}:${zone.keyLow}:${zone.keyHigh}`
          + `:${zone.velocityLow}:${zone.velocityHigh}`
        : null;
      const existing = alternative ? byAlternative.get(alternative) : null;
      if (existing) {
        existing.zoneIds.push(zone.id);
        continue;
      }
      const pad = {
        id: `${map.id}:${zone.id}`,
        label: (zone.roundRobinGroup || zone.id).replaceAll('_', ' '),
        zoneIds: [zone.id],
        sourceIndex: zone.sourceIndex,
        regionIndex: zone.regionIndex,
        keyLow: zone.keyLow,
        keyHigh: zone.keyHigh,
        velocityLow: zone.velocityLow,
        velocityHigh: zone.velocityHigh,
        velocity: Math.round((zone.velocityLow + zone.velocityHigh) / 2) / 127,
        controlGroup: zone.controlGroup ?? null,
        chokeGroup: zone.chokeGroup ?? '',
        roundRobinGroup: zone.roundRobinGroup ?? '',
        selectionMode: map.selectionMode,
        simultaneousLayers: map.selectionMode === 'layered',
        editable: false,
      };
      pads.push(pad);
      if (alternative) byAlternative.set(alternative, pad);
    }
  return pads;
}

export function selectedRegion(document, pad) {
  if (!pad) return null;
  const source = document?.sources?.[pad.sourceIndex];
  const region = source?.regions?.[pad.regionIndex];
  return source && region ? { sourceId: source.id, regionId: region.id } : null;
}

export function visibleParameters(document, pad) {
  return (document?.parameters ?? []).filter(parameter =>
    parameter.scope === 'instrument'
    || (pad?.controlGroup != null && parameter.scope === 'sampleGroup'
        && parameter.controlGroup === pad.controlGroup));
}

export function normalizedDraft(draft, cancelled) {
  if (cancelled || typeof draft !== 'string' || !draft.trim()) return null;
  const value = Number(draft);
  return Number.isFinite(value) && value >= 0 && value <= 1 ? value : null;
}

export function needsDocumentRefresh(stateGeneration, documentGeneration) {
  return Number.isInteger(stateGeneration) && stateGeneration > documentGeneration;
}

export function padReleaseHandlers(release) {
  return {
    onPointerUp: release,
    onPointerCancel: release,
    onPointerLeave: release,
    onBlur: release,
    onKeyUp(event) {
      if (event.key === ' ' || event.key === 'Enter') {
        event.preventDefault();
        return release();
      }
    },
  };
}

export function auditionFocusRelease(audition, clearPressed) {
  return () => {
    clearPressed();
    return audition.releaseAll();
  };
}
