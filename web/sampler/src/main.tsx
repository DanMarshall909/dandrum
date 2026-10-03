import React, { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { preparedPads, selectedRegion, visibleParameters,
  padReleaseHandlers, auditionFocusRelease } from './model.mjs';
import { createHostKnob } from '../../shared/host-knob.mjs';
import { admittedParameter } from '../../shared/parameter-value.mjs';
import { createPreparedParameterDocument } from '../../shared/prepared-parameter-document.mjs';
import '../../shared/host-knob.css';
import { createNoteAudition } from '../../tb303/src/note-audition.mjs';
import { createMeterTransport } from '../../shared/meter-transport.mjs';
import { meterView } from '../../shared/meter-view.mjs';
import { createSpectrumTransport } from '../../shared/spectrum-transport.mjs';
import { preparedSpectrumView, paintPreparedSpectrum } from '../../shared/prepared-spectrum.mjs';
import { createWaveformTransport } from '../../shared/waveform-transport.mjs';
import { preparedWaveformView, paintPreparedWaveform } from '../../shared/prepared-waveform.mjs';
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
type PreparedDocument = {
  generation: number;
  instrumentId: string;
  sources: PreparedSource[];
  maps: unknown[];
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

function useHost() {
  const [state, setState] = useState<HostState | null>(null);
  const [document, setDocument] = useState<PreparedDocument | null>(null);
  const [error, setError] = useState('');
  const documents = useMemo(() => createPreparedParameterDocument(
    () => invoke('getPreparedDocument'),
    (next: PreparedDocument) => setDocument(next),
    (reason: unknown) => setError(String(reason))), []);

  const acceptState = (next: HostState) => {
    if (!next || !Number.isInteger(next.generation) || !Array.isArray(next.parameters))
      return;
    void documents.acceptGeneration(next.generation);
    setState(current => current && (next.generation < current.generation
      || (next.generation === current.generation && next.sequence < current.sequence))
      ? current : next);
  };
  const refresh = async () => {
    const next = await invoke('getParameterState') as HostState;
    acceptState(next);
    await documents.acceptGeneration(next.generation);
  };

  useEffect(() => {
    const backend = window.__JUCE__?.backend;
    if (!backend) {
      setError('Open this panel in the Dandrum plugin host.');
      return;
    }
    const changed = acceptState;
    backend.addEventListener('parameterStateChanged', changed);
    void refresh().catch(reason => setError(String(reason)));
    return () => {
      backend.removeEventListener?.('parameterStateChanged', changed);
      documents.close();
    };
  }, []);

  const command = async (name: string, ...args: unknown[]) => {
    try {
      const reply = await invoke(name, ...args);
      if (reply?.status && reply.status !== 'accepted')
        throw new Error(`Host rejected ${name}: ${reply.status}`);
      if (name !== 'endGesture') setError('');
      if (name === 'setParameter')
        acceptState(await invoke('getParameterState') as HostState);
      return reply;
    } catch (reason) {
      setError(String(reason));
      if (name === 'setParameter')
        void invoke('getParameterState').then(next => acceptState(next as HostState))
          .catch(() => {});
      throw reason;
    }
  };
  return { state, document, error, command, reportError: (reason: unknown) => setError(String(reason)) };
}

function MasterMeter({ generation, reportError }: {
  generation: number; reportError: (reason: unknown) => void;
}) {
  const [display, setDisplay] = useState(() => meterView(null));
  useEffect(() => {
    const transport = createMeterTransport(invoke, (packet: any) => setDisplay(meterView(packet)));
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
  return <section className="panel meter" aria-label="Master output meter">
    <div className="section-heading"><h2><svg {...iconProps('level')} />Master output</h2>
      <span>{display.valid ? display.complete ? 'LIVE' : 'HISTORY GAP' : 'WAITING FOR AUDIO'}</span></div>
    {display.channels.map((channel, index) => <div className="meter-row" key={channel.name}>
      <strong>{channel.name}</strong>
      <div className="meter-track" role="meter" aria-label={`${channel.name} peak level`}
        aria-valuemin={0} aria-valuemax={1} aria-valuenow={channel.peak}>
        <i className="meter-peak" style={{ width: `${channel.peak * 100}%` }} />
        <i className="meter-rms" style={{ width: `${channel.rms * 100}%` }} />
      </div>
      <button className={channel.clipped ? 'clip latched' : 'clip'}
        disabled={!channel.clipped || channel.ticket === '0'}
        onClick={() => void invoke('ackMeterClip', index, generation, channel.ticket)
          .then(() => setDisplay(current => ({ ...current, channels: current.channels.map(
            (item, position) => position === index ? { ...item, clipped: false } : item) })))
          .catch(reportError)}>CLIP</button>
    </div>)}
  </section>;
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
    const transport = createWaveformTransport(invoke, (packet: { status: WaveStatus }) =>
      setStatus(packet.status));
    void transport.select({ sourceId, regionId, channel: 0, buckets: 512,
      generation: document.generation }).catch(reportError);
    const timer = window.setInterval(() => void transport.poll().catch(reportError), 1000 / 30);
    return () => { window.clearInterval(timer); void transport.close().catch(reportError); };
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
  useEffect(() => {
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
    let transport: ReturnType<typeof createSpectrumTransport> | null = null;
    const visible = () => {
      if (window.document.hidden) {
        void transport?.close(); transport = null; setStatus(null);
      } else if (!transport) {
        transport = createSpectrumTransport(invoke, (packet: { status: WaveStatus }) => setStatus(packet.status));
        void transport.select({ sourceId, regionId, channel: 0, generation: document.generation }).catch(reportError);
      }
    };
    setStatus(null); visible();
    const timer = window.setInterval(() => void transport?.poll().catch(reportError), 1000 / 30);
    window.document.addEventListener('visibilitychange', visible);
    return () => {
      window.clearInterval(timer); window.document.removeEventListener('visibilitychange', visible);
      void transport?.close(); transport = null;
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

function SamplerApp() {
  const { state, document, error, command, reportError } = useHost();
  const pads = useMemo(() => preparedPads(document), [document]);
  const [sampleDisplay, setSampleDisplay] = useState<'wave' | 'spectral'>('wave');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = pads.find(pad => pad.id === selectedId) ?? pads[0] ?? null;
  const region = selectedRegion(document, selected);
  const parameters = visibleParameters(document, selected) as PreparedParameter[];
  const audition = useMemo(() => createNoteAudition(invoke, reportError), []);
  const [pressed, setPressed] = useState<string[]>([]);
  useEffect(() => {
    void audition.releaseAll();
    setPressed([]);
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

  const press = (pad: typeof pads[number]) => {
    if (!document) return;
    setSelectedId(pad.id);
    setPressed(current => current.includes(pad.id) ? current : [...current, pad.id]);
    void audition.press(pad.keyLow, pad.velocity, document.generation);
  };
  const release = (pad: typeof pads[number]) => {
    setPressed(current => current.filter(id => id !== pad.id));
    void audition.release(pad.keyLow);
  };
  const current = state && document && state.generation === document.generation;
  const displayControls = <div className="sample-display-controls" role="group" aria-label="Sample display">
    <button data-sample-display="wave" aria-pressed={sampleDisplay === 'wave'} onClick={() => setSampleDisplay('wave')}>Wave</button>
    <button data-sample-display="spectral" aria-pressed={sampleDisplay === 'spectral'} onClick={() => setSampleDisplay('spectral')}>Spectral</button>
  </div>;
  return <main className="sampler">
    <header className="top"><div><p className="eyebrow">DANDRUM · PREPARED INSTRUMENT</p>
      <h1>Drum Sampler</h1><p>{current ? document.instrumentId : 'Loading prepared instrument…'}</p></div>
      <span className="generation">{current ? `GEN ${document.generation}` : 'WAITING'}</span></header>
    {error && <p className="error" role="alert"><svg {...iconProps('error')} />{error}</p>}
    {current && <>
      <section className="panel keymap" aria-label="Prepared key map">
        <div className="section-heading"><h2><svg {...iconProps('keyboard')} />Key map</h2><span>PREPARED · READ ONLY</span></div>
        <div className="pads">{pads.map(pad => <button key={pad.id}
          className={`pad ${selected?.id === pad.id ? 'selected' : ''} ${pressed.includes(pad.id) ? 'pressed' : ''}`}
          aria-label={`${pad.label}, MIDI ${pad.keyLow}, velocity ${pad.velocityLow} to ${pad.velocityHigh}`}
          onPointerDown={event => { event.currentTarget.setPointerCapture(event.pointerId); press(pad); }}
          {...padReleaseHandlers(() => release(pad))}
          onClick={() => setSelectedId(pad.id)}
          onKeyDown={event => { if (!event.repeat && (event.key === ' ' || event.key === 'Enter')) {
            event.preventDefault(); press(pad); } }}>
          <strong>{pad.label}</strong>
          <small>NOTE {pad.keyLow}{pad.keyHigh !== pad.keyLow ? `–${pad.keyHigh}` : ''}
            {' · '}VEL {pad.velocityLow}–{pad.velocityHigh}</small>
          {pad.zoneIds.length > 1 && <small><svg {...iconProps('alternate', { size: 12 })} />{pad.zoneIds.length} round robin alternatives</small>}
          {pad.chokeGroup && <small><svg {...iconProps('choke', { size: 12 })} />CHOKE {pad.chokeGroup}</small>}
        </button>)}</div>
        {selected && <p className="details">{selected.selectionMode} selection · {selected.zoneIds.join(', ')}
          {selected.controlGroup != null ? ` · control group ${selected.controlGroup}` : ''}</p>}
      </section>
      <div className="lower">
        <div className="left-column">
          <section className="panel controls" aria-label="Host modulatable controls">
            <div className="section-heading"><h2><svg {...iconProps('host')} />Public controls</h2>
              <span>{parameters.length} SHARED / SELECTED PAD</span></div>
            <div className="control-list">{parameters.map(descriptor => <HostKnob
              key={`${descriptor.id}:${state.generation}`} label={descriptor.name || descriptor.id}
              parameter={admittedParameter(state, document, descriptor.id)}
              command={command} onError={reportError} />)}</div>
          </section>
          <MasterMeter generation={document.generation} reportError={reportError} />
        </div>
        <div className="right-column">
          {region && document.capabilities.preparedWaveform
            ? <div className="sample-display">
                {sampleDisplay === 'wave'
                  ? <PreparedWaveform document={document} sourceId={region.sourceId} regionId={region.regionId} reportError={reportError} controls={displayControls} />
                  : <PreparedSpectrum document={document} sourceId={region.sourceId} regionId={region.regionId} reportError={reportError} controls={displayControls} />}
              </div>
            : <section className="panel">Prepared sample analysis unavailable</section>}
          <section className="panel availability" aria-label="Prepared capabilities">
            <div className="section-heading"><h2><svg {...iconProps('lock')} />Structure</h2><span>INSPECT ONLY</span></div>
            <p>Source and region assignments are prepared outside the plugin.</p>
            <p>Layer details: {document.capabilities.synthLayer || document.capabilities.nestedPatchLayer
              || document.capabilities.moduleChain ? 'Available where prepared' : 'Unavailable'}</p>
            <p>Output bus details unavailable.</p>
          </section>
        </div>
      </div>
    </>}
  </main>;
}

createRoot(document.getElementById('root')!).render(<SamplerApp />);
