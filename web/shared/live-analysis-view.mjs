const frame = value => typeof value === 'string' && value.length <= 20
  && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= 0xffffffffffffffffn ? BigInt(value) : null;
const positive = value => { const parsed = frame(value); return parsed !== null && parsed > 0n; };
const clamp = (value, low, high) => Math.max(low, Math.min(high, value));

// Live coordinates belong to the captured output stream, never a sample region.
// Subtract exact frame identities before converting the bounded window to Number.
export function liveAnalysisView(packet, channel, width, height, mode) {
  const settings = packet?.settings;
  const start = frame(packet?.startFrame), end = frame(packet?.endFrame);
  if (!Number.isFinite(width) || width <= 0 || !Number.isFinite(height) || height <= 0
      || !Number.isInteger(channel) || channel < 0 || channel > 1
      || (mode !== 'scope' && mode !== 'spectrum')
      || packet?.bus !== 'master' || !Number.isInteger(packet.generation)
      || packet.generation < 1 || packet.generation > 0xffffffff
      || !positive(packet.sequence) || !positive(packet.streamId) || !positive(packet.selectionId)
      || !positive(packet.captureSequence) || start === null || end === null || end - start !== 1024n
      || !Number.isInteger(packet.sampleRateHz) || packet.sampleRateHz <= 0
      || !Number.isInteger(packet.channelMask) || packet.channelMask < 1 || packet.channelMask > 3
      || typeof packet.gap !== 'boolean' || settings?.window !== 'periodicHann'
      || settings.scaling !== 'oneSidedPeakDbFS' || settings.channelPolicy !== 'selectedChannel'
      || settings.fftSize !== 1024 || settings.hopFrames !== '256' || settings.floorDbFS !== -120
      || !Array.isArray(packet.frequencyHz) || packet.frequencyHz.length !== 513
      || packet.frequencyHz.some((hz, i) => hz !== i * packet.sampleRateHz / 1024)
      || !Array.isArray(packet.channels)) return null;
  const selected = [0, 1].filter(index => packet.channelMask & (1 << index));
  if (packet.channels.length !== selected.length || !selected.includes(channel)) return null;
  for (let index = 0; index < selected.length; ++index) {
    const data = packet.channels[index];
    if (data?.channel !== selected[index] || !Array.isArray(data.scope) || data.scope.length !== 128
        || !Array.isArray(data.magnitudeDbFS) || data.magnitudeDbFS.length !== 513
        || data.magnitudeDbFS.some(db => !Number.isFinite(db))) return null;
    for (let i = 0; i < 128; ++i) {
      const bucket = data.scope[i];
      if (frame(bucket?.startFrame) !== start + BigInt(i * 8)
          || frame(bucket.endFrame) !== start + BigInt((i + 1) * 8)
          || !Number.isFinite(bucket.minimum) || !Number.isFinite(bucket.maximum)
          || bucket.minimum > bucket.maximum) return null;
    }
  }
  const data = packet.channels.find(item => item.channel === channel);
  const minimumHz = packet.frequencyHz[1], maximumHz = packet.frequencyHz[512];
  const y = sample => (1 - clamp(sample, -1, 1)) * height / 2;
  return { width, height, mode, channel, generation: packet.generation, sequence: packet.sequence,
    streamId: packet.streamId, selectionId: packet.selectionId, startFrame: packet.startFrame,
    endFrame: packet.endFrame, gap: packet.gap, sampleRateHz: packet.sampleRateHz,
    durationMs: 1024 * 1000 / packet.sampleRateHz, minimumHz, maximumHz,
    buckets: data.scope.map((bucket, i) => ({ x: (i + 0.5) / 128 * width,
      top: y(bucket.maximum), bottom: y(bucket.minimum) })),
    points: data.magnitudeDbFS.slice(1).map((db, i) => ({ frequencyHz: packet.frequencyHz[i + 1],
      x: Math.log(i + 1) / Math.log(512) * width,
      y: Math.max(0, -clamp(db, -120, 0)) / 120 * height })) };
}

export function paintLiveAnalysis(context, view) {
  if (!context || !view) return;
  context.fillStyle = '#161616'; context.fillRect(0, 0, view.width, view.height);
  context.fillStyle = '#444444'; context.fillRect(0, Math.round(view.height / 2), view.width, 1);
  if (view.mode === 'scope') {
    context.fillStyle = '#f6aa69';
    for (const bucket of view.buckets) {
      const top = Math.round(bucket.top), bottom = Math.round(bucket.bottom);
      context.fillRect(Math.round(bucket.x), top, 1, Math.max(2, bottom - top + 1));
    }
  } else {
    context.strokeStyle = '#f6aa69'; context.lineWidth = 1;
    context.beginPath(); context.moveTo(view.points[0].x, view.points[0].y);
    for (const point of view.points.slice(1)) context.lineTo(point.x, point.y);
    context.stroke();
  }
}
