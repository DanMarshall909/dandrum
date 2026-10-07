// A snapshot must cover this control's latest accepted value write. Gesture
// boundaries do not establish a parameter value; equal-sequence automation does.
export function acceptedKnobWrite(current, reply, value) {
  if (reply?.status !== 'accepted' || !Number.isInteger(reply.generation)
      || !Number.isInteger(reply.sequence) || !Number.isFinite(value) || value < 0 || value > 1
      || (current && (reply.generation < current.generation
        || (reply.generation === current.generation && reply.sequence < current.sequence)))) return current;
  return { generation: reply.generation, sequence: reply.sequence, value };
}

export function knobSnapshotValue(parameter, write) {
  if (write && parameter?.generation === write.generation
      && (!Number.isInteger(parameter.sequence) || parameter.sequence < write.sequence)) return write.value;
  return parameter?.value ?? 0;
}

// A late rejection belongs to the interaction that queued the command.
// The gesture serializer still reports the error and preserves its ordering.
export function createKnobCommand(invoke, owner, onWrite, restore) {
  return async (name, value, generation, submittedBy = owner()) => {
    try {
      const reply = await invoke(name, value, generation);
      if (name === 'setParameter') onWrite(reply, value);
      return reply;
    } catch (reason) { restore(submittedBy); throw reason; }
  };
}
