// Browser commands are coalesced by pitch. A release requested while note-on
// is in flight is sent after admission, even if the page receives rapid input.
/**
 * @param {(name: string, ...args: unknown[]) => Promise<unknown>} invoke
 * @param {(error: unknown) => void} [onError]
 */
export function createNoteAudition(invoke, onError = () => {}) {
  const notes = new Map();
  let heartbeatPending = null;

  const slotFor = note => {
    if (!notes.has(note))
      notes.set(note, { desired: false, sent: false, velocity: 0.9,
        generation: undefined, running: null });
    return notes.get(note);
  };

  const pump = note => {
    const slot = slotFor(note);
    if (slot.running) return slot.running;
    slot.running = (async () => {
      while (slot.sent !== slot.desired) {
        const next = slot.desired;
        try {
          const args = next ? [note, slot.velocity] : [note];
          if (slot.generation !== undefined) args.push(slot.generation);
          await invoke(next ? 'noteOn' : 'noteOff', ...args);
        } catch (error) {
          onError(error);
        }
        slot.sent = next;
      }
    })().finally(() => {
      slot.running = null;
      if (slot.sent !== slot.desired) void pump(note);
    });
    return slot.running;
  };

  return {
    /** @param {number} note @param {number} velocity @param {number | undefined} generation */
    press(note, velocity = 0.9, generation = undefined) {
      const slot = slotFor(note);
      slot.velocity = velocity;
      slot.generation = generation;
      slot.desired = true;
      return pump(note);
    },
    release(note) {
      const slot = slotFor(note);
      slot.desired = false;
      return pump(note);
    },
    releaseAll() {
      return Promise.all([...notes.keys()].map(note => {
        notes.get(note).desired = false;
        return pump(note);
      }));
    },
    isPressed(note) { return slotFor(note).desired; },
    keepAlive() {
      if (![...notes.values()].some(slot => slot.desired)) return Promise.resolve();
      if (heartbeatPending) return heartbeatPending;
      heartbeatPending = (async () => {
        try { await invoke('noteHeartbeat'); }
        catch (error) { onError(error); }
      })().finally(() => { heartbeatPending = null; });
      return heartbeatPending;
    },
  };
}
