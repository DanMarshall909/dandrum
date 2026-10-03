// Renderer coordinates derived from copied prepared metadata and numeric jobs.
// Frame strings stay exact until their offset from the selected region is known.
const maxFrame = 18446744073709551615n;
export const preparedFrame = value => {
  if (typeof value !== 'string' || !/^\d+$/.test(value)) return null;
  const parsed = BigInt(value);
  return parsed <= maxFrame ? parsed : null;
};

const markerColour = kind => kind === 'loopStart' || kind === 'loopEnd'
  ? '#e2bf72' : kind === 'sliceStart' || kind === 'sliceEnd'
    ? '#ab9ee9' : '#8da79a';

export function preparedRegionView(document, sourceId, regionId, width, height) {
  if (!Number.isFinite(width) || width <= 0 || !Number.isFinite(height) || height <= 0)
    return null;
  const source = document?.sources?.find(item => item.id === sourceId);
  const region = source?.regions?.find(item => item.id === regionId);
  if (!region
      || !Number.isInteger(source.sampleRateHz) || source.sampleRateHz <= 0
      || !Number.isFinite(region.fadeInMs) || region.fadeInMs < 0
      || !Number.isFinite(region.fadeOutMs) || region.fadeOutMs < 0)
    return null;
  const start = preparedFrame(region.startFrame);
  const end = preparedFrame(region.endFrame);
  const count = preparedFrame(source.frameCount);
  if (start === null || end === null || count === null || start >= end || end > count)
    return null;
  const span = end - start;
  const x = value => Number(value - start) / Number(span) * width;
  const y = value => (1 - Math.max(-1, Math.min(1, value))) * height / 2;
  const markers = [{ kind: 'regionStart', x: 0 }, { kind: 'regionEnd', x: width }];
  if (region.fadeInMs > 0)
    markers.push({ kind: 'fadeInEnd', x: Math.min(Number(span),
      region.fadeInMs * source.sampleRateHz / 1000) / Number(span) * width });
  if (region.fadeOutMs > 0)
    markers.push({ kind: 'fadeOutStart', x: (Number(span) - Math.min(Number(span),
      region.fadeOutMs * source.sampleRateHz / 1000)) / Number(span) * width });
  if (region.loop) {
    const loopStart = preparedFrame(region.loop.startFrame);
    const loopEnd = preparedFrame(region.loop.endFrame);
    if (loopStart === null || loopEnd === null || loopStart < start || loopEnd > end
        || loopStart >= loopEnd) return null;
    markers.push({ kind: 'loopStart', x: x(loopStart) },
      { kind: 'loopEnd', x: x(loopEnd) });
  }
  for (const slice of source.slices ?? []) {
    const sliceStart = preparedFrame(slice.startFrame);
    const sliceEnd = preparedFrame(slice.endFrame);
    if (sliceStart === null || sliceEnd === null) return null;
    if (sliceStart > start && sliceStart < end)
      markers.push({ kind: 'sliceStart', id: slice.id, x: x(sliceStart) });
    if (sliceEnd > start && sliceEnd < end)
      markers.push({ kind: 'sliceEnd', id: slice.id, x: x(sliceEnd) });
  }
  return { source, region, start, end, span, x, y, markers, width, height,
    sampleRateHz: source.sampleRateHz, durationSeconds: Number(span) / source.sampleRateHz };
}

export function preparedWaveformView(document, sourceId, regionId, snapshot, width, height) {
  if (snapshot?.state !== 'ready' || snapshot.generation !== document?.generation) return null;
  const view = preparedRegionView(document, sourceId, regionId, width, height);
  const result = snapshot.result;
  if (!view || !result || result.sourceId !== sourceId || result.regionId !== regionId
      || result.sampleRateHz !== view.sampleRateHz
      || !Number.isInteger(result.channel) || result.channel < 0
      || result.channel >= view.source.channelCount
      || result.startFrame !== view.region.startFrame || result.endFrame !== view.region.endFrame
      || !Array.isArray(result.buckets)) return null;
  const { start, end, span, y } = view;
  const buckets = [];
  for (const bucket of result.buckets) {
    const bucketStart = preparedFrame(bucket.startFrame);
    const bucketEnd = preparedFrame(bucket.endFrame);
    if (bucketStart === null || bucketEnd === null || bucketStart < start
        || bucketStart >= bucketEnd || bucketEnd > end
        || !Number.isFinite(bucket.minimum) || !Number.isFinite(bucket.maximum)
        || bucket.minimum > bucket.maximum) return null;
    buckets.push({ x: (Number(bucketStart - start)
      + Number(bucketEnd - bucketStart) / 2) / Number(span) * width,
    top: y(bucket.maximum), bottom: y(bucket.minimum) });
  }
  return { width, height, sampleRateHz: view.sampleRateHz,
    durationSeconds: view.durationSeconds, buckets, markers: view.markers };
}

export function paintPreparedWaveform(context, view) {
  if (!context || !view) return;
  context.fillStyle = '#111916';
  context.fillRect(0, 0, view.width, view.height);
  context.fillStyle = '#414d45';
  context.fillRect(0, Math.round(view.height / 2), view.width, 1);
  context.fillStyle = '#7ce0aa';
  for (const bucket of view.buckets) {
    const top = Math.round(bucket.top);
    const bottom = Math.round(bucket.bottom);
    context.fillRect(Math.round(bucket.x), top, 1, Math.max(2, bottom - top + 1));
  }
  for (const marker of view.markers) {
    context.fillStyle = markerColour(marker.kind);
    context.fillRect(Math.max(0, Math.min(view.width - 1, Math.round(marker.x))),
      0, 1, view.height);
  }
}
