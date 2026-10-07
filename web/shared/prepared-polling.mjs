// Prepared transports publish ready only after a complete coherent result.
// Each owner is closed on view retirement; a retry gets a fresh owner.
export function createPreparedPolling(factory, onState, onError, clock = globalThis) {
  let timer = null, closed = false;
  const stop = () => {
    if (timer !== null) { clock.clearInterval(timer); timer = null; }
  };
  const transport = factory(packet => {
    if (closed) return;
    if (['ready', 'failed', 'cancelled', 'stale'].includes(packet.status?.state)) stop();
    onState(packet);
  });
  return {
    async select(selection) {
      if (closed) return false;
      stop();
      timer = clock.setInterval(async () => {
        try { await transport.poll(); }
        catch (reason) { if (!closed) { stop(); onError(reason); } }
      }, 1000 / 30);
      try {
        const accepted = await transport.select(selection);
        if (accepted === false) stop();
        return accepted;
      } catch (reason) { if (!closed) { stop(); onError(reason); } return false; }
    },
    async close() {
      if (closed) return;
      closed = true; stop(); await transport.close();
    },
  };
}
