import { preparedFrame } from './prepared-waveform.mjs';

// One admission and one page may be outstanding. Numeric storage is bounded to
// 1024 columns of 513 bins; only a complete coherent result reaches the view.
export function createSpectrumTransport(invoke, onState) {
  let desired = null, active = null, pumping = null, polling = false, closed = false;
  const accepted = reply => reply?.status === 'accepted'
    && typeof reply.job_id === 'string' && /^\d+$/.test(reply.job_id)
    && preparedFrame(reply.job_id) > 0n;
  const cancel = async id => { try { await invoke('cancelSpectrogram', id); } catch { /* session teardown also retires jobs */ } };
  const failed = (selection, error) => onState({ selection,
    status: { state: 'failed', generation: selection.generation, error: String(error) } });
  const metadataKey = page => JSON.stringify([page.sourceId, page.regionId, page.channel,
    page.sampleRateHz, page.startFrame, page.endFrame, page.contentRevision,
    [page.settings.window, page.settings.scaling, page.settings.channelPolicy,
      page.settings.fftSize, page.settings.hopFrames, page.settings.floorDbFS], page.frequencyHz]);

  const drain = async () => {
    while (desired && !closed) {
      const selection = desired; desired = null;
      const previous = active; active = null;
      if (previous) await cancel(previous.jobId);
      if (closed) break;
      let reply;
      try {
        reply = await invoke('requestSpectrogram', selection.sourceId, selection.regionId,
          selection.channel, selection.generation);
      } catch (error) { if (!closed && !desired) failed(selection, error); continue; }
      if (!accepted(reply) || reply.generation !== selection.generation) {
        if (accepted(reply)) await cancel(reply.job_id);
        if (!closed && !desired) failed(selection, 'Spectrogram request rejected');
        continue;
      }
      if (closed || desired) { await cancel(reply.job_id); continue; }
      active = { jobId: reply.job_id, selection, columns: [], metadata: null, key: null };
      onState({ selection, status: { state: 'running', generation: selection.generation } });
    }
    return true;
  };
  const startDrain = () => {
    if (!pumping) pumping = drain().finally(() => {
      pumping = null;
      if (desired && !closed) void startDrain();
    });
    return pumping;
  };

  return {
    select(selection) {
      if (closed || typeof selection?.sourceId !== 'string' || !selection.sourceId
          || typeof selection.regionId !== 'string' || !selection.regionId
          || !Number.isInteger(selection.channel) || selection.channel < 0 || selection.channel > 65535
          || !Number.isInteger(selection.generation) || selection.generation < 0 || selection.generation > 4294967295)
        return Promise.resolve(false);
      desired = { ...selection }; return startDrain();
    },
    async poll() {
      if (closed || polling || !active) return false;
      polling = true;
      const requested = active;
      try {
        const status = await invoke('getSpectrogramJobStatus', requested.jobId, requested.columns.length);
        if (closed || active !== requested) return false;
        if (['failed', 'cancelled', 'stale'].includes(status?.state)) {
          active = null; onState({ selection: requested.selection, status }); return true;
        }
        if (status?.generation !== requested.selection.generation || status?.job_id !== requested.jobId)
          throw new Error('Spectrogram page identity mismatch');
        if (status.state === 'running') return false;
        const page = status.result, offset = requested.columns.length;
        const start = preparedFrame(page?.startFrame), end = preparedFrame(page?.endFrame);
        const hop = preparedFrame(page?.settings?.hopFrames);
        if (status.state !== 'ready' || !page || !Number.isInteger(page.totalColumns)
            || page.totalColumns < 1 || page.totalColumns > 1024 || page.columnOffset !== offset
            || !Array.isArray(page.columns) || page.columns.length !== Math.min(16, page.totalColumns - offset)
            || page.columns.length < 1 || !Array.isArray(page.frequencyHz) || page.frequencyHz.length !== 513
            || !Number.isInteger(page.sampleRateHz) || page.sampleRateHz < 1
            || page.settings?.fftSize !== 1024 || page.settings.window !== 'periodicHann'
            || page.settings.scaling !== 'oneSidedPeakDbFS' || page.settings.channelPolicy !== 'selectedChannel'
            || !Number.isFinite(page.settings.floorDbFS) || page.settings.floorDbFS >= 0
            || start === null || end === null || start >= end || hop === null || hop === 0n
            || BigInt(page.totalColumns) !== 1n + (end - start - 1n) / hop
            || typeof page.contentRevision !== 'string' || !/^[0-9a-f]{64}$/.test(page.contentRevision)
            || page.frequencyHz.some((hz, bin) => !Number.isFinite(hz) || hz !== bin * page.sampleRateHz / 1024)
            || page.sourceId !== requested.selection.sourceId || page.regionId !== requested.selection.regionId
            || page.channel !== requested.selection.channel
            || page.columns.some((c, i) => !Array.isArray(c.magnitudeDbFS) || c.magnitudeDbFS.length !== 513
              || c.magnitudeDbFS.some(db => !Number.isFinite(db))
              || preparedFrame(c.startFrame) !== start + BigInt(offset + i) * hop
              || preparedFrame(c.endFrame) !== (start + BigInt(offset + i) * hop + 1024n > end
                ? end : start + BigInt(offset + i) * hop + 1024n)))
          throw new Error('Invalid spectrogram page');
        const key = metadataKey(page);
        if (requested.metadata && (requested.key !== key || requested.metadata.totalColumns !== page.totalColumns))
          throw new Error('Spectrogram page metadata changed');
        if (!requested.metadata) {
          requested.metadata = { sourceId: page.sourceId, regionId: page.regionId, channel: page.channel,
            sampleRateHz: page.sampleRateHz, startFrame: page.startFrame, endFrame: page.endFrame,
            contentRevision: page.contentRevision, settings: page.settings, frequencyHz: page.frequencyHz,
            totalColumns: page.totalColumns };
          requested.key = key;
        }
        requested.columns.push(...page.columns);
        if (requested.columns.length < page.totalColumns) return false;
        active = null;
        onState({ selection: requested.selection, status: { ...status,
          result: { ...requested.metadata, columns: requested.columns } } });
        return true;
      } catch (error) {
        if (closed || active !== requested) return false;
        active = null; await cancel(requested.jobId);
        if (!closed && !active && !desired) failed(requested.selection, error);
        return true;
      } finally { polling = false; }
    },
    async close() {
      closed = true; desired = null;
      const previous = active; active = null;
      if (previous) await cancel(previous.jobId);
    },
    activeJobId() { return active?.jobId ?? null; },
  };
}
