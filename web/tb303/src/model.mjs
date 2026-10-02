// These IDs are the public surface declared by the prepared TB-303 patch.
// Values remain normalised; the host owns conversion to DSP units.
export const controls = Object.freeze([
  { id: 'amp.release_ms', label: 'RELEASE', position: 'tuning' },
  { id: 'filter.cutoff', label: 'CUT OFF FREQ', position: 'bank' },
  { id: 'filter.resonance', label: 'RESONANCE', position: 'bank' },
  { id: 'filter.envelope_modulation', label: 'ENV MOD', position: 'bank' },
  { id: 'filter.decay_ms', label: 'DECAY', position: 'bank' },
  { id: 'accent.brightness', label: 'ACCENT', position: 'bank' },
  { id: 'slide.time_ms', label: 'SLIDE TIME', position: 'program' },
]);

export const preparedCapabilities = Object.freeze({
  waveformEditing: false,
  patternEditing: false,
  transport: false,
  noteAudition: true,
});

export function displayValue(state, id) {
  const entry = state?.parameters?.find(parameter => parameter.id === id);
  return typeof entry?.value === 'number' && Number.isFinite(entry.value)
    ? entry.value : null;
}

export function acceptState(current, incoming, lastAdmittedSequence = 0) {
  if (!incoming || !Number.isInteger(incoming.generation)
      || !Number.isInteger(incoming.sequence) || !Array.isArray(incoming.parameters))
    return current;
  if (current && (incoming.generation < current.generation
      || (incoming.generation === current.generation
          && incoming.sequence < Math.max(current.sequence, lastAdmittedSequence))))
    return current;
  return incoming;
}
