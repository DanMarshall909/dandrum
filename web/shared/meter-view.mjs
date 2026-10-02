const level = value => Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;
const ticket = value => typeof value === 'string' && /^\d+$/.test(value) ? value : '0';

export function meterView(packet) {
  const valid = packet?.display_valid === true;
  const channels = ['L', 'R'].map((name, channel) => ({
    name,
    peak: valid ? level(packet.display_peak?.[channel]) : 0,
    rms: valid ? level(packet.display_rms?.[channel]) : 0,
    clipped: valid && packet.display_clipped?.[channel] === true,
    ticket: valid ? ticket(packet.clip_ticket?.[channel]) : '0',
  }));
  return {
    generation: Number.isInteger(packet?.generation) ? packet.generation : 0,
    complete: valid && packet.display_complete === true,
    valid,
    channels,
  };
}
