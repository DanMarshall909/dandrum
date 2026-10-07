/** @typedef {{generation: number, complete: boolean, valid: boolean,
 * channels: {name: string, peak: number, rms: number, clipped: boolean, ticket: string}[]}} MeterDisplay */
const level = value => Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
const ticket = value => typeof value === 'string' && /^\d+$/.test(value) ? value : '0';

/** @param {any} packet @param {MeterDisplay=} current @returns {MeterDisplay} */
export function meterView(packet, current) {
  const valid = packet?.display_valid === true;
  const channels = ['L', 'R'].map((name, channel) => ({
    name,
    peak: valid ? level(packet.display_peak?.[channel]) : 0,
    rms: valid ? level(packet.display_rms?.[channel]) : 0,
    clipped: valid && packet.display_clipped?.[channel] === true,
    ticket: valid ? ticket(packet.clip_ticket?.[channel]) : '0',
  }));
  const next = {
    generation: Number.isInteger(packet?.generation) ? packet.generation : 0,
    complete: valid && packet.display_complete === true,
    valid,
    channels,
  };
  if (current && current.generation === next.generation && current.valid === next.valid
      && current.complete === next.complete && current.channels.every((previous, index) => {
        const channel = channels[index];
        return previous.peak === channel.peak && previous.rms === channel.rms
          && previous.clipped === channel.clipped && previous.ticket === channel.ticket;
      })) return current;
  return next;
}
