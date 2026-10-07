// Host automation can change values without advancing the UI command sequence.
export function acceptParameterState(current, incoming, admittedSequence = 0) {
  if (!incoming || !Number.isInteger(incoming.generation)
      || !Number.isInteger(incoming.sequence) || !Array.isArray(incoming.parameters)) return current;
  if (current && (incoming.generation < current.generation
      || (incoming.generation === current.generation
          && incoming.sequence < Math.max(current.sequence, admittedSequence)))) return current;
  if (current && incoming.generation === current.generation && incoming.sequence === current.sequence
      && incoming.parameters.length === current.parameters.length
      && incoming.parameters.every((parameter, index) => {
        const previous = current.parameters[index];
        return parameter.id === previous.id && parameter.name === previous.name
          && parameter.value === previous.value;
      })) return current;
  return incoming;
}

// One read in flight and one replacement refresh. Reads never hold up the
// gesture serializer; native admission remains the write completion boundary.
export function createParameterController(invoke, onState, onError) {
  let current = null, admitted = 0, writes = 0, epoch = 0;
  let pending = null, dirty = false, closed = false;
  const accept = incoming => {
    if (closed) return;
    if (current && incoming?.generation > current.generation) {
      admitted = 0; writes = 0; epoch++;
    } else if (writes > 0) return;
    const next = acceptParameterState(current, incoming, admitted);
    if (next !== current) { current = next; onState(next); }
  };
  const refresh = () => {
    if (closed) return Promise.resolve();
    if (pending) { dirty = true; return pending; }
    pending = Promise.resolve().then(() => invoke('getParameterState')).then(accept)
      .catch(reason => { if (!closed) onError(reason); })
      .finally(() => {
        pending = null;
        if (dirty && !closed) { dirty = false; void refresh(); }
      });
    return pending;
  };
  return {
    accept, refresh,
    async command(name, ...args) {
      const write = name === 'setParameter', commandEpoch = epoch;
      if (write) writes++;
      try {
        const reply = await invoke(name, ...args);
        if (typeof reply === 'string') throw new Error(reply);
        if (reply?.status && reply.status !== 'accepted')
          throw new Error(`Host rejected ${name}: ${reply.status}`);
        if (reply?.status === 'accepted' && reply.generation === current?.generation)
          admitted = Math.max(admitted, reply.sequence ?? 0);
        if (!closed && name !== 'endGesture') onError('');
        return reply;
      } catch (reason) {
        if (!closed) onError(reason);
        throw reason;
      } finally {
        if (write) {
          if (commandEpoch === epoch) writes = Math.max(0, writes - 1);
          void refresh();
        }
      }
    },
    close() { closed = true; dirty = false; },
  };
}
