import React from 'react';

// Deterministic placeholder audio so mockups render the same every time. JUCE draws real
// min/max peaks from the prepared AudioThumbnail; nothing here is baked into an asset.
function rng(seed) { let s = seed >>> 0; return () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; }; }
const SHAPES = {
  kick: (t, r) => Math.exp(-t * 5.5) * (0.65 + 0.35 * Math.abs(Math.sin(t * 60 * (1 - t * 0.6)))) + r() * 0.04 * Math.exp(-t * 30),
  snare: (t, r) => Math.exp(-t * 7) * (0.35 + 0.65 * r()) + Math.exp(-t * 25) * 0.4,
  'snare-soft': (t, r) => 0.55 * (Math.exp(-t * 8) * (0.35 + 0.65 * r()) + Math.exp(-t * 25) * 0.3),
  'hat-closed': (t, r) => Math.exp(-t * 18) * (0.3 + 0.7 * r()),
  'hat-open': (t, r) => Math.exp(-t * 3.2) * (0.25 + 0.6 * r()) * (t < 0.01 ? t * 100 : 1),
  break: (t, r) => { const hits = [0, .125, .25, .3125, .5, .625, .75, .875]; let a = 0.06 * r(); for (const h of hits) { if (t >= h) a = Math.max(a, Math.exp(-(t - h) * (h % .25 === 0 ? 22 : 40)) * (h % .5 === 0 ? 0.95 : 0.6) * (0.5 + 0.5 * r())); } return a; },
};
export function makePeaks(kind = 'kick', n = 400, seed = 7) {
  const r = rng(seed + kind.length * 31), f = SHAPES[kind] || SHAPES.kick, out = [];
  for (let i = 0; i < n; i++) out.push(Math.min(1, f(i / n, r)));
  return out;
}

const pct = (v) => `${(v * 100).toFixed(3)}%`;

// Spectral energy per frequency bin (0 = low, 1 = high) for each placeholder sound.
const SPECTRA = {
  kick: (f, t) => Math.exp(-f * 9) * (1 + 0.6 * Math.exp(-t * 40)) + 0.25 * Math.exp(-t * 60) * Math.exp(-f * 2),
  snare: (f, t) => 0.55 * Math.exp(-Math.pow((f - 0.12) * 9, 2)) + 0.5 * Math.exp(-f * 1.4) * Math.exp(-t * 4),
  'snare-soft': (f, t) => 0.5 * Math.exp(-Math.pow((f - 0.12) * 9, 2)) + 0.35 * Math.exp(-f * 2) * Math.exp(-t * 5),
  'hat-closed': (f) => Math.pow(f, 1.4) * 0.9 + 0.05,
  'hat-open': (f) => Math.pow(f, 1.2) * 0.85 + 0.06,
  break: (f, t, r) => { const lo = Math.exp(-f * 8), hi = Math.pow(f, 1.3); const k = r < 0.5 ? lo : 0.6 * lo + 0.6 * hi; return k; },
};
function spectralColor(v, ramp) {
  const x = Math.max(0, Math.min(1, v)) * (ramp.length - 1), i = Math.floor(x), t = x - i;
  const a = ramp[i], b = ramp[Math.min(ramp.length - 1, i + 1)];
  return `rgb(${a.map((c, k) => Math.round(c + (b[k] - c) * t)).join(',')})`;
}
const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));

/** Spectrogram canvas. JUCE: compute once at patch load (FFT 1024, hop 256, log-frequency) into a cached Image. */
function Spectrogram({ data, kind, reversed, regionStart, regionEnd }) {
  const ref = React.useRef(null);
  React.useEffect(() => {
    const cv = ref.current; if (!cv) return;
    const cs = getComputedStyle(cv);
    const v = (n, d) => (cs.getPropertyValue(n).trim() || d);
    const ramp = [v('--dd-ink-0', '#130F0C'), v('--dd-ink-5', '#41362C'), v('--dd-vermilion-lo', '#B0662F'), v('--dd-vermilion', '#E08A4E'), v('--dd-paper-1', '#F2E6D3')].map(hex);
    const cols = data.length, rows = 64;
    cv.width = cols; cv.height = rows;
    const ctx = cv.getContext('2d');
    const prof = SPECTRA[kind] || SPECTRA.kick;
    let s = 99;
    const rnd = () => { s = (s * 1664525 + 1013904223) >>> 0; return s / 4294967296; };
    for (let x = 0; x < cols; x++) {
      const i = reversed ? cols - 1 - x : x;
      const t = i / cols, amp = data[i];
      const hitR = rnd();
      for (let y = 0; y < rows; y++) {
        const f = 1 - y / (rows - 1);
        const e = amp * prof(f, t, hitR) * (0.75 + 0.5 * rnd());
        const db = Math.max(0, 1 + Math.log10(Math.max(1e-4, e)) / 2.2);
        const inRegion = x / cols >= regionStart && x / cols <= regionEnd;
        ctx.fillStyle = spectralColor(inRegion ? db : db * 0.45, ramp);
        ctx.fillRect(x, y, 1, 1);
      }
    }
  }, [data, kind, reversed, regionStart, regionEnd]);
  return <canvas ref={ref} style={{ position: 'absolute', left: 0, top: 18, width: '100%', height: 'calc(100% - 18px)', imageRendering: 'auto' }} aria-hidden="true" />;
}

function ViewToggle({ value, onChange }) {
  const opt = (id, label) => (
    <button type="button" aria-pressed={value === id} onClick={() => onChange(id)}
      style={{ height: 14, padding: '0 5px', border: 0, borderRadius: 2, cursor: 'pointer', fontFamily: 'var(--font-ui)', fontSize: 10, fontWeight: 600, letterSpacing: '0.06em', textTransform: 'uppercase',
        background: value === id ? 'var(--dd-ink-5)' : 'transparent', color: value === id ? 'var(--text-primary)' : 'var(--text-tertiary)' }}>{label}</button>
  );
  return <div role="radiogroup" aria-label="Display" style={{ position: 'absolute', bottom: 6, right: 6, display: 'flex', gap: 1, padding: 1, borderRadius: 3, background: 'var(--dd-ink-0)', border: '1px solid var(--border-hairline)', zIndex: 2 }}>{opt('wave', 'Wave')}{opt('spectral', 'Spectral')}</div>;
}

/**
 * Waveform panel for the selected sample region. Everything shown is prepared data except
 * `cursor` (playback position) and `startOffset` (live parameter). Not an editor: markers are not draggable.
 */
export function WaveformPanel({
  kind = 'kick', peaks, height = 200, regionStart = 0, regionEnd = 1, fadeIn = 0, fadeOut = 0,
  loopStart, loopEnd, crossfade = 0, slices, selectedSlice, cursor, startOffset = 0, reversed = false,
  hostAutomated = false, startModulation, compact = false, missing = false, label, style,
  display: displayProp, defaultDisplay = 'wave', onDisplayChange, showDisplayToggle = true,
}) {
  const [displayLocal, setDisplayLocal] = React.useState(defaultDisplay);
  const display = displayProp ?? displayLocal;
  const setDisplay = (d) => { if (displayProp == null) setDisplayLocal(d); onDisplayChange && onDisplayChange(d); };
  const spectral = display === 'spectral';
  // slices: numbers (positions) or { pos, name } — names are prepared patch data
  const sliceList = slices ? slices.map((x) => (typeof x === 'number' ? { pos: x } : x)) : null;
  const data = React.useMemo(() => peaks || makePeaks(kind, compact ? 240 : 400), [peaks, kind, compact]);
  const N = data.length;
  const shown = reversed ? data.map((_, i) => data[i]) : data;
  const W = 1000, H = 100, mid = H / 2;
  const path = React.useMemo(() => {
    const src = reversed ? [...shown].reverse() : shown;
    let top = `M0 ${mid}`, bot = '';
    src.forEach((p, i) => { const x = (i / (N - 1)) * W; const a = p * (mid - 4); top += `L${x.toFixed(1)} ${(mid - a).toFixed(1)}`; bot = `L${x.toFixed(1)} ${(mid + a).toFixed(1)}` + bot; });
    return top + `L${W} ${mid}` + bot.replace(/^L/, 'L') + 'Z';
  }, [shown, reversed, N]);
  const regionW = regionEnd - regionStart;
  const startPos = regionStart + startOffset * regionW;
  const flag = { position: 'absolute', top: 0, height: 16, padding: '0 4px', display: 'flex', alignItems: 'center', fontFamily: 'var(--font-value)', fontSize: 10, lineHeight: 1, borderRadius: '0 2px 2px 0', whiteSpace: 'nowrap' };

  return (
    <div style={{ position: 'relative', height, background: 'var(--surface-well)', borderRadius: 'var(--radius-1)', overflow: 'hidden', boxShadow: 'var(--inset-well)', ...style }}>
      {/* centre line + grid */}
      {!spectral && <div style={{ position: 'absolute', left: 0, right: 0, top: '50%', height: 1, background: 'var(--dd-line-1)' }} />}
      {showDisplayToggle && !missing && <ViewToggle value={display} onChange={setDisplay} />}
      {missing ? (
        <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', fontFamily: 'var(--font-ui)', fontSize: 'var(--type-body)', color: 'var(--dd-error)' }}>Sample file not found — region cannot be drawn</div>
      ) : spectral ? (
        <>
          <Spectrogram data={data} kind={kind} reversed={reversed} regionStart={regionStart} regionEnd={regionEnd} />
          <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ position: 'absolute', left: 0, top: 18, width: '100%', height: 'calc(100% - 18px)' }} aria-hidden="true">
            {fadeIn > 0 && <path d={`M${regionStart * W} ${H}L${(regionStart + fadeIn * regionW) * W} 0`} style={{ stroke: 'var(--dd-paper-1)', strokeWidth: 1.5, fill: 'none', vectorEffect: 'non-scaling-stroke', opacity: 0.8 }} />}
            {fadeOut > 0 && <path d={`M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} ${H}`} style={{ stroke: 'var(--dd-paper-1)', strokeWidth: 1.5, fill: 'none', vectorEffect: 'non-scaling-stroke', opacity: 0.8 }} />}
          </svg>
          {!compact && height >= 120 && ['20k', '2k', '200', '20'].map((f, i) => <div key={f} style={{ position: 'absolute', left: 4, top: `calc(18px + ${i} * (100% - 18px) / 3.4)`, fontFamily: 'var(--font-value)', fontSize: 10, color: 'var(--text-tertiary)', pointerEvents: 'none' }}>{f}</div>)}
        </>
      ) : (
        <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ position: 'absolute', left: 0, top: 18, width: '100%', height: `calc(100% - 18px)` }} aria-hidden="true">
          <defs>
            <clipPath id={`rg-${kind}`}><rect x={regionStart * W} y="0" width={regionW * W} height={H} /></clipPath>
          </defs>
          <path d={path} style={{ fill: 'var(--color-waveform)', opacity: 0.28 }} />
          <path d={path} clipPath={`url(#rg-${kind})`} style={{ fill: 'var(--color-waveform-region)' }} />
          {/* fades: gain ramps drawn over the region */}
          {fadeIn > 0 && <path d={`M${regionStart * W} ${H}L${(regionStart + fadeIn * regionW) * W} 0`} style={{ stroke: 'var(--dd-paper-1)', strokeWidth: 1.5, fill: 'none', vectorEffect: 'non-scaling-stroke' }} />}
          {fadeIn > 0 && <path d={`M${regionStart * W} 0L${(regionStart + fadeIn * regionW) * W} 0L${regionStart * W} ${H}Z`} style={{ fill: 'var(--surface-well)', opacity: 0.55 }} />}
          {fadeOut > 0 && <path d={`M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} ${H}`} style={{ stroke: 'var(--dd-paper-1)', strokeWidth: 1.5, fill: 'none', vectorEffect: 'non-scaling-stroke' }} />}
          {fadeOut > 0 && <path d={`M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} 0L${regionEnd * W} ${H}Z`} style={{ fill: 'var(--surface-well)', opacity: 0.55 }} />}
        </svg>
      )}
      {/* outside-region dim */}
      <div style={{ position: 'absolute', top: 0, bottom: 0, left: 0, width: pct(regionStart), background: 'var(--dd-scrim)', opacity: spectral ? 0.5 : 1 }} />
      <div style={{ position: 'absolute', top: 0, bottom: 0, right: 0, width: pct(1 - regionEnd), background: 'var(--dd-scrim)', opacity: spectral ? 0.5 : 1 }} />
      {/* region bounds */}
      {[regionStart, regionEnd].map((p, i) => (
        <div key={i} style={{ position: 'absolute', top: 0, bottom: 0, left: pct(p), width: 1, marginLeft: i ? -1 : 0, background: 'var(--dd-paper-2)' }}>
          {!slices && <div style={{ ...flag, left: i ? undefined : 0, right: i ? 0 : undefined, borderRadius: i ? '2px 0 0 2px' : '0 2px 2px 0', background: 'var(--dd-ink-5)', color: 'var(--text-secondary)' }}>{i ? 'END' : 'START'}</div>}
        </div>
      ))}
      {/* loop */}
      {loopStart != null && loopEnd != null && (
        <>
          <div style={{ position: 'absolute', bottom: 0, height: 6, left: pct(loopStart), width: pct(loopEnd - loopStart), background: 'var(--dd-paper-3)', opacity: 0.5 }} />
          {crossfade > 0 && <div style={{ position: 'absolute', bottom: 0, height: 6, left: pct(loopEnd - crossfade), width: pct(crossfade), background: 'repeating-linear-gradient(135deg, var(--dd-paper-1) 0 2px, transparent 2px 5px)' }} />}
          {[loopStart, loopEnd].map((p, i) => (
            <div key={i} style={{ position: 'absolute', top: 18, bottom: 0, left: pct(p), width: 0, borderLeft: '1px dashed var(--dd-paper-2)' }}>
              <div style={{ ...flag, top: 'auto', bottom: 8, left: i ? undefined : 0, right: i ? 0 : undefined, background: 'var(--dd-ink-5)', color: 'var(--text-primary)', borderRadius: 2 }}>{i ? 'L▸' : '◂L'}</div>
            </div>
          ))}
        </>
      )}
      {/* slices */}
      {sliceList && sliceList.map((sl, i) => {
        const p = sl.pos;
        const next = sliceList[i + 1] ? sliceList[i + 1].pos : regionEnd;
        const sel = i === selectedSlice;
        return (
          <React.Fragment key={i}>
            {sel && <div style={{ position: 'absolute', top: 18, bottom: 0, left: pct(p), width: pct(next - p), background: 'var(--dd-vermilion)', opacity: 0.14 }} />}
            <div style={{ position: 'absolute', top: 0, bottom: 0, left: pct(p), width: sel ? 2 : 1, background: sel ? 'var(--dd-vermilion)' : 'var(--dd-paper-3)' }} />
            {/* flag: number + optional name, clipped to the slice's own width (selected slice may overflow) */}
            <div title={sl.name ? `Slice ${i + 1} · ${sl.name}` : `Slice ${i + 1}`}
              style={{ ...flag, left: pct(p), maxWidth: sel ? 'none' : `calc(${pct(next - p)} - 2px)`, minWidth: 14, gap: 4, overflow: 'hidden', zIndex: sel ? 2 : 1,
                background: sel ? 'var(--dd-vermilion)' : 'var(--dd-ink-5)', color: sel ? 'var(--text-on-accent)' : 'var(--text-secondary)', fontWeight: 600 }}>
              <span style={{ flex: 'none' }}>{i + 1}</span>
              {sl.name && !compact && <span style={{ fontFamily: 'var(--font-ui)', fontSize: 11, fontWeight: sel ? 700 : 600, overflow: 'hidden', textOverflow: 'ellipsis', color: sel ? 'var(--text-on-accent)' : 'var(--text-primary)' }}>{sl.name}</span>}
            </div>
          </React.Fragment>
        );
      })}
      {/* live start offset */}
      {startOffset > 0 && !slices && (
        <div style={{ position: 'absolute', top: 18, bottom: 0, left: pct(startPos), width: 0, borderLeft: `2px solid ${hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)'}` }}>
          <div style={{ position: 'absolute', top: -1, left: -6, width: 0, height: 0, borderLeft: '5px solid transparent', borderRight: '5px solid transparent', borderTop: `6px solid ${hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)'}` }} />
        </div>
      )}
      {startModulation && !slices && (
        <div style={{ position: 'absolute', top: 20, height: 2, left: pct(startPos), width: pct(startModulation.depth * regionW), background: startModulation.color || 'var(--dd-mod-a)', opacity: 0.8 }} />
      )}
      {/* playback cursor */}
      {cursor != null && <div style={{ position: 'absolute', top: 18, bottom: 0, left: pct(cursor), width: 1, background: 'var(--color-cursor)', boxShadow: '0 0 0 1px rgba(19,15,12,0.6)' }} />}
      {reversed && <div style={{ position: 'absolute', left: 6, bottom: 6, fontFamily: 'var(--font-ui)', fontSize: 'var(--type-micro)', fontWeight: 600, letterSpacing: 'var(--tracking-caps)', color: 'var(--text-secondary)', background: 'var(--dd-ink-3)', padding: '2px 5px', borderRadius: 2 }}>◂ REVERSED</div>}
      {label && <div style={{ position: 'absolute', right: 6, top: 22, fontFamily: 'var(--font-value)', fontSize: 10, color: 'var(--text-tertiary)' }}>{label}</div>}
    </div>
  );
}
