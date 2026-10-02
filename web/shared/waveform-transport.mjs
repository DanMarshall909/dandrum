// One prepared waveform request and one status poll may be in flight. New
// selections coalesce while native admission is pending; late replies are
// cancelled before a superseded result can reach the renderer.
export function createWaveformTransport(invoke, onState) {
  let desired = null;
  let active = null;
  let pumping = null;
  let polling = false;
  let closed = false;

  const acceptedJob = reply => reply?.status === 'accepted'
    && typeof reply.job_id === 'string' && /^\d+$/.test(reply.job_id)
    && reply.job_id !== '0';

  const drain = async () => {
    while (desired && !closed) {
      const selection = desired;
      desired = null;
      if (active) {
        const previous = active;
        active = null;
        await invoke('cancelWaveform', previous.jobId);
      }
      if (closed) break;
      let reply;
      try {
        reply = await invoke('requestWaveform', selection.sourceId, selection.regionId,
          selection.channel, selection.buckets, selection.generation);
      } catch (error) {
        if (!closed && !desired)
          onState({ selection, status: { state: 'failed', error: String(error) } });
        continue;
      }
      if (acceptedJob(reply) && reply.generation !== selection.generation)
        await invoke('cancelWaveform', reply.job_id);
      if (!acceptedJob(reply) || reply.generation !== selection.generation) {
        if (!closed && !desired)
          onState({ selection, status: { state: 'failed', error: String(reply) } });
        continue;
      }
      if (closed || desired) {
        await invoke('cancelWaveform', reply.job_id);
        continue;
      }
      active = { jobId: reply.job_id, selection };
      onState({ selection, status: { state: 'running', generation: selection.generation } });
    }
  };

  const startDrain = () => {
    if (!pumping)
      pumping = drain().finally(() => { pumping = null; });
    return pumping;
  };

  return {
    select(selection) {
      if (closed || typeof selection?.sourceId !== 'string' || !selection.sourceId
          || typeof selection.regionId !== 'string' || !selection.regionId
          || !Number.isInteger(selection.channel) || selection.channel < 0
          || selection.channel > 65535 || !Number.isInteger(selection.buckets)
          || selection.buckets < 1 || selection.buckets > 4096
          || !Number.isInteger(selection.generation) || selection.generation < 0)
        return Promise.resolve(false);
      desired = { ...selection };
      return startDrain();
    },
    async poll() {
      if (closed || polling || !active) return false;
      polling = true;
      const requested = active;
      try {
        const status = await invoke('getWaveformJobStatus', requested.jobId);
        if (closed || active !== requested)
          return false;
        if (status?.state === 'stale' || status?.state === 'cancelled'
            || status?.state === 'failed') {
          active = null;
          onState({ selection: requested.selection, status });
          return true;
        }
        if (status?.generation !== requested.selection.generation) return false;
        if (status.state === 'running') return false;
        active = null;
        onState({ selection: requested.selection, status });
        return true;
      } finally {
        polling = false;
      }
    },
    async close() {
      closed = true;
      desired = null;
      const previous = active;
      active = null;
      if (previous) await invoke('cancelWaveform', previous.jobId);
    },
    activeJobId() { return active?.jobId ?? null; },
  };
}
