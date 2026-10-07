// One in-flight host write and one replacement value per control. The host
// receives the last value before each gesture ends, even if it stalls mid-drag.
export function createParameterGesture(command, onRejected, captureOwner = () => undefined) {
  let drag = null;
  let discretePending = null;
  let discreteWorker = null;

  const report = reason => onRejected(reason);
  // Capture ownership when work is queued, before any preceding gesture waits.
  // Unowned callers retain their three-argument command contract.
  const invoke = (name, value, generation, owner) => command(name, value, generation,
    ...(owner === undefined ? [] : [owner]));
  const runDiscrete = () => {
    if (discreteWorker) return discreteWorker;
    const previousDrag = drag?.closed;
    discreteWorker = (async () => {
      if (previousDrag) await previousDrag;
      while (discretePending) {
        const { value, generation, owner } = discretePending;
        discretePending = null;
        let begun = false;
        try {
          await invoke('beginGesture', undefined, generation, owner);
          begun = true;
          await invoke('setParameter', value, generation, owner);
        } catch (reason) { report(reason); }
        finally {
          if (begun) {
            try { await invoke('endGesture', undefined, generation, owner); }
            catch (reason) { report(reason); }
          }
        }
      }
    })().finally(() => {
      discreteWorker = null;
      if (discretePending) void runDiscrete();
    });
    return discreteWorker;
  };

  const pumpDrag = current => {
    if (current.pump) return current.pump;
    current.pump = (async () => {
      if (!await current.begun) { current.pending = null; return; }
      while (current.pending !== null) {
        const { value, owner } = current.pending;
        current.pending = null;
        try { await invoke('setParameter', value, current.generation, owner); }
        catch (reason) { report(reason); current.pending = null; return; }
      }
    })().finally(() => { current.pump = null; });
    return current.pump;
  };

  return {
    begin(generation) {
      if (drag && !drag.ending) return;
      const owner = captureOwner();
      const previousDrag = drag?.closed;
      const previous = discreteWorker;
      let close;
      const current = {
        generation, pending: null, pump: null, ending: null,
        closed: new Promise(resolve => { close = resolve; }), close: () => close(),
        begun: (async () => {
          if (previousDrag) await previousDrag;
          if (previous) await previous;
          try { await invoke('beginGesture', undefined, generation, owner); return true; }
          catch (reason) { report(reason); return false; }
        })(),
      };
      drag = current;
    },
    change(value, generation) {
      if (drag && !drag.ending) {
        drag.pending = { value, owner: captureOwner() };
        void pumpDrag(drag);
      } else {
        discretePending = { value, generation, owner: captureOwner() };
        void runDiscrete();
      }
    },
    commit(value, generation) {
      discretePending = { value, generation, owner: captureOwner() };
      void runDiscrete();
    },
    async end() {
      if (!drag) return;
      const current = drag;
      if (current.ending) return current.ending;
      const owner = captureOwner();
      current.ending = (async () => {
        try {
          if (current.pump) await current.pump;
          else if (current.pending !== null) await pumpDrag(current);
          if (await current.begun) {
            try { await invoke('endGesture', undefined, current.generation, owner); }
            catch (reason) { report(reason); }
          }
        } finally {
          if (drag === current) drag = null;
          current.close();
        }
      })();
      return current.ending;
    },
    async flush() {
      if (discreteWorker) await discreteWorker;
      if (drag?.pump) await drag.pump;
    },
  };
}
