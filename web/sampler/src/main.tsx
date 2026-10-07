import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { preparedPads, selectedRegion, visibleParameters,
  padReleaseHandlers, auditionFocusRelease } from './model.mjs';
import { createHostKnob } from '../../shared/host-knob.mjs';
import { createKeyMap } from './key-map.mjs';
import { createLayerStack } from './layer-stack.mjs';
import { createOutputBuses, acknowledgeOutputClip } from './output-buses.mjs';
import './key-map.css';
import './layer-stack.css';
import './output-buses.css';
import { admittedParameter } from '../../shared/parameter-value.mjs';
import { createHostParameters } from '../../shared/host-parameters.mjs';
import '../../shared/host-knob.css';
import { createNoteAudition } from '../../tb303/src/note-audition.mjs';
import { createMeterTransport } from '../../shared/meter-transport.mjs';
import { meterView } from '../../shared/meter-view.mjs';
import { createSpectrumTransport } from '../../shared/spectrum-transport.mjs';
import { preparedSpectrumView, paintPreparedSpectrum } from '../../shared/prepared-spectrum.mjs';
import { createPreparedPolling } from '../../shared/prepared-polling.mjs';
import { createWaveformTransport } from '../../shared/waveform-transport.mjs';
import { preparedWaveformView, paintPreparedWaveform } from '../../shared/prepared-waveform.mjs';
import { createLiveAnalysisTransport } from '../../shared/live-analysis-transport.mjs';
import { liveAnalysisView, paintLiveAnalysis } from '../../shared/live-analysis-view.mjs';
import './styles.css';
import '../../shared/design-tokens.css';
import '../../shared/design-fonts.css';
import { iconProps } from '../../shared/design-icons.mjs';
import '../../shared/design-icons.css';

type HostParameter = { id: string; name: string; value: number };
type HostState = { generation: number; sequence: number; parameters: HostParameter[] };
type PreparedParameter = { id: string; name: string; scope: string; controlGroup?: number;
  minValue: number; maxValue: number; normalisedDefaultValue: number };
type PreparedRegion = { id: string };
type PreparedSource = { id: string; sampleRateHz: number; regions: PreparedRegion[] };
type PreparedOutputBus = { id: string; name: string; main: boolean;
  channels: string[]; meterBusId: string };
type PreparedDocument = {
  generation: number;
  instrumentId: string;
  sources: PreparedSource[];
  maps: unknown[];
  outputBuses: PreparedOutputBus[];
  parameters: PreparedParameter[];
  capabilities: { sampleKeyMap: boolean; preparedWaveform: boolean;
    synthLayer: boolean; nestedPatchLayer: boolean; moduleChain: boolean };
};
type Backend = {
  getNativeFunction: (name: string) => (...args: unknown[]) => Promise<unknown>;
  addEventListener: (name: string, listener: (state: HostState) => void) => void;
  removeEventListener?: (name: string, listener: (state: HostState) => void) => void;
};
type WaveStatus = { state: string; generation?: number; result?: unknown; error?: string };

declare global {
  interface Window { __JUCE__?: { backend?: Backend } }
}

async function invoke(name: string, ...args: unknown[]): Promise<any> {
  const native = window.__JUCE__?.backend?.getNativeFunction(name);
  if (!native) throw new Error(`Host command ${name} is unavailable`);
  const reply = await native(...args);
  if (typeof reply === 'string') throw new Error(reply);
  return reply;
}

const HostKnob = createHostKnob(React);
const PreparedKeyMap = createKeyMap(React);
const PreparedLayerStack = createLayerStack(React);
const PreparedOutputBuses = createOutputBuses(React);

const useHost = createHostParameters(React);

function OutputBusPanel({ document, reportError }: {
  document: PreparedDocument; reportError: (reason: unknown) => void;
}) {
  const generation = document.generation;
  const [display, setDisplay] = useState(() => meterView(null));
  useEffect(() => {
    const transport = createMeterTransport(invoke, (packet: any) => setDisplay((current: any) => meterView(packet, current)));
    const visible = () => void transport.setVisible(!window.document.hidden).catch(reportError);
    visible();
    void transport.start(generation).catch(reportError);
    const timer = window.setInterval(() => void transport.tick().catch(reportError), 1000 / 30);
    window.document.addEventListener('visibilitychange', visible);
    return () => {
      window.clearInterval(timer);
      window.document.removeEventListener('visibilitychange', visible);
      void transport.setVisible(false).catch(reportError);
    };
  }, [generation]);
  return <PreparedOutputBuses buses={document.outputBuses} generation={generation} meter={display}
    headerIcon={<svg {...iconProps('level')} />}
    onAcknowledge={(_busId: string, channel: number, observedGeneration: number, ticket: string) =>
      void acknowledgeOutputClip(invoke, setDisplay, channel, observedGeneration, ticket)
        .catch(reportError)} />;
}

function PreparedWaveform({ document, sourceId, regionId, reportError, controls }: {
  document: PreparedDocument; sourceId: string; regionId: string;
  reportError: (reason: unknown) => void; controls: React.ReactNode;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState<WaveStatus | null>(null);
  const [width, setWidth] = useState(1);
  useEffect(() => {
    setStatus(null);
    const transport = createPreparedPolling((onState: (packet: any) => void) => createWaveformTransport(invoke, onState),
      (packet: { status: WaveStatus }) => setStatus(packet.status), reportError, window);
    void transport.select({ sourceId, regionId, channel: 0, buckets: 512,
      generation: document.generation }).catch(reportError);
    return () => { void transport.close().catch(reportError); };
  }, [document.generation, sourceId, regionId]);
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const resize = () => setWidth(Math.max(1, Math.round(element.clientWidth)));
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const view = useMemo(() => preparedWaveformView(document, sourceId, regionId,
    status, width, 180), [document, sourceId, regionId, status, width]);
  useLayoutEffect(() => {
    const element = canvas.current;
    const context = element?.getContext('2d');
    if (!element || !context) return;
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.round(width * ratio);
    element.height = Math.round(180 * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    if (view) paintPreparedWaveform(context, view);
    else { context.fillStyle = '#111916'; context.fillRect(0, 0, width, 180); }
  }, [view, width]);
  return <section className="panel waveform" aria-label="Prepared sample waveform">
    <div className="section-heading"><h2>{sourceId}.{regionId}</h2>
      <span>{view ? `${view.sampleRateHz} Hz · ${view.durationSeconds.toFixed(3)} s`
        : status?.state === 'failed' ? status.error || 'WAVEFORM UNAVAILABLE'
        : 'PREPARING WAVEFORM'}</span></div>
    <div className="sample-plot">
      <canvas ref={canvas} aria-label={`Signed waveform for ${sourceId}.${regionId}`} />
      {controls}
    </div>
  </section>;
}

function PreparedSpectrum({ document, sourceId, regionId, reportError, controls }: {
  document: PreparedDocument; sourceId: string; regionId: string;
  reportError: (reason: unknown) => void; controls: React.ReactNode;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [status, setStatus] = useState<WaveStatus | null>(null);
  const [width, setWidth] = useState(1);
  const [attempt, setAttempt] = useState(0);
  const height = 128;
  useEffect(() => {
    let transport: ReturnType<typeof createPreparedPolling> | null = null;
    const visible = () => {
      if (window.document.hidden) {
        void transport?.close().catch(reportError); transport = null; setStatus(null);
      } else if (!transport) {
        transport = createPreparedPolling((onState: (packet: any) => void) => createSpectrumTransport(invoke, onState),
          (packet: { status: WaveStatus }) => setStatus(packet.status), reportError, window);
        void transport.select({ sourceId, regionId, channel: 0, generation: document.generation }).catch(reportError);
      }
    };
    setStatus(null); visible();
    window.document.addEventListener('visibilitychange', visible);
    return () => {
      window.document.removeEventListener('visibilitychange', visible);
      void transport?.close().catch(reportError); transport = null;
    };
  }, [document.generation, sourceId, regionId, attempt]);
  useEffect(() => {
    const element = canvas.current;
    if (!element) return;
    const resize = () => setWidth(Math.max(1, Math.round(element.clientWidth)));
    resize(); const observer = new ResizeObserver(resize); observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const view = useMemo(() => preparedSpectrumView(document, sourceId, regionId, status, width, height),
    [document, sourceId, regionId, status, width]);
  useLayoutEffect(() => {
    const element = canvas.current, context = element?.getContext('2d');
    if (!element || !context) return;
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.round(width * ratio); element.height = Math.round(height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    const style = getComputedStyle(element);
    const ramp = ['--dd-ink-0', '--dd-ink-5', '--dd-vermilion-lo', '--dd-vermilion', '--dd-paper-1']
      .map(name => style.getPropertyValue(name).trim());
    context.fillStyle = ramp[0]; context.fillRect(0, 0, width, height);
    if (view) paintPreparedSpectrum(context, view, ramp);
  }, [view, width]);
  const failed = status && ['failed', 'cancelled', 'stale'].includes(status.state);
  return <section className="panel spectrogram" aria-label="Prepared sample spectrum">
    <div className="section-heading"><h2>{sourceId}.{regionId}</h2>
      <span>{view ? `${view.sampleRateHz} Hz · ${view.durationSeconds.toFixed(3)} s`
        : failed ? 'SPECTRUM UNAVAILABLE' : 'PREPARING SPECTRUM'}</span></div>
    <div className="spectral-plot">
      <span className="frequency-max" data-spectral-label="frequency-max">{view ? `${(view.maximumHz / 1000).toFixed(0)} kHz` : ''}</span>
      <span className="frequency-min" data-spectral-label="frequency-min">{view ? `${Math.round(view.minimumHz)} Hz` : ''}</span>
      <canvas ref={canvas} aria-label={`Log-frequency spectrum for ${sourceId}.${regionId}`} />
      {controls}
    </div>
    <div className="spectral-time-axis">
      <span data-spectral-label="time-start">{view ? `${view.startSeconds.toFixed(3)} s` : ''}</span>
      <span data-spectral-label="time-end">{view ? `${view.endSeconds.toFixed(3)} s` : ''}</span>
    </div>
    <p className="spectral-settings" data-spectral-label="settings">{view
      ? `Hann · FFT ${view.settings.fftSize} · hop ${view.settings.hopFrames} · ${view.floorDbFS}..0 dBFS · ch 1 · DC omitted`
      : failed ? status.error || `Analysis ${status.state}` : status?.state === 'ready' ? 'Invalid spectral data' : 'Waiting for analysis'}</p>
    {failed && <button className="spectral-retry" onClick={() => setAttempt(current => current + 1)}>Retry analysis</button>}
  </section>;
}

function useLiveAnalysis(generation: number | undefined, active: boolean, reportError: (reason: unknown) => void) {
  const [packet, setPacket] = useState<any>(null);
  const [unavailable, setUnavailable] = useState('');
  const transport = useRef<ReturnType<typeof createLiveAnalysisTransport> | null>(null);
  const activeNow = useRef(active); activeNow.current = active;
  const setVisible = async (stream: ReturnType<typeof createLiveAnalysisTransport>, owner: number, shown: boolean) => {
    await stream.setVisible(shown);
    if (shown && transport.current === stream && activeNow.current && !window.document.hidden) {
      const admitted = await stream.start(owner, 3);
      if (transport.current !== stream || !activeNow.current) return;
      setUnavailable(admitted ? '' : 'Live analysis unavailable; choose a display to retry');
    }
  };
  useEffect(() => {
    setPacket(null); setUnavailable('');
    if (!generation) return;
    const stream = createLiveAnalysisTransport(invoke, (next: any) => {
      if (!liveAnalysisView(next, 0, 1, 1, 'scope')) throw new Error('Invalid live audio measurement');
      setPacket(next);
    });
    transport.current = stream;
    const visible = () => {
      const shown = activeNow.current && !window.document.hidden;
      if (!shown) setPacket(null);
      void setVisible(stream, generation, shown).catch(reportError);
    };
    const hide = () => { setPacket(null); void stream.setVisible(false).catch(reportError); };
    visible();
    const timer = window.setInterval(() => void stream.tick().catch(reportError), 1000 / 30);
    window.document.addEventListener('visibilitychange', visible);
    window.addEventListener('pagehide', hide); window.addEventListener('pageshow', visible);
    return () => {
      window.clearInterval(timer); transport.current = null;
      window.document.removeEventListener('visibilitychange', visible);
      window.removeEventListener('pagehide', hide); window.removeEventListener('pageshow', visible);
      void stream.close().catch(reportError);
    };
  }, [generation]);
  useEffect(() => {
    const stream = transport.current;
    const shown = active && !window.document.hidden;
    if (!shown) { setPacket(null); setUnavailable(''); }
    if (stream && generation)
      void setVisible(stream, generation, shown).catch(reportError);
  }, [active, generation]);
  return { packet: packet?.generation === generation ? packet : null, unavailable };
}

function LiveAnalysisPanel({ packet, unavailable, mode, controls }: {
  packet: any; unavailable: string; mode: 'scope' | 'spectrum'; controls: React.ReactNode;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [width, setWidth] = useState(1);
  const [channel, setChannel] = useState(0);
  useLayoutEffect(() => {
    const element = canvas.current!;
    const resize = () => setWidth(Math.max(1, Math.round(element.clientWidth)));
    const observer = new ResizeObserver(resize); observer.observe(element); resize();
    return () => observer.disconnect();
  }, []);
  const view = useMemo(() => liveAnalysisView(packet, channel, width, 160, mode), [packet, channel, width, mode]);
  useLayoutEffect(() => {
    const element = canvas.current!;
    const ratio = window.devicePixelRatio;
    element.width = Math.round(width * ratio); element.height = Math.round(160 * ratio);
    const context = element.getContext('2d');
    if (!context) return;
    context.scale(ratio, ratio);
    context.fillStyle = '#161616'; context.fillRect(0, 0, width, 160);
    paintLiveAnalysis(context, view);
  }, [view, width]);
  return <section className="panel live-analysis" aria-label={`Live master ${mode}`}
    data-generation={view?.generation} data-mode={view?.mode} data-channel={view?.channel}
    data-start-frame={view?.startFrame} data-end-frame={view?.endFrame}>
    <div className="section-heading"><h2>Master {mode}</h2>
      <span>{view ? view.gap ? 'HISTORY GAP' : 'LIVE' : unavailable ? 'UNAVAILABLE' : 'WAITING FOR AUDIO'}</span>
      <div className="live-channels" role="group" aria-label="Master output channel">
        {[0, 1].map(index => <button key={index} data-live-channel={index} aria-pressed={channel === index}
          onClick={() => setChannel(index)}>{index === 0 ? 'L' : 'R'}</button>)}
      </div>
    </div>
    <div className="sample-plot"><canvas ref={canvas} aria-label={`Live ${mode}, channel ${channel === 0 ? 'L' : 'R'}`} />{controls}</div>
    <div className="live-axis"><span>{mode === 'scope' ? 'signed −1..+1' : `${view?.minimumHz ?? '—'} Hz · DC omitted`}</span>
      <span>{mode === 'scope' ? `${view?.durationMs.toFixed(3) ?? '—'} ms` : `${view?.maximumHz ?? '—'} Hz · −120..0 dBFS`}</span></div>
    {unavailable && <p role="status" className="spectral-settings">{unavailable}</p>}
    <p className="spectral-settings">{view ? `Output frames ${view.startFrame}..${view.endFrame} · ${view.sampleRateHz} Hz · ` : ''}
      Hann · FFT 1024 · hop 256 · -120..0 dBFS</p>
  </section>;
}

function SamplerApp() {
  const { state, document, error, command, reportError } = useHost(window.__JUCE__?.backend);
  const pads = useMemo(() => preparedPads(document), [document]);
  const [sampleDisplay, setSampleDisplay] = useState<'wave' | 'spectral' | 'scope' | 'live-spectrum'>('wave');
  const current = state && document && state.generation === document.generation;
  const live = useLiveAnalysis(document?.generation,
    Boolean(current && (sampleDisplay === 'scope' || sampleDisplay === 'live-spectrum')), reportError);
  const [selection, setSelection] = useState<{
    generation: number; padId: string; zoneId: string;
  } | null>(null);
  const selected = pads.find(pad => selection?.generation === document?.generation
    && pad.id === selection?.padId) ?? pads[0] ?? null;
  const selectedZoneId = selection?.generation === document?.generation
    && selection?.padId === selected?.id ? selection?.zoneId : selected?.zoneIds[0];
  const region = selectedRegion(document, selected, selectedZoneId);
  const parameters = visibleParameters(document, selected, selectedZoneId) as PreparedParameter[];
  const audition = useMemo(() => createNoteAudition(invoke, reportError), []);
  const [pressed, setPressed] = useState<string[]>([]);
  useEffect(() => {
    void audition.releaseAll();
    setPressed([]);
    setSelection(null);
  }, [document?.generation]);
  useEffect(() => {
    const heartbeat = window.setInterval(() => void audition.keepAlive(), 500);
    const release = auditionFocusRelease(audition, () => setPressed([]));
    const visibility = () => { if (window.document.hidden) release(); };
    window.addEventListener('pagehide', release);
    window.addEventListener('blur', release);
    window.document.addEventListener('visibilitychange', visibility);
    return () => {
      window.clearInterval(heartbeat);
      window.removeEventListener('pagehide', release);
      window.removeEventListener('blur', release);
      window.document.removeEventListener('visibilitychange', visibility);
      void audition.releaseAll();
    };
  }, [audition]);

  const select = (pad: typeof pads[number], zoneId = pad.zoneIds[0]) => {
    if (document) setSelection({ generation: document.generation, padId: pad.id, zoneId });
  };
  const press = (pad: typeof pads[number]) => {
    if (!document) return;
    select(pad);
    setPressed(current => current.includes(pad.id) ? current : [...current, pad.id]);
    void audition.press(pad.keyLow, pad.velocity, document.generation);
  };
  const release = (pad: typeof pads[number]) => {
    setPressed(current => current.filter(id => id !== pad.id));
    void audition.release(pad.keyLow);
  };
  const displayControls = <div className="sample-display-controls" role="group" aria-label="Sample display">
    <button data-sample-display="wave" aria-pressed={sampleDisplay === 'wave'} onClick={() => setSampleDisplay('wave')}>Wave</button>
    <button data-sample-display="spectral" aria-pressed={sampleDisplay === 'spectral'} onClick={() => setSampleDisplay('spectral')}>Spectral</button>
    <button data-sample-display="scope" aria-pressed={sampleDisplay === 'scope'} onClick={() => setSampleDisplay('scope')}>Scope</button>
    <button data-sample-display="live-spectrum" aria-pressed={sampleDisplay === 'live-spectrum'} onClick={() => setSampleDisplay('live-spectrum')}>Live FFT</button>
  </div>;
  return <main className="sampler">
    <header className="top"><div><p className="eyebrow">DANDRUM · PREPARED INSTRUMENT</p>
      <h1>Drum Sampler</h1><p>{current ? document.instrumentId : 'Loading prepared instrument…'}</p></div>
      <span className="generation">{current ? `GEN ${document.generation}` : 'WAITING'}</span></header>
    {error && <p className="error" role="alert"><svg {...iconProps('error')} />{error}</p>}
    {current && <div className="editor-content">
      <div className="map-column">
      <section className="panel keymap" aria-label="Prepared key map">
        <div className="section-heading"><h2><svg {...iconProps('keyboard')} />Key map</h2><span>PREPARED · READ ONLY</span></div>
        {document.capabilities.sampleKeyMap
          ? <PreparedKeyMap key={document.generation} pads={pads} selectedId={selected?.id}
              onSelect={select}
              onNoteOn={(note: number, velocity: number) => void audition.press(note, velocity, document.generation)}
              onNoteOff={(note: number) => void audition.release(note)} />
          : <p className="details">Key map unavailable for this instrument</p>}
        <div className="pads">{pads.map(pad => <button key={pad.id}
          className={`pad ${selected?.id === pad.id ? 'selected' : ''} ${pressed.includes(pad.id) ? 'pressed' : ''}`}
          aria-label={`${pad.label}, MIDI ${pad.keyLow}, velocity ${pad.velocityLow} to ${pad.velocityHigh}`}
          onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); press(pad); }}
          {...padReleaseHandlers(() => release(pad))}
          onClick={() => select(pad)}
          onKeyDown={event => { if (!event.repeat && (event.key === ' ' || event.key === 'Enter')) {
            event.preventDefault(); press(pad); } }}>
          <strong>{pad.label}</strong>
          <small>NOTE {pad.keyLow}{pad.keyHigh !== pad.keyLow ? `–${pad.keyHigh}` : ''}
            {' · '}VEL {pad.velocityLow}–{pad.velocityHigh}</small>
          {pad.zoneIds.length > 1 && <small><svg {...iconProps('alternate', { size: 12 })} />{pad.zoneIds.length} round robin alternatives</small>}
          {pad.chokeGroup && <small><svg {...iconProps('choke', { size: 12 })} />CHOKE {pad.chokeGroup}</small>}
        </button>)}</div>
        {selected && <>
          <p className="details">{selected.selectionMode} selection · {selectedZoneId}</p>
          {selected.zoneIds.length > 1 && <div className="alternatives" role="group" aria-label="Sample alternatives">
            <span>Inspect alternative</span>
            {selected.zoneIds.map(zoneId => <button key={zoneId} data-zone-id={zoneId}
              aria-pressed={selectedZoneId === zoneId} onClick={() => select(selected, zoneId)}>
              {zoneId.replaceAll('_', ' ')}
            </button>)}
            <small>Audition follows {selected.selectionMode.replaceAll('_', ' ')} selection.</small>
          </div>}
        </>}
      </section>
        <PreparedLayerStack key={document.generation} document={document} pad={selected}
          selectedZoneId={selectedZoneId} onSelect={select} readOnlyIcon={<svg {...iconProps('lock')} />} />
      </div>
      <div className="instrument-column">
          {sampleDisplay === 'scope' || sampleDisplay === 'live-spectrum'
            ? <LiveAnalysisPanel {...live} mode={sampleDisplay === 'scope' ? 'scope' : 'spectrum'} controls={displayControls} />
            : region && document.capabilities.preparedWaveform
            ? <div className="sample-display">
                {sampleDisplay === 'wave'
                  ? <PreparedWaveform document={document} sourceId={region.sourceId} regionId={region.regionId} reportError={reportError} controls={displayControls} />
                  : <PreparedSpectrum document={document} sourceId={region.sourceId} regionId={region.regionId} reportError={reportError} controls={displayControls} />}
              </div>
            : <section className="panel sample-display unavailable-analysis">Prepared sample analysis unavailable{displayControls}</section>}
          <section className="panel controls" id="sampler-public-controls" aria-label="Host modulatable controls">
            <div className="section-heading"><h2><svg {...iconProps('host')} />Public controls</h2>
              <span>{parameters.length} SHARED / SELECTED PAD</span></div>
            <div className="control-list">{parameters.map(descriptor => <HostKnob
              key={`${descriptor.id}:${state.generation}`} label={descriptor.name || descriptor.id}
              parameter={admittedParameter(state, document, descriptor.id)}
              command={command} onError={reportError} />)}</div>
          </section>
          <OutputBusPanel document={document} reportError={reportError} />
      </div>
    </div>}
  </main>;
}

createRoot(document.getElementById('root')!).render(<SamplerApp />);
