const generationId = value => Number.isInteger(value) && value > 0 && value <= 0xffffffff;
const sequenceId = value => typeof value === 'string' && value.length <= 20
  && /^[1-9]\d*$/.test(value) && BigInt(value) <= 0xffffffffffffffffn;

// One controller owns the live stream of one host bridge session. Native
// admission is synchronous; its Promise reply may arrive after close/hide.
export function createLiveAnalysisTransport(invoke, onPacket) {
  let generation = 0, mask = 0, started = false, closed = false;
  let starting = null, changing = null, closing = null;
  let desiredVisible = true, confirmedVisible = true;
  let pendingPacket = false, revision = 0, lastSequence = 0n;

  const reconcileVisibility = () => {
    if (changing) return changing;
    if (!started || closed || desiredVisible === confirmedVisible) return Promise.resolve();
    changing = (async () => {
      while (!closed && desiredVisible !== confirmedVisible) {
        const requested = desiredVisible;
        if (await invoke('setLiveAnalysisVisible', requested, generation) !== true)
          throw new Error('Live analysis visibility was rejected');
        confirmedVisible = requested;
      }
    })().finally(() => { changing = null; });
    return changing;
  };

  return {
    start(nextGeneration, nextMask = 3) {
      if (closed || !generationId(nextGeneration) || !Number.isInteger(nextMask)
          || nextMask < 1 || nextMask > 3) return Promise.resolve(false);
      if (started) return Promise.resolve(generation === nextGeneration && mask === nextMask);
      if (starting) return generation === nextGeneration && mask === nextMask
        ? starting : Promise.resolve(false);
      generation = nextGeneration; mask = nextMask;
      starting = (async () => {
        if (await invoke('subscribeLiveAnalysis', generation, mask) !== true || closed) return false;
        started = true;
        await reconcileVisibility();
        return !closed;
      })().finally(() => { starting = null; });
      return starting;
    },
    setVisible(visible) {
      const next = Boolean(visible);
      if (desiredVisible !== next) { desiredVisible = next; ++revision; }
      return reconcileVisibility();
    },
    async tick() {
      if (closed || !started || !desiredVisible || !confirmedVisible || changing || pendingPacket)
        return false;
      pendingPacket = true;
      const requestedRevision = revision;
      try {
        const packet = await invoke('getLiveAnalysisPacket', generation);
        if (!packet || closed) return false;
        if (!generationId(packet.generation) || !sequenceId(packet.sequence))
          throw new Error('Invalid live analysis packet identity');
        let paintError;
        try {
          const sequence = BigInt(packet.sequence);
          if (desiredVisible && revision === requestedRevision
              && packet.generation === generation && sequence > lastSequence) {
            lastSequence = sequence;
            onPacket(packet);
          }
        } catch (error) { paintError = error; }
        const acknowledged = await invoke('ackLiveAnalysisPacket', packet.sequence, packet.generation) === true;
        if (paintError) throw paintError;
        return !closed && acknowledged;
      } finally { pendingPacket = false; }
    },
    close() {
      if (closing) return closing;
      closed = true; ++revision;
      const hadAdmission = started || starting !== null;
      started = false;
      // Do not wait for an old reply: the native admission already happened.
      // Emit cleanup before a replacement controller can submit its start.
      closing = (async () => hadAdmission
        ? await invoke('unsubscribeLiveAnalysis', generation) === true : false)();
      return closing;
    },
    inFlight() { return pendingPacket; },
  };
}
