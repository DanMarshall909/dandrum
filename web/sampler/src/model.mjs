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
        mapId: map.id,
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

function selectedTarget(document, pad, zoneId) {
  if (zoneId === undefined) return pad;
  if (!pad?.zoneIds.includes(zoneId)) return null;
  return document?.maps?.find(map => map.id === pad.mapId)
    ?.zones.find(zone => zone.id === zoneId) ?? null;
}

export function preparedSource(document, pad, zoneId) {
  const zone = selectedTarget(document, pad, zoneId);
  const source = document?.sources?.[zone?.sourceIndex];
  const region = source?.regions?.[zone?.regionIndex];
  return zone && source && region ? { zone, source, region } : null;
}

export function selectedRegion(document, pad, zoneId) {
  const assignment = preparedSource(document, pad, zoneId);
  return assignment ? { sourceId: assignment.source.id, regionId: assignment.region.id } : null;
}

export function visibleParameters(document, pad, zoneId) {
  const target = selectedTarget(document, pad, zoneId);
  return (document?.parameters ?? []).filter(parameter =>
    parameter.scope === 'instrument'
    || (target?.controlGroup != null && parameter.scope === 'sampleGroup'
        && parameter.controlGroup === target.controlGroup));
}

const noteNames = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
const blackNotes = [1, 3, 6, 8, 10];
const whiteIndex = { 0: 0, 2: 1, 4: 2, 5: 3, 7: 4, 9: 5, 11: 6 };

// Adapt the supplied KeyMap's semitone grid and aligned piano to prepared
// ranges. Alternatives share a rectangle; overlap does not establish layering.
export function keyMapLayout(pads, availableWidth, zoom = 1, compact = false) {
  if (pads.length === 0) return null;
  const lowNote = Math.min(...pads.map(pad => pad.keyLow));
  const highNote = Math.max(...pads.map(pad => pad.keyHigh));
  const count = highNote - lowNote + 1;
  const keyWidth = (Math.max(31, availableWidth) - 30) / count * Math.max(1, Math.min(8, zoom));
  const gridHeight = compact ? 100 : 140;
  const keyboardHeight = compact ? 40 : 56;
  const name = note => noteNames[note % 12] + (Math.floor(note / 12) - 1);
  const notes = Array.from({ length: count }, (_, index) => {
    const note = lowNote + index;
    return { note, name: name(note), black: blackNotes.includes(note % 12),
      left: index * keyWidth, width: keyWidth };
  });
  const whiteKeys = [];
  for (let note = Math.max(0, lowNote - 1); note <= Math.min(127, highNote + 1); note++) {
    if (blackNotes.includes(note % 12)) continue;
    whiteKeys.push({ note, name: name(note),
      left: (Math.floor(note / 12) * 12 + whiteIndex[note % 12] * 12 / 7 - lowNote) * keyWidth,
      width: 12 / 7 * keyWidth });
  }
  return { lowNote, highNote, keyWidth, gridHeight, keyboardHeight,
    width: count * keyWidth, notes, whiteKeys,
    zones: pads.map(pad => ({ id: pad.id,
      left: (pad.keyLow - lowNote) * keyWidth,
      top: (127 - pad.velocityHigh) / 127 * gridHeight,
      width: (pad.keyHigh - pad.keyLow + 1) * keyWidth,
      height: (pad.velocityHigh - pad.velocityLow + 1) / 127 * gridHeight })) };
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
