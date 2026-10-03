import { preparedFrame, preparedRegionView } from './prepared-waveform.mjs';

export function preparedSpectrumView(document, sourceId, regionId, snapshot, width, height) {
  if (snapshot?.state !== 'ready' || snapshot.generation !== document?.generation
      || !Number.isInteger(height) || height < 1) return null;
  const view = preparedRegionView(document, sourceId, regionId, width, height);
  const result = snapshot.result;
  const settings = result?.settings;
  if (!view || !result || result.sourceId !== sourceId || result.regionId !== regionId
      || result.sampleRateHz !== view.sampleRateHz || !Number.isInteger(result.channel)
      || result.channel < 0 || result.channel >= view.source.channelCount
      || result.startFrame !== view.region.startFrame || result.endFrame !== view.region.endFrame
      || settings?.window !== 'periodicHann' || settings.scaling !== 'oneSidedPeakDbFS'
      || settings.channelPolicy !== 'selectedChannel' || settings.fftSize !== 1024
      || !Number.isFinite(settings.floorDbFS) || settings.floorDbFS >= 0
      || !Array.isArray(result.frequencyHz) || result.frequencyHz.length !== settings.fftSize / 2 + 1
      || !Array.isArray(result.columns) || result.columns.length < 1 || result.columns.length > 1024)
    return null;
  const hop = preparedFrame(settings.hopFrames);
  if (hop === null || hop === 0n) return null;
  const frequencies = result.frequencyHz;
  if (frequencies.some((hz, i) => !Number.isFinite(hz)
      || hz !== i * view.sampleRateHz / settings.fftSize)) return null;
  const minimumHz = frequencies[1], maximumHz = frequencies.at(-1);
  const logRange = Math.log(maximumHz / minimumHz);
  const rowBins = Array.from({ length: height }, (_, row) => {
    const low = minimumHz * Math.exp(logRange * (1 - (row + 1) / height));
    const high = minimumHz * Math.exp(logRange * (1 - row / height));
    const first = Math.max(1, Math.ceil(low / minimumHz));
    const last = Math.min(frequencies.length - 1, Math.floor(high / minimumHz));
    const nearest = Math.max(1, Math.min(frequencies.length - 1, Math.round(Math.sqrt(low * high) / minimumHz)));
    return first <= last ? [first, last] : [nearest, nearest];
  });
  const columns = [];
  for (let i = 0; i < result.columns.length; ++i) {
    const column = result.columns[i];
    const start = preparedFrame(column.startFrame), end = preparedFrame(column.endFrame);
    if (start === null || end === null || start !== view.start + BigInt(i) * hop
        || start >= end || end > view.end || end - start > BigInt(settings.fftSize)
        || !Array.isArray(column.magnitudeDbFS) || column.magnitudeDbFS.length !== frequencies.length
        || column.magnitudeDbFS.some(db => !Number.isFinite(db))) return null;
    columns.push({ left: view.x(start), right: view.x(start + hop > view.end ? view.end : start + hop),
      rows: rowBins.map(([first, last]) => {
        let level = settings.floorDbFS;
        for (let bin = first; bin <= last; ++bin) level = Math.max(level, column.magnitudeDbFS[bin]);
        return level;
      }) });
  }
  if (BigInt(columns.length) * hop < view.span) return null;
  return { ...view, minimumHz, maximumHz, floorDbFS: settings.floorDbFS, settings, columns,
    startSeconds: Number(view.start) / view.sampleRateHz, endSeconds: Number(view.end) / view.sampleRateHz };
}

export function spectralColour(db, floor, ramp) {
  const level = Math.max(0, Math.min(1, (db - floor) / -floor)) * (ramp.length - 1);
  const index = Math.floor(level), fraction = level - index;
  const rgb = hex => [1, 3, 5].map(offset => parseInt(hex.slice(offset, offset + 2), 16));
  const low = rgb(ramp[index]), high = rgb(ramp[Math.min(index + 1, ramp.length - 1)]);
  return `rgb(${low.map((c, i) => Math.round(c + (high[i] - c) * fraction)).join(',')})`;
}

export function paintPreparedSpectrum(context, view, ramp) {
  if (!context || !view) return;
  for (const column of view.columns)
    for (let row = 0; row < view.height; ++row) {
      context.fillStyle = spectralColour(column.rows[row], view.floorDbFS, ramp);
      const left = Math.round(column.left), right = Math.round(column.right);
      context.fillRect(left, row, Math.max(1, right - left), 1);
    }
  for (const marker of view.markers) {
    context.fillStyle = marker.kind.startsWith('loop') ? '#e2bf72'
      : marker.kind.startsWith('slice') ? '#ab9ee9' : '#8da79a';
    context.fillRect(Math.max(0, Math.min(view.width - 1, Math.round(marker.x))), 0, 1, view.height);
  }
}
