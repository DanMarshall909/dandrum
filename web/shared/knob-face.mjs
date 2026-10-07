export function createKnobFace(React) {
  const h = React.createElement;
  const point = (c, r, degrees) => {
    const angle = (degrees - 90) * Math.PI / 180;
    return [c + r * Math.cos(angle), c + r * Math.sin(angle)];
  };
  const arc = (c, r, from, to) => {
    if (Math.abs(to - from) < 0.01) return '';
    const a = Math.min(from, to), b = Math.max(from, to);
    const [x0, y0] = point(c, r, a), [x1, y1] = point(c, r, b);
    return `M${x0.toFixed(2)} ${y0.toFixed(2)}A${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  };

  return function KnobFace({ size, value, disabled, active, dragging, hovered }) {
    const c = size / 2;
    const trackMax = size >= 60 ? 4 : size >= 44 ? 3 : 2.5;
    const trackMin = size >= 44 ? 1.5 : 1.25;
    const rTrack = c - (size >= 44 ? 6.5 : 5) - trackMax / 2;
    const rCap = rTrack - trackMax / 2 - (size >= 44 ? 2.5 : 2);
    const [x0, y0] = point(c, rTrack + trackMin / 2 + 1, -135);
    const [x1, y1] = point(c, rTrack + trackMin / 2 + (size >= 44 ? 4 : 3), -135);
    return h('svg', { width: size, height: size, viewBox: `0 0 ${size} ${size}`, 'aria-hidden': true },
      h('path', { d: arc(c, rTrack, -135, 135), fill: 'none',
        stroke: disabled ? 'var(--dd-ink-4)' : 'var(--color-track)', strokeWidth: trackMin, strokeLinecap: 'round' }),
      !disabled && h('path', { 'data-dd-knob-part': 'value-arc', d: arc(c, rTrack, -135, -135 + value * 270),
        fill: 'none', stroke: 'var(--color-value)', strokeWidth: active ? trackMax : trackMin, strokeLinecap: 'round' }),
      h('line', { x1: x0, y1: y0, x2: x1, y2: y1, stroke: 'var(--dd-paper-3)',
        strokeWidth: 1.5, strokeLinecap: 'round', opacity: disabled ? 0.4 : 1 }),
      h('circle', { cx: c, cy: c + 1.5, r: rCap, fill: 'rgba(0,0,0,0.45)' }),
      h('circle', { 'data-dd-knob-part': 'cap', cx: c, cy: c, r: rCap,
        fill: disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-ink-6)' : hovered ? 'var(--dd-cap-hover)' : 'var(--dd-ink-5)',
        stroke: 'var(--dd-line-3)', strokeWidth: 1, strokeOpacity: disabled ? 0.3 : 0.6 }),
      h('path', { d: arc(c, rCap - 1, -60, 60), fill: 'none', stroke: 'rgba(255,255,255,0.09)', strokeWidth: 1 }));
  };
}
