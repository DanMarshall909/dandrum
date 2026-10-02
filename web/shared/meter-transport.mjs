// Renderer-neutral browser side of the bounded meter protocol. The caller
// chooses its display rate and calls tick; a stalled native promise never
// creates a second meter request or acknowledgement.
export function createMeterTransport(invoke, onPacket) {
  let generation = null;
  let started = false;
  let starting = null;
  let desiredVisible = true;
  let confirmedVisible = true;
  let visibilityChange = null;
  let pendingPacket = false;

  const reconcileVisibility = () => {
    if (!started || visibilityChange || desiredVisible === confirmedVisible)
      return visibilityChange ?? Promise.resolve();
    visibilityChange = (async () => {
      while (desiredVisible !== confirmedVisible) {
        const requested = desiredVisible;
        if (await invoke('setMeterVisible', requested, generation) !== true)
          throw new Error('Meter visibility change was rejected');
        confirmedVisible = requested;
      }
    })().finally(() => { visibilityChange = null; });
    return visibilityChange;
  };

  return {
    start(nextGeneration) {
      if (!Number.isInteger(nextGeneration) || nextGeneration < 0)
        return Promise.resolve(false);
      if (started) return Promise.resolve(generation === nextGeneration);
      if (starting) return starting;
      starting = (async () => {
        if (await invoke('subscribeMeter', nextGeneration) !== true)
          return false;
        generation = nextGeneration;
        started = true;
        await reconcileVisibility();
        return true;
      })().finally(() => { starting = null; });
      return starting;
    },
    setVisible(visible) {
      desiredVisible = Boolean(visible);
      return reconcileVisibility();
    },
    async tick() {
      if (!started || !desiredVisible || !confirmedVisible || visibilityChange
          || pendingPacket)
        return false;
      pendingPacket = true;
      try {
        const packet = await invoke('getMeterPacket');
        if (!packet) return false;
        if (!Number.isInteger(packet.generation) || typeof packet.sequence !== 'string')
          return false;
        let renderError;
        try {
          if (packet.generation === generation) onPacket(packet);
        } catch (error) {
          renderError = error;
        }
        const acknowledged = await invoke('ackMeterPacket', packet.sequence,
                                          packet.generation) === true;
        if (renderError) throw renderError;
        return acknowledged;
      } finally {
        pendingPacket = false;
      }
    },
    inFlight() { return pendingPacket; },
  };
}
