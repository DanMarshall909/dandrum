// Das Sampler — app shell, state, playback simulation.
(() => {
const { ContextMenu, SegmentedControl, Button, Toggle } = window.DandrumDesignSystem_3c2eab;
const D = window.SAMPLER_DATA;

function pickRegion(pad, vel, counters, note) {
  const layerIdx = pad.layers.findIndex((l) => vel * 127 >= l.vel[0] && vel * 127 <= l.vel[1] + 0.99);
  const li = layerIdx < 0 ? 0 : layerIdx;
  const layer = pad.layers[li];
  let alt = 0;
  if (layer.regions.length > 1) {
    const c = (counters[note] = (counters[note] || 0) + 1);
    alt = layer.policy === 'Weighted' ? (((c * 7919) % 16) / 16 < 1 / 1.6 ? 0 : 1) : c % layer.regions.length;
  }
  let flat = 0; for (let i = 0; i < li; i++) flat += pad.layers[i].regions.length;
  return { layer: li, alt, flatIdx: flat + alt, region: layer.regions[alt] };
}

function SamplerApp() {
  const init = window.SAMPLER_INIT || {};
  const [layout, setLayout] = React.useState(init.layout || 'full');
  const [patchKind, setPatchKind] = React.useState(init.patch || 'kit');
  const patch = patchKind === 'kit' ? D.kit : D.breakPatch;
  const [selected, setSelected] = React.useState(init.selected ?? 38);
  const [regionIdx, setRegionIdx] = React.useState(1);
  const [view, setView] = React.useState(init.view || 'region');
  const [slice, setSlice] = React.useState(2);
  const [values, setValues] = React.useState({ pitch: 0.5, start: 0.08, level: 0.8, pan: 0.5, variation: 0.35 });
  const [mods, setMods] = React.useState({
    pitch: [{ slot: 'A', source: 'Velocity', depth: 0.12 }, { slot: 'B', source: 'Random per hit', depth: -0.08 }],
    level: [{ slot: 'A', source: 'Velocity', depth: 0.25 }],
    variation: [{ slot: 'B', source: 'Random per hit', depth: 0.3 }],
  });
  const [liveMod, setLiveMod] = React.useState({});
  const [activity, setActivity] = React.useState({});
  const [cursor, setCursor] = React.useState(null);
  const [menu, setMenu] = React.useState(null);
  const [assigning, setAssigning] = React.useState(null);
  const [hostParam, setHostParam] = React.useState(init.host || null);
  const [hostFlash, setHostFlash] = React.useState(null);
  const [missing, setMissing] = React.useState(!!init.missing);
  const [reloading, setReloading] = React.useState(false);
  const [hoverId, setHoverId] = React.useState(null);
  const [pattern, setPattern] = React.useState(!!init.pattern);
  const [chokeFlash, setChokeFlash] = React.useState(false);
  const voicesRef = React.useRef([]);
  const counters = React.useRef({});
  const selRef = React.useRef(selected); selRef.current = selected;
  const valRef = React.useRef(values); valRef.current = values;
  const compact = layout === 'compact';
  const winRef = React.useRef(null);
  const W = compact ? 820 : 1200, H = compact ? 560 : 800;
  const [scale, setScale] = React.useState(1);
  React.useEffect(() => { const f = () => setScale(Math.min(1, (window.innerWidth - 8) / W)); f(); window.addEventListener('resize', f); return () => window.removeEventListener('resize', f); }, [W]);
  const toLocal = (cx, cy) => { const r = winRef.current.getBoundingClientRect(); return { x: (cx - r.left) / scale, y: (cy - r.top) / scale }; };

  // open initial menu for the "menu" state screen
  React.useEffect(() => {
    if (init.menu) {
      const t = setTimeout(() => {
        const el = document.querySelector('[aria-label="' + init.menuLabel + '"]');
        const r = el ? el.getBoundingClientRect() : { left: 400, bottom: 500, width: 0 };
        const p = toLocal(r.right + 4, r.top);
        setMenu({ param: init.menu, x: p.x, y: Math.min(p.y, H - 400), sub: false });
      }, 300);
      return () => clearTimeout(t);
    }
  }, []);

  const trigger = React.useCallback((note, vel) => {
    const now = performance.now();
    if (patch.kind === 'break') {
      const i = note - 36; if (i < 0 || i >= patch.slices.length) return;
      const len = ((patch.slices[i + 1] ?? 1) - patch.slices[i]) * patch.len;
      voicesRef.current = voicesRef.current.filter((v) => false);
      voicesRef.current.push({ note, t0: now, len, from: patch.slices[i], to: patch.slices[i + 1] ?? 1, vel });
      setSlice(i);
      setActivity((a) => ({ ...a, [note]: { level: 1, velocity: vel } }));
      setLiveMod({ A: vel, B: Math.random() });
      return;
    }
    const pad = patch.pads[note]; if (!pad) return;
    let pick = pickRegion(pad, vel, counters.current, note);
    if (missing && pick.region.id === 'hat-open-b') pick = { ...pick, alt: 0, flatIdx: 0, region: pad.layers[0].regions[0] };
    const ratio = Math.pow(2, (valRef.current.pitch - 0.5) * 2);
    const r = pick.region;
    const len = (r.end - r.start) * r.len / ratio;
    // choke
    if (pad.choke) {
      Object.entries(patch.pads).forEach(([n, p]) => { if (Number(n) !== note && p.choke === pad.choke) { voicesRef.current = voicesRef.current.filter((v) => v.note !== Number(n)); setActivity((a) => a[n] && a[n].level > 0.02 ? { ...a, [n]: { ...a[n], level: 0, choked: true } } : a); setChokeFlash(true); setTimeout(() => setChokeFlash(false), 180); } });
    }
    voicesRef.current = voicesRef.current.filter((v) => v.note !== note || voicesRef.current.filter((x) => x.note === note).length < pad.voices);
    voicesRef.current.push({ note, t0: now, len, from: r.start + valRef.current.start * (r.end - r.start) * 0.3, to: r.end, vel });
    setActivity((a) => ({ ...a, [note]: { level: 1, velocity: vel, layer: pad.layers.length > 1 ? pick.layer : -1, alt: pick.alt, choked: false, region: r.id } }));
    setLiveMod({ A: vel, B: ((note * 31 + (counters.current[note] || 0) * 17) % 100) / 100 });
    if (note === selRef.current) setRegionIdx(pick.flatIdx);
  }, [patch, missing]);

  // frame loop: decay pad levels, move cursor
  React.useEffect(() => {
    let raf;
    const tick = () => {
      const now = performance.now();
      voicesRef.current = voicesRef.current.filter((v) => now - v.t0 < v.len);
      setActivity((a) => {
        let changed = false; const n = { ...a };
        for (const k of Object.keys(n)) {
          const vs = voicesRef.current.filter((v) => v.note === Number(k));
          const lvl = vs.length ? Math.max(...vs.map((v) => 1 - (now - v.t0) / v.len)) : 0;
          if (Math.abs((n[k].level || 0) - lvl) > 0.005) { n[k] = { ...n[k], level: lvl }; changed = true; }
          if (n[k].choked && now % 1000 < 20) { /* keep */ }
        }
        return changed ? n : a;
      });
      const sv = voicesRef.current.filter((v) => patch.kind === 'break' || v.note === selRef.current).slice(-1)[0];
      setCursor(sv ? sv.from + (sv.to - sv.from) * ((now - sv.t0) / sv.len) : null);
      if (!voicesRef.current.length) setLiveMod((m) => (m.A != null || m.B != null ? {} : m));
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [patch]);

  // clear choked flag after a moment
  React.useEffect(() => {
    const ch = Object.entries(activity).filter(([, a]) => a.choked);
    if (!ch.length) return;
    const t = setTimeout(() => setActivity((a) => { const n = { ...a }; ch.forEach(([k]) => { n[k] = { ...n[k], choked: false }; }); return n; }), 500);
    return () => clearTimeout(t);
  }, [activity]);

  // demo pattern
  React.useEffect(() => {
    if (!pattern) return;
    const seq = patch.kind === 'break'
      ? [36, 37, 38, 39, 40, 41, 42, 43].map((n) => [n])
      : [[36, 42], [42], [38, 42], [42], [36, 42], [36, 46], [38, 42], [46]];
    let i = 0;
    const id = setInterval(() => { seq[i % seq.length].forEach((n) => trigger(n, n === 38 ? (i % 4 === 2 ? 0.95 : 0.6) : n === 42 ? 0.55 + (i % 2) * 0.2 : 0.85)); i++; }, 136);
    return () => clearInterval(id);
  }, [pattern, trigger, patch]);

  // simulated host automation
  React.useEffect(() => {
    if (!hostParam) return;
    let t = 0;
    const id = setInterval(() => { t += 0.05; setValues((v) => ({ ...v, [hostParam]: hostParam === 'pan' ? 0.5 + Math.sin(t) * 0.3 : 0.62 + Math.sin(t) * 0.18 })); setHostFlash(hostParam); }, 50);
    return () => { clearInterval(id); setHostFlash(null); };
  }, [hostParam]);

  React.useEffect(() => {
    const k = (e) => { if (e.key === 'Escape') { setAssigning(null); setMenu(null); } };
    window.addEventListener('keydown', k); return () => window.removeEventListener('keydown', k);
  }, []);

  const pad = patch.kind === 'kit' ? patch.pads[selected] : null;
  const regions = pad ? pad.layers.flatMap((l) => l.regions) : [];
  const region = regions[Math.min(regionIdx, regions.length - 1)];
  const missingRegionId = missing ? 'hat-open-b' : null;

  const status = reloading ? { kind: 'busy', title: 'Preparing patch…', text: 'loading sample regions', short: 'Preparing…' }
    : missing ? { kind: 'error', title: '1 sample file missing', text: 'hat_open_b.wav · Open Hat alternate B will not play', short: '1 file missing' }
    : { kind: 'ok', title: 'Patch loaded', text: patch.kind === 'kit' ? '4 sounds · 7 sample regions' : '1 sample · 8 slices', short: 'Loaded' };

  const onReload = () => { setReloading(true); voicesRef.current = []; setTimeout(() => { setReloading(false); }, 900); };
  const setDepth = (param, slot, d) => setMods((m) => ({ ...m, [param]: m[param].map((a) => (a.slot === slot ? { ...a, depth: d } : a)) }));
  const remove = (param, slot) => setMods((m) => ({ ...m, [param]: (m[param] || []).filter((a) => a.slot !== slot) }));
  const add = (param, s) => setMods((m) => ({ ...m, [param]: [...(m[param] || []).filter((a) => a.slot !== s.slot), { slot: s.slot, source: s.name, depth: 0.25 }] }));
  const startAssign = (param, s) => { add(param, s); setAssigning({ slot: s.slot, name: s.name }); setMenu(null); };
  const onMenu = (param, e) => { const p = toLocal(e.clientX, e.clientY); setMenu({ param, x: Math.min(p.x, W - 256), y: Math.min(p.y, H - 380), sub: false }); };

  const menuSpec = menu && buildModMenu({ param: menu.param, value: values[menu.param], mods, setDepth, remove, startAssign, hostParam,
    reset: (p) => setValues((v) => ({ ...v, [p]: D.liveParams.find((x) => x.id === p).def })), close: () => setMenu(null), sub: menu.sub, setSub: (s) => setMenu((m) => ({ ...m, sub: s })) });

  const levels = [-60 + Math.max(0, ...Object.values(activity).map((a) => a.level || 0)) * 52, -60 + Math.max(0, ...Object.values(activity).map((a) => a.level || 0)) * 49];
  const startMod = (mods.start || [])[0];

  return (
    <div style={{ height: (H + 80) * scale, width: W * scale, display: 'flex', justifyContent: 'center' }}>
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 12, transform: `scale(${scale})`, transformOrigin: 'top center', flex: 'none' }}>
      <div ref={winRef} data-screen-label={compact ? 'Compact' : 'Main'} style={{ width: W, height: H, display: 'flex', flexDirection: 'column', background: 'var(--surface-window)', border: '1px solid #000', boxShadow: '0 20px 60px rgba(0,0,0,0.5)', overflow: 'hidden', position: 'relative', transform: 'translateZ(0)', flex: 'none' }}
        onClick={() => menu && setMenu(null)}>
        <SamplerHeader patch={patch} status={status} compact={compact} onLayout={setLayout} onReload={onReload} levels={levels} />
        {missing && !compact && (
          <div style={{ padding: '4px 4px 0' }}>
            <window.DandrumDesignSystem_3c2eab.StatusMessage kind="error" compact title="Open Hat · alternate B unavailable"
              action={<div style={{ display: 'flex', gap: 6 }}><Button size="sm" icon="folder">Locate file…</Button><Button size="sm" variant="primary" icon="reload" onClick={() => { setMissing(false); onReload(); }}>Reload Patch</Button></div>}>
              hat_open_b.wav was not found. Open Hat plays alternate A only until the file is restored and the patch is reloaded.
            </window.DandrumDesignSystem_3c2eab.StatusMessage>
          </div>
        )}
        <div style={{ flex: 1, minHeight: 0, display: 'grid', gridTemplateColumns: compact ? '240px minmax(0,1fr)' : '360px minmax(0,1fr)', gap: 4, padding: 4 }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minHeight: 0 }}>
            <PadGrid patch={patch} selected={selected} onSelect={(n) => { setSelected(n); setRegionIdx(0); }} onTrigger={trigger} activity={activity} compact={compact} chokeFlash={chokeFlash} missing={missing} />
            <PadDetails note={selected} pad={pad} regionIdx={Math.min(regionIdx, Math.max(0, regions.length - 1))} onRegion={setRegionIdx} activeRegion={(activity[selected] && activity[selected].level > 0.02) ? activity[selected].region : null} compact={compact} missingRegionId={missingRegionId} patch={patch} />
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minHeight: 0 }}>
            <WaveView patch={patch} view={view} onView={setView} pad={pad} region={region} regionIdx={Math.min(regionIdx, Math.max(0, regions.length - 1))} onRegion={setRegionIdx}
              cursor={cursor} startOffset={values.start} hostStart={hostFlash === 'start'} startMod={startMod ? { depth: startMod.depth, color: 'var(--dd-mod-' + startMod.slot.toLowerCase() + ')' } : null}
              compact={compact} slice={slice} onSlice={(i) => { setSlice(i); trigger(36 + i, 0.8); }}
              onLoadBreak={() => { setPatchKind('break'); setSelected(38); setView('slices'); onReload(); }}
              onLoadKit={() => { setPatchKind('kit'); setSelected(38); setView('region'); onReload(); }}
              missingRegion={missing && region && region.id === 'hat-open-b'} />
            <LiveControls values={values} onChange={(id, v) => setValues((x) => ({ ...x, [id]: v }))} mods={mods} liveMod={liveMod} assigning={assigning}
              hostParam={hostFlash} onMenu={onMenu} onAssignTo={(p) => add(p, { slot: assigning.slot, name: assigning.name })} compact={compact} hoverId={hoverId} setHoverId={setHoverId} />
          </div>
        </div>
        <SamplerStatusBar assigning={assigning} onEndAssign={() => setAssigning(null)} compact={compact}
          voices={`${voicesRef.current.length} active · limit ${pad ? pad.voices : 4} per sound`} />
        {menuSpec && <div onClick={(e) => e.stopPropagation()}><ContextMenu key={menu.param + menu.sub} x={menu.x} y={menu.y} style={{ position: 'absolute' }} title={menuSpec.title} items={menuSpec.items} onClose={() => setMenu(null)} /></div>}
      </div>
      <div style={{ width: W, display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap', padding: '8px 10px', border: '1px dashed var(--dd-line-2)', borderRadius: 4, fontFamily: 'var(--font-ui)', fontSize: 12, color: 'var(--dd-paper-3)' }}>
        <span style={{ fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', fontSize: 11 }}>Mockup controls</span>
        <Toggle compact label="Play demo pattern" checked={pattern} onChange={setPattern} />
        <span>Host automates</span>
        <SegmentedControl compact value={hostParam || 'none'} onChange={(v) => setHostParam(v === 'none' ? null : v)} options={[{ id: 'none', label: 'Off' }, { id: 'level', label: 'Level' }, { id: 'pan', label: 'Pan' }, { id: 'start', label: 'Start' }]} />
        <Toggle compact label="Missing sample" checked={missing} onChange={setMissing} />
        <SegmentedControl compact value={patchKind} onChange={(v) => { setPatchKind(v); setView(v === 'break' ? 'slices' : 'region'); }} options={[{ id: 'kit', label: 'Drum kit' }, { id: 'break', label: 'Break patch' }]} />
      </div>
    </div>
    </div>
  );
}

Object.assign(window, { SamplerApp });
})();
